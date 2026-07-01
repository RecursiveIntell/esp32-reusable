# ri-esp-local-language

ESP32-S3 local-language contracts.

![ri-esp-local-language architecture](assets/architecture.svg)

## What it provides

canonical prompt/output table, S3 receipt, integration receipt, OLED text.

It depends on `ri-esp-policy` and `heapless`; it represents language receipts, not policy.

## Install

```toml
[dependencies]
ri-esp-local-language = "0.1"
```

## Quick start

```rust
use ri_esp_local_language::{canonical_output, S3LanguageReceipt};
use ri_esp_policy::PromptId;

assert_eq!(canonical_output(PromptId::SafeAction), "no claim without evidence.");
let r = S3LanguageReceipt::static_passed(PromptId::SafeAction);
assert_eq!(r.output.as_str(), "no claim without evidence.");
```

## API shape

- no_std by default
- fixed-capacity data structures where strings/payloads are needed
- explicit receipt/schema constants where the crate crosses firmware/host boundaries
- small public API intended to be readable in firmware reviews

## Verification

From the workspace root:

```bash
cargo test -p ri-esp-local-language
cargo +esp check -p ri-esp-local-language --target xtensa-esp32s3-none-elf -Z build-std=core,alloc
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
