# High-ROI ESP32 / ESP32-S3 paths for the RecursiveIntell stack

Date: 2026-06-30

## Executive verdict

The highest-ROI path is not "largest local LLM on ESP32." The highest-ROI path is:

> ESP32/ESP32-S3 as a governed physical-world AI endpoint: sensors + local sentinel + display + receipt + escalation to Gemma/Ollama on GTX 1070.

This uses what the stack already has:

- `esp32-sensor-hub`: DHT/OLED/WiFi/HTTP `/ai` forwarding to GTX endpoint.
- `ri-esp-proof`: no_std route/receipt crate: sensor reading + sentinel confidence -> local/AI decision -> JSON receipt -> feature payload.
- `ri-esp-tiered`: fixed-size no_std feature transfer payloads.
- `ri-esp-llm`: int4/KV/RNN/sampling primitives.
- `spec-core`: no_std int4 transformer primitives and rollback-capable protocol.
- broader RecursiveIntell stack: provenance, semantic memory, quant-governor/poly-kv/turbo-quant/fib-quant patterns.

## Evidence checked

Local artifacts:

- `/home/sikmindz/projects/esp32-sensor-hub/README.md`
- `/home/sikmindz/projects/esp32-sensor-hub/docs/architecture.md`
- `/home/sikmindz/projects/esp32-reusable/README.md`
- `/home/sikmindz/projects/esp32-reusable/CRATE_DISTILLATION.md`
- `/home/sikmindz/projects/esp32-reusable/crates/ri-esp-proof/README.md`
- `/home/sikmindz/projects/spec-engine/SPEC.md`
- `/home/sikmindz/projects/tiered-edge-ai/README.md`

Semantic-memory facts checked:

- ESP32 sensor hub Gemma endpoint: `gemma4:12b` via Ollama on GTX 1070.
- Tiered Edge AI: ESP32-S3 sentinel + heavier inference tier.
- Spec Engine: no_std int4 transformer core + speculative protocol.
- ESP32-S3 LLM feasibility: realistic local LSTM/RNN > transformer for useful speed.
- ESP-NN status: only C kernels expose S3 SIMD; Rust lacks direct Xtensa SIMD intrinsics.

External GitHub API spot-checks:

- `TilelliLab/atome-lm`: 46 stars, Apache-2.0, pushed 2026-06-29. Ternary microcontroller LM, ESP32-WROOM-32 demo ~1 tok/s.
- `asad-shafi/Running-Tiny-Lanuage-Model-on-ESP32`: 6 stars, ESP32-S3 tiny LM via SynapEdge.
- `asad-shafi/synapedge`: 12 stars, Apache-2.0, ONNX-to-C embedded compiler.
- `beancookie/xiaoclaw`: 35 stars, MIT, ESP32-S3 voice assistant / local agent claims.
- `brangi/blitzed`: 3 stars, Rust training + INT8 C generation for ESP32 classifier-style models.
- `nimblegate/nimblecube`: 0 stars, no_std HDC sensor similarity memory for ESP32-S3.
- Searches for `esp32 compressed kv cache` returned no notable direct hits.
- Searches for `esp32 speculative decoding` returned generic/new repos, not a mature ESP32-specific stack.

## ROI ranking

### 1. Ship the Gemma4 physical sensor endpoint proof

Status: already partly built.

What it proves:

- ESP32/S3 gathers physical sensor context.
- Local proof layer decides whether to escalate.
- GTX 1070 runs `gemma4:12b` through Ollama.
- ESP32 displays result and/or exposes `/ai` endpoint.
- Receipt captures device, sensor state, route, model, response metadata.

Why high ROI:

- Uses hardware + model path already requested.
- Strong demo: physical world -> local device -> real LLM -> local display/API.
- Avoids weak claim of running a big model on ESP32.
- Differentiates from normal ESP32 cloud assistants because the sensor context and proof receipt are first-class.

Build next:

1. Add receipt fields from `ri-esp-proof` into the PlatformIO JSON path.
2. Ensure `/ai` endpoint includes a short operator prompt and latest sensor snapshot.
3. Run `tools/run_gemma_ai.sh` on GTX box with `OLLAMA_MODEL=gemma4:12b`.
4. Hit ESP32 `/ai` from LAN.
5. Save a machine-readable receipt with request, response, model name, latency, sensor values.

Acceptance gate:

- `curl http://ESP32_IP/ai -d '{"prompt":"what should I do about this room?"}'` returns a Gemma-backed response containing the actual sensor values.
- OLED/TFT shows at least `AI OK` or the first response line.
- Server logs receipt JSON with model=`gemma4:12b`.

Claim boundary:

- Safe: local-first sensor-conditioned AI endpoint.
- Not safe yet: production monitoring system, safety-critical control, model-quality superiority.

### 2. Make `ri-esp-proof` the reusable decision/receipt layer for every ESP32 node

Status: crate exists and tested.

Why high ROI:

- Converts one-off sensor demos into governed nodes.
- Reuses RecursiveIntell strengths: receipts, bitemporal/provenance thinking, no shadow truth.
- Independent of which sensor is attached.

Build next:

- Add `ProofReceiptV1` schema doc.
- Add route reasons: `low_confidence`, `sensor_missing`, `temperature_out_of_range`, `humidity_out_of_range`, `operator_request`, `schedule_tick`, `anomaly_match`.
- Add host-side parser so receipts can be archived by semantic memory later.

Acceptance gate:

- Host tests prove JSON round-trip and `FeatureTransfer` conversion.
- ESP target check passes:
  `cargo +esp check -p ri-esp-proof --target xtensa-esp32s3-none-elf -Z build-std=core,alloc`

### 3. Sensor anomaly sentinel using HDC / tiny non-neural memory

Status: not built in stack yet; external signal exists (`nimblecube`).

Why high ROI:

- Better fit for ESP32 than local LLM.
- Can run constantly with integer-only operations.
- Can trigger Gemma escalation when sensor pattern is unusual.
- It is aligned with the stack's memory/retrieval identity.

What to build:

- `ri-esp-sentinel` crate or module under `ri-esp-llm`/new `ri-esp-memory`.
- Binary hypervector encoder for sensor windows.
- Hamming-distance anomaly score.
- Output confidence to `ri-esp-proof`.

Use case:

- Temperature/humidity/gas/motion pattern becomes a compact binary fingerprint.
- If current window differs from learned normal patterns, escalate to Gemma.

Acceptance gate:

- Host test: normal synthetic windows stay local; anomalous windows forward to AI.
- ESP check passes.
- Hardware demo sends only anomalous windows to GTX endpoint.

### 4. Multi-sensor schema and plug-in sensor registry

Status: sensor hub currently top-level temp/humidity fields; docs already say future sensors should move under `sensors` object.

Why high ROI:

- User explicitly wants "lots of sensors at different times."
- Prevents every new sensor from causing firmware/API rewrites.
- Makes Gemma endpoint sensor-agnostic.

Build next:

JSON shape:

```json
{
  "device_id":"esp32-sensor-hub-01",
  "sensors": {
    "dht22": {"temperature_c":29.5,"humidity_pct":71},
    "mq2": {"gas_raw":381},
    "pir": {"motion":false}
  },
  "proof": {...}
}
```

Crate shape:

- `ri-esp-core::SensorReadingKind`
- `ri-esp-core::SensorValue`
- fixed-capacity map/list for no_std payloads.

Acceptance gate:

- Add one fake second sensor in firmware payload without changing Gemma endpoint code.

### 5. ESP-NN FFI wrapper for Rust

Status: research says ESP-NN C kernels are the only practical way to access ESP32-S3 SIMD; Rust has no direct Xtensa vector intrinsics.

Why high ROI:

- Potentially ecosystem-useful beyond this project.
- Can accelerate LSTM/FC/classifier paths.
- Directly attacks the speed bottleneck.

Caveat:

- ESP-NN supports FC/conv/pooling/ReLU; not attention, transformer, LSTM cell as a high-level op.
- Useful first target is fully-connected gate matmul for LSTM or simple classifiers.

Build next:

- `ri-esp-nn-sys`: bindgen/static C wrapper for ESP-NN FC kernel.
- `ri-esp-nn`: safe no_std-ish Rust wrapper for int8 fully-connected.
- Benchmark against pure Rust matmul on ESP32-S3.

Acceptance gate:

- Same int8 FC output as reference within quantized tolerance.
- Hardware benchmark shows speedup over scalar Rust.

Claim boundary:

- Safe only after real hardware timing. Do not claim acceleration from compile alone.

### 6. Char-LSTM / small-RNN local sentinel model

Status: stack has RNN helpers and prior trained char-RNN/dying-LLM work.

Why high ROI:

- Feasible on ESP32-S3 in a way local transformers are not.
- Can be used as a tiny local classifier/sentinel, not chatbot.
- Avoids KV cache cost.

Best use:

- Local intent/signal classifier from compact sensor text/state.
- Local confidence generator for `ri-esp-proof`.
- Toy text generation only as demo/art, not core product value.

Build next:

- Train tiny char/LSTM classifier to emit `normal`, `watch`, `escalate` on serialized sensor context.
- Quantize to q8/q4 constants.
- Run in firmware loop.

Acceptance gate:

- Hardware log: inference latency + output class + no watchdog reset.
- Receipt includes local sentinel confidence and route decision.

### 7. Retrieval/n-gram speculation instead of neural ESP32 speculative decoding

Status: spec-core exists, but semantic memory notes show standard neural speculative decoding is likely counterproductive when ESP32 draft is slower than the verifier.

Why high ROI:

- Preserves the interesting protocol without wasting ESP32 compute.
- REST/n-gram speculation can be near-zero compute.
- Works well for repetitive device status text and tool/action phrases.

What to build:

- ESP32 sends candidate phrase IDs or short templates, not neural draft logits.
- Gemma/host verifies/expands into final answer.
- Use sensor state + previous phrases as retrieval keys.

Acceptance gate:

- Host benchmark: templated/n-gram suggestions accepted often enough to reduce response latency or token count.
- If no latency win, keep it as compression/template protocol, not speculation claim.

### 8. ESP32 as local tool endpoint / action target for Gemma

Status: `/status`, `/sensors`, `/ai` exist in firmware. Need reverse control path.

Why high ROI:

- Turns ESP32 from passive telemetry into an agent-actuated physical endpoint.
- Very aligned with RecursiveIntell operator-grade local agent work.

Build next:

- Host Gemma endpoint returns structured action proposals:
  - `display_message`
  - `set_led`
  - `sample_now`
  - `start_watch_mode`
- ESP32 accepts only allowlisted action commands.
- Receipt records action accepted/rejected and reason.

Acceptance gate:

- Gemma recommends a display message based on sensor context.
- Host sends command to ESP32.
- ESP32 displays it and logs action receipt.

Safety boundary:

- No control of heaters, locks, medicine, mains power, or safety-critical systems without separate hard interlocks.

### 9. Board-profile + display productization

Status: `ri-esp-board-profiles` and `ri-esp-display` exist; learned pin traps are valuable.

Why high ROI:

- Prevents repeated bring-up pain.
- Enables fast demos on original ESP32 and ESP32-S3.
- Display feedback makes demos feel real and debuggable.

Build next:

- Example firmware for:
  - ESP32-S3 + I2C OLED.
  - ESP32-2432S028 + ILI9341 bit-bang fallback.
- Small display widgets: WiFi, sensor status, AI route, response first line.

Acceptance gate:

- Same app logic works with OLED and TFT by swapping display implementation.

### 10. Receipt ingestion into semantic memory

Status: not wired end-to-end, but strongly aligned with the user's stack.

Why high ROI:

- Makes the physical world queryable over time.
- Differentiates from normal IoT dashboards.
- Creates evidence-backed local memory: "what did the sensor see and what did Gemma decide?"

Build next:

- Host receiver writes JSONL receipts.
- Batch ingester promotes selected receipts into semantic memory namespace `esp32-sensors`.
- Use bitemporal timestamps: sensor observed time + receipt ingested time.

Acceptance gate:

- Query semantic memory: "when did humidity exceed 70% and what did the AI recommend?"
- Returns receipt-backed facts, not a summary-only blob.

## Kill / postpone

### Kill as primary goal: largest on-device LLM

Reason:

- ESP32-S3 can store only ~7M int8 or ~14M int4 params in the best case, and useful runtime budget is smaller.
- Real local LLM quality will be weak.
- Better story is physical sensor endpoint + Gemma escalation.

### Postpone: full neural speculative decoding with ESP32 as draft for local GTX/UNO verifier

Reason:

- If GTX/Gemma is local and faster than ESP32, ESP32 neural draft does not speed it up.
- The compute ratio is wrong.
- Keep protocol pieces, but apply them to retrieval/template/n-gram speculation or remote/high-latency cases.

### Postpone: `ri-esp-net` crate

Reason:

- Network stack boundary is still unstable:
  - PlatformIO Arduino is practical now.
  - esp-radio/Embassy is promising but volatile.
  - esp-idf-svc is probably best for WiFi-heavy Rust products.

## Recommended next 7-day sequence

1. Finish Gemma4 ESP32 sensor endpoint proof.
   - Pull/verify `gemma4:12b` on GTX box.
   - Run `tools/run_gemma_ai.sh`.
   - Hit ESP32 `/ai` with a sensor-conditioned prompt.
   - Save receipt.

2. Add proof receipt to PlatformIO firmware payload.
   - Device ID, event ID, sensor state, route reason, model route.

3. Add multi-sensor schema.
   - Keep temp/humidity compatibility but add `sensors` object.

4. Add host receipt logger.
   - JSONL first. Semantic-memory ingestion second.

5. Build anomaly sentinel prototype.
   - Start HDC/integer memory; do not start with another transformer.

6. Add display status path.
   - OLED: `LOCAL`, `AI->`, `AI OK`, first response line.

7. Only after the demo works, decide Rust firmware migration.
   - PlatformIO is fine for proof.
   - Rust crates are the reusable substrate.

## Short claim-safe positioning

Safe claim after the next proof works:

> RecursiveIntell can turn ESP32/ESP32-S3 boards into local-first physical AI endpoints: sensor context is collected on-device, routed through a no_std proof/receipt layer, escalated to a local Gemma/Ollama GPU tier when needed, and logged as evidence-backed receipts.

Not safe yet:

- "first ESP32 LLM"
- "best ESP32 AI stack"
- "production-ready autonomous control"
- "faster than existing ESP32 AI systems"
- "large LLM running on ESP32"
