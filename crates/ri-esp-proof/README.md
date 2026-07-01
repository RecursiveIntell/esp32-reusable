# ri-esp-proof

Proof receipts for physical AI endpoints.

![ri-esp-proof architecture](assets/architecture.svg)

## What it provides

route receipts, local-language fields, feature transfer bridge.

It depends on policy/local-language/tiered crates and joins them into route/provenance receipts.

## Install

```toml
[dependencies]
ri-esp-proof = "0.1"
```

## Quick start

```rust
use ri_esp_core::EnvironmentReading;
use ri_esp_policy::PolicyContext;
use ri_esp_proof::{ProofConfig, ProofEngine};

let sensor = EnvironmentReading::new(31.0, 70.0, None);
let ctx = PolicyContext::new(sensor, 3_000).with_last_read_ms(2_900);
let mut engine = ProofEngine::new(ProofConfig::default());
let receipt = engine.evaluate_with_policy("esp32-node", ctx, "gtx1070-local");
assert_eq!(receipt.local_language_prompt.as_str(), "high heat and humidity. action is ");
```

## API shape

- no_std by default
- fixed-capacity data structures where strings/payloads are needed
- explicit receipt/schema constants where the crate crosses firmware/host boundaries
- small public API intended to be readable in firmware reviews

## Verification

From the workspace root:

```bash
cargo test -p ri-esp-proof
cargo +esp check -p ri-esp-proof --target xtensa-esp32s3-none-elf -Z build-std=core,alloc
```

This crate is also covered by the workspace gate:

```bash
cargo test --workspace
```

## Integration path

Use this crate as one boundary in the larger ESP32 physical-AI stack:

```text
sensor reading -> deterministic policy -> local-language receipt -> proof receipt -> display/log/optional escalation
```

## Claim boundary

This crate is a reusable building block. It does not certify physical wiring, safety-critical behavior, production deployment, or model quality outside tests/receipts stated in the repository.

## License

MIT OR Apache-2.0
