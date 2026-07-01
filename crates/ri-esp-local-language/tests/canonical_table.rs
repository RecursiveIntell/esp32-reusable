use ri_esp_local_language::{
    canonical_output, IntegrationReceipt, S3LanguageReceipt, LOCAL_LANGUAGE_SCHEMA,
    SENSOR_POLICY_INTEGRATION_SCHEMA,
};
use ri_esp_policy::PromptId;

#[test]
fn canonical_prompt_outputs_match_real_s3_hard_receipt() {
    let cases = [
        (PromptId::HighHeatHumidity, "escalate."),
        (PromptId::HotRoom, "check airflow."),
        (PromptId::HumidRoom, "ventilate."),
        (PromptId::LocalFirstMeans, "decide before cloud."),
        (PromptId::MissingSensor, "no claim."),
        (PromptId::NormalRoom, "log receipt."),
        (PromptId::SafeAction, "no claim without evidence."),
        (PromptId::StaleData, "wait for fresh data."),
    ];
    for (prompt, expected) in cases {
        assert_eq!(canonical_output(prompt), expected);
    }
}

#[test]
fn static_s3_receipt_preserves_schema_prompt_output_and_passed() {
    let receipt = S3LanguageReceipt::static_passed(PromptId::HighHeatHumidity);
    assert_eq!(receipt.schema, LOCAL_LANGUAGE_SCHEMA);
    assert_eq!(
        receipt.prompt.as_str(),
        "high heat and humidity. action is "
    );
    assert_eq!(receipt.output.as_str(), "escalate.");
    assert!(receipt.passed);
}

#[test]
fn integration_receipt_carries_oled_text() {
    let local = S3LanguageReceipt::static_passed(PromptId::SafeAction);
    let receipt = IntegrationReceipt::new("esp32-case", "derived.unsupported_cold_or_dry", local);
    assert_eq!(receipt.schema, SENSOR_POLICY_INTEGRATION_SCHEMA);
    assert_eq!(receipt.policy_prompt.as_str(), "safe action is ");
    assert_eq!(receipt.oled_text.as_str(), "no claim without evidence.");
}
