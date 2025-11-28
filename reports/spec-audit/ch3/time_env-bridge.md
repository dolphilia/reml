# Core.Time × Core.IO Env ブリッジ検証 (2025-12-12)

- 変更内容
  - `compiler/rust/runtime/src/io/env.rs` を追加し、`TZ` / `LC_TIME` / `LANG` からタイムゾーン・ロケールのヒントを収集する `time_env_snapshot()` を実装。
  - `TimeError` に `with_env_snapshot` を追加し、`time.env.timezone` / `time.env.locale` メタデータを診断・監査へ転写。
  - `timezone.rs` の Capability 検証・オフセット検証で環境スナップショットを添付し、`collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_iana.json` から `time.env.*` を観測できるようにした。
- 検証
  - `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core_time time::tests::time_error_includes_env_snapshot_metadata`
  - `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_iana.json --output reports/spec-audit/ch3/time_timezone_lookup-env.json`
- 期待される監査メタデータ
  - `time.env.timezone` = `TZ` もしくは `None`
  - `time.env.locale` = `LC_TIME` もしくは `LANG`
  - 既存の `time.platform` / `time.timezone` と同じキー空間に統合
