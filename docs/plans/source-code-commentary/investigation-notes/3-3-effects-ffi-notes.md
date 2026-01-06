# 第3部 第10章: エフェクトとFFI実行 調査メモ

## 参照した資料
- `compiler/frontend/src/effects/mod.rs:1-3`
- `compiler/frontend/src/effects/diagnostics.rs:1-150`
- `compiler/frontend/src/diagnostic/effects.rs:1-260`
- `compiler/frontend/src/typeck/env.rs:160-360`
- `compiler/frontend/src/typeck/capability.rs:124-210`
- `compiler/frontend/src/typeck/driver.rs:1030-1068`
- `compiler/frontend/src/typeck/driver.rs:8394-8461`
- `compiler/frontend/src/typeck/driver.rs:9304-9434`
- `compiler/frontend/src/streaming/flow.rs:11-160`
- `compiler/frontend/src/bin/reml_frontend.rs:545-705`
- `compiler/frontend/src/bin/reml_frontend.rs:3600-3639`
- `compiler/frontend/src/ffi_executor.rs:1-51`
- `compiler/runtime/src/ffi/mod.rs:1-3`
- `compiler/runtime/src/ffi/dsl/mod.rs:1-805`
- `docs/spec/1-3-effects-safety.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`

## 調査メモ

### `effects` モジュールは診断ユーティリティ中心
- `compiler/frontend/src/effects/mod.rs` は `diagnostics` の再公開のみで、効果推論本体は他モジュールに散在している。(`compiler/frontend/src/effects/mod.rs:1-3`)
- `EffectDiagnostic` は `CapabilityMismatch` を受け取り、診断 `extensions` と `audit_metadata` に `effect.stage.*` / `capability.*` キーを整形して追加する。(`compiler/frontend/src/effects/diagnostics.rs:6-109`)

### 効果ステージ文脈の構築
- `StageContext` は実行時ステージ (`runtime`) と必要ステージ (`capability`) の 2 本立てで保持し、`StageTraceStep` を内包する。(`compiler/frontend/src/typeck/env.rs:185-243`)
- `StageContext::resolve` は CLI/環境変数/能力レジストリから既定ステージを決め、`build_stage_trace` によって「なぜこのステージになったか」を追跡する。(`compiler/frontend/src/typeck/env.rs:203-243`)
- `StageRequirement` は `Exact` と `AtLeast` を持ち、`satisfies` で実行時ステージが要件を満たすか評価する。(`compiler/frontend/src/typeck/env.rs:317-355`)
- `RuntimeCapability` は CLI 由来の `capability@stage` 表記をパースし、StageContext の `runtime_capabilities` として渡される。(`compiler/frontend/src/typeck/capability.rs:155-210`)

### 効果使用の収集と Capability 検証
- `collect_perform_effects` は `ExprKind::PerformCall` を中心に AST を深く走査し、効果名と span を `EffectUsage` に集約する。(`compiler/frontend/src/typeck/driver.rs:9304-9400`)
- `detect_capability_violations` は収集した効果を `CapabilityDescriptor` に解決し、`StageRequirement::merged_with` を使って要求ステージを合成する。(`compiler/frontend/src/typeck/driver.rs:8394-8450`)
- 実行時ステージが満たない場合は `TypecheckViolation::stage_mismatch` を生成し、満たしているが Capability が未提供なら `residual_leak` を生成する。(`compiler/frontend/src/typeck/driver.rs:8441-8458`)
- `TypecheckViolation::stage_mismatch` は `CapabilityMismatch` を内包し、後段の診断メタデータ生成に利用される。(`compiler/frontend/src/typeck/driver.rs:1030-1063`)

### 診断・監査メタデータへの展開
- `EffectAuditContext` / `StageAuditPayload` は `StageContext` と `RuntimeCapability` を JSON 拡張へ展開するためのコンテナで、`CapabilityRegistry` のメタ情報も付加する。(`compiler/frontend/src/diagnostic/effects.rs:46-250`)
- `apply_extensions` は `effects` / `bridge` / `capability` など複数キーに同じ情報を展開し、監査ログと診断 JSON の両方で参照できるようにする。(`compiler/frontend/src/diagnostic/effects.rs:253-350`)
- CLI は起動時に `StageAuditPayload` を作り、パイプライン開始時・実行中の診断生成時に再利用する。(`compiler/frontend/src/bin/reml_frontend.rs:545-705`)
- 型検査違反を診断化する際に `CapabilityMismatch` があると `EffectDiagnostic::apply_stage_violation` を呼び、`effect.stage.required` などのキーを拡張に注入する。(`compiler/frontend/src/bin/reml_frontend.rs:3600-3639`)

### RuntimeBridgeSignal との連携
- `RuntimeBridgeSignal` はストリーミング実行に紐づく bridge 情報を保持する構造体で、`EffectAuditContext` に統合できる。(`compiler/frontend/src/streaming/flow.rs:34-76`, `compiler/frontend/src/diagnostic/effects.rs:46-206`)
- 現状の CLI 実装では `StreamFlowState::latest_bridge_signal` を読むだけで、シグナル生成元は見当たらないため、通常は `None` のままになる。(`compiler/frontend/src/bin/reml_frontend.rs:688-701`, `compiler/frontend/src/streaming/flow.rs:154-160`)

### FFI 実行エンジンの最小実装
- `install_cli_ffi_executor` は `FfiCallExecutor` を登録し、既に登録済みのエラーは無視する。(`compiler/frontend/src/ffi_executor.rs:7-20`)
- `CliFfiExecutor` の実装は `libm::cos` だけを特別扱いし、それ以外は `ffi.call.failed` のエラーで返す。(`compiler/frontend/src/ffi_executor.rs:23-50`)
- `reml_frontend` の `main` で `install_cli_ffi_executor` を呼び、CLI で FFI 実行基盤を初期化する。(`compiler/frontend/src/bin/reml_frontend.rs:545-548`)

### Core.Ffi.Dsl のランタイム API
- `FfiType` / `FfiFnSig` / `FfiRawFn` / `FfiWrappedFn` が DSL の中心となる型で、`FfiCallSpec::to_signature` が MIR 由来の文字列型を解析する。(`compiler/runtime/src/ffi/dsl/mod.rs:24-168`, `296-564`)
- `FfiRawFn::call` は `call_handler` → `FFI_CALL_EXECUTOR` の順で実行を試み、未登録なら `ffi.call.executor_missing` を返す。(`compiler/runtime/src/ffi/dsl/mod.rs:331-343`)
- `FfiWrappedFn::call` は引数数・型・NULL・所有権を検証し、違反時に `ffi.wrap.*` 系のエラーを返す。(`compiler/runtime/src/ffi/dsl/mod.rs:438-551`)
- `FfiError` は診断コード・拡張メタデータ・監査メタデータを保持し、`GuardDiagnostic` へ変換可能。(`compiler/runtime/src/ffi/dsl/mod.rs:648-713`)
- `insert_call_audit_metadata` は `ffi.call` 監査イベントのスキーマを整形する。(`compiler/runtime/src/ffi/dsl/mod.rs:760-805`)

### 仕様との対応
- 効果タグと安全境界は `docs/spec/1-3-effects-safety.md` の効果分類・unsafe 境界と対応する。
- FFI の DSL・ラッパ設計は `docs/spec/3-9-core-async-ffi-unsafe.md` の §2.4.1 と整合しているが、CLI 実行エンジンは最小実装であり、仕様にある `call_with_capability` 等の機能は未実装。

### 未確認事項 / TODO
- `RuntimeBridgeSignal` を生成する実際の実装箇所を確認し、監査メタデータへの反映が実運用で発生するかを把握する。
- FFI 実行エンジンが `compiler/runtime` 側でどのように差し替えられるか（CLI 以外のエントリポイント）を追跡する。
