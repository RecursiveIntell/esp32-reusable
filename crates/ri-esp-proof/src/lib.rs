#![no_std]

use heapless::String;
use ri_esp_core::EnvironmentReading;
use ri_esp_local_language::DEFAULT_LOCAL_LANGUAGE_MODEL;
use ri_esp_policy::{PolicyContext, PolicyReason, PromptId, SensorPolicy};
use ri_esp_tiered::FeatureTransfer;

pub const PROOF_RECEIPT_SCHEMA: &str = "ri_esp_proof_receipt_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofDecision {
    LocalOnly,
    ForwardToAi,
}

impl ProofDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnly => "local_only",
            Self::ForwardToAi => "forward_to_ai",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteReason {
    LocalConfident,
    LowConfidence,
    SensorMissing,
    StaleReading,
    TemperatureAndHumidityOutOfRange,
    TemperatureOutOfRange,
    HumidityOutOfRange,
    UnsupportedColdOrDry,
    OperatorRequest,
    ScheduleTick,
    AnomalyMatch,
}

impl RouteReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalConfident => "local_confident",
            Self::LowConfidence => "low_confidence",
            Self::SensorMissing => "sensor_missing",
            Self::StaleReading => "stale_reading",
            Self::TemperatureAndHumidityOutOfRange => "temperature_and_humidity_out_of_range",
            Self::TemperatureOutOfRange => "temperature_out_of_range",
            Self::HumidityOutOfRange => "humidity_out_of_range",
            Self::UnsupportedColdOrDry => "unsupported_cold_or_dry",
            Self::OperatorRequest => "operator_request",
            Self::ScheduleTick => "schedule_tick",
            Self::AnomalyMatch => "anomaly_match",
        }
    }

    pub const fn decision(self) -> ProofDecision {
        match self {
            Self::LocalConfident | Self::UnsupportedColdOrDry => ProofDecision::LocalOnly,
            _ => ProofDecision::ForwardToAi,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProofConfig {
    pub confidence_threshold: f32,
    pub min_temperature_c: f32,
    pub max_temperature_c: f32,
    pub min_humidity_pct: f32,
    pub max_humidity_pct: f32,
}

impl Default for ProofConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.75,
            min_temperature_c: 15.0,
            max_temperature_c: 30.0,
            min_humidity_pct: 20.0,
            max_humidity_pct: 75.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProofReceipt {
    pub schema_version: &'static str,
    pub event_id: u64,
    pub device_id: String<48>,
    pub timestamp_ms: u64,
    pub sensor: EnvironmentReading,
    pub sentinel_confidence: f32,
    pub decision: ProofDecision,
    pub route_reason: RouteReason,
    pub reason: String<48>,
    pub ai_route: String<64>,
    pub local_language_model: String<48>,
    pub local_language_prompt: String<96>,
}

impl ProofReceipt {
    pub fn to_feature_transfer<const N: usize>(&self) -> Option<FeatureTransfer<N>> {
        let mut transfer = FeatureTransfer::new(
            self.sensor.temperature_c.unwrap_or(0.0),
            self.sentinel_confidence,
            self.timestamp_ms,
        );

        if let Some(v) = self.sensor.temperature_c {
            transfer.features.push(v).ok()?;
        }
        if let Some(v) = self.sensor.humidity_pct {
            transfer.features.push(v).ok()?;
        }
        if let Some(v) = self.sensor.heat_index_c {
            transfer.features.push(v).ok()?;
        }

        Some(transfer)
    }

    pub fn to_json<const N: usize>(&self) -> Option<String<N>> {
        let mut out = String::<N>::new();
        push_str(&mut out, "{")?;
        push_json_str(&mut out, "schema", self.schema_version, false)?;
        push_json_u64(&mut out, "event_id", self.event_id, false)?;
        push_json_str(&mut out, "device_id", self.device_id.as_str(), false)?;
        push_json_u64(&mut out, "timestamp_ms", self.timestamp_ms, false)?;
        push_json_str(&mut out, "decision", self.decision.as_str(), false)?;
        push_json_str(&mut out, "reason", self.reason.as_str(), false)?;
        push_json_str(&mut out, "route_reason", self.route_reason.as_str(), false)?;
        push_json_f32(
            &mut out,
            "sentinel_confidence",
            self.sentinel_confidence,
            false,
        )?;
        push_json_str(&mut out, "ai_route", self.ai_route.as_str(), false)?;
        push_json_str(
            &mut out,
            "local_language_model",
            self.local_language_model.as_str(),
            false,
        )?;
        push_json_str(
            &mut out,
            "local_language_prompt",
            self.local_language_prompt.as_str(),
            false,
        )?;
        push_str(&mut out, "\"sensor\":{")?;
        push_json_opt_f32(&mut out, "temperature_c", self.sensor.temperature_c, false)?;
        push_json_opt_f32(&mut out, "humidity_pct", self.sensor.humidity_pct, false)?;
        push_json_opt_f32(&mut out, "heat_index_c", self.sensor.heat_index_c, true)?;
        push_str(&mut out, "}}")?;
        Some(out)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProofEngine {
    config: ProofConfig,
    next_event_id: u64,
}

struct BuildReceiptInput<'a> {
    device_id: &'a str,
    timestamp_ms: u64,
    sensor: EnvironmentReading,
    sentinel_confidence: f32,
    ai_route: &'a str,
    route_reason: RouteReason,
    local_language_model: &'a str,
    local_language_prompt: &'a str,
}

impl ProofEngine {
    pub const fn new(config: ProofConfig) -> Self {
        Self {
            config,
            next_event_id: 1,
        }
    }

    pub fn evaluate(
        &mut self,
        device_id: &str,
        timestamp_ms: u64,
        sensor: EnvironmentReading,
        sentinel_confidence: f32,
        ai_route: &str,
    ) -> ProofReceipt {
        let reason = self.reason(sensor, sentinel_confidence);
        self.build_receipt(
            device_id,
            timestamp_ms,
            sensor,
            sentinel_confidence,
            ai_route,
            reason,
        )
    }

    pub fn evaluate_with_route_reason(
        &mut self,
        device_id: &str,
        timestamp_ms: u64,
        sensor: EnvironmentReading,
        sentinel_confidence: f32,
        ai_route: &str,
        route_reason: RouteReason,
    ) -> ProofReceipt {
        self.build_receipt(
            device_id,
            timestamp_ms,
            sensor,
            sentinel_confidence,
            ai_route,
            route_reason,
        )
    }

    pub fn evaluate_with_policy(
        &mut self,
        device_id: &str,
        ctx: PolicyContext<'_>,
        ai_route: &str,
    ) -> ProofReceipt {
        let policy_decision = SensorPolicy::default().evaluate(ctx);
        let route_reason = route_reason_from_policy(policy_decision.reason);
        self.build_receipt_with_language(BuildReceiptInput {
            device_id,
            timestamp_ms: ctx.uptime_ms,
            sensor: ctx.reading,
            sentinel_confidence: policy_decision.confidence,
            ai_route,
            route_reason,
            local_language_model: DEFAULT_LOCAL_LANGUAGE_MODEL,
            local_language_prompt: policy_decision.prompt(),
        })
    }

    fn build_receipt(
        &mut self,
        device_id: &str,
        timestamp_ms: u64,
        sensor: EnvironmentReading,
        sentinel_confidence: f32,
        ai_route: &str,
        route_reason: RouteReason,
    ) -> ProofReceipt {
        let prompt_id = default_prompt_for_route_reason(route_reason);
        self.build_receipt_with_language(BuildReceiptInput {
            device_id,
            timestamp_ms,
            sensor,
            sentinel_confidence,
            ai_route,
            route_reason,
            local_language_model: DEFAULT_LOCAL_LANGUAGE_MODEL,
            local_language_prompt: prompt_id.prompt_text(),
        })
    }

    fn build_receipt_with_language(&mut self, input: BuildReceiptInput<'_>) -> ProofReceipt {
        let event_id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);

        let mut device = String::<48>::new();
        let _ = device.push_str(input.device_id);
        let mut route = String::<64>::new();
        let _ = route.push_str(input.ai_route);
        let mut reason_s = String::<48>::new();
        let _ = reason_s.push_str(input.route_reason.as_str());
        let mut language_model = String::<48>::new();
        let _ = language_model.push_str(input.local_language_model);
        let mut language_prompt = String::<96>::new();
        let _ = language_prompt.push_str(input.local_language_prompt);

        ProofReceipt {
            schema_version: PROOF_RECEIPT_SCHEMA,
            event_id,
            device_id: device,
            timestamp_ms: input.timestamp_ms,
            sensor: input.sensor,
            sentinel_confidence: input.sentinel_confidence,
            decision: input.route_reason.decision(),
            route_reason: input.route_reason,
            reason: reason_s,
            ai_route: route,
            local_language_model: language_model,
            local_language_prompt: language_prompt,
        }
    }

    fn reason(&self, sensor: EnvironmentReading, sentinel_confidence: f32) -> RouteReason {
        if sentinel_confidence < self.config.confidence_threshold {
            return RouteReason::LowConfidence;
        }
        match sensor.temperature_c {
            Some(v) if v < self.config.min_temperature_c || v > self.config.max_temperature_c => {
                return RouteReason::TemperatureOutOfRange;
            }
            None => return RouteReason::SensorMissing,
            _ => {}
        }
        match sensor.humidity_pct {
            Some(v) if v < self.config.min_humidity_pct || v > self.config.max_humidity_pct => {
                return RouteReason::HumidityOutOfRange;
            }
            None => return RouteReason::SensorMissing,
            _ => {}
        }
        RouteReason::LocalConfident
    }
}

fn route_reason_from_policy(reason: PolicyReason) -> RouteReason {
    match reason {
        PolicyReason::ProofPromptOverride => RouteReason::OperatorRequest,
        PolicyReason::SensorMissing => RouteReason::SensorMissing,
        PolicyReason::StaleReading => RouteReason::StaleReading,
        PolicyReason::HotHumid => RouteReason::TemperatureAndHumidityOutOfRange,
        PolicyReason::HotRoom => RouteReason::TemperatureOutOfRange,
        PolicyReason::HumidRoom => RouteReason::HumidityOutOfRange,
        PolicyReason::UnsupportedColdOrDry => RouteReason::UnsupportedColdOrDry,
        PolicyReason::Normal => RouteReason::LocalConfident,
    }
}

fn default_prompt_for_route_reason(route_reason: RouteReason) -> PromptId {
    match route_reason {
        RouteReason::SensorMissing => PromptId::MissingSensor,
        RouteReason::StaleReading => PromptId::StaleData,
        RouteReason::TemperatureAndHumidityOutOfRange => PromptId::HighHeatHumidity,
        RouteReason::TemperatureOutOfRange => PromptId::HotRoom,
        RouteReason::HumidityOutOfRange => PromptId::HumidRoom,
        RouteReason::UnsupportedColdOrDry => PromptId::SafeAction,
        RouteReason::LocalConfident => PromptId::NormalRoom,
        RouteReason::LowConfidence
        | RouteReason::OperatorRequest
        | RouteReason::ScheduleTick
        | RouteReason::AnomalyMatch => PromptId::NormalRoom,
    }
}

fn push_str<const N: usize>(out: &mut String<N>, s: &str) -> Option<()> {
    out.push_str(s).ok()
}

fn push_json_str<const N: usize>(
    out: &mut String<N>,
    key: &str,
    value: &str,
    last: bool,
) -> Option<()> {
    push_str(out, "\"")?;
    push_str(out, key)?;
    push_str(out, "\":\"")?;
    push_str(out, value)?;
    push_str(out, "\"")?;
    if !last {
        push_str(out, ",")?;
    }
    Some(())
}

fn push_json_u64<const N: usize>(
    out: &mut String<N>,
    key: &str,
    value: u64,
    last: bool,
) -> Option<()> {
    push_str(out, "\"")?;
    push_str(out, key)?;
    push_str(out, "\":")?;
    push_u64(out, value)?;
    if !last {
        push_str(out, ",")?;
    }
    Some(())
}

fn push_json_f32<const N: usize>(
    out: &mut String<N>,
    key: &str,
    value: f32,
    last: bool,
) -> Option<()> {
    push_str(out, "\"")?;
    push_str(out, key)?;
    push_str(out, "\":")?;
    push_f32(out, value)?;
    if !last {
        push_str(out, ",")?;
    }
    Some(())
}

fn push_json_opt_f32<const N: usize>(
    out: &mut String<N>,
    key: &str,
    value: Option<f32>,
    last: bool,
) -> Option<()> {
    push_str(out, "\"")?;
    push_str(out, key)?;
    push_str(out, "\":")?;
    match value {
        Some(v) => push_f32(out, v)?,
        None => push_str(out, "null")?,
    }
    if !last {
        push_str(out, ",")?;
    }
    Some(())
}

fn push_u64<const N: usize>(out: &mut String<N>, mut value: u64) -> Option<()> {
    if value == 0 {
        return out.push('0').ok();
    }
    let mut buf = [0u8; 20];
    let mut len = 0;
    while value > 0 {
        buf[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        out.push(buf[len] as char).ok()?;
    }
    Some(())
}

fn push_i32<const N: usize>(out: &mut String<N>, value: i32) -> Option<()> {
    if value < 0 {
        out.push('-').ok()?;
        push_u64(out, value.unsigned_abs() as u64)
    } else {
        push_u64(out, value as u64)
    }
}

fn push_f32<const N: usize>(out: &mut String<N>, value: f32) -> Option<()> {
    if value.is_nan() {
        return push_str(out, "null");
    }
    let scaled = (value * 10.0) as i32;
    let whole = scaled / 10;
    let frac = (scaled % 10).abs();
    push_i32(out, whole)?;
    if frac != 0 {
        out.push('.').ok()?;
        out.push((b'0' + frac as u8) as char).ok()?;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_ids_increment() {
        let mut engine = ProofEngine::new(ProofConfig::default());
        let sensor = EnvironmentReading::new(22.0, 45.0, None);
        let a = engine.evaluate("node", 1, sensor, 0.9, "gpu");
        let b = engine.evaluate("node", 2, sensor, 0.9, "gpu");
        assert_eq!(a.event_id, 1);
        assert_eq!(b.event_id, 2);
    }

    #[test]
    fn operator_reason_forces_forward_route() {
        let mut engine = ProofEngine::new(ProofConfig::default());
        let sensor = EnvironmentReading::new(22.0, 45.0, None);
        let r = engine.evaluate_with_route_reason(
            "node",
            1,
            sensor,
            0.99,
            "gemma4:12b",
            RouteReason::OperatorRequest,
        );
        assert_eq!(r.decision, ProofDecision::ForwardToAi);
        assert_eq!(r.reason.as_str(), "operator_request");
    }
}
