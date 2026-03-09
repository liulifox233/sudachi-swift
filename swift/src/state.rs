use std::sync::Arc;

use sudachi::analysis::node::{LatticeNode, PathCost, ResultNode};
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::lexicon::word_infos::WordInfo;
use sudachi::dic::subset::InfoSubset;
use sudachi::dic::word_id::WordId;
use sudachi::input_text::InputBuffer;
use sudachi::pos::PosMatcher;

use crate::error::SudachiError;
use crate::types::{
    part_of_speech_from_components, SudachiPartOfSpeech, SudachiPartialPartOfSpeech,
    SudachiProjection, SudachiSplitMode,
};

pub(crate) struct DictionaryState {
    pub(crate) dict: Arc<JapaneseDictionary>,
    pub(crate) default_projection: SudachiProjection,
    pub(crate) part_of_speech: Vec<SudachiPartOfSpeech>,
    conjugating_matcher: PosMatcher,
    normalized_nouns_matcher: PosMatcher,
}

pub(crate) struct MorphemeListSnapshot {
    pub(crate) dictionary: Arc<DictionaryState>,
    pub(crate) input: Arc<InputBuffer>,
    pub(crate) nodes: Vec<MorphemeNodeSnapshot>,
    pub(crate) subset: InfoSubset,
    pub(crate) projection: SudachiProjection,
}

#[derive(Clone)]
pub(crate) struct MorphemeNodeSnapshot {
    pub(crate) raw_surface: String,
    pub(crate) word_info: WordInfo,
    pub(crate) word_id: WordId,
    pub(crate) total_cost: i32,
    pub(crate) original_begin_char: usize,
    pub(crate) original_end_char: usize,
    pub(crate) modified_begin_char: usize,
    pub(crate) modified_end_char: usize,
    pub(crate) modified_begin_byte: usize,
    pub(crate) modified_end_byte: usize,
}

pub(crate) fn build_dictionary_state(
    system_dict_path: String,
    config_path: Option<String>,
    resource_dir: Option<String>,
) -> Result<Arc<DictionaryState>, SudachiError> {
    let config = Config::new(
        config_path.map(Into::into),
        resource_dir.map(Into::into),
        Some(system_dict_path.into()),
    )?;
    let default_projection = config.projection.into();
    let dict = JapaneseDictionary::from_cfg(&config).map_err(SudachiError::dictionary)?;
    Ok(Arc::new(DictionaryState::new(dict, default_projection)))
}

pub(crate) fn build_tokenized_snapshot(
    dictionary: Arc<DictionaryState>,
    text: &str,
    mode: SudachiSplitMode,
    subset: InfoSubset,
    projection: SudachiProjection,
) -> Result<Arc<MorphemeListSnapshot>, SudachiError> {
    let mut tokenizer = StatefulTokenizer::new(dictionary.dict.clone(), mode.into());
    tokenizer.set_subset(subset);
    tokenizer.reset().push_str(text);
    tokenizer
        .do_tokenize()
        .map_err(SudachiError::tokenization)?;

    let mut input = InputBuffer::new();
    let mut result_nodes = Vec::new();
    let mut actual_subset = InfoSubset::empty();
    tokenizer.swap_result(&mut input, &mut result_nodes, &mut actual_subset);

    let nodes = result_nodes
        .iter()
        .map(|node| MorphemeNodeSnapshot::from_result_node(&input, node))
        .collect();

    Ok(Arc::new(MorphemeListSnapshot {
        dictionary,
        input: Arc::new(input),
        nodes,
        subset: actual_subset,
        projection,
    }))
}

pub(crate) fn build_lookup_snapshot(
    dictionary: Arc<DictionaryState>,
    surface: &str,
    projection: SudachiProjection,
) -> Result<Arc<MorphemeListSnapshot>, SudachiError> {
    let mut input = InputBuffer::new();
    input.reset().push_str(surface);
    input.start_build().map_err(SudachiError::tokenization)?;
    input
        .build(dictionary.dict.grammar())
        .map_err(SudachiError::tokenization)?;

    let end_chars = input.ch_idx(surface.len());
    let subset = InfoSubset::all();
    let nodes = dictionary
        .dict
        .lexicon()
        .lookup(surface.as_bytes(), 0)
        .filter(|entry| entry.end == surface.len())
        .map(|entry| {
            let word_info = dictionary
                .dict
                .lexicon()
                .get_word_info_subset(entry.word_id, subset)
                .map_err(SudachiError::dictionary)?;
            Ok(MorphemeNodeSnapshot::new(
                &input,
                word_info,
                entry.word_id,
                0,
                0,
                end_chars,
                0,
                surface.len(),
            ))
        })
        .collect::<Result<Vec<_>, SudachiError>>()?;

    Ok(Arc::new(MorphemeListSnapshot {
        dictionary,
        input: Arc::new(input),
        nodes,
        subset,
        projection,
    }))
}

pub(crate) fn build_pos_matcher(
    dictionary: &Arc<DictionaryState>,
    patterns: &[SudachiPartialPartOfSpeech],
) -> Result<PosMatcher, SudachiError> {
    let mut ids = Vec::new();

    for pattern in patterns {
        let start_len = ids.len();
        for (idx, pos) in dictionary.part_of_speech.iter().enumerate() {
            if pattern.matches(pos) {
                ids.push(idx as u16);
            }
        }

        if ids.len() == start_len {
            return Err(SudachiError::invalid_argument(format!(
                "POS pattern {:?} did not match any elements",
                pattern
            )));
        }
    }

    Ok(PosMatcher::new(ids))
}

pub(crate) fn ensure_same_dictionary(
    left: &Arc<DictionaryState>,
    right: &Arc<DictionaryState>,
) -> Result<(), SudachiError> {
    if Arc::ptr_eq(left, right) {
        Ok(())
    } else {
        Err(SudachiError::invalid_argument(
            "objects were created from different dictionaries",
        ))
    }
}

impl DictionaryState {
    fn new(dict: JapaneseDictionary, default_projection: SudachiProjection) -> Self {
        let part_of_speech = dict
            .grammar()
            .pos_list
            .iter()
            .map(|pos| part_of_speech_from_components(pos))
            .collect();

        let conjugating_matcher = build_precomputed_matcher(&dict, |pos| {
            matches!(pos[0].as_str(), "動詞" | "形容詞" | "助動詞")
        });
        let normalized_nouns_matcher =
            build_precomputed_matcher(&dict, |pos| pos.get(5).map(|x| x.as_str()) == Some("*"));

        Self {
            dict: Arc::new(dict),
            default_projection,
            part_of_speech,
            conjugating_matcher,
            normalized_nouns_matcher,
        }
    }

    pub(crate) fn part_of_speech_for_id(&self, pos_id: u16) -> Option<SudachiPartOfSpeech> {
        self.part_of_speech.get(pos_id as usize).cloned()
    }

    pub(crate) fn project(
        &self,
        projection: SudachiProjection,
        node: &MorphemeNodeSnapshot,
    ) -> String {
        match projection {
            SudachiProjection::Surface => node.raw_surface.clone(),
            SudachiProjection::Normalized => node.word_info.normalized_form().to_owned(),
            SudachiProjection::Reading => node.word_info.reading_form().to_owned(),
            SudachiProjection::Dictionary => node.word_info.dictionary_form().to_owned(),
            SudachiProjection::DictionaryAndSurface => {
                if self.conjugating_matcher.matches_id(node.word_info.pos_id()) {
                    node.raw_surface.clone()
                } else {
                    node.word_info.dictionary_form().to_owned()
                }
            }
            SudachiProjection::NormalizedAndSurface => {
                if self.conjugating_matcher.matches_id(node.word_info.pos_id()) {
                    node.raw_surface.clone()
                } else {
                    node.word_info.normalized_form().to_owned()
                }
            }
            SudachiProjection::NormalizedNouns => {
                if self
                    .normalized_nouns_matcher
                    .matches_id(node.word_info.pos_id())
                {
                    node.word_info.normalized_form().to_owned()
                } else {
                    node.raw_surface.clone()
                }
            }
        }
    }
}

impl MorphemeListSnapshot {
    pub(crate) fn with_nodes(&self, nodes: Vec<MorphemeNodeSnapshot>) -> Self {
        Self {
            dictionary: self.dictionary.clone(),
            input: self.input.clone(),
            nodes,
            subset: self.subset,
            projection: self.projection,
        }
    }

    pub(crate) fn split_node(
        &self,
        index: usize,
        mode: SudachiSplitMode,
    ) -> Result<Vec<MorphemeNodeSnapshot>, SudachiError> {
        let node = self
            .nodes
            .get(index)
            .ok_or_else(|| SudachiError::invalid_argument("morpheme index was out of range"))?;
        let mode: sudachi::analysis::Mode = mode.into();

        let split_ids: &[WordId] = match mode {
            sudachi::analysis::Mode::A => {
                ensure_split_subset(self.subset, InfoSubset::SPLIT_A, "splitA")?;
                node.word_info.a_unit_split()
            }
            sudachi::analysis::Mode::B => {
                ensure_split_subset(self.subset, InfoSubset::SPLIT_B, "splitB")?;
                node.word_info.b_unit_split()
            }
            sudachi::analysis::Mode::C => return Ok(Vec::new()),
        };

        if split_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut byte_offset = node.modified_begin_byte;
        let mut char_offset = node.modified_begin_char;
        let mut result = Vec::with_capacity(split_ids.len());

        for (idx, word_id) in split_ids.iter().copied().enumerate() {
            let word_info = self
                .dictionary
                .dict
                .lexicon()
                .get_word_info_subset(word_id, self.subset)
                .map_err(SudachiError::dictionary)?;

            let (char_end, byte_end) = if idx + 1 == split_ids.len() {
                (node.modified_end_char, node.modified_end_byte)
            } else {
                let byte_end = byte_offset + word_info.head_word_length();
                let char_end = self.input.ch_idx(byte_end);
                (char_end, byte_end)
            };

            result.push(MorphemeNodeSnapshot::new(
                self.input.as_ref(),
                word_info,
                word_id,
                i32::MAX,
                char_offset,
                char_end,
                byte_offset,
                byte_end,
            ));

            char_offset = char_end;
            byte_offset = byte_end;
        }

        Ok(result)
    }
}

impl MorphemeNodeSnapshot {
    fn new(
        input: &InputBuffer,
        word_info: WordInfo,
        word_id: WordId,
        total_cost: i32,
        modified_begin_char: usize,
        modified_end_char: usize,
        modified_begin_byte: usize,
        modified_end_byte: usize,
    ) -> Self {
        let original_begin_char = input.to_orig_char_idx(modified_begin_char);
        let original_end_char = input.to_orig_char_idx(modified_end_char);
        let original_begin_byte = input.to_orig_byte_idx(modified_begin_char);
        let original_end_byte = input.to_orig_byte_idx(modified_end_char);
        let raw_surface = input.original()[original_begin_byte..original_end_byte].to_owned();

        Self {
            raw_surface,
            word_info,
            word_id,
            total_cost,
            original_begin_char,
            original_end_char,
            modified_begin_char,
            modified_end_char,
            modified_begin_byte,
            modified_end_byte,
        }
    }

    fn from_result_node(input: &InputBuffer, node: &ResultNode) -> Self {
        Self::new(
            input,
            node.word_info().clone(),
            node.word_id(),
            node.total_cost(),
            node.begin(),
            node.end(),
            node.begin_bytes(),
            node.end_bytes(),
        )
    }
}

fn build_precomputed_matcher<F>(dictionary: &JapaneseDictionary, mut predicate: F) -> PosMatcher
where
    F: FnMut(&Vec<String>) -> bool,
{
    let ids = dictionary
        .grammar()
        .pos_list
        .iter()
        .enumerate()
        .filter_map(|(idx, pos)| {
            if predicate(pos) {
                Some(idx as u16)
            } else {
                None
            }
        });
    PosMatcher::new(ids)
}

fn ensure_split_subset(
    subset: InfoSubset,
    required: InfoSubset,
    field_name: &str,
) -> Result<(), SudachiError> {
    if subset.contains(required) {
        Ok(())
    } else {
        Err(SudachiError::invalid_argument(format!(
            "split requires {field_name} to be loaded in tokenizer fields"
        )))
    }
}
