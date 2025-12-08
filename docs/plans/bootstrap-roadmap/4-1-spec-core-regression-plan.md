# 4.1 Phase 4 spec_core / practical 回帰是正計画

## 背景と課題

- `tooling/examples/run_examples.sh --suite spec_core` / `--suite practical`（`run_phase4_suite.py` 発行）により、Phase 4 で整備した `.reml` シナリオが Rust フロントエンドでは一貫して受理されていないことが判明した。
- `reports/spec-audit/ch4/spec-core-dashboard.md` / `practical-suite-index.md` では **全シナリオが `parser.syntax.expected_tokens` と `typeck.aborted.ast_unavailable`（または CLI 正常終了だが診断ゼロ）** の状態であり、Chapter 1〜3 の仕様試験が成立していない。
- 実装側は `docs/spec/1-5-formal-grammar-bnf.md` のトップレベル規則（`module` + `use` + `fn/let`）を解析できず、`--output json` で AST/Typed AST を得られないため型推論や効果診断が一切回らない。Phase 4 M1 exit 条件（Scenario 85% 実行）を満たすには、回帰要因を特定し段階的に是正する計画が必要。

## 目的

1. `examples/spec_core/`（Chapter 1〜2 BNF/推論）および `examples/practical/`（Chapter 3 実務ケース）を **Rust フロントエンド CLI で解析・型検査できる状態** に戻す。
2. `phase4-scenario-matrix.csv` に登録された `diagnostic_keys` と CLI 出力を照合し、`reports/spec-audit/ch4/*.md` で Pass/Fail を追跡できるようにする。
3. 解析の障害を修正する過程で、仕様側の不足が判明した場合は `docs/spec/1-x`〜`3-x` へ追記する判断材料（spec_fix/impl_fix）を明確にする。

## スコープ

- **含む**: Rust フロントエンド (`compiler/rust/frontend`) の Parser/Typeck/CLI オプション是正、`run_phase4_suite.py` の診断差分検知を活かしたレポーティング改善、`reports/spec-audit/ch4/` の定期更新。必要に応じて `RunConfig` / `ParseRunner` / `DiagnosticFilter` の既定値も調整する。
- **含まない**: `.reml` シナリオ自体の削減や仕様変更の強行。実行環境依存（ファイルI/Oの実処理、Capability 実体）の stub 化は別タスクとして扱い、本計画では Parser/Typeck が構文どおりに動くことを優先する。

## 現状確認（2025-12-07 実行ログより）

| 分類 | 代表 Scenario | 期待診断 | 実際の CLI 出力 | 備考 |
| --- | --- | --- | --- | --- |
| Module/Use トップレベル | `CH1-MOD-003`, `CH1-LET-001` 他多数 | `[]` または `language.*` 系 | `parser.syntax.expected_tokens`, `typeck.aborted.ast_unavailable` | `module` 直後の `use` 群を許容できず、`effect` or `fn` を要求している |
| `@cfg` 属性 | `CH1-ATTR-101`, `CH1-ATTR-102` | `language.cfg.unsatisfied_branch` など | `parser.syntax.expected_tokens` | ブロック属性の構文が Parser に登録されていない |
| Effect/Type 診断 | `CH1-EFF-701`, `CH1-IMPL-302` | `effects.purity.*`, `typeclass.impl.duplicate` | 同上 | Parser で脱落するため型診断に到達しない |
| Chapter2 Core.Parse | `CH2-PARSE-*` | `core.parse.recover.branch` 等 | 同上 | Parser 自身の self-test すら開始できない |
| Chapter3 practical | `CH3-IO-*`, `CH3-PLG-310` など | ステージ/IO/Capability 診断 | 同上 | Top-level で `use Core.*` が失敗し、実行前に脱落 |
| FFI/Core Prelude | `cargo test --package reml_frontend spec_core`（`tests/core_iter_*`）| `core_iter_*` スナップショット、`core_prelude` 依存の CLI 診断 | `reml_runtime_ffi` が `capability::*` 参照でコンパイル不能 | `compiler/rust/frontend/Cargo.toml` の dev-dep で `reml_runtime_ffi` + `core_prelude` を要求するが、`ffi/src/lib.rs` には `capability` module がなく、`core_prelude` が `crate::capability::registry` を参照するためリンク切れ（`compiler/rust/runtime/src/prelude/collectors/mod.rs:32`） |

## 作業計画

### フェーズA: Parser BNF 整合
1. **トップレベル定義と `use` 再導入**（4.2 週）  
   - `parser/mod.rs` の `parse_top_level_prefix` が `module` 宣言の後に `UseDecl` を許容していない箇所を是正し、BNF（1-5 §1）に合わせる。  
   - `syntax.expected_tokens` が `effect`/`fn` しか提示しない状況を、`UseDecl`/`Attr`/`ValDecl` まで含むよう `ExpectedToken` 生成ロジックを更新。  
   - `CH1-MOD-003` / `CH1-LET-001` / `CH1-LET-002` を use-case とした unit / integration テストを `compiler/rust/frontend/tests/spec_core/` に追加。

2. **属性 (`@cfg`, `@pure`) とブロック式の Parser 修正**（4.3 週）  
   - `AttrList` がブロック式（`{ ... }`）や `fn` 前に付与された場合に落ちる箇所を修正し、`docs/spec/1-1-syntax.md §B.6` のサンプルを CLI で解析できるようにする。  
   - `CH1-ATTR-101/102`, `CH1-EFF-701` をターゲットに parser-only テストを追加。

3. **Conductor/DSL, Streaming Parser の最小受理**（4.4 週）  
   - `conductor` ブロックや `run_stream` テストが構文エラーになる箇所を特定し、`docs/spec/1-5` の派生構文に合わせたノードを復活。  
   - `CH1-DSL-801`, `CH2-STREAM-301` を通すまで Parser を段階調整。

### フェーズB: Typeck / Effect 診断の復元
4. **型推論 / 効果行の非アクティブ化回収**（5.1 週）  
   - Parserが通るようになった後も `typeck.aborted.ast_unavailable` が解消しない場合、`TypecheckDriver` が AST を拒否する条件（`allow_module_body` 等）の見直しを行う。  
   - `CH1-INF-601/602`, `CH1-EFF-701`, `CH1-IMPL-302` を `cargo test -p reml_e2e -- --scenario spec-core` に組み込み、期待診断と照合する自動テストを用意。

#### ✅ 5.1 週 実施ログ（Typeck / Effect 診断の復元）

- `compiler/rust/frontend/src/typeck/driver.rs` を拡張し、`ExprKind::Block` / `StmtKind::{Decl,Assign,Defer}` を追加解析できるようにした。`let`/`var` 束縛をスコープ毎に一般化し、`DeclKind::Var` で `type_annotation` が無い場合は `language.inference.value_restriction` を発火させる。  
- `@pure` 関数が `perform` を呼び出した際に `effects.purity.violated` を生成する `FunctionContext` を追加し、`TypecheckViolation` に `PurityViolation` を新設した。`collect_perform_effects` もブロック/ラムダを辿るよう更新済み。  
- `compiler/rust/frontend/tests/spec_core/mod.rs` に `CH1-INF-601/602`・`CH1-EFF-701` を対象とした typeck テストを追加し、`typeck.aborted.ast_unavailable` が発生しないことと新診断が出力されることを `cargo test -p reml_frontend --test spec_core` で確認した。  
- ⚠️ `CH1-IMPL-302` は現状 Parser が `trait` / `impl` 構文を受理できず `parser.syntax.expected_tokens` で脱落するため、Typeck 層へ AST が渡らない。`examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml` を解析すると `parser.diagnostics` が 1 件返ることを確認済みで、Phase A の `impl` サポートが完了するまで本シナリオは pending とする。

5. **Core.Parse/Runtime 仕様のアクティブ化**（5.3 週）  
   - `CH2-PARSE-*` 用に `Parse.run` / `Parse.run_with_recovery` が CLI から呼び出せるよう `core::Prelude` の module import を整備。  
   - `CH3-RUNTIME-601`, `CH3-PLG-310` など Capability 関連は stub 実装で構文エラーを避け、診断 (`runtime.bridge.stage_mismatch` など) が出力できるようにする。

### フェーズC: 自動実行とレポートの固定化
6. **`run_phase4_suite.py` のサマリ強化と CI 組み込み**（5.4 週）  
   - 現在 `--allow-failures` 前提のレポート生成を、既定では「失敗があれば exit 1」としつつ、失敗時のログ保存（`reports/spec-audit/ch4/logs/`）を追加。  
   - `.github/workflows/phase4-spec-core.yml`（新規）で `run_examples.sh --suite spec_core` → `--suite practical` を nightly で回し、成功件数/KPI を記録。

7. **Phase4 Scenario Matrix の自動同期**（5.5 週）  
   - `ScenarioResult` を `phase4-scenario-matrix.csv` の `resolution_notes` に反映する補助スクリプト（`tooling/examples/update_phase4_resolution.py` 仮）を用意し、Pass/Fail に応じて `ok/impl_fix/spec_fix` を更新。  
   - `reports/spec-audit/ch4/spec-core-dashboard.md` / `practical-suite-index.md` を Phase4 README で参照し、週次レビュー資料として扱う。

### フェーズD: `reml_runtime_ffi` capability shim 回収（新規）

Rust Frontend の `spec_core` テストは `reml_runtime_ffi` を dev-dep として `core_prelude` 機能を有効化する（`compiler/rust/frontend/Cargo.toml:47-51`）。しかし FFI 側では `core_prelude` を疑似的に `#[path = "../../src/prelude/..."]` で取り込んでいるだけで、依存している `crate::capability::registry::*` ツリーが存在せずビルドが止まる。Phase 4 では `core_iter_*` 系スナップショットから Chapter 1 の効果/Stage カバレッジを得る必要があるため、`reml_runtime_ffi` に capability shim を導入して `cargo test --package reml_frontend spec_core` が常に起動できる状態を作る。

1. **依存パスの棚卸しと仕様根拠の整理**（5.6 週）  
   - `compiler/rust/runtime/ffi/src/lib.rs:16-65` と `core_prelude` 配下の `collectors/mod.rs`, `iter/mod.rs` を洗い出し、`crate::capability::registry` 以外の未解決参照が無いことを確認。必要に応じて `docs/spec/3-1-core-prelude-iteration.md` と `docs/spec/3-6-core-diagnostics-audit.md` の要件（監査ログ + Stage Requirement）を引用し、shim が仕様面の整合を壊さないかレビュー項目を用意する。  
   - `phase4-scenario-matrix.csv` に `FFI-CORE-PRELUDE-001`（`core_iter_effects`, `core_iter_adapters`, `core_iter_collectors`, `core_iter_pipeline`）行を追加し、`resolution=pending` のまま本フェーズの出口条件に紐付ける。

2. **capability shim の実装計画**（5.7 週）  
   - 既存の `registry.rs` を `crate::capability::registry` として再輸出する薄いモジュール（`ffi/src/capability.rs`）を追加し、`CapabilityError` / `CapabilityRegistry` / `BridgeIntent` / `RuntimeBridgeRegistry` が `core_prelude` から見えるようにする。  
   - shim を追加したら `cargo check -p reml_runtime_ffi --features core_prelude`、`cargo test --package reml_frontend core_iter_effects` を CI へ組み込み、`spec_core` スイートが `parser.syntax.expected_tokens` 以外の理由で停止しない状態を KPI として記録する。  
   - `docs/plans/rust-migration/1-3-dual-write-runbook.md` の `capability` 共有セクションと整合するか確認し、必要であれば同 runbook へ補足する。

3. **検証とフォローアップ**（5.8 週）  
   - shim 経由で `core_prelude` が利用可能になったら、`tests/core_iter_*` の snapshot を更新し `reports/spec-audit/ch4/spec-core-dashboard.md` に `FFI/Core Prelude` の pass 率を新設。  
   - `capability` shim の将来廃止に備え、`reml_runtime` モジュールを直接依存として使う長期方針を `docs/notes/core-library-outline.md` へ TODO 記録し、Phase 5 で `reml_core_prelude` を共通 crate 化する提案を追記する。

## 成果物と KPI

- `parser.syntax.expected_tokens` / `typeck.aborted.ast_unavailable` が Phase 4 の spec_core/practical スイートで発生しないこと（期待診断があるケースを除く）。  
- `reports/spec-audit/ch4/spec-core-dashboard.md` における **Pass 率 70% 以上**、Phase 4 M1 exit 条件の 85% へ段階的に到達。  
- `cargo test -p reml_e2e -- --scenario spec-core` / `--scenario practical` を追加し、CI で `spec.chapter1.pass_rate`, `spec.chapter3.pass_rate` KPI を更新。  
- 主要な spec_fix/impl_fix の判断を `phase4-scenario-matrix.csv` の `resolution_notes` に残し、Phase 5 以降のハンドオーバー資料として利用可能にする。

## 依存関係とフォローアップ

- Parser/Typeck 修正は Phase 3 の `docs/spec/1-x` / `docs/spec/3-x` 更新と連動するため、仕様差分を検出した場合は `2-5-spec-drift-remediation.md` の手順に沿って仕様側へ反映。  
- Core.IO / Capability の挙動差分は `3-5-core-io-path-plan.md` や `3-8-core-runtime-capability-plan.md` の残課題と共有し、必要なら Phase 3 計画へ逆流させる。  
- Self-host フェーズ（Phase 5）へ進む前に本計画の KPI を満たし、`reports/spec-audit/ch4` を Stage 0/1/2 のリグレッションベースとして採用する。
