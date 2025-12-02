# effect_handler テレメトリ確認 (2029-07-25)

## `scripts/poc_dualwrite_compare.sh effect_handler --trace`

- コマンド: `scripts/poc_dualwrite_compare.sh --mode diag --run-id 20290725-effect-stage-rust --cases tmp/effect_handler_case.txt`
- 入力: `compiler/ocaml/tests/golden/effects/effect_handler_demo.reml`
- 主要フラグ: Rust 側 `--trace --emit-audit-log --experimental-effects --type-row-mode dual-write --effect-stage beta`
- 成果物: `reports/dual-write/front-end/w4-diagnostics/20290725-effect-stage-rust/effect_handler/diagnostics.{ocaml,rust}.json`
- `effects-metrics.rust.err.log` と `typeck-debug.metrics.err.log` は既知の P2 ギャップ (`typeck_debug_match`) により Warning を出力するが、`extensions.effects.stage.trace` には CLI/Env/Runtime を通じた Stage Trace が揃い `bridge.stage.*` と同期している。

## `stage_violation.reml` での CapabilityMismatch 検証

- コマンド: `cargo run --quiet --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json tmp/stage_violation.reml`
- 入力: `tmp/stage_violation.reml`

```reml
fn main() = perform Console 1
```

- 生成診断: `effects.contract.stage_mismatch`（`reports/spec-audit/ch1/stage_violation-20290725-diagnostics.json`）
- 追加されたキー:
  - `extensions.effects.stage.mismatch` と `extensions.effect.capability = "console"`
  - `audit_metadata["capability.id"] = "console"`、`capability.expected_stage = "at_least:beta"`, `capability.actual_stage = "at_least:stable"`
  - `metadata["capability.mismatch"] = {"expected":"at_least:beta","actual":"at_least:stable"}`
- `EffectDiagnostic::apply_stage_violation` により `effect.stage.required/actual` と `AuditEnvelope.metadata` が同一内容で同期され、`StageAuditPayload` と独立した Capability 単位の差分追跡が可能になった。

これにより 3.6 計画の 4.1 節で要求されていた Stage/Capability テレメトリの注入を Rust Frontend で確認できた。
