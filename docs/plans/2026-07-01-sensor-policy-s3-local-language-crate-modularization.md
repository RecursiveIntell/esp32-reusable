# Sensor Policy -> S3 Local Language Crate Modularization Plan

> For Hermes: Use test-driven-development for implementation. Use subagent-driven-development only for independent follow-up phases after the API contracts below are locked.

Goal: Extract the latest ESP32 sensor-policy -> ESP32-S3 local-language bridge into reusable no_std crates and update existing crates so firmware/host tests stop copy-pasting policy tables, receipt schemas, and route logic.

Architecture: Keep deterministic policy separate from language generation. `ri-esp-policy` owns sensor-state classification and canonical prompt selection. `ri-esp-local-language` owns canonical prompt/output contracts and S3 receipt/integration receipt types. `ri-esp-proof` records provenance and references local-language model/prompt fields, but does not become a policy brain. Host Python/Arduino C++ remain consumers until a Rust firmware rewrite exists.

Tech stack: Rust 2021, no_std default, heapless 0.8, existing `ri-esp-core`, `ri-esp-tiered`, PlatformIO/Arduino C++ as downstream reference implementation, Python receiver/test harness as oracle fixtures.

---

## Evidence-backed current state

Repo path/date:
- Reusable workspace: `/home/sikmindz/projects/esp32-reusable`
- Date checked: 2026-07-01

Commands that passed:
- `cargo test --workspace`
  - Current result: 18 tests passed, 0 failed.
- Hard-test receipt check from `/home/sikmindz/projects/esp32-sensor-hub/sensor_policy_s3_hard_test_receipt.json`:
  - schema: `ri_sensor_policy_s3_hard_test_v1`
  - ok: true
  - policy_cases: 9
  - http_cases: 9
  - real_s3_rows: 24
  - unique_prompts: 8

Commands/status that failed or are bounded:
- `git -C /home/sikmindz/projects/esp32-reusable status --short` reports not a git repository. Do not include commit steps until this workspace is placed under git.
- Physical OLED pixels are not certified without attached OLED hardware. The hard test only certifies the exact `oled_text` input path.

Source inventory checked:
- Existing crate plan/inventory: `/home/sikmindz/projects/esp32-reusable/CRATE_DISTILLATION.md`
- Existing crates:
  - `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-core/src/lib.rs`
  - `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-tiered/src/lib.rs`
  - `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/src/lib.rs`
  - `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-llm/src/*`
  - `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-display/src/*`
- Sensor hub firmware/reference implementation:
  - `/home/sikmindz/projects/esp32-sensor-hub/src/main.cpp`
  - `/home/sikmindz/projects/esp32-sensor-hub/tools/sensor_receiver.py`
  - `/home/sikmindz/projects/esp32-sensor-hub/tools/test_sensor_policy_s3_language.py`
  - `/home/sikmindz/projects/esp32-sensor-hub/SENSOR_POLICY_S3_LOCAL_LANGUAGE_INTEGRATION_2026-07-01.md`
  - `/home/sikmindz/projects/esp32-sensor-hub/SENSOR_POLICY_S3_HARD_TEST_REPORT_2026-07-01.md`
  - `/home/sikmindz/projects/esp32-sensor-hub/RECEIPT_WORK_2026-07-01.md`
- S3 language firmware/reference implementation:
  - `/home/sikmindz/projects/esp32-s3-lstm-proof/src/main.cpp`
  - `/home/sikmindz/projects/esp32-s3-lstm-proof/tools/local_sentinel_policy.py`
  - `/home/sikmindz/projects/esp32-s3-lstm-proof/FINAL_H320_UTILITY_TEST_REPORT_2026-07-01.md`
  - `/home/sikmindz/projects/esp32-s3-lstm-proof/LOCAL_SENTINEL_POLICY_RECEIPT_2026-07-01.md`
  - `/home/sikmindz/projects/esp32-s3-lstm-proof/ESP_NN_LSTM_P7_REPORT.md`
- Older source projects from distillation:
  - `/home/sikmindz/projects/tiered-edge-ai/bridge/src/lib.rs`
  - `/home/sikmindz/projects/tiered-edge-ai/esp32-s3/src/*`

External ecosystem spot-check:
- `embedded-hal` 1.0 is already the right generic hardware-trait anchor.
- `heapless` is already the right fixed-capacity no_std container.
- `serde-json-core` exists for no_std JSON, but do not add it yet; current crates use hand-rolled fixed-capacity JSON strings and dependency-light design.
- `postcard` exists for no_std binary serialization, but do not add it yet; `ri-esp-tiered` already has a simple fixed binary protocol.
- `embedded-graphics`/SSD1306 crates exist, but this plan is policy/receipt extraction, not display-driver certification.

## Canonical behavior to preserve

Policy thresholds from firmware/Python hard test:
- missing sensor -> prompt `missing sensor. action is ` -> output `no claim.`
- stale sensor age > 120000 ms -> prompt `stale data. action is ` -> output `wait for fresh data.`
- hot threshold: temp_f >= 82.0
- cold unsupported threshold: temp_f <= 60.0
- humid threshold: humidity_pct >= 65.0
- dry unsupported threshold: humidity_pct <= 25.0
- hot + humid -> prompt `high heat and humidity. action is ` -> output `escalate.`
- hot only -> prompt `hot room. action is ` -> output `check airflow.`
- humid only -> prompt `humid room. action is ` -> output `ventilate.`
- cold or dry unsupported -> prompt `safe action is ` -> output `no claim without evidence.`
- normal -> prompt `normal room. action is ` -> output `log receipt.`
- non-policy trained prompt: `local first means ` -> output `decide before cloud.`

Receipt schemas to preserve:
- `ri_esp_proof_receipt_v1`
- `ri_esp_sensor_receipt_v1`
- `ri_esp32s3_local_language_v1`
- `ri_sensor_policy_to_s3_language_integration_v1`
- `ri_sensor_policy_s3_hard_test_v1`

Claim boundary:
- Deterministic policy chooses action/prompt.
- ESP32-S3 H320 p15 model only phrases the short local action/status.
- `ri-esp-local-language` can encode expected/passed receipt fields but must not claim free-form model safety.
- OLED physical rendering remains hardware-bound and cannot be guaranteed by crate tests.

---

## P0 — Certification blockers / highest ROI

### Task 1: Create `ri-esp-policy` crate with RED tests for all hard-test policy cases

Objective: Move canonical sensor-policy prompt mapping out of Python/C++ into a no_std Rust crate.

Files:
- Modify: `/home/sikmindz/projects/esp32-reusable/Cargo.toml`
- Create: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-policy/Cargo.toml`
- Create: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-policy/src/lib.rs`
- Create: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-policy/tests/policy_cases.rs`

Step 1: Write failing tests first.

Tests must assert all 9 cases from `tools/test_sensor_policy_s3_language.py`:
- proof override hot+humid
- missing sensor
- stale sensor
- derived hot+humid
- derived hot
- derived humid
- derived cold unsupported -> safe
- derived dry unsupported -> safe
- normal

Desired API:
```rust
use ri_esp_core::{EnvironmentReading, ReadingStatus};
use ri_esp_policy::{PolicyContext, SensorPolicy, PromptId};

let ctx = PolicyContext::new(EnvironmentReading::new(31.0, 70.0, None), 3_000)
    .with_last_read_ms(2_900);
let decision = SensorPolicy::default().evaluate(ctx);
assert_eq!(decision.prompt_id, PromptId::HighHeatHumidity);
assert_eq!(decision.prompt(), "high heat and humidity. action is ");
```

Step 2: Run RED:
- `cargo test -p ri-esp-policy`
- Expected: fail because crate/API does not exist.

Step 3: Implement minimal no_std crate:
- `PromptId` enum
- `PolicyReason` enum
- `PolicyDecision` struct: prompt_id, reason, ai_route bool, confidence f32, prompt accessor
- `PolicyContext` struct: EnvironmentReading, uptime_ms, last_read_ms Option<u64>, proof_prompt_override Option<&str>
- `SensorPolicy` thresholds matching hard test.

Step 4: Run GREEN:
- `cargo test -p ri-esp-policy`
- Expected: all policy tests pass.

### Task 2: Create `ri-esp-local-language` crate with canonical prompt/output table and receipt types

Objective: Move S3 p15 prompt/output and receipt contracts out of Python/C++ into reusable no_std Rust.

Files:
- Modify: `/home/sikmindz/projects/esp32-reusable/Cargo.toml`
- Create: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-local-language/Cargo.toml`
- Create: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-local-language/src/lib.rs`
- Create: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-local-language/tests/canonical_table.rs`

Step 1: Write failing tests first.

Tests must assert:
- 8 unique real-S3 prompt/output pairs from hard receipt.
- `S3_LANGUAGE_RECEIPT` schema fields can be represented.
- Integration receipt includes `policy_prompt`, `policy_prompt_source`, `local_language.output`, and `oled_text`.

Desired API:
```rust
use ri_esp_policy::PromptId;
use ri_esp_local_language::{canonical_output, S3LanguageReceipt, IntegrationReceipt};

assert_eq!(canonical_output(PromptId::HighHeatHumidity), "escalate.");
let receipt = S3LanguageReceipt::static_passed(PromptId::HighHeatHumidity);
assert_eq!(receipt.schema, "ri_esp32s3_local_language_v1");
assert_eq!(receipt.output.as_str(), "escalate.");
```

Step 2: Run RED:
- `cargo test -p ri-esp-local-language`
- Expected: fail because crate/API does not exist.

Step 3: Implement minimal crate:
- depend on `ri-esp-policy`, `heapless`
- constants for schemas and firmware/model labels
- canonical output function
- fixed-capacity `S3LanguageReceipt`
- fixed-capacity `IntegrationReceipt`
- optional JSON builders only if needed; do not add serde yet.

Step 4: Run GREEN:
- `cargo test -p ri-esp-local-language`
- Expected: all canonical table/receipt tests pass.

### Task 3: Update `ri-esp-proof` to carry local-language fields and stale/hot+humid route reasons

Objective: Make proof receipts match the firmware payload now shipped by sensor hub.

Files:
- Modify: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/Cargo.toml`
- Modify: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/src/lib.rs`
- Modify: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/tests/proof_flow.rs`
- Modify: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/PROOF_RECEIPT_SCHEMA.md`
- Modify: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/README.md`

Step 1: RED tests:
- receipt JSON contains `local_language_model` and `local_language_prompt`
- stale reading maps to route reason `stale_reading`
- hot+humid maps to route reason `temperature_and_humidity_out_of_range`
- cold/dry unsupported maps to safe prompt via `ri-esp-policy`, not misleading normal.

Step 2: Run RED:
- `cargo test -p ri-esp-proof`
- Expected: fail on missing fields/variants.

Step 3: Implement:
- Add `RouteReason::StaleReading`
- Add `RouteReason::TemperatureAndHumidityOutOfRange`
- Add local language fields to `ProofReceipt`
- Add `evaluate_with_policy` helper that calls `ri-esp-policy` and stores prompt/model.
- Preserve existing `evaluate` behavior for backwards compatibility by using normal prompt/model defaults.

Step 4: Run GREEN:
- `cargo test -p ri-esp-proof`
- Expected: pass.

### Task 4: Add workspace integration tests that mirror the Python hard-test receipt

Objective: Lock Rust crate behavior against the verified Python/firmware oracle.

Files:
- Create: `/home/sikmindz/projects/esp32-reusable/tests/sensor_policy_s3_contract.rs` if root tests are supported, otherwise create under `crates/ri-esp-local-language/tests/`.

Test matrix:
- All 9 policy payload cases converted into Rust `PolicyContext` fixtures.
- All 8 unique prompt/output pairs.
- JSON string output contains schema names and OLED text where applicable.

Commands:
- `cargo test --workspace`
- Expected after implementation: pass all tests.

---

## P1 — Product hardening

### Task 5: Update `ri-esp-core` with optional sensor freshness helper

Objective: Avoid copy-pasting stale-age calculations.

Files:
- Modify: `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-core/src/lib.rs`

Desired API:
```rust
let r = EnvironmentReading::new(22.0, 45.0, None).with_status(ReadingStatus::Stale);
```

Add only if it reduces code in `ri-esp-policy`; otherwise skip.

### Task 6: Update docs/readmes for crate boundaries

Objective: Make future extraction obvious.

Files:
- Modify: `/home/sikmindz/projects/esp32-reusable/README.md`
- Modify: `/home/sikmindz/projects/esp32-reusable/CRATE_DISTILLATION.md`
- Create or update README files for new crates.

Include:
- policy vs language boundary
- claim boundary
- hardware-certification boundary
- hard-test receipt source paths

### Task 7: Optional host testkit crate or scripts

Objective: Only after P0 stabilizes, decide whether to create `ri-esp-integration-testkit`.

Default decision: hold.

Reason: Current hard test is Python and talks to serial/HTTP. A Rust no_std crate should not absorb host serial concerns. If needed, create a separate std-only tool crate later.

---

## P2 — Future wiring / not now

- Do not publish crates yet. Prior distillation explicitly says publish only after at least two firmware apps consume them.
- Do not add `ri-esp-net` yet; networking world is still split between Arduino/PlatformIO, esp-idf-svc, and no_std esp-radio/embassy.
- Do not claim OLED physical correctness without attached OLED hardware.
- Do not treat S3 model as free-form policy logic.
- Do not add serde/postcard/embedded-graphics dependencies unless a downstream consumer forces it.

## Final verification gate

Run from `/home/sikmindz/projects/esp32-reusable`:

```bash
cargo test --workspace
```

Expected:
- existing 18 tests still pass
- new `ri-esp-policy` tests pass
- new `ri-esp-local-language` tests pass
- updated `ri-esp-proof` tests pass

If ESP Rust target toolchain is configured, also run:

```bash
cargo +esp check --target xtensa-esp32s3-none-elf -Z build-std=core,alloc --workspace --lib
```

Expected:
- all no_std library crates check for ESP32-S3 target.

## Non-claims

This plan does not certify:
- physical OLED wiring/pixels
- ESP32-S3 model quality outside canonical prompts
- safety-critical control
- crates.io publish readiness
