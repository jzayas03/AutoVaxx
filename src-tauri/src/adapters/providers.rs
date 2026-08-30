use crate::error::AppError;
use crate::ports::{
    BarcodeInput, BarcodeParser, FieldProposal, LocalAiProvider, ParsedBarcode, RegistryAdapter,
    RegistryValidationResult, SpeechToTextProvider,
};

#[derive(Default)]
pub struct UnavailableLocalAiProvider;

impl LocalAiProvider for UnavailableLocalAiProvider {
    fn propose_fields(&self, _local_text: &str) -> Result<Vec<FieldProposal>, AppError> {
        Err(AppError::ProviderUnavailable)
    }
}

#[derive(Default)]
pub struct UnavailableSpeechToTextProvider;

impl SpeechToTextProvider for UnavailableSpeechToTextProvider {
    fn transcribe_local_audio(&self, _audio: &[u8], _language: &str) -> Result<String, AppError> {
        Err(AppError::ProviderUnavailable)
    }
}

#[derive(Default)]
pub struct DisabledRegistryAdapter;

impl RegistryAdapter for DisabledRegistryAdapter {
    fn validate(
        &self,
        _canonical_revision_id: &str,
        profile_version: &str,
    ) -> Result<RegistryValidationResult, AppError> {
        Ok(RegistryValidationResult {
            profile_version: profile_version.to_owned(),
            ready: false,
            issue_codes: vec!["REGISTRY_PROFILE_NOT_VERIFIED".to_owned()],
        })
    }

    fn render(
        &self,
        _canonical_revision_id: &str,
        _profile_version: &str,
    ) -> Result<Vec<u8>, AppError> {
        Err(AppError::ProviderUnavailable)
    }
}

#[derive(Default)]
pub struct UnavailableBarcodeInput;

impl BarcodeInput for UnavailableBarcodeInput {
    fn read_token(&self) -> Result<String, AppError> {
        Err(AppError::ProviderUnavailable)
    }
}

#[derive(Default)]
pub struct SyntaxOnlyBarcodeParser;

impl BarcodeParser for SyntaxOnlyBarcodeParser {
    fn parse_syntax(&self, raw_token: &str) -> Result<ParsedBarcode, AppError> {
        let trimmed = raw_token.trim();
        if trimmed.is_empty() || trimmed.len() > 4096 {
            return Err(AppError::Validation);
        }
        Ok(ParsedBarcode {
            symbology: "UNRESOLVED".to_owned(),
            raw_token: trimmed.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_optional_providers_fail_closed_without_blocking_manual_domain_use() {
        assert!(matches!(
            UnavailableLocalAiProvider.propose_fields("synthetic"),
            Err(AppError::ProviderUnavailable)
        ));
        assert!(matches!(
            UnavailableSpeechToTextProvider.transcribe_local_audio(&[], "es"),
            Err(AppError::ProviderUnavailable)
        ));
        assert!(
            !DisabledRegistryAdapter
                .validate("revision", "unverified")
                .unwrap()
                .ready
        );
        assert!(
            DisabledRegistryAdapter
                .render("revision", "unverified")
                .is_err()
        );
        assert!(UnavailableBarcodeInput.read_token().is_err());
    }

    #[test]
    fn barcode_parser_does_not_resolve_meaning() {
        let parsed = SyntaxOnlyBarcodeParser.parse_syntax(" 010123 ").unwrap();
        assert_eq!(parsed.symbology, "UNRESOLVED");
        assert_eq!(parsed.raw_token, "010123");
    }
}
