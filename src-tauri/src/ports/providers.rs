use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldProposal {
    pub field_id: String,
    pub proposed_value: String,
    pub schema_version: String,
}

pub trait LocalAiProvider: Send + Sync {
    fn propose_fields(&self, local_text: &str) -> Result<Vec<FieldProposal>, AppError>;
}

pub trait SpeechToTextProvider: Send + Sync {
    fn transcribe_local_audio(&self, audio: &[u8], language: &str) -> Result<String, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryValidationResult {
    pub profile_version: String,
    pub ready: bool,
    pub issue_codes: Vec<String>,
}

pub trait RegistryAdapter: Send + Sync {
    fn validate(
        &self,
        canonical_revision_id: &str,
        profile_version: &str,
    ) -> Result<RegistryValidationResult, AppError>;
    fn render(
        &self,
        canonical_revision_id: &str,
        profile_version: &str,
    ) -> Result<Vec<u8>, AppError>;
}

pub trait BarcodeInput: Send + Sync {
    fn read_token(&self) -> Result<String, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBarcode {
    pub symbology: String,
    pub raw_token: String,
}

pub trait BarcodeParser: Send + Sync {
    fn parse_syntax(&self, raw_token: &str) -> Result<ParsedBarcode, AppError>;
}
