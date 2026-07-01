# ri-esp-proof receipt schema v1

Schema id: `ri_esp_proof_receipt_v1`

A proof receipt records why an ESP32/S3 physical-world node stayed local or escalated to an AI tier. Current receipts also carry the local-language model/prompt selected for the OLED/status phrase path.

Required fields:

- `schema`: constant `ri_esp_proof_receipt_v1`
- `event_id`: monotonic per-device event counter
- `device_id`: stable board/node id
- `timestamp_ms`: device uptime or observed timestamp in milliseconds
- `decision`: `local_only` or `forward_to_ai`
- `reason`: compatibility string, same value as `route_reason`
- `route_reason`: one of:
  - `local_confident`
  - `low_confidence`
  - `sensor_missing`
  - `stale_reading`
  - `temperature_and_humidity_out_of_range`
  - `temperature_out_of_range`
  - `humidity_out_of_range`
  - `unsupported_cold_or_dry`
  - `operator_request`
  - `schedule_tick`
  - `anomaly_match`
- `sentinel_confidence`: 0.0-1.0 local sentinel/proof confidence
- `ai_route`: model/server route label, e.g. `gemma4:12b@ollama`
- `local_language_model`: local phrase model label, currently `esp32s3_h320_p15`
- `local_language_prompt`: canonical deterministic prompt selected for the local-language layer
- `sensor`: object with `temperature_c`, `humidity_pct`, `heat_index_c`; values may be `null`

Claim boundary:

This receipt is route/provenance evidence. It is not a safety certification, does not prove model answer quality, and does not physically certify OLED pixels without attached display hardware.
