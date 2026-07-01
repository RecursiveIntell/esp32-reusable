#![no_std]

//! Deterministic sensor-policy crate for the ESP32/S3 physical-AI endpoint stack.
//!
//! Inspects a normalized `ri_esp_core::EnvironmentReading`, checks freshness,
//! chooses a canonical `PromptId`, and returns a `PolicyDecision` with reason,
//! confidence, route hint, and exact prompt text.
//!
//! It does not run language generation and does not claim model correctness.

use ri_esp_core::{EnvironmentReading, ReadingStatus};

/// Canonical prompt identifiers matching the 2026-07-01 hard test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptId {
    HighHeatHumidity,
    HotRoom,
    HumidRoom,
    MissingSensor,
    NormalRoom,
    SafeAction,
    StaleData,
    LocalFirstMeans,
}

impl PromptId {
    /// Returns the canonical prompt text used to seed the S3 local-language model.
    pub const fn prompt_text(self) -> &'static str {
        match self {
            PromptId::HighHeatHumidity => "high heat and humidity. action is ",
            PromptId::HotRoom => "hot room. action is ",
            PromptId::HumidRoom => "humid room. action is ",
            PromptId::MissingSensor => "missing sensor. action is ",
            PromptId::NormalRoom => "normal room. action is ",
            PromptId::SafeAction => "safe action is ",
            PromptId::StaleData => "stale data. action is ",
            PromptId::LocalFirstMeans => "local first means ",
        }
    }
}

/// Why the policy chose this prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyReason {
    ProofPromptOverride,
    SensorMissing,
    StaleReading,
    HotHumid,
    HotRoom,
    HumidRoom,
    UnsupportedColdOrDry,
    Normal,
}

/// Input context for a policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicyContext<'a> {
    pub reading: EnvironmentReading,
    pub uptime_ms: u64,
    pub last_read_ms: u64,
    pub proof_prompt_override: Option<&'a str>,
}

impl<'a> PolicyContext<'a> {
    pub fn new(reading: EnvironmentReading, uptime_ms: u64) -> Self {
        Self {
            reading,
            uptime_ms,
            last_read_ms: 0,
            proof_prompt_override: None,
        }
    }

    pub fn with_last_read_ms(mut self, ms: u64) -> Self {
        self.last_read_ms = ms;
        self
    }

    pub fn with_proof_prompt_override(mut self, prompt: &'a str) -> Self {
        self.proof_prompt_override = Some(prompt);
        self
    }
}

/// The result of a policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolicyDecision {
    pub prompt_id: PromptId,
    pub reason: PolicyReason,
    pub ai_route: bool,
    pub confidence: f32,
    prompt_text: &'static str,
}

impl PolicyDecision {
    /// Returns the canonical prompt text for this decision.
    pub fn prompt(&self) -> &'static str {
        self.prompt_text
    }
}

/// Thresholds for sensor policy decisions.
pub struct SensorPolicy {
    pub stale_ms: u64,
    pub hot_f: f32,
    pub cold_f: f32,
    pub humid_pct: f32,
    pub dry_pct: f32,
}

impl Default for SensorPolicy {
    fn default() -> Self {
        Self {
            stale_ms: 120_000,
            hot_f: 82.0,
            cold_f: 60.0,
            humid_pct: 65.0,
            dry_pct: 25.0,
        }
    }
}

fn match_prompt_id(s: &str) -> PromptId {
    match s {
        "high heat and humidity. action is " => PromptId::HighHeatHumidity,
        "hot room. action is " => PromptId::HotRoom,
        "humid room. action is " => PromptId::HumidRoom,
        "missing sensor. action is " => PromptId::MissingSensor,
        "normal room. action is " => PromptId::NormalRoom,
        "safe action is " => PromptId::SafeAction,
        "stale data. action is " => PromptId::StaleData,
        "local first means " => PromptId::LocalFirstMeans,
        _ => PromptId::NormalRoom,
    }
}

const fn ai_route_for(id: PromptId) -> bool {
    match id {
        PromptId::HighHeatHumidity => true,
        PromptId::HotRoom => true,
        PromptId::HumidRoom => true,
        PromptId::MissingSensor => true,
        PromptId::StaleData => true,
        PromptId::NormalRoom => false,
        PromptId::SafeAction => false,
        PromptId::LocalFirstMeans => false,
    }
}

fn decision(id: PromptId, reason: PolicyReason) -> PolicyDecision {
    let conf = match id {
        PromptId::HighHeatHumidity | PromptId::HotRoom | PromptId::HumidRoom => 0.9,
        PromptId::MissingSensor => 0.0,
        PromptId::StaleData => 0.3,
        PromptId::NormalRoom => 1.0,
        PromptId::SafeAction => 0.5,
        PromptId::LocalFirstMeans => 0.8,
    };
    PolicyDecision {
        prompt_id: id,
        reason,
        ai_route: ai_route_for(id),
        confidence: conf,
        prompt_text: id.prompt_text(),
    }
}

impl SensorPolicy {
    pub fn evaluate(&self, ctx: PolicyContext) -> PolicyDecision {
        // 1. Explicit override — operator forces a specific prompt.
        if let Some(override_prompt) = ctx.proof_prompt_override {
            let id = match_prompt_id(override_prompt);
            return PolicyDecision {
                prompt_id: id,
                reason: PolicyReason::ProofPromptOverride,
                ai_route: ai_route_for(id),
                confidence: 1.0,
                prompt_text: id.prompt_text(),
            };
        }

        let r = &ctx.reading;

        // 2. Sensor missing or not connected.
        if r.status == ReadingStatus::SensorReadFailed || r.status == ReadingStatus::NotConnected {
            return decision(PromptId::MissingSensor, PolicyReason::SensorMissing);
        }

        // 3. Explicit stale status from the reading itself.
        if r.status == ReadingStatus::Stale {
            return decision(PromptId::StaleData, PolicyReason::StaleReading);
        }

        // 4. Timing-based staleness.
        if ctx.uptime_ms.saturating_sub(ctx.last_read_ms) > self.stale_ms {
            return decision(PromptId::StaleData, PolicyReason::StaleReading);
        }

        // 5-9. Temperature and humidity thresholds.
        let temp_c = r.temperature_c.unwrap_or(25.0);
        let humidity = r.humidity_pct.unwrap_or(50.0);
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;

        if temp_f >= self.hot_f && humidity >= self.humid_pct {
            return decision(PromptId::HighHeatHumidity, PolicyReason::HotHumid);
        }

        if temp_f >= self.hot_f {
            return decision(PromptId::HotRoom, PolicyReason::HotRoom);
        }

        if humidity >= self.humid_pct {
            return decision(PromptId::HumidRoom, PolicyReason::HumidRoom);
        }

        if temp_f <= self.cold_f || humidity <= self.dry_pct {
            return decision(PromptId::SafeAction, PolicyReason::UnsupportedColdOrDry);
        }

        decision(PromptId::NormalRoom, PolicyReason::Normal)
    }
}
