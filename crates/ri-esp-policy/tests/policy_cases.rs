use ri_esp_core::{EnvironmentReading, ReadingStatus};
use ri_esp_policy::{PolicyContext, PolicyReason, PromptId, SensorPolicy};

fn ctx(
    temp_c: f32,
    humidity_pct: f32,
    uptime_ms: u64,
    last_read_ms: u64,
) -> PolicyContext<'static> {
    PolicyContext::new(
        EnvironmentReading::new(temp_c, humidity_pct, None),
        uptime_ms,
    )
    .with_last_read_ms(last_read_ms)
}

#[test]
fn proof_override_uses_known_canonical_prompt() {
    let reading = EnvironmentReading::new(31.0, 72.0, None);
    let decision = SensorPolicy::default().evaluate(
        PolicyContext::new(reading, 1_000)
            .with_proof_prompt_override("high heat and humidity. action is "),
    );
    assert_eq!(decision.prompt_id, PromptId::HighHeatHumidity);
    assert_eq!(decision.reason, PolicyReason::ProofPromptOverride);
    assert!(decision.ai_route);
    assert_eq!(decision.prompt(), "high heat and humidity. action is ");
}

#[test]
fn missing_sensor_maps_to_no_claim_prompt() {
    let reading = EnvironmentReading::failed();
    let decision = SensorPolicy::default().evaluate(PolicyContext::new(reading, 2_000));
    assert_eq!(decision.prompt_id, PromptId::MissingSensor);
    assert_eq!(decision.reason, PolicyReason::SensorMissing);
    assert_eq!(decision.prompt(), "missing sensor. action is ");
}

#[test]
fn stale_sensor_maps_to_wait_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(22.0, 42.0, 500_000, 100_000));
    assert_eq!(decision.prompt_id, PromptId::StaleData);
    assert_eq!(decision.reason, PolicyReason::StaleReading);
    assert!(decision.ai_route);
}

#[test]
fn hot_humid_maps_to_escalation_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(31.0, 70.0, 3_000, 2_900));
    assert_eq!(decision.prompt_id, PromptId::HighHeatHumidity);
    assert_eq!(decision.reason, PolicyReason::HotHumid);
    assert!(decision.ai_route);
}

#[test]
fn hot_only_maps_to_airflow_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(28.5, 45.0, 4_000, 3_900));
    assert_eq!(decision.prompt_id, PromptId::HotRoom);
    assert_eq!(decision.prompt(), "hot room. action is ");
}

#[test]
fn humid_only_maps_to_ventilate_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(23.0, 67.0, 5_000, 4_900));
    assert_eq!(decision.prompt_id, PromptId::HumidRoom);
    assert_eq!(decision.prompt(), "humid room. action is ");
}

#[test]
fn cold_unsupported_maps_to_safe_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(14.0, 45.0, 5_500, 5_400));
    assert_eq!(decision.prompt_id, PromptId::SafeAction);
    assert_eq!(decision.reason, PolicyReason::UnsupportedColdOrDry);
    assert_eq!(decision.prompt(), "safe action is ");
}

#[test]
fn dry_unsupported_maps_to_safe_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(22.0, 22.0, 5_700, 5_600));
    assert_eq!(decision.prompt_id, PromptId::SafeAction);
    assert_eq!(decision.reason, PolicyReason::UnsupportedColdOrDry);
}

#[test]
fn normal_maps_to_log_receipt_prompt() {
    let decision = SensorPolicy::default().evaluate(ctx(22.0, 45.0, 6_000, 5_900));
    assert_eq!(decision.prompt_id, PromptId::NormalRoom);
    assert_eq!(decision.reason, PolicyReason::Normal);
    assert!(!decision.ai_route);
}

#[test]
fn explicit_stale_status_maps_to_stale_prompt() {
    let reading = EnvironmentReading {
        temperature_c: Some(22.0),
        humidity_pct: Some(42.0),
        heat_index_c: None,
        status: ReadingStatus::Stale,
    };
    let decision = SensorPolicy::default().evaluate(PolicyContext::new(reading, 10));
    assert_eq!(decision.prompt_id, PromptId::StaleData);
}
