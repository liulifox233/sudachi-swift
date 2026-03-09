uniffi::setup_scaffolding!();

mod error;
mod objects;
mod state;
mod types;

pub use error::SudachiError;
pub use objects::{
    SudachiDictionary, SudachiMorpheme, SudachiMorphemeList, SudachiPosMatcher, SudachiTokenizer,
};
pub use types::{
    SudachiInfoField, SudachiPartOfSpeech, SudachiPartialPartOfSpeech, SudachiProjection,
    SudachiSplitMode, SudachiWordInfo,
};
