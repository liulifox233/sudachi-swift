uniffi::setup_scaffolding!();

use std::sync::Arc;
use sudachi::analysis::Mode;
use sudachi::analysis::stateless_tokenizer::StatelessTokenizer;
use sudachi::analysis::Tokenize;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::config::Config;

#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum WrappedSudachiError {
    #[error("Sudachi Error: {0}")]
    Error(String),
}

impl From<sudachi::error::SudachiError> for WrappedSudachiError {
    fn from(err: sudachi::error::SudachiError) -> Self {
        WrappedSudachiError::Error(err.to_string())
    }
}

impl From<sudachi::config::ConfigError> for WrappedSudachiError {
    fn from(err: sudachi::config::ConfigError) -> Self {
        WrappedSudachiError::Error(err.to_string())
    }
}

#[derive(uniffi::Record)]
pub struct WrappedMorpheme {
    pub surface: String,
    pub pos: Vec<String>,
    pub normalized_form: String,
    pub dictionary_form: String,
    pub reading_form: String,
}

#[derive(uniffi::Enum)]
pub enum WrappedMode {
    A,
    B,
    C,
}

#[derive(uniffi::Object)]
pub struct WrappedDictionary {
    dict: Arc<JapaneseDictionary>,
}

#[uniffi::export]
impl WrappedDictionary {
    #[uniffi::constructor]
    pub fn new(config_path: Option<String>, resource_dir: Option<String>) -> Result<Arc<Self>, WrappedSudachiError> {
        let mut config = Config::new(
            config_path.map(|p| p.into()),
            resource_dir.map(|p| p.into()),
            None
        )?;
        
        // Ensure system dictionary is set if not in config
        if config.system_dict.is_none() {
            config.system_dict = Some("system.dic".into());
        }

        let dict = JapaneseDictionary::from_cfg(&config)?;
        Ok(Arc::new(Self { dict: Arc::new(dict) }))
    }

    pub fn tokenize(&self, text: &str, mode: WrappedMode) -> Vec<WrappedMorpheme> {
        let tokenizer = StatelessTokenizer::new(self.dict.clone());
        let mode = match mode {
            WrappedMode::A => Mode::A,
            WrappedMode::B => Mode::B,
            WrappedMode::C => Mode::C,
        };
        
        let morphemes = match tokenizer.tokenize(text, mode, false) {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };
        
        morphemes.iter().map(|m| {
            WrappedMorpheme {
                surface: m.surface().to_string(),
                pos: m.part_of_speech().iter().map(|s| s.to_string()).collect(),
                normalized_form: m.normalized_form().to_string(),
                dictionary_form: m.dictionary_form().to_string(),
                reading_form: m.reading_form().to_string(),
            }
        }).collect()
    }
}
