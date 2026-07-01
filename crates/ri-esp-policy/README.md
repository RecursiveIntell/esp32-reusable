# ri-esp-policy

Deterministic sensor policy.

![ri-esp-policy architecture](assets/architecture.svg)

## What it provides

sensor readings to canonical local-language PromptId decisions.

It depends on `ri-esp-core` only and is safe to use from firmware and host tests.

## Install

```toml
[dependencies]
ri-esp-policy = "0.1"
```

## Quick start

```rust
use ri_esp_core::EnvironmentReading;
use ri_esp_policy::{PolicyContext, SensorPolicy, PromptId};

let ctx = PolicyContext::new(EnvironmentReading::new(31.0, 70.0, None), 3_000)
    .with_last_read_ms(2_900);
let d = SensorPolicy::default().evaluate(ctx);
assert_eq!(d.prompt_id, PromptId::HighHeatHumidity);
assert_eq!(d.prompt(), "high heat and humidity. action is ");
```

## API shape

- no_std by default
- fixed-capacity data structures where strings/payloads are needed
- explicit receipt/schema constants where the crate crosses firmware/host boundaries
- small public API intended to be readable in firmware reviews

## Verification

From the workspace root:

```bash
cargo test -p ri-esp-policy
cargo +esp check -p ri-esp-policy --target xtensa-esp32s3-none-elf -Z build-std=core,alloc
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
