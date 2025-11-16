# P2 Rust バックエンド追加ギャップレポート（2028-02）

`p2-spec-compliance-gap.md` では LLVM バックエンドおよびランタイムの仕様未達項目を三つのテーマで整理した。本稿はその調査結果に含まれていない領域について、OCaml 実装と Rust 実装（2028-02 ブランチ）のコードを直接比較して得られた追加ギャップを記録する。P2 計画（`2-0`〜`2-3`）で定義された成果物に照らし、Phase 3 以降へ持ち越さないための補完作業を洗い出すことが目的である。

## 1. 調査方針

- 参照仕様: `docs/spec/0-1-project-purpose.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`
- 参照計画: `docs/plans/rust-migration/overview.md`, `docs/plans/rust-migration/2-0-llvm-backend-plan.md`, `docs/plans/rust-migration/2-1-runtime-integration.md`, `docs/plans/rust-migration/2-3-p2-backend-integration-roadmap.md`
- 実装比較:
  - OCaml: `compiler/ocaml/src/runtime_capability_resolver.ml`, `compiler/ocaml/src/diagnostic.ml`, `compiler/ocaml/src/parser_driver.ml`, `compiler/ocaml/src/runtime_bridge_registry.ml`, `compiler/ocaml/src/codegen/ffi_stub_builder.ml`, `compiler/ocaml/src/llvm_gen/runtime_link.ml`
  - Rust: `compiler/rust/frontend/src/bin/poc_frontend.rs`, `compiler/rust/frontend/src/diagnostic/effects.rs`, `compiler/rust/frontend/src/streaming/flow.rs`, `compiler/rust/runtime/ffi/src/*`, `compiler/rust/backend/llvm/src/{ffi_lowering.rs,integration.rs}`

## 2. 概要

| ID | 領域 | 状態 | 主な不足 | 参照 |
| --- | --- | --- | --- | --- |
| ✅ P2R-01 | Stage コンテキスト解決 | 未実装 | CLI／環境変数／`REML_RUNTIME_CAPABILITIES` JSON を統合して Stage trace を生成する `Runtime_capability_resolver` 相当の処理が Rust に存在しない。Stage trace 拡張や監査メタデータも CLI 入力の写経のみ。 | `runtime_capability_resolver.ml`, `main.ml`, `diagnostic.ml`, `poc_frontend.rs`, `diagnostic/effects.rs` |
| ✅ P2R-02 | ランタイム Bridge バックプレッシャ診断 | 未実装 | `bridge.stage.backpressure` / `effects.contract.stage_mismatch` を Streaming parser から発火する仕組みが Rust にはない。Stage mismatch を `Runtime_bridge_registry.stream_signal` で監査する経路も欠落。 | `parser_driver.ml`, `runtime_bridge_registry.ml`, `poc_frontend.rs`, `streaming/flow.rs` |
| ✅ P2R-03 | FFI スタブ計画と Register Save Area | 未実装 | OCaml の `ffi_stub_builder.ml` が提供するターゲット別スタブ計画・Darwin 向け register save area メタデータが Rust `ffi_lowering.rs` では再現されていない。 | `codegen/ffi_stub_builder.ml`, `ffi_lowering.rs` |
| ✅ P2R-04 | LLVM 生成物のリンク & ランタイム連携 | 未実装 | OCaml 版が `llc`→`clang` 連携で `runtime/native` をリンクするのに対し、Rust バックエンドは MIR → JSON スナップショット生成のみで実行ファイルを生成しない。 | `llvm_gen/runtime_link.ml`, `backend/llvm/src/integration.rs` |

## 3. 詳細ギャップ

### 3.1 P2R-01: Stage コンテキスト解決と `stage_trace` 伝搬

- **仕様背景**: `docs/spec/3-8-core-runtime-capability.md` §1 は `CapabilityHandle` を Stage 付帯情報とともに公開し、`stage_trace` を診断や監査キーへ出力することを要求している。また `docs/spec/3-6-core-diagnostics-audit.md` §2 では `effect.stage.*` / `effect.capability.*` の拡張を公式化している。
- **OCaml 実装**:
  - `Runtime_capability_resolver.resolve` は CLI `--effect-stage`、`REMLC_EFFECT_STAGE`／`REML_RUNTIME_STAGE`、`REML_RUNTIME_CAPABILITIES`（JSON / ターゲット別 override）を順に評価し、`stage_trace` を構築する（`compiler/ocaml/src/runtime_capability_resolver.ml:1-282`）。
  - `compiler/ocaml/src/main.ml:904-945` で解析前に Stage 文脈を決定し、`Type_inference_effect.runtime_context` および `Parser_run_config` へ伝播する。
  - `compiler/ocaml/src/diagnostic.ml:900-1008` では `effect.stage_trace`・`stage_trace` 監査拡張を挿入している。
- **Rust 現状**:
  - `compiler/rust/frontend/src/bin/poc_frontend.rs:224-530` の CLI は Stage を CLI 引数とワークスペース JSON のみに依存させ、環境変数や `REML_RUNTIME_CAPABILITIES` のような JSON registry を参照しない。
  - `StageAuditPayload`（`同:1170-1210`）および `effects::EffectAuditContext`（`compiler/rust/frontend/src/diagnostic/effects.rs:1-80`）は `stage_trace` を CLI 渡しの文字列から手作りしており、解析パイプライン内で Stage 決定過程を記録する経路が無い。
  - `CapabilityRegistry::verify_capability_stage`（`compiler/rust/runtime/ffi/src/registry.rs:152-208`）は Stage 判定のみで `stage_trace` を持たない。
- **必要対応**:
  - `Runtime_capability_resolver` 相当のモジュールを Rust で実装し、CLI/環境変数/JSON/ターゲット override を統合して `StageContext` と `stage_trace` を生成する。
  - `StageAuditPayload` と `EffectAuditContext` に `stage_trace` を注入できる API を追加し、`build_parser_diagnostics` / Typecheck dual-write / Audit で OCaml と同じ `effect.stage_trace` / `effect.stage.capabilities` を出力する。
  - `CapabilityRegistry` 側でも Stage 決定経路を保持し `AuditContext` へ渡す（P2G-02 の effect_scope 対応と併せて実装）。

### 3.2 P2R-02: ランタイム Bridge バックプレッシャ診断の欠落

- **仕様背景**: `docs/spec/3-6-core-diagnostics-audit.md` §3 は `bridge.stage.backpressure` / `effects.contract.stage_mismatch` の診断拡張と監査キーを要求し、`docs/spec/3-8-core-runtime-capability.md` §4 では Runtime Bridge の Stage 違反を検出して Rollback 診断を発行することを Phase 2 成果物に含めている。
- **OCaml 実装**:
  - Streaming parser は `build_bridge_stage_diagnostic`（`compiler/ocaml/src/parser_driver.ml:633-682`）で backpressure 理由を検知し、`Runtime_bridge_registry.stream_signal`（`compiler/ocaml/src/runtime_bridge_registry.ml:1-100`）へ渡して `bridge.stage.backpressure` 診断・`effects.contract.stage_mismatch` を生成、`AuditEnvelope` に `bridge.stream.*` を書き込む。
  - CLI メトリクス（`compiler/ocaml/src/main.ml:995-1007`）にも await/resume/backpressure カウンタを反映し、`Cli.Stats` や `collect-iterator-audit-metrics.py` が参照できる。
- **Rust 現状**:
  - Streaming 実装は `StreamFlowState`（`compiler/rust/frontend/src/streaming/flow.rs:1-70`）と `build_parser_diagnostics`（`compiler/rust/frontend/src/bin/poc_frontend.rs:940-1150`）で recover ダイアグラムや Expected Token を付与するのみで、Bridge Stage 不一致を診断・監査する経路が存在しない。
  - `compiler/rust/frontend/src/diagnostic/effects.rs` には `bridge.stage.*` 拡張を出力するロジックがあるが、`EffectAuditContext::stage_trace` は CLI 情報の写経のため Streaming runtime から実際の backpressure イベントが渡されない。
- **必要対応**:
  - `StreamingRunner`（`reml_frontend::streaming`）に bridge policy／await/resume/backpressure カウンタと Stage 判定 API を追加し、`Runtime_bridge_registry` の Rust 版を実装して `bridge.stage.backpressure` / `effects.contract.stage_mismatch` を生成する。
  - Streaming CLI／diagnostic へ `bridge` 拡張を注入し、`collect-iterator-audit-metrics.py --section streaming` が参照する JSON スキーマを OCaml と揃える。
  - CLI stats / dual-write へ `await_count` 等のメトリクスを出力して比較できるようにする。

### 3.3 P2R-03: FFI スタブ計画と Register Save Area 情報の欠落

- **仕様背景**: `docs/plans/rust-migration/2-0-llvm-backend-plan.md` および `docs/spec/3-9-core-async-ffi-unsafe.md` §10 は、ターゲット別 stub template・呼出規約・ABI・`bridge.darwin.register_save_area.*` 監査キーを Phase 2 で整備することを完了条件に含めている。
- **OCaml 実装**:
  - `compiler/ocaml/src/codegen/ffi_stub_builder.ml:1-210` は Linux/Windows/macOS のテンプレート、所有権/ABI の正規化、Darwin 用 register save area（`gpr/vector` スロット情報）および `bridge.*` 監査タグを構築する。
- **Rust 現状**:
  - `compiler/rust/backend/llvm/src/ffi_lowering.rs:1-60` は `RemlType` -> `TypeLayout` の変換と署名文字列の生成のみを提供し、ターゲット別のスタブ計画・register save area メタデータを保持しない。
  - `BackendDiffSnapshot`（`integration.rs`）にも `bridge.*` 監査タグや Darwin register 情報を格納する欄が無い。
- **必要対応**:
  - `ffi_lowering.rs` へ stub template／register save area 設定を追加し、`LoweredFfiCall` に監査タグ（`bridge.platform`, `bridge.arch`, `bridge.darwin.register_save_area.*`）を保持させる。
  - `collect-iterator-audit-metrics.py` が参照する JSON に Rust 側のフィールドを出力し、OCaml 版と比較できるようにする。

### 3.4 P2R-04: LLVM 生成物のリンクとランタイム連携不足

- **仕様背景**: `docs/plans/rust-migration/2-1-runtime-integration.md` では Rust バックエンドが `runtime/native` の `libreml_runtime.a` とリンクし、`REML_RUNTIME_PATH` を尊重して Windows/MSVC も含む実行ラインを整備することを要求している。
- **OCaml 実装**:
  - `compiler/ocaml/src/llvm_gen/runtime_link.ml:1-140` が `llc -filetype=obj` → `clang ... libreml_runtime.a` を起動し、`REML_RUNTIME_PATH`／ローカルビルドパスを自動検出して実行ファイルを生成する。
- **Rust 現状**:
  - `compiler/rust/backend/llvm/src/integration.rs:1-210` は MIR JSON を読み込み `BackendDiffSnapshot` を生成するのみで、LLVM IR/オブジェクトファイル生成や `runtime/native` とのリンクを行っていない。`TargetMachine` 周りでも実際の `llc` / `opt` 呼び出しや `REML_RUNTIME_PATH` 検出が未実装。
- **必要対応**:
  - MIR → LLVM IR → オブジェクト → リンクまでを Rust 側でも自動化し、`runtime/native` の静的ライブラリを検出する処理を `runtime_link.ml` と同等に用意する。
  - `scripts/poc_dualwrite_compare.sh` から Rust バックエンドを呼び出した際に OCaml 版と同じ成果物（obj/exe）を比較できるよう CLI ゲートを整備する。

## 4. 具体的な計画

### P2R-01

 1. OCaml 側の `Runtime_capability_resolver` および `effect_profile` に載っている Stage trace の構造を読み込み、CLI/環境変数/`REML_RUNTIME_CAPABILITIES` JSON の順で Stage context を再構築する仕組みを Rust に再現する。Stage trace の各ステップ（`cli_option`／`env_var`／`run_config`／`capability_json`／`runtime_candidate`）を `StageTraceStep` で表現し、標準的な StageRequirement を生成できるようにする。
 2. `poc_frontend` で新設した `StageContext::resolve` を呼び出し、`--effect-stage` など CLI オーバーライドと `target_inference` の Triple を含むコンテクストで `StageContext` を構築し、そのまま型推論設定と診断拡張に渡す。`StageAuditPayload`/`EffectAuditContext` は `stage_trace` を受け取った上で従来の `runtime`・`runtime_capability` ステップを追加するようにし、診断メタデータに CLI→環境→JSON の伝搬順を含める。
 3. 新たな Stage context が `collect-iterator-audit-metrics.py` などの監査指標と矛盾しないよう、`StageTraceStep` で記録されるフィールド名と順序を既存の `stage_trace` スキーマ（`effects.stage.trace`/`stage_trace`）に合わせ、必要に応じて `REML_RUNTIME_CAPABILITIES` ファイルを読み込んだ場合でも `runtime_capabilities` と `bridge` 拡張が補完されることを検証する。

### P2R-02: ランタイム Bridge バックプレッシャ診断

1. OCaml 実装 (`parser_driver.ml` + `runtime_bridge_registry.ml`) が `bridge.stage.backpressure` と `effects.contract.stage_mismatch` を Streaming parser にどう通知し、`Runtime_bridge_registry.stream_signal` が Runtime 状態と橋渡ししているかを精査し、要求されるトレース情報と診断タグの一覧を `docs/spec/3-6-core-diagnostics-audit.md` §3 および `docs/spec/3-8-core-runtime-capability.md` §2 に照らして整理する。
2. Rust の `compiler/rust/frontend/src/streaming/flow.rs` に、バックプレッシャ置換点（Runtime が対向 Stage を受け入れない／過負荷）を検知するフックを追加し、`RuntimeBridgeSignal` と `StreamingEffectContext` を経由して `bridge.stage.backpressure` を発火できるよう Event を伝搬させる。同時に `poc_frontend.rs` で `Runtime_bridge_registry` にステージ選択と信号登録のエントリを準備し、解析実行までの流れ全体で Stage mismatch の理由・タイムスタンプ・`stage_trace` を保持する。
3. `compiler/rust/runtime/ffi/src/registry.rs` 側で `Runtime_bridge_registry.stream_signal` に Stage/backpressure 状態を保持するキャッシュを設け、`bridge::StageMismatch` の際に期待される Stage、現在の Stage trace、`Runtime_capability_resolver` 相当のコンテキストをアクセシブルにする。Runtime 側の `StageTrace` は `effects.contract.stage_mismatch` の `EffectAuditContext` に渡して `stage_trace` メタデータを拡張する。
4. `compiler/rust/frontend/src/diagnostic/effects.rs` へ `BridgeBackpressure`/`ContractStageMismatch` 效果用の診断生成器を追加し、`stage_trace` に当該 Runtime 信号の発生元情報（`parser_offset`・`stream_sequence`）と Bridge registry の `stage_capability` を含める。監査ログ／テスト資産として `docs/plans/rust-migration/2-1-runtime-integration.md` や `docs/plans/rust-migration/2-3-p2-backend-integration-roadmap.md` で定義されたシナリオ（バックプレッシャ断続、Stage mismatch）を使用し、期待されるタグを JSON/テキストで比較するテストケースを `compiler/rust/frontend/tests/` に追加する。
5. `streaming/flow` から出力する診断が CLI や `REML_RUNTIME_CAPABILITIES` に影響されることを `docs/plans/rust-migration/unified-porting-principles.md` の「環境差異の明示」方針に則ってログ・ドキュメント化し、`docs-migrations.log` に新しい Bridge backpressure 診断カテゴリを記録して CI/監査チームで追跡できるようにする。

### P2R-03: FFI スタブ計画と Register Save Area 情報の再現

1. `compiler/ocaml/src/codegen/ffi_stub_builder.ml` の `stub_template`, `register_save_area`, `audit_tags_of_plan` で出力している `bridge.platform` / `bridge.target` / `bridge.arch` / `bridge.darwin.register_save_area.*` の構成を整理し、`docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` で求められる `AuditEnvelope.metadata.bridge` スキーマと `docs/spec/3-6-core-diagnostics-audit.md` §3.2 の必須タグとを突き合わせる。
2. `compiler/rust/backend/llvm/src/ffi_lowering.rs` にターゲット Triple に基づく stub plan コンテキスト（ターゲット文字列、Platform ラベル、arch、backend ABI、既定 ownership）の構造体を導入し、`LoweredFfiCall` が `bridge.*` タグと Darwin register save area を `audit_tags` で保持できるように改修する。`collect-iterator-audit-metrics.py` および既存の JSON スナップショット（`scripts/poc_dualwrite_compare.sh` など）に投入されるフィールド名と順序を明示し、OCaml と同じタグ群を出力できることを確認する。
3. `compiler/rust/backend/llvm/src/bridge_metadata.rs` では `record_stub` を新しい stub plan に合わせて更新し、`BridgeStubMetadata` が `FfiStubPlan` から `bridge.platform/target/arch/abi/ownership` と `bridge.darwin.register_save_area.*` を `extras` に追加するようにする。これにより `BackendDiffSnapshot` と `collect-iterator-audit-metrics.py` で参照される `reml.bridge.stubs` ログにも詳細が反映される。
4. 実装後に `docs/plans/rust-migration/2-0-llvm-backend-plan.md` で言及している「bridge.* 監査エントリ」と `docs/plans/rust-migration/2-3-p2-backend-integration-roadmap.md` の差分検証フローをつなぎ、追加タグの存在を記録した JSON スナップショットを `scripts/poc_dualwrite_compare.sh` で比較検証できることをコメントまたは README に残す。

### P2R-04: LLVM 生成物のリンク & ランタイム連携の自動化

 1. `compiler/ocaml/src/llvm_gen/runtime_link.ml` に記述された `REML_RUNTIME_PATH` 検出、`llc` → `clang`/`cc` の呼び出し系列、macOS/Linux のリンカ設定、`runtime/native/build`・インストール済みパスの順で `libreml_runtime.a` を探索するロジックを Rust 側の `runtime_link.rs` へ写す。`RuntimeLinkError`/`LinkCommand` で出力結果をキャプチャし、失敗時の診断を `diagnostic/audit` に回せるようにする。
 2. `scripts/poc_dualwrite_compare.sh` の Rust 側実行コードパスに `--runtime-link` 相当のフラグを追加し、`cargo run --bin poc_frontend` 実行後に `rust.json` と並列して生成される `.o`/実行ファイルを OCaml 側成果物（`diagnostics.ocaml.*` のケース出力）と比較するゲートを追加する。`runtime_link.rs` で `Platform` 情報・`LinkCommand` を明示的に出力し、監査ログに `runtime.linker`/`runtime.platform` などを追加する。
 3. `docs/plans/rust-migration/2-1-runtime-integration.md` §6-§8 で整理された実行パス（`REML_RUNTIME_PATH`・`CapabilityRegistry`・MSVC/GNU/clang のターゲット差異）と `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §6.3 のトレースを参照しつつ、Rust 側で `llc`/`clang` が失敗したときの再試行方針・監査イベント（`bridge.runtime.link` など）のテンプレートを `docs/migrations.log` に記録する。
 4. 実装後は `scripts/poc_dualwrite_compare.sh --mode diag` で生じる `reports/dual-write/front-end/poc/<run>/` 以下に `.o`/実行ファイルのハッシュや `runtime_link` の `stdout` を保存し、OCaml 実行時と Rust 実行時でファイルサイズ・シンボル数を比較する自動差分チェックを追加する。
