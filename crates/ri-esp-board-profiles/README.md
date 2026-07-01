# ri-esp-board-profiles

Board profile constants and pin traps.

![ri-esp-board-profiles architecture](assets/architecture.svg)

## What it provides

ESP32/ESP32-S3 pin safety, PSRAM guardrails, board profile data.

It is dependency-light and designed for embedded reuse.

## Install

```toml
[dependencies]
ri-esp-board-profiles = "0.1"
```

## Quick start

```rust
// See crate docs/tests for complete examples.
```

## API shape

- no_std by default
- fixed-capacity data structures where strings/payloads are needed
- explicit receipt/schema constants where the crate crosses firmware/host boundaries
- small public API intended to be readable in firmware reviews

## Verification

From the workspace root:

```bash
cargo test -p ri-esp-board-profiles
cargo +esp check -p ri-esp-board-profiles --target xtensa-esp32s3-none-elf -Z build-std=core,alloc
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
