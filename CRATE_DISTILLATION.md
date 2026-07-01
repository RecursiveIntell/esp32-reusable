# ESP32/ESP32-S3 reusable crate distillation

Date: 2026-06-30

## Inventory checked

1. `esp32s3-edge-ai/edge-ai-starter`
   - early esp-hal/MicroFlow scaffold
   - mostly bootstrap and sine inference proof
   - value: canonical no_std project skeleton and build.rs pattern

2. `tiered-edge-ai/esp32-s3`
   - ESP32-S3 no_std binaries: sentinel, WiFi connect, inference smoke, trained char LM, LLM limit bench
   - value: WiFi/embassy bring-up, spec-core transformer smoke, compressed KV benchmark patterns, trained q8 RNN pattern

3. `tiered-edge-ai/bridge`
   - std Rust bridge types: tier config, quant scheme, feature transfer binary protocol
   - value: should be converted to no_std/fixed-capacity payloads for firmware reuse

4. `dying-llm`
   - original ESP32 firmware for ESP32-2432S028 with ILI9341 TFT over bit-bang SPI
   - value: working board profile, bit-bang TFT fallback, trained RNN loop, display primitives

5. `dying-llm-esp32s3-i2c`
   - ESP32-S3 firmware using HD44780 20x4 over PCF8574 I2C
   - value: generic LCD driver, I2C pin/pullup lessons, RNN loop for S3

6. `esp32-sensor-hub`
   - PlatformIO/Arduino ESP32 sensor node: DHT, SSD1306 OLED, WiFi fallback, HTTP post, `/ai` forward to GTX 1070 endpoint
   - value: product-shaped sensor/AI endpoint contract, but Arduino C++ rather than Rust

## Strong crate boundaries

### 1. `ri-esp-core`
Keep. This should be the tiny stable base crate.

Owns:
- `SensorReading`
- `EnvironmentReading`
- `DeviceStatus`
- `Display` trait
- `Sensor` trait
- timing/status enums

Must stay `#![no_std]`, dependency-light, no esp-hal dependency.

### 2. `ri-esp-board-profiles`
Keep. Hardware knowledge is currently scattered in memory/docs/comments.

Owns:
- original ESP32 dev board profile
- ESP32-2432S028 ILI9341 profile
- ESP32-S3 WROOM-1 N16R8 safe I2C pins
- constants for known-bad pins like S3 GPIO8/9 for octal PSRAM

This avoids re-learning board pin traps every firmware.

### 3. `ri-esp-display`
Keep. Highest immediate reuse value.

Owns:
- HD44780 20x4 via PCF8574 I2C
- ILI9341 bit-bang SPI command/window/fill/text primitives
- optional small display widgets: status line, progress bar, scrolling text

Do not bind this crate to esp-hal `Output`; use `embedded-hal` traits so both ESP32 and ESP32-S3 callers can pass their GPIO outputs.

### 4. `ri-esp-llm`
Keep. This is the reusable embedded-AI core, separate from spec-core.

Owns:
- int4 pack/unpack
- fixed-capacity KV cache
- softmax/argmax/sampling
- q8 char-RNN inference helper
- LCG RNG
- lifespan/degrade/rebirth policy helpers

Boundary with `spec-core`:
- `spec-core` remains transformer-specific.
- `ri-esp-llm` provides generic embedded inference utilities and small-RNN helpers.

### 5. `ri-esp-tiered`
Keep. Convert `tiered-edge-ai/bridge` into firmware-safe wire types.

Owns:
- fixed-size feature transfer payloads
- sensor context payloads
- confidence/wake decisions
- no_std binary encode/decode

No `Vec`, no serde requirement in default firmware mode. Add optional `std`/`serde` later for host tools.

### 6. `ri-esp-policy`
Created 2026-07-01 after the sensor-policy -> S3 hard test.

Owns:
- deterministic sensor-state -> canonical prompt mapping
- freshness/stale handling
- missing sensor handling
- hot/humid/cold/dry/normal classification
- policy reasons and route hint confidence

Boundary:
- This crate decides policy.
- It does not generate language.
- It should remain no_std and depend only on `ri-esp-core`.

### 7. `ri-esp-local-language`
Created 2026-07-01 after real ESP32-S3 H320 p15 hard-test receipts.

Owns:
- `ri_esp32s3_local_language_v1` schema constants
- `ri_sensor_policy_to_s3_language_integration_v1` schema constants
- canonical prompt/output table proven by the S3 hard test
- fixed-capacity `S3LanguageReceipt` and `IntegrationReceipt`
- OLED-ready `oled_text` contract

Boundary:
- This crate records/represents S3 local-language outputs.
- It does not choose policy.
- It does not certify model behavior outside canonical prompts.

### 8. `ri-esp-net`
Not created yet. Hold until API stabilizes.

Reason: two networking worlds exist:
- Arduino/PlatformIO: mature WiFi + HTTP now
- Rust no_std esp-radio/embassy: promising but volatile
- Rust std esp-idf-svc: likely best Rust path for WiFi-heavy sensor nodes

Create `ri-esp-net` only after deciding Rust std vs no_std for WiFi products.

## What to kill / not crate yet

- Do not create one mega `esp32-utils` crate. It will become a dependency junk drawer.
- Do not force Arduino C++ sensor hub into Rust crates yet; extract its data contract and behavior first.
- Do not publish crates until at least two firmware apps consume them.
- Do not make board profiles hide pin decisions too much; keep constants explicit and visible.

## Migration order

1. Move LCD + TFT drivers into `ri-esp-display`.
2. Move int4/KV/RNN/sampling duplicated code into `ri-esp-llm`.
3. Replace `tiered-edge-ai/bridge` `Vec` protocol with `ri-esp-tiered` fixed-capacity payloads.
4. Extract sensor-policy canonical prompt mapping into `ri-esp-policy`.
5. Extract ESP32-S3 H320 p15 prompt/output receipt contracts into `ri-esp-local-language`.
6. Update `ri-esp-proof` to carry local-language model/prompt fields and stale/hot+humid/safe route reasons.
7. Refactor `dying-llm` and `dying-llm-esp32s3-i2c` to consume `ri-esp-display` + `ri-esp-llm`.
8. Refactor `tiered-edge-ai/esp32-s3` smoke/bench binaries to use `ri-esp-llm` cache implementations.
9. Decide whether the sensor hub gets a Rust `esp-idf-svc` implementation or remains PlatformIO C++ with a shared JSON/wire spec.

## Success condition

A new ESP32/S3 firmware should start with:

- choose board profile
- choose display driver
- choose sensor/input type
- choose local-only vs tiered endpoint behavior
- wire 20-50 lines of app code

Not 300-500 lines of copied LCD/TFT/KV/RNN/WiFi boilerplate.
