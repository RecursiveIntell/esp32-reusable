use ri_esp_core::EnvironmentReading;
use ri_esp_policy::{PolicyContext, PromptId};
use ri_esp_proof::{ProofConfig, ProofDecision, ProofEngine, RouteReason};

#[test]
fn low_confidence_sensor_context_forwards_to_ai() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(29.5, 71.0, Some(31.0));

    let receipt = engine.evaluate("esp32-proof-01", 42_000, sensor, 0.42, "gtx1070-local");

    assert_eq!(receipt.event_id, 1);
    assert_eq!(receipt.decision, ProofDecision::ForwardToAi);
    assert_eq!(receipt.reason.as_str(), "low_confidence");
    assert_eq!(receipt.sensor.temperature_c, Some(29.5));
}

#[test]
fn healthy_confidence_and_normal_environment_stays_local() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(22.0, 45.0, Some(21.9));

    let receipt = engine.evaluate("esp32-proof-01", 43_000, sensor, 0.91, "gtx1070-local");

    assert_eq!(receipt.event_id, 1);
    assert_eq!(receipt.decision, ProofDecision::LocalOnly);
    assert_eq!(receipt.reason.as_str(), "local_confident");
}

#[test]
fn receipt_json_contains_sensor_ai_route_and_decision() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(12.0, 45.0, None);
    let receipt = engine.evaluate("esp32-proof-01", 44_000, sensor, 0.92, "gtx1070-local");

    let json = receipt.to_json::<768>().unwrap();

    assert!(json.contains("\"device_id\":\"esp32-proof-01\""));
    assert!(json.contains("\"decision\":\"forward_to_ai\""));
    assert!(json.contains("\"reason\":\"temperature_out_of_range\""));
    assert!(json.contains("\"temperature_c\":12"));
    assert!(json.contains("\"ai_route\":\"gtx1070-local\""));
}

#[test]
fn feature_transfer_payload_is_available_for_gpu_forwarding() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(29.5, 71.0, Some(31.0));
    let receipt = engine.evaluate("esp32-proof-01", 42_000, sensor, 0.42, "gtx1070-local");

    let transfer = receipt.to_feature_transfer::<4>().unwrap();

    assert!(transfer.should_wake(0.75));
    assert_eq!(transfer.features.len(), 3);
    assert_eq!(transfer.features[0], 29.5);
    assert_eq!(transfer.features[1], 71.0);
    assert_eq!(transfer.features[2], 31.0);
}

#[test]
fn policy_evaluation_records_s3_local_language_fields() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(31.0, 70.0, None);
    let ctx = PolicyContext::new(sensor, 3_000).with_last_read_ms(2_900);

    let receipt = engine.evaluate_with_policy("esp32-proof-01", ctx, "gtx1070-local");

    assert_eq!(
        receipt.route_reason,
        RouteReason::TemperatureAndHumidityOutOfRange
    );
    assert_eq!(
        receipt.local_language_model.as_str(),
        "esp32s3-h320-p15-char-lstm"
    );
    assert_eq!(
        receipt.local_language_prompt.as_str(),
        PromptId::HighHeatHumidity.prompt_text()
    );
    let json = receipt.to_json::<1024>().unwrap();
    assert!(json.contains("\"local_language_model\":\"esp32s3-h320-p15-char-lstm\""));
    assert!(json.contains("\"local_language_prompt\":\"high heat and humidity. action is \""));
}

#[test]
fn policy_evaluation_records_stale_route() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(22.0, 42.0, None);
    let ctx = PolicyContext::new(sensor, 500_000).with_last_read_ms(100_000);

    let receipt = engine.evaluate_with_policy("esp32-proof-01", ctx, "gtx1070-local");

    assert_eq!(receipt.route_reason, RouteReason::StaleReading);
    assert_eq!(receipt.decision, ProofDecision::ForwardToAi);
    assert_eq!(
        receipt.local_language_prompt.as_str(),
        "stale data. action is "
    );
}

#[test]
fn policy_evaluation_keeps_cold_dry_on_safe_prompt() {
    let mut engine = ProofEngine::new(ProofConfig::default());
    let sensor = EnvironmentReading::new(14.0, 45.0, None);
    let ctx = PolicyContext::new(sensor, 5_500).with_last_read_ms(5_400);

    let receipt = engine.evaluate_with_policy("esp32-proof-01", ctx, "gtx1070-local");

    assert_eq!(receipt.local_language_prompt.as_str(), "safe action is ");
    assert_eq!(receipt.route_reason, RouteReason::UnsupportedColdOrDry);
}
