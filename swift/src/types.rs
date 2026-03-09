use sudachi::config::SurfaceProjection;
use sudachi::dic::lexicon::word_infos::{WordInfo, WordInfoData};
use sudachi::dic::subset::InfoSubset;
use sudachi::dic::word_id::WordId;

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SudachiSplitMode {
    A,
    B,
    C,
}

impl From<SudachiSplitMode> for sudachi::analysis::Mode {
    fn from(mode: SudachiSplitMode) -> Self {
        match mode {
            SudachiSplitMode::A => sudachi::analysis::Mode::A,
            SudachiSplitMode::B => sudachi::analysis::Mode::B,
            SudachiSplitMode::C => sudachi::analysis::Mode::C,
        }
    }
}

impl From<sudachi::analysis::Mode> for SudachiSplitMode {
    fn from(mode: sudachi::analysis::Mode) -> Self {
        match mode {
            sudachi::analysis::Mode::A => SudachiSplitMode::A,
            sudachi::analysis::Mode::B => SudachiSplitMode::B,
            sudachi::analysis::Mode::C => SudachiSplitMode::C,
        }
    }
}

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SudachiProjection {
    Surface,
    Normalized,
    Reading,
    Dictionary,
    DictionaryAndSurface,
    NormalizedAndSurface,
    NormalizedNouns,
}

impl From<SudachiProjection> for SurfaceProjection {
    fn from(value: SudachiProjection) -> Self {
        match value {
            SudachiProjection::Surface => SurfaceProjection::Surface,
            SudachiProjection::Normalized => SurfaceProjection::Normalized,
            SudachiProjection::Reading => SurfaceProjection::Reading,
            SudachiProjection::Dictionary => SurfaceProjection::Dictionary,
            SudachiProjection::DictionaryAndSurface => SurfaceProjection::DictionaryAndSurface,
            SudachiProjection::NormalizedAndSurface => SurfaceProjection::NormalizedAndSurface,
            SudachiProjection::NormalizedNouns => SurfaceProjection::NormalizedNouns,
        }
    }
}

impl From<SurfaceProjection> for SudachiProjection {
    fn from(value: SurfaceProjection) -> Self {
        match value {
            SurfaceProjection::Surface => SudachiProjection::Surface,
            SurfaceProjection::Normalized => SudachiProjection::Normalized,
            SurfaceProjection::Reading => SudachiProjection::Reading,
            SurfaceProjection::Dictionary => SudachiProjection::Dictionary,
            SurfaceProjection::DictionaryAndSurface => SudachiProjection::DictionaryAndSurface,
            SurfaceProjection::NormalizedAndSurface => SudachiProjection::NormalizedAndSurface,
            SurfaceProjection::NormalizedNouns => SudachiProjection::NormalizedNouns,
        }
    }
}

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SudachiInfoField {
    Surface,
    PartOfSpeech,
    NormalizedForm,
    DictionaryForm,
    ReadingForm,
    WordStructure,
    SplitA,
    SplitB,
    SynonymGroupId,
}

impl From<SudachiInfoField> for InfoSubset {
    fn from(value: SudachiInfoField) -> Self {
        match value {
            SudachiInfoField::Surface => InfoSubset::SURFACE,
            SudachiInfoField::PartOfSpeech => InfoSubset::POS_ID,
            SudachiInfoField::NormalizedForm => InfoSubset::NORMALIZED_FORM,
            SudachiInfoField::DictionaryForm => InfoSubset::DIC_FORM_WORD_ID,
            SudachiInfoField::ReadingForm => InfoSubset::READING_FORM,
            SudachiInfoField::WordStructure => InfoSubset::WORD_STRUCTURE,
            SudachiInfoField::SplitA => InfoSubset::SPLIT_A,
            SudachiInfoField::SplitB => InfoSubset::SPLIT_B,
            SudachiInfoField::SynonymGroupId => InfoSubset::SYNONYM_GROUP_ID,
        }
    }
}

#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct SudachiPartOfSpeech {
    pub level1: String,
    pub level2: String,
    pub level3: String,
    pub level4: String,
    pub conjugation_type: String,
    pub conjugation_form: String,
}

#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq, Default)]
pub struct SudachiPartialPartOfSpeech {
    #[uniffi(default = None)]
    pub level1: Option<String>,
    #[uniffi(default = None)]
    pub level2: Option<String>,
    #[uniffi(default = None)]
    pub level3: Option<String>,
    #[uniffi(default = None)]
    pub level4: Option<String>,
    #[uniffi(default = None)]
    pub conjugation_type: Option<String>,
    #[uniffi(default = None)]
    pub conjugation_form: Option<String>,
}

impl SudachiPartialPartOfSpeech {
    pub(crate) fn matches(&self, pos: &SudachiPartOfSpeech) -> bool {
        matches_partial(&self.level1, &pos.level1)
            && matches_partial(&self.level2, &pos.level2)
            && matches_partial(&self.level3, &pos.level3)
            && matches_partial(&self.level4, &pos.level4)
            && matches_partial(&self.conjugation_type, &pos.conjugation_type)
            && matches_partial(&self.conjugation_form, &pos.conjugation_form)
    }
}

#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct SudachiWordInfo {
    pub surface: String,
    pub head_word_length: u16,
    pub pos_id: u16,
    pub normalized_form: String,
    pub dictionary_form_word_id: i32,
    pub dictionary_form: String,
    pub reading_form: String,
    pub a_unit_split: Vec<u32>,
    pub b_unit_split: Vec<u32>,
    pub word_structure: Vec<u32>,
    pub synonym_group_ids: Vec<u32>,
}

impl From<WordInfo> for SudachiWordInfo {
    fn from(word_info: WordInfo) -> Self {
        let word_info: WordInfoData = word_info.into();
        Self {
            surface: word_info.surface.clone(),
            head_word_length: word_info.head_word_length,
            pos_id: word_info.pos_id,
            normalized_form: copy_if_empty(word_info.normalized_form, &word_info.surface),
            dictionary_form_word_id: word_info.dictionary_form_word_id,
            dictionary_form: copy_if_empty(word_info.dictionary_form, &word_info.surface),
            reading_form: copy_if_empty(word_info.reading_form, &word_info.surface),
            a_unit_split: raw_word_ids(word_info.a_unit_split.as_slice()),
            b_unit_split: raw_word_ids(word_info.b_unit_split.as_slice()),
            word_structure: raw_word_ids(word_info.word_structure.as_slice()),
            synonym_group_ids: word_info.synonym_group_ids,
        }
    }
}

pub(crate) fn subset_from_fields(
    fields: Option<Vec<SudachiInfoField>>,
    projection: SudachiProjection,
) -> InfoSubset {
    let base = fields
        .map(|fields| {
            fields.into_iter().fold(InfoSubset::empty(), |acc, field| {
                acc | InfoSubset::from(field)
            })
        })
        .unwrap_or_else(InfoSubset::all);

    let projection_subset = SurfaceProjection::from(projection).required_subset();
    (base
        | projection_subset
        | InfoSubset::SURFACE
        | InfoSubset::HEAD_WORD_LENGTH
        | InfoSubset::POS_ID)
        .normalize()
}

pub(crate) fn part_of_speech_from_components(pos: &[String]) -> SudachiPartOfSpeech {
    SudachiPartOfSpeech {
        level1: pos.first().cloned().unwrap_or_default(),
        level2: pos.get(1).cloned().unwrap_or_default(),
        level3: pos.get(2).cloned().unwrap_or_default(),
        level4: pos.get(3).cloned().unwrap_or_default(),
        conjugation_type: pos.get(4).cloned().unwrap_or_default(),
        conjugation_form: pos.get(5).cloned().unwrap_or_default(),
    }
}

fn matches_partial(pattern: &Option<String>, value: &str) -> bool {
    match pattern {
        None => true,
        Some(pattern) => pattern == value,
    }
}

fn raw_word_ids(ids: &[WordId]) -> Vec<u32> {
    ids.iter().map(|id| id.as_raw()).collect()
}

fn copy_if_empty(value: String, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value
    }
}
