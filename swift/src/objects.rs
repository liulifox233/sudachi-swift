use std::sync::Arc;

use crate::error::SudachiError;
use crate::state::{
    build_dictionary_state, build_lookup_snapshot, build_pos_matcher, build_tokenized_snapshot,
    ensure_same_dictionary, DictionaryState, MorphemeListSnapshot, MorphemeNodeSnapshot,
};
use crate::types::{
    subset_from_fields, SudachiInfoField, SudachiPartOfSpeech, SudachiPartialPartOfSpeech,
    SudachiProjection, SudachiSplitMode, SudachiWordInfo,
};

#[derive(uniffi::Object)]
pub struct SudachiDictionary {
    state: Arc<DictionaryState>,
}

#[derive(uniffi::Object)]
pub struct SudachiTokenizer {
    dictionary: Arc<DictionaryState>,
    mode: SudachiSplitMode,
    subset: sudachi::dic::subset::InfoSubset,
    projection: SudachiProjection,
}

#[derive(uniffi::Object)]
pub struct SudachiMorphemeList {
    snapshot: Arc<MorphemeListSnapshot>,
}

#[derive(uniffi::Object)]
pub struct SudachiMorpheme {
    snapshot: Arc<MorphemeListSnapshot>,
    index: usize,
}

#[derive(uniffi::Object)]
pub struct SudachiPosMatcher {
    dictionary: Arc<DictionaryState>,
    matcher: sudachi::pos::PosMatcher,
}

#[uniffi::export]
impl SudachiDictionary {
    #[uniffi::constructor]
    pub fn new(
        system_dict_path: String,
        config_path: Option<String>,
        resource_dir: Option<String>,
    ) -> Result<Arc<Self>, SudachiError> {
        Ok(Arc::new(Self {
            state: build_dictionary_state(system_dict_path, config_path, resource_dir)?,
        }))
    }

    pub fn create_tokenizer(
        &self,
        mode: Option<SudachiSplitMode>,
        fields: Option<Vec<SudachiInfoField>>,
        projection: Option<SudachiProjection>,
    ) -> Arc<SudachiTokenizer> {
        let mode = mode.unwrap_or(SudachiSplitMode::C);
        let projection = projection.unwrap_or(self.state.default_projection);
        let subset = subset_from_fields(fields, projection);

        Arc::new(SudachiTokenizer {
            dictionary: self.state.clone(),
            mode,
            subset,
            projection,
        })
    }

    pub fn lookup(&self, surface: &str) -> Result<Arc<SudachiMorphemeList>, SudachiError> {
        Ok(Arc::new(SudachiMorphemeList {
            snapshot: build_lookup_snapshot(
                self.state.clone(),
                surface,
                self.state.default_projection,
            )?,
        }))
    }

    pub fn part_of_speech(&self, pos_id: u16) -> Option<SudachiPartOfSpeech> {
        self.state.part_of_speech_for_id(pos_id)
    }

    pub fn make_pos_matcher(
        &self,
        patterns: Vec<SudachiPartialPartOfSpeech>,
    ) -> Result<Arc<SudachiPosMatcher>, SudachiError> {
        Ok(Arc::new(SudachiPosMatcher {
            dictionary: self.state.clone(),
            matcher: build_pos_matcher(&self.state, &patterns)?,
        }))
    }
}

#[uniffi::export]
impl SudachiTokenizer {
    pub fn mode(&self) -> SudachiSplitMode {
        self.mode
    }

    pub fn tokenize(&self, text: &str) -> Result<Arc<SudachiMorphemeList>, SudachiError> {
        Ok(Arc::new(SudachiMorphemeList {
            snapshot: build_tokenized_snapshot(
                self.dictionary.clone(),
                text,
                self.mode,
                self.subset,
                self.projection,
            )?,
        }))
    }
}

#[uniffi::export]
impl SudachiMorphemeList {
    pub fn count(&self) -> u64 {
        self.snapshot.nodes.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.snapshot.nodes.is_empty()
    }

    pub fn internal_cost(&self) -> i32 {
        if self.snapshot.nodes.is_empty() {
            return 0;
        }

        let first = self.snapshot.nodes.first().unwrap().total_cost;
        let last = self.snapshot.nodes.last().unwrap().total_cost;
        last - first
    }

    pub fn morpheme_at(&self, index: u64) -> Result<Arc<SudachiMorpheme>, SudachiError> {
        let index =
            usize::try_from(index).map_err(|_| SudachiError::invalid_argument("index overflow"))?;
        if index >= self.snapshot.nodes.len() {
            return Err(SudachiError::invalid_argument(format!(
                "Morpheme index out of range: len is {} but index was {}",
                self.snapshot.nodes.len(),
                index
            )));
        }

        Ok(Arc::new(SudachiMorpheme {
            snapshot: self.snapshot.clone(),
            index,
        }))
    }

    pub fn morphemes(&self) -> Vec<Arc<SudachiMorpheme>> {
        (0..self.snapshot.nodes.len())
            .map(|index| {
                Arc::new(SudachiMorpheme {
                    snapshot: self.snapshot.clone(),
                    index,
                })
            })
            .collect()
    }
}

#[uniffi::export]
impl SudachiMorpheme {
    pub fn begin(&self) -> u64 {
        self.node().original_begin_char as u64
    }

    pub fn end(&self) -> u64 {
        self.node().original_end_char as u64
    }

    pub fn surface(&self) -> String {
        self.snapshot
            .dictionary
            .project(self.snapshot.projection, self.node())
    }

    pub fn raw_surface(&self) -> String {
        self.node().raw_surface.clone()
    }

    pub fn part_of_speech(&self) -> SudachiPartOfSpeech {
        self.snapshot
            .dictionary
            .part_of_speech_for_id(self.node().word_info.pos_id())
            .expect("POS ID should always exist in the originating dictionary")
    }

    pub fn part_of_speech_id(&self) -> u16 {
        self.node().word_info.pos_id()
    }

    pub fn dictionary_form(&self) -> String {
        self.node().word_info.dictionary_form().to_owned()
    }

    pub fn normalized_form(&self) -> String {
        self.node().word_info.normalized_form().to_owned()
    }

    pub fn reading_form(&self) -> String {
        self.node().word_info.reading_form().to_owned()
    }

    pub fn split(
        &self,
        mode: SudachiSplitMode,
        add_single: Option<bool>,
    ) -> Result<Arc<SudachiMorphemeList>, SudachiError> {
        let add_single = add_single.unwrap_or(true);
        let mut nodes = self.snapshot.split_node(self.index, mode)?;

        if nodes.is_empty() && add_single {
            nodes.push(self.node().clone());
        }

        Ok(Arc::new(SudachiMorphemeList {
            snapshot: Arc::new(self.snapshot.with_nodes(nodes)),
        }))
    }

    pub fn is_oov(&self) -> bool {
        self.node().word_id.is_oov()
    }

    pub fn word_id(&self) -> u32 {
        self.node().word_id.as_raw()
    }

    pub fn dictionary_id(&self) -> i32 {
        if self.node().word_id.is_oov() {
            -1
        } else {
            self.node().word_id.dic() as i32
        }
    }

    pub fn synonym_group_ids(&self) -> Vec<u32> {
        self.node().word_info.synonym_group_ids().to_vec()
    }

    pub fn word_info(&self) -> SudachiWordInfo {
        self.node().word_info.clone().into()
    }

    pub fn length(&self) -> u64 {
        self.end() - self.begin()
    }
}

#[uniffi::export]
impl SudachiPosMatcher {
    pub fn matches(&self, morpheme: &SudachiMorpheme) -> Result<bool, SudachiError> {
        ensure_same_dictionary(&self.dictionary, &morpheme.snapshot.dictionary)?;
        Ok(self.matcher.matches_id(morpheme.part_of_speech_id()))
    }

    pub fn count(&self) -> u64 {
        self.matcher.num_entries() as u64
    }

    pub fn entries(&self) -> Vec<SudachiPartOfSpeech> {
        self.dictionary
            .part_of_speech
            .iter()
            .enumerate()
            .filter_map(|(idx, pos)| {
                if self.matcher.matches_id(idx as u16) {
                    Some(pos.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn union(&self, other: &SudachiPosMatcher) -> Result<Arc<SudachiPosMatcher>, SudachiError> {
        ensure_same_dictionary(&self.dictionary, &other.dictionary)?;
        Ok(Arc::new(SudachiPosMatcher {
            dictionary: self.dictionary.clone(),
            matcher: self.matcher.union(&other.matcher),
        }))
    }

    pub fn intersection(
        &self,
        other: &SudachiPosMatcher,
    ) -> Result<Arc<SudachiPosMatcher>, SudachiError> {
        ensure_same_dictionary(&self.dictionary, &other.dictionary)?;
        Ok(Arc::new(SudachiPosMatcher {
            dictionary: self.dictionary.clone(),
            matcher: self.matcher.intersection(&other.matcher),
        }))
    }

    pub fn subtract(
        &self,
        other: &SudachiPosMatcher,
    ) -> Result<Arc<SudachiPosMatcher>, SudachiError> {
        ensure_same_dictionary(&self.dictionary, &other.dictionary)?;
        Ok(Arc::new(SudachiPosMatcher {
            dictionary: self.dictionary.clone(),
            matcher: self.matcher.difference(&other.matcher),
        }))
    }

    pub fn inverted(&self) -> Arc<SudachiPosMatcher> {
        let values = (0..self.dictionary.part_of_speech.len())
            .map(|idx| idx as u16)
            .filter(|id| !self.matcher.matches_id(*id));

        Arc::new(SudachiPosMatcher {
            dictionary: self.dictionary.clone(),
            matcher: sudachi::pos::PosMatcher::new(values),
        })
    }
}

impl SudachiMorpheme {
    fn node(&self) -> &MorphemeNodeSnapshot {
        &self.snapshot.nodes[self.index]
    }
}
