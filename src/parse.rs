use crate::*;
use std::marker::*;
use wasmparser::*;

pub struct WassetParser<'a, A: AssetSchema> {
    manifest: WassetManifest,
    module: &'a [u8],
    marker: PhantomData<fn(A)>
}

impl<'a, A: AssetSchema> WassetParser<'a, A> {
    pub fn parse(module: &'a [u8]) -> Result<Self, WassetError> {
        let mut contents = module;
        let mut parser = Parser::new(0);
        let mut offsets = FxHashMap::default();

        loop {
            let payload = match parser.parse(contents, true).map_err(WassetError::from_deserialize)? {
                Chunk::Parsed { consumed, payload } => {
                    contents = &contents[consumed..];
                    payload
                }
                // this state isn't possible with `eof = true`
                Chunk::NeedMoreData(_) => unreachable!(),
            };

            match payload {
                Payload::CodeSectionStart { size, .. } => {
                    parser.skip_section();
                    contents = &contents[size as usize..];
                }
                Payload::CustomSection(c) => Self::parse_module_custom_section(c, &mut offsets)?,
                Payload::End(_) => break,
                _ => {}
            }
        }

        let manifest = Self::collect_manifests(offsets)?;
        Ok(Self {
            manifest,
            module,
            marker: PhantomData
        })
    }

    pub fn ids(&self) -> impl '_ + Iterator<Item = WassetId> {
        self.manifest.asset_ranges.keys().copied()
    }

    pub fn load(&self, id: WassetId) -> Result<Option<A>, WassetError> {
        if let Some(range) = self.manifest.asset_ranges.get(&id) {
            Ok(Some(self.load_by_range(range.clone())?))
        }
        else {
            Ok(None)
        }
    }

    pub fn manifest(&self) -> &WassetManifest {
        &self.manifest
    }

    fn load_by_range(&self, range: Range<u32>) -> Result<A, WassetError> {
        if let Some(slice) = self.module.get(range.start as usize..range.end as usize) {
            Ok(rmp_serde::from_slice(slice).map_err(WassetError::from_deserialize)?)
        }
        else {
            todo!("Wasset error: index out of range")
        }
    }

    fn collect_manifests(offsets: FxHashMap<Uuid, WassetOffsets>) -> Result<WassetManifest, WassetError> {
        let mut manifest = WassetManifest::default();
        for manifest_offset in offsets.into_values() {
            let manifest_instance = rmp_serde::from_slice::<WassetManifest>(manifest_offset.manifest).map_err(WassetError::from_deserialize)?;
            for (id, range) in manifest_instance.asset_ranges {
                manifest.asset_ranges.insert(id, range.start + manifest_offset.data_offset..range.end + manifest_offset.data_offset);
            }
        }
        Ok(manifest)
    }

    fn parse_module_custom_section(reader: CustomSectionReader<'a>, offsets: &mut FxHashMap<Uuid, WassetOffsets<'a>>) -> Result<(), WassetError> {
        const ASSET_MANIFEST_SECTION_PREFIX: &str = "__wasset_manifest:";
        const ASSET_DATA_SECTION_PREFIX: &str = "__wasset_data:";

        if reader.name().starts_with(ASSET_MANIFEST_SECTION_PREFIX) {
            let id = Uuid::try_parse(&reader.name()[ASSET_MANIFEST_SECTION_PREFIX.len()..])
                .map_err(WassetError::from_deserialize)?;
            offsets.entry(id).or_default().manifest = reader.data();
        }
        else if reader.name().starts_with(ASSET_DATA_SECTION_PREFIX) {
            let id = Uuid::try_parse(&reader.name()[ASSET_DATA_SECTION_PREFIX.len()..])
                .map_err(WassetError::from_deserialize)?;
                offsets.entry(id).or_default().data_offset = reader.data_offset() as u32;
        }

        Ok(())
    }
}

impl<'a, A: AssetSchema> IntoIterator for &'a WassetParser<'a, A> {
    type Item = (WassetId, Result<A, WassetError>);
    type IntoIter = WassetIter<'a, A>;

    fn into_iter(self) -> Self::IntoIter {
        WassetIter {
            iter: self.manifest.asset_ranges.iter(),
            parser: self
        }
    }
}

pub struct WassetIter<'a, A: AssetSchema> {
    iter: std::collections::hash_map::Iter<'a, WassetId, Range<u32>>,
    parser: &'a WassetParser<'a, A>
}

impl<'a, A: AssetSchema> Iterator for WassetIter<'a, A> {
    type Item = (WassetId, Result<A, WassetError>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(id, range)| (*id, self.parser.load_by_range(range.clone())))
    }
}

#[derive(Copy, Clone, Debug, Default)]
struct WassetOffsets<'a> {
    data_offset: u32,
    manifest: &'a [u8],
}