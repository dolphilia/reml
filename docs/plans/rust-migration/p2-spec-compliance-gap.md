# P2 バックエンド・ランタイム仕様未達リスト（2028-02 現状）

Phase P2（LLVM バックエンド統合・ランタイム連携）の成果物を `docs/spec` と `docs/plans/rust-migration/2-x` で定義された要件に照らして確認したところ、Rust 実装側では未処理もしくは PoC のまま停滞している領域が複数見つかった。本書では `2-0-llvm-backend-plan.md` / `2-1-runtime-integration.md` / `2-2-adapter-layer-guidelines.md` の対象範囲を三分割し、優先度の高いギャップを `P2G-XX` として整理する。

## 0. 参照資料

- 仕様: `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`, `docs/spec/3-10-core-env.md`
- 計画: `docs/plans/rust-migration/overview.md`, `docs/plans/rust-migration/2-0-llvm-backend-plan.md`, `docs/plans/rust-migration/2-1-runtime-integration.md`, `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md`
- 実装: `compiler/rust/backend/llvm/src/{codegen.rs,verify.rs}`, `compiler/rust/runtime/ffi/src/{lib.rs,registry.rs}`, `compiler/rust/backend/README.md`

各表には ID（`P2G-XX.YY` 形式）を付与し、差分項目を参照しやすくしています。

## 1. P2G-01: LLVM バックエンドのターゲット診断・ブリッジメタデータ不足

`docs/spec/3-6-core-diagnostics-audit.md:329-352` で定義された `DiagnosticDomain::Target`／`effects.contract.stage_mismatch` の監査キー、ならびに `docs/spec/3-9-core-async-ffi-unsafe.md:992-1017` で規定された `AuditEnvelope.metadata["bridge"]`・`reml.bridge.*` メタデータは LLVM lowering 側で生成することが求められる。しかし Rust バックエンドの `Verifier` は空モジュール検知のみで、ターゲット不一致・ABI/Stage 情報を一切出力していない（`compiler/rust/backend/llvm/src/verify.rs:85-140`）。`CodegenContext` も `Vec<String>` のメタデータしか持たず、モジュールフラグや `reml.bridge.stubs` を構築できない（`compiler/rust/backend/llvm/src/codegen.rs:89-193`）。

### P2G-01

| ID | 項目 | 仕様 / 計画 | Rust 現状 | 補うべき差分 |
| --- | --- | --- | --- | --- |
| `P2G-01.01` | ✅ ターゲット診断 | `target.profile.missing` や `target.config.mismatch` を `Diagnostic.extensions["target"]` と `AuditEnvelope.metadata["target"]` に記録する（`docs/spec/3-6-core-diagnostics-audit.md:989-152`）。 | `Verifier` が発行する診断は `llvm.module.empty` / `type.layout.invalid` などバックエンド内部専用のみで、`Target` ドメインのコード・拡張を含まない（`compiler/rust/backend/llvm/src/verify.rs:85-118`）。 | MIR/IR 差分検証から `RunConfigTarget`・`PlatformInfo` を受け取り、`TargetDiagnosticEmitter` を実装して `target.*` 診断と監査ログを生成する。`scripts/poc_dualwrite_compare.sh` へ `--emit-target-diag` を追加し、OCaml 版と同じ JSON を保存する。 |
| `P2G-01.02` | ✅ ブリッジメタデータ | LLVM lowering で `reml.bridge.version` フラグ・`reml.bridge.stubs` metadata を出力し、`bridge.stub_index`／`bridge.callconv` などを `AuditEnvelope` と突き合わせる（`docs/spec/3-9-core-async-ffi-unsafe.md:1006-1017`、`docs/plans/rust-migration/2-0-llvm-backend-plan.md:52-80`）。 | `ModuleIr` は単なる説明文字列を保持するのみで、LLVM モジュールフラグや Named Metadata を扱っていない。`Verifier` でも `opt -verify`／`llc` 実行ログを検証していない。 | `TargetMachine` に `module_flags`／`named_metadata` を追加し、`inkwell` or `llvm-sys` を用いて実際の IR を生成。`generate_snapshot` で `opt -verify`／`llc` を呼び出し、成功可否・ABI 情報を監査ログへ記録する。`reml.bridge.stubs` を差分ハーネスから比較できるよう JSON スキーマを固定する。 |
| `P2G-01.03` | ✅ DataLayout/ABI スキーマ | `docs/plans/rust-migration/2-0-llvm-backend-plan.md:101-150` の完了条件では Linux/macOS/Windows 3 ターゲットで `TargetMachine::{Triple,DataLayout,RelocModel}` を OCaml 実装と一致させ、`AuditEnvelope.metadata["backend"]` に保存する必要がある。 | `TargetMachineBuilder` は 4 トリプル・静的な System V レイアウトしか持たず、 Windows AAPCS64 等の `DataLayout` 生成・`CPU/features` 切替をサポートしない（`compiler/rust/backend/llvm/src/target_machine.rs:1-120`）。 | `TargetMachineBuilder` に `from_profile(RunConfigTarget)` 相当のヘルパを追加し、OCaml 版の `TargetSpec` からトリプル・CPU 名・feature string・`DataLayout` を自動転写する。トリプルごとに `msvc`/`gnu` を自動判別し、`collect-iterator-audit-metrics.py` が参照する監査フィールド（`backend.triple`, `backend.abi`）を構築する。 |

## 2. P2G-02: Capability Registry の型付きハンドル・効果域チェック未実装

`docs/spec/3-8-core-runtime-capability.md:17-70` は `CapabilityHandle` を GC/IO/Async 等の列挙体で公開し、`verify_capability_stage` が Stage だけでなく `effect_scope` も検証すると定めている（`同:137-160`）。`docs/plans/rust-migration/2-1-runtime-integration.md:33-38` でも同じ API が P2 成果物として指定されている。現状の `compiler/rust/runtime/ffi/src/registry.rs` では `CapabilityHandle` が単なる `CapabilityDescriptor` のコピーであり、`effect_scope` を無視した Stage 判定しか行っていない（`registry.rs:96-206`）。`verify_conductor_contract` も存在せず、DSL/ランタイム契約をまとめて検証できない。

### P2G-02

| ID | 項目 | 仕様 / 計画 | Rust 現状 | 補うべき差分 |
| --- | --- | --- | --- | --- |
| `P2G-02.01` | ✅ CapabilityHandle | 各 Capability ごとの型付きバリアントを提供し、呼び出し側が `GcCapability` 等の API に安全にアクセスできる（`docs/spec/3-8-core-runtime-capability.md:50-70`）。 | `CapabilityHandle` は `CapabilityDescriptor` をラップするだけで、実体や関数テーブルを保持しない。結果として FFI 層は `CapabilityId` ベースの分岐を都度実装する必要がある。 | `CapabilityHandle` を `enum CapabilityHandle { Gc(GcCapability), ... }` として再実装し、`register` で型ごとのストレージへ格納する。ハンドルに `descriptor()` と Capability 固有 API を実装し、Rust 側の各モジュールが `match` で型安全に操作できるようにする。 |
| `P2G-02.02` | ✅ effect_scope 検査 | `verify_capability_stage` / `verify_conductor_contract` は Stage と効果タグの両方を照合し、違反時は `CapabilityError::StageViolation` に `required_stage`・`effect_scope` を添付する（`docs/spec/3-8-core-runtime-capability.md:137-160`）。 | `verify_capability_stage` では Stage のみ検査しており、`effect_scope` に要求された効果タグが含まれているかを確認していない (`registry.rs:181-206`)。 | `StageRequirement` 判定後に `required_effects` を受け取って `effect_scope` との包含関係を検査する。効果不足時は `CapabilityError::EffectViolation`（新設）を返し、`docs/spec/3-6-core-diagnostics-audit.md:329-337` の `effects.contract.stage_mismatch` に必要な `effect.capability` 情報を添付する。 |
| `P2G-02.03` | 契約検証 API | DSL / conductor 契約をまとめて検証する `verify_conductor_contract`、`manifest_path` を参照した監査メタデータ出力を提供する（`docs/spec/3-8-core-runtime-capability.md:141-156`）。 | API が存在しないため、`docs/plans/rust-migration/1-3-dual-write-runbook.md` が要求する DSL マニフェスト照合を Rust 実装で実行できない。 | `ConductorCapabilityContract` 型・`verify_conductor_contract` を追加し、`manifest_path`／`StageRequirement`／`effect_scope` をまとめて照合して `AuditEnvelope.metadata["effect.*"]` を生成する。 |

## 3. P2G-03: FFI 契約診断・監査ログのキー不足

`docs/spec/3-9-core-async-ffi-unsafe.md:992-1014` は `ffi.contract.*` 診断と `AuditEnvelope.metadata.bridge` の必須キー一覧（`status`, `target`, `abi`, `ownership`, `extern_symbol`, `return` など）を定めている。さらに `docs/spec/3-6-core-diagnostics-audit.md:329-337` では Stage 違反時に `effect.stage.required` 等を監査に残すことを要求しており、`docs/plans/rust-migration/2-1-runtime-integration.md:5-29` でも `AuditEnvelope.metadata.bridge.*` の生成が完了条件となっている。現状の `compiler/rust/runtime/ffi/src/lib.rs` では `audited_bridge_call` がステータスと Stage だけを記録し（`lib.rs:345-389`）、`record_bridge_with_metadata` も `TODO` のままになっている（`lib.rs:339-343`）。`ffi.contract.*` 診断や `expected_abi`／`extern_symbol`／`return` メタデータは一切出力されていない。

### P2G-03

| ID | 項目 | 仕様 / 計画 | Rust 現状 | 補うべき差分 |
| --- | --- | --- | --- | --- |
| `P2G-03.01` | 監査キー | `AuditEnvelope.metadata["bridge"]` へ `status`,`target`,`arch`,`abi`,`expected_abi`,`ownership`,`extern_symbol`,`return` 等を必須で記録する（`docs/spec/3-9-core-async-ffi-unsafe.md:1006-1014`）。 | `BridgeAuditMetadata` には `status`/`ownership`/`target`/`platform`/`abi`/`symbol` だけが含まれ、`expected_abi`・`extern_name`・`return` など仕様で要求されているキーが欠けている。`record_bridge_with_metadata` も `AuditContext` と連動していない（`lib.rs:212-343`）。 | `BridgeAuditMetadata` に `expected_abi`, `extern_symbol`, `link_name`, `return_info` を追加し、`audited_bridge_call` が `ffi.call.start/end` 以外に `ffi.call.result` を発行するよう拡張する。`record_bridge_with_metadata` を `AuditContext` に統合し、`collect-iterator-audit-metrics.py` が参照する JSON を生成する。 |
| `P2G-03.02` | 契約診断 | `ffi.contract.symbol_missing` / `ffi.contract.unsupported_abi` 等の診断を `docs/spec/3-6-core-diagnostics-audit.md` §2.4.3 に従って出力し、`Diagnostic.extensions["bridge"]` を共有する。 | Rust 側では契約検証ロジックがなく、`audited_bridge_call` は成功/失敗のみを返す。`ffi.contract.*` のコードや Stage 逸脱 (`effects.contract.stage_mismatch`) を検出する仕組みが実装されていない。 | `ffi_contract` モジュールを追加し、シンボル名・所有権・ABI を静的に検証する。違反時は `DiagnosticDomain::Runtime` で `ffi.contract.*` を生成し、`AuditContext` へ `expected_abi` と `effect.stage.required` を書き込む。ランタイム呼び出し前に `CapabilityRegistry` で Stage と effect_scope を再検証し、失敗した際は `effects.contract.stage_mismatch` を Rust 実装から直接発火する。 |
| `P2G-03.03` | 戻り値の所有権 | 仕様では Borrowed/Transferred の戻り値処理を `return` メタデータに必須フィールドとして記録する（`docs/spec/3-9-core-async-ffi-unsafe.md:1012-1014`）。 | `acquire_borrowed_result` / `acquire_transferred_result` は存在するが、呼び出し側がどちらを使用したかを `Audit` に残していない。 | `RuntimeString::to_bridge_metadata` と `ForeignPtr` ラッパを拡張し、戻り値処理の `wrap` / `release_handler` / `rc_adjustment` を `AuditContext` 経由で記録する。 |

## 4. P2G-04: アダプタ層・環境ターゲット推論の欠落

`docs/spec/3-10-core-env.md:1-152` は `get_env`／`set_env`／`infer_target_from_env`／`resolve_run_config_target` の API と `DiagnosticDomain::Target` の連携を定義し、`docs/plans/rust-migration/2-2-adapter-layer-guidelines.md:24-48` では `compiler/rust/adapter/` を主成果物に挙げている。現在の Rust ツリーには `backend/`, `frontend/`, `runtime/` しか存在せず、環境アクセスやターゲット推論を担うアダプタ層が未着手である。結果として `RunConfig.extensions["target"]` の構築や `target.config.*` 診断の発火を Rust 実装だけで完結できない。

### P2G-04

| ID | 項目 | 仕様 / 計画 | Rust 現状 | 補うべき差分 |
| --- | --- | --- | --- | --- |
| `P2G-04.01` | 環境 API | `get_env` / `set_env` / `remove_env` が `EnvError`（`EnvErrorKind` 付き）を返し、変更系は `AuditEvent::EnvMutation` を残す（`docs/spec/3-10-core-env.md:14-41`）。 | Rust 実装に対応するモジュールがなく、環境操作は `std::env` を直接呼ぶ想定すら記述されていない。 | `compiler/rust/adapter/env.rs`（仮）を追加し、`Result<T, AdapterError>` で Specification と同じエラー型を再現する。`AuditContext` を受け取って `env.operation`／`env.key` を記録し、CI から JSON を確認できるようにする。 |
| `P2G-04.02` | ターゲット推論 | `infer_target_from_env`・`resolve_run_config_target`・`merge_runtime_target` を実装し、`RunConfig.extensions["target"]` と `Diagnostic.extensions["cfg"]` を同期させる（`docs/spec/3-10-core-env.md:120-152`、`docs/plans/rust-migration/2-2-adapter-layer-guidelines.md:24-33`）。 | `RunConfigTarget` を Rust 側で構築する仕組みがないため、Rust バックエンドへターゲット三要素（os / arch / abi）を渡せない。 | `adapter::target` モジュールを新設し、`REML_TARGET_*` 環境変数から `TargetProfile` を生成 → `RunConfigTarget` へ昇格 → `DiagnosticDomain::Target` の差分比較を出力する。P2 の `scripts/poc_dualwrite_compare.sh` から呼び出し、OCaml 実装と同じ JSON/監査形式を保存する。 |
| `P2G-04.03` | サブシステム API | FS/Network/Time/Random/Process それぞれで `Capability` / `effect` ラベル / 監査キーを維持するのが完了条件（`docs/plans/rust-migration/2-2-adapter-layer-guidelines.md:34-48`）。 | `compiler/rust/` 下に `adapter/` ディレクトリが存在せず、計画書で挙げられた成果物が未着手。 | `cargo new compiler/rust/adapter` でワークスペースを追加し、サブモジュールごとに API スケルトンとテスト（`adapter/fs.rs` 等）を配置する。`docs/spec/3-10-core-env.md` の効果タグを表すトレイト境界を定義し、`CapabilityRegistry` と Stage 連携させる。 |

---

上記 `P2G-01`〜`P2G-04` は Phase P2 の Go/No-Go 判定に直結するため、各項目について以下の優先順位で作業を進めることを推奨する。

1. **バックエンド診断整備（P2G-01）**: dual-write 比較に直結し、`opt -verify` / `llc` の失敗を CI で捕捉する基盤を整える。
2. **Capability/FFI 契約（P2G-02 / P2G-03）**: Stage 逸脱や `ffi_bridge.audit_pass_rate` を Rust 実装でも測定できるようにし、OCaml 実装と同じ監査データを生成する。
3. **アダプタ層（P2G-04）**: `RunConfig.extensions["target"]` と `DiagnosticDomain::Target` を Rust パスで完結させ、Phase 3 の CI 計画 (`3-0-ci-and-dual-write-strategy.md`) に渡す。

進捗を更新する際は `docs/migrations.log` に主要ファイルの追加・移動を追記し、`docs/plans/rust-migration/2-3-p2-backend-integration-roadmap.md` の成果物一覧に完了状況をリンクさせる。

## 5. 具体的な計画

### P2G-01.01

- `docs/spec/3-6-core-diagnostics-audit.md` §7 および `docs/spec/3-10-core-env.md` で定義された `target.profile.missing` / `target.config.mismatch` は `DiagnosticDomain::Target` の差分を表す最重要コードであり、`extensions["target"]` に `profile_id`・`triple`・`compared_with` を、`AuditEnvelope.metadata["target"]` に `requested`/`detected` のオブジェクトを記録することが求められている。まずは OCaml 側の `RunConfigTarget` → `PlatformInfo` 比較処理を洗い出し、Rust 側が保持すべき値（`os`/`arch`/`family`/`abi`/`runtime_revision` など）と `Diagnostics`/`Audit` に載せる粒度を明文化する。
- `compiler/rust/backend/llvm/src/verify.rs` に新たな `TargetDiagnosticEmitter`（たとえば `target_diagnostics.rs`）を置き、`RunConfigTarget` と `PlatformInfo` を入力に `target.*` 診断と `AuditLog` エントリを生成する。診断は `target.profile.missing`（`RunConfigTarget` に対応する `TargetProfile` が未設定または不一致）／`target.config.mismatch`（`PlatformInfo` の `os`/`arch`/`family` が期待値と異なる）を返し、`Diagnostic.extensions["target"]` に `profile_id`/`triple`/`detected` を添付する。`AuditLog` 側では `target.requested`・`target.detected`・`target.verdict`（`pass/fail`）を記録し、既存の `AuditEnvelope.metadata` との整合性を保つ。
- `TargetMachine` または `CodegenContext` を通じて `RunConfigTarget` の情報を `ModuleIr` に渡し、`Verifier::verify_module` で `TargetDiagnosticEmitter` を呼び出すタイミングを作る。`scripts/poc_dualwrite_compare.sh` に `--emit-target-diag` オプションを追加して、Rust/OCaml 双方のデータを `reports/dual-write/target-diag/<run-id>/diagnostics.{rust,ocaml}.json` に保存し、JSON Schema（`target.profile` / `target.detected` / `audit.metadata["target"]`）を差分ハーネスで比較する。
- 計画の検証は `scripts/poc_dualwrite_compare.sh --mode diag --emit-target-diag` で実行し、`reports/dual-write/target-diag/<run-id>` 配下に新しい `audit.` や `target-diag.json` を生成する。出力を `jq` などで OCaml と比較し、`target.profile.missing` や `target.config.mismatch` の存在とメタデータが仕様通りに添付されていることを確認する。差分が残る項目は新しい TODO コメントとして `reports/dual-write/README.md` へ記録し、次フェーズで `TargetMachine` の `Triple`/`DataLayout` 補間と併せて完了させる。

### P2G-01.02

- `docs/spec/3-9-core-async-ffi-unsafe.md:1017` が定める `reml.bridge.version = 1` と `reml.bridge.stubs` の Named Metadata に記録されるキー群（`bridge.stub_index`/`bridge.callconv`/`bridge.platform` など）は Rust 側の LLVM lowering が生成しない限り `AuditEnvelope.metadata["bridge"]` と突合できず、`ffi_bridge.audit_pass_rate` の算出がブロックされてしまう。仕様との齟齬を解消するには、各 FFI 呼び出しごとのスタブ情報を `TargetMachine`/`FfiCallSignature` から抽出してモジュール内に保持し、最終的な JSON 監査ログ・差分スナップショットに載せる必要がある。
- そのため `compiler/rust/backend/llvm/src/bridge_metadata.rs` に `BridgeMetadataContext` を追加し、`CodegenContext` が `MirFunction` の FFI 呼び出しを走査するたびに `bridge_metadata.record_stub` を呼び出してメタデータを蓄積する。蓄積された情報は `ModuleIr` に引き継がれ、`Verifier` の `AuditLog` と `BackendDiffSnapshot` の `bridge_metadata` エントリに展開されることで、Dual Write ハーネス（`scripts/poc_dualwrite_compare.sh --mode diag` など）において `reml.bridge.*` の文字列を OCaml と比較できるようにした。
- 次のフェーズではこの骨格を使って `scripts/poc_dualwrite_compare.sh` や `reports/dual-write` 側の JSON に `bridge_metadata` フィールドを加え、OCaml ゴールデンの `reml.bridge.stubs` と `ffi_bridge.audit_pass_rate` の値を常に比較できるようにする。また、`tooling/ci/collect-iterator-audit-metrics.py` で `audit.bridge.stub`／`audit.bridge.version` を読み取って `ffi_bridge.audit_pass_rate` を計測できるよう pipe することで、実際の ABI/所有権判定の差分が監査ゲートに反映されることを確認する。

### P2G-01.03

- `docs/plans/rust-migration/2-0-llvm-backend-plan.md:101-150` で OCaml 側 `target_config.ml` に記録された Triple/DataLayout/ABI の組み合わせ（System V ↔ Linux、MSVC/GNU ↔ Windows、darwin_aapcs64 ↔ macOS）を Rust 側にも写し、`AuditEnvelope.metadata["backend"]` に `backend.triple`/`backend.abi`/`backend.datalayout`/`backend.reloc_model` を載せる。差分検証（`reports/dual-write`）と `collect-iterator-audit-metrics.py` の監査要件はこれらのキーを前提としており、3 つの主要ターゲットで `TargetMachine::{Triple,DataLayout,RelocModel}` が OCaml 実装と一致することが Go/No-Go 条件となる。
- `compiler/rust/backend/llvm/src/target_machine.rs` に Ocaml の `target_config` 相当の `TargetSpec` を追加し、`RunConfigTarget`→`Triple`→DataLayout/CPU/feature/ABI を自動転写する `TargetMachineBuilder::from_run_config` を導入する。`RunConfigTarget` の `features` や `abi` を踏まえた文字列を `TargetMachine.features` と `backend_abi` にまとめ、`TargetDiagnosticContext`/`TargetDiagnosticEmitter` が正しい `backend.abi` を扱えるようにする。
- `compiler/rust/backend/llvm/src/verify.rs` の `Verifier` で `backend.triple`/`backend.abi`/`backend.datalayout`/`backend.reloc_model` を `AuditLog` に記録し、`compiler/rust/backend/llvm/src/integration.rs` の `BackendDiffSnapshot` に `backend_abi` を追加して JSON 出力へ含める。これにより `scripts/poc_dualwrite_compare.sh --mode diff` や `reports/dual-write` のスナップショットが監査ログと同期した `backend.*` metadata を出力できるようになる。
- 検証フェーズでは `cargo check --manifest-path compiler/rust/backend/llvm/Cargo.toml` を含むビルド確認に加え、`scripts/poc_dualwrite_compare.sh` で生成される JSON に `backend.triple`/`backend.abi` のエントリが現れること、`reports/dual-write/<run>/audit.*` に新しい `backend` フィールドが含まれていることを `jq`/差分ハーネスで検証する。必要なら `reports/dual-write/README.md` や `collect-iterator-audit-metrics.py` に追記して監査キーを参照できるようにする。

### P2G-02.01

- `docs/spec/3-8-core-runtime-capability.md:1.1` および `docs/plans/rust-migration/2-1-runtime-integration.md:2.1.4` が想定している `CapabilityHandle` は `Gc`/`Io`/`Async` といった型付きバリアントを持ち、その内部に関数テーブルや生成済みオブジェクトを保持するものだ。一方で現状の `compiler/rust/runtime/ffi/src/registry.rs` の `CapabilityHandle` は `CapabilityDescriptor` の複製にすぎず、FFI 側で毎回 `CapabilityId` 文字列に依存した分岐をせざるを得ない。これでは `docs/spec/3-8` で求められる型安全な `GcCapability` の `gc` API や `AuditCapability` の `record_event` が Rust から直接呼べないため、P2G-02 の着手前提となる `CapabilityHandle` の再設計が必要である。
- まず `CapabilityHandle` 本体を `enum CapabilityHandle` 形式に改め、`GcCapability`/`IoCapability`/`AsyncRuntimeCapability` 等の具象構造体（関数テーブルや状態を含む）を `CapabilityHandle::Gc(GcCapability)` のようなバリアントで保持するようにする。新たに `capability_handle.rs`（または `capability_types.rs`）を作成し、各 Capability の挙動を表すトレイト／構造体とテスト用のダミー実装を置く。`CapabilityHandle` には `descriptor()` と `as_gc()` などの `Option<&GcCapability>` 型のアクセスヘルパ（あるいは `CapabilityHandle::match_capability<F>(&self, f: F) where F: FnOnce(&GcCapability)`）を定義し、FFI 層が `match` で目的の API にアクセスできるようにする。
- `CapabilityRegistry` は現在 `HashMap<CapabilityId, CapabilityDescriptor>` だけを保持しているが、これを `HashMap<CapabilityId, CapabilityEntry>` にして `CapabilityEntry` に `descriptor: CapabilityDescriptor` と `handle: CapabilityHandle` を含める。`register` は `CapabilityDescriptor` に加え、構築済み `CapabilityHandle`（たとえば `CapabilityHandle::Gc(GcCapability::new(...))`）も同時にシングルトンへ追加し、重複登録や `StageRequirement` の不一致を `CapabilityError` で検出する。`verify_capability_stage` は `StageRequirement` をパスした `CapabilityEntry` から型付きハンドルをクローン（`Arc` でも可）して返すようにし、`descriptor()` のみに依存していた既存コードも流用できるようにする。
- `ffi` 側では `CapabilityHandle` を `match handle` で解体し、型付き API をそのまま呼び出すようにする。たとえば `ffi::gc::collect` 内で `CapabilityHandle::Gc(cap)` を期待し、`cap.collect(...)` を直接呼ぶ。DSL/Conductor 層も `CapabilityHandle` を `match` することで `GcCapability`/`AuditCapability` 等のコード分岐を削減できる。新しい使用例や想定される `CapabilityHandle` ライフサイクルは `docs/guides/runtime-bridges.md` と `docs/guides/reml-ffi-handbook.md` に追記し、型付きハンドルの導入をリファレンスとして残す。
- 検証は `compiler/rust/runtime/ffi/Cargo.toml` の `cargo test` で `CapabilityRegistry` の登録・検証・重複エラー・型付きハンドル取得をカバーし、`CapabilityHandle::as_gc()` が `Some` を返すことや `verify_capability_stage` が `StageViolation` を適切に返すことを確認する。必要であれば `docs/plans/rust-migration/p1-spec-compliance-gap.md` の「CapabilityRegistry」関連チェックリストを横断し、新しい API を測るための TODO を `docs/migrations.log` に残す。

### P2G-02.02

- `verify_capability_stage` は `StageRequirement` の判定を通過したうえで `required_effects`（たとえば `SecurityPolicy::required_effects`）を受け取り、登録済 `CapabilityDescriptor::effect_scope` との包含関係を検証します。要求効果が一つでも欠けていれば `CapabilityError::EffectViolation { id, required, missing, actual_scope }` を返し、`effects.contract.stage_mismatch` の `effect.capability`/`effect.stage.required`/`effect.stage.actual` を `docs/spec/3-6-core-diagnostics-audit.md:329-337` に従って構築できるようにします。Stage 要件違反では `CapabilityError::StageViolation` に `required_stage`/`actual_stage`/`capability_id` を保持し、`docs/spec/3-8-core-runtime-capability.md:137-160` に記載された Stage＋効果の連携を保証します。
- `audited_bridge_call` では `options.security_policy.required_effects` を新パラメータとして `verify_capability_stage` に渡し、`BridgeError::Capability` に `EffectViolation` が含まれることで DSL/Conductor 層へ必要な診断が届くようにします。必要であれば `SecurityPolicy` に getter（例: `required_effects()`）を追加して `CallOptions` から複製せず共有し、`SecurityPolicy::verify` は補助的な構成チェックに留めます。
- `registry.rs` のテストには効果不足を検出するケースを追加し、`EffectViolation` の `missing`/`actual_scope` が想定通りであることと、正常系では `last_verified_at` が更新されることを検証します。将来的な監査連携として `scripts/poc_dualwrite_compare.sh` や `collect-iterator-audit-metrics.py` が `reports/dual-write` の `diagnostics` JSON に `effect.capability`/`effect.stage.*` を含めた差分比較をできるよう、TODO を `reports/dual-write/README.md` に残すことも検討します。

### P2G-02.03

- `docs/spec/3-8-core-runtime-capability.md:141-156` は `ConductorCapabilityContract` で `manifest_path` を含む要求集合を受け取り、各要件に `verify_capability_stage` を適用して Stage と `effect_scope` の両方を検証することを求めている。まずは `docs/plans/rust-migration/1-3-dual-write-runbook.md` に記述された DSL/Conductor 側の契約発行フローと manifest の構造を整理し、Rust 側が受け取る `run.target.capabilities` もしくは `with_capabilities` 由来のデータの型とメタデータ（`stage`/`declared_effects`/`source_span`/`manifest_path`）を明確にします。
- `compiler/rust/runtime/ffi/src/registry.rs` の `CapabilityRegistry` 周辺に `ConductorCapabilityContract` / `ConductorCapabilityRequirement` 型を導入し、`verify_conductor_contract(contract: ConductorCapabilityContract)` を新設します。各 `requirement` に `verify_capability_stage` を適用し、失敗した場合は `CapabilityError::EffectViolation` や `StageViolation` に `required_stage`/`actual_stage`/`required_effects`/`effect_scope`/`capability_id` を埋め込みます。成功した要件は `AuditCapability` へ `effect.stage.required`・`effect.stage.actual`・`effect.capability`・`effect.scope` などを `AuditEnvelope.metadata["effect.*"]` で記録し、`manifest_path` が指定されていれば `effect.manifest_path` も残します。
- `manifest_path` が与えられた場合は `compiler/rust/runtime/ffi/src/manifest.rs`（または新規 `manifest_contract.rs`）で `reml.toml` や `run.target.capabilities` を読み、契約側 (`ConductorCapabilityRequirement.source_span`) との対応チェックと差分をログに残します。CLI/IDE が共有する `run.target.capabilities` との整合性を確認することで、DSL マニフェストと実行契約が一致しないケースを `AuditEvent::CapabilityMismatch`（`docs/spec/3-6-core-diagnostics-audit.md`）として記録できるようにします。
- `verify_conductor_contract` の呼び出し元（たとえば DSL マニフェストの `with_capabilities` 経路や既存の Runtime Bridge 初期化処理）を探索し、`manifest_path`／`contract.requirements` を `AuditEnvelope` と `Diagnostic` にセットするフローを追加します。`scripts/poc_dualwrite_compare.sh --mode diag` で `reports/dual-write/<run>/diagnostics.rust.json` に `effect.stage.*`/`effect.manifest_path` が出力され、OCaml 実装と JSON スキーマを比較することで差違を検出できるようにします。
- テスト戦略として `compiler/rust/runtime/ffi/tests/registry.rs` などに `verify_conductor_contract` の成功・Stage/Effect 失敗・manifest mismatch を含むケースと、それぞれの `AuditEnvelope` 追跡を追加します。`CapabilityError` の `effect_scope`/`missing` フィールドと `Audit` の `effect.stage.actual`/`required`/`manifest_path` を確認し、`scripts/poc_dualwrite_compare.sh` で収集した `diagnostics` を `jq`/`python` で比較するパイプラインを README に記録します。
