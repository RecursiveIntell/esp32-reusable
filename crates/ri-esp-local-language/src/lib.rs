#![no_std]

//! Contract crate for the ESP32-S3 H320 p15 local-language layer.
//!
//! Owns receipt schema constants, the canonical prompt-to-output table verified
//! by the 2026-07-01 real S3 hard test, and fixed-capacity receipt structs.
//!
//! Does not choose policy — use `ri-esp-policy` for deterministic prompt selection.

use heapless::String;
use ri_esp_policy::PromptId;

/// Schema constant for S3 local-language receipts.
pub const LOCAL_LANGUAGE_SCHEMA: &str = "ri_esp32s3_local_language_v1";

/// Schema constant for sensor-policy to S3 local-language integration receipts.
pub const SENSOR_POLICY_INTEGRATION_SCHEMA: &str = "ri_sensor_policy_to_s3_language_integration_v1";

/// Default model identifier for the ESP32-S3 local-language layer.
pub const DEFAULT_LOCAL_LANGUAGE_MODEL: &str = "esp32s3-h320-p15-char-lstm";

/// Returns the canonical S3 model output for a given prompt id.
///
/// Verified by 24 real ESP32-S3 H320 p15 hardware generations on 2026-07-01.
pub fn canonical_output(id: PromptId) -> &'static str {
    match id {
        PromptId::HighHeatHumidity => "escalate.",
        PromptId::HotRoom => "check airflow.",
        PromptId::HumidRoom => "ventilate.",
        PromptId::LocalFirstMeans => "decide before cloud.",
        PromptId::MissingSensor => "no claim.",
        PromptId::NormalRoom => "log receipt.",
        PromptId::SafeAction => "no claim without evidence.",
        PromptId::StaleData => "wait for fresh data.",
    }
}

/// Fixed-capacity receipt for a single S3 local-language generation.
#[derive(Debug, Clone)]
pub struct S3LanguageReceipt {
    pub schema: &'static str,
    pub prompt: String<64>,
    pub output: String<64>,
    pub passed: bool,
}

impl S3LanguageReceipt {
    /// Creates a receipt from the canonical prompt/output table with passed=true.
    pub fn static_passed(id: PromptId) -> Self {
        let mut prompt = String::new();
        let _ = prompt.push_str(id.prompt_text());

        let mut output = String::new();
        let _ = output.push_str(canonical_output(id));

        Self {
            schema: LOCAL_LANGUAGE_SCHEMA,
            prompt,
            output,
            passed: true,
        }
    }
}

/// Fixed-capacity integration receipt linking sensor policy to S3 language output.
#[derive(Debug, Clone)]
pub struct IntegrationReceipt {
    pub schema: &'static str,
    pub device_id: String<64>,
    pub derived_reason: String<64>,
    pub policy_prompt: String<64>,
    pub oled_text: String<64>,
}

impl IntegrationReceipt {
    /// Creates an integration receipt from a device id, derived reason, and
    /// the local S3 language receipt.
    pub fn new(device_id: &str, derived_reason: &str, local: S3LanguageReceipt) -> Self {
        let mut did = String::new();
        let _ = did.push_str(device_id);

        let mut dr = String::new();
        let _ = dr.push_str(derived_reason);

        let mut pp = String::new();
        let _ = pp.push_str(local.prompt.as_str());

        let mut ot = String::new();
        let _ = ot.push_str(local.output.as_str());

        Self {
            schema: SENSOR_POLICY_INTEGRATION_SCHEMA,
            device_id: did,
            derived_reason: dr,
            policy_prompt: pp,
            oled_text: ot,
        }
    }
}
