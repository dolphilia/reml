# 4.1 Phase 4 spec_core / practical 回帰是正計画

## 背景と課題

- `tooling/examples/run_examples.sh --suite spec_core` / `--suite practical`（`run_phase4_suite.py` 発行）により、Phase 4 で整備した `.reml` シナリオが Rust フロントエンドでは一貫して受理されていないことが判明した。
- `reports/spec-audit/ch5/spec-core-dashboard.md` / `practical-suite-index.md` では **全シナリオが `parser.syntax.expected_tokens` と `typeck.aborted.ast_unavailable`（または CLI 正常終了だが診断ゼロ）** の状態であり、Chapter 1〜3 の仕様試験が成立していない。
- 実装側は `docs/spec/1-5-formal-grammar-bnf.md` のトップレベル規則（`module` + `use` + `fn/let`）を解析できず、`--output json` で AST/Typed AST を得られないため型推論や効果診断が一切回らない。Phase 4 M1 exit 条件（Scenario 85% 実行）を満たすには、回帰要因を特定し段階的に是正する計画が必要。

## 目的

1. `examples/spec_core/`（Chapter 1〜2 BNF/推論）および `examples/practical/`（Chapter 3 実務ケース）を **Rust フロントエンド CLI で解析・型検査できる状態** に戻す。
2. `phase4-scenario-matrix.csv` に登録された `diagnostic_keys` と CLI 出力を照合し、`reports/spec-audit/ch5/*.md` で Pass/Fail を追跡できるようにする。
3. 解析の障害を修正する過程で、仕様側の不足が判明した場合は `docs/spec/1-x`〜`3-x` へ追記する判断材料（spec_fix/impl_fix）を明確にする。

## FFI 回帰接続（Phase 4 追加）

- `phase4-scenario-matrix.csv` に FFI シナリオを追加し、`FFI-BINDGEN-001` / `FFI-DSL-001` / `FFI-BUILD-001` / `FFI-WIT-001` を回帰対象として追跡する。
- 生成物の参照先:
  - `expected/ffi/bindgen/minimal/counter_bindings.reml`
  - `expected/ffi/bindgen/minimal/bindings.manifest.json`
  - `expected/ffi/dsl/wrapped_safe.audit.json`
  - `expected/ffi/dsl/unsafe_direct.audit.json`
- 実行ログの保存先:
  - `reports/spec-audit/ch5/logs/ffi-build-*.md`
  - `reports/spec-audit/ch5/logs/ffi-dsl-*.md`
  - `reports/spec-audit/ch5/logs/ffi-bindgen-*.md`
- WIT 調査は `docs/notes/ffi/ffi-wasm-component-model-log.md` と `docs/guides/ffi/ffi-wit-poc.md` の更新内容を対象とし、PoC 実施後に外部ツール名と生成物パスを追記する。

## 標準ライブラリ回帰接続（Phase 4 追加）

- `phase4-scenario-matrix.csv` に標準ライブラリ系のシナリオ（`CH3-TEST-401` / `CH3-CLI-401` / `CH3-PRETTY-401` / `CH3-DOC-401` / `CH3-LSP-401`）を追加し、Phase 4 の実行対象として追跡する。
- Core.Parse.Cst のロスレス経路は `CH2-PARSE-930` として登録し、CstPrinter の既定スタイル（空白/改行/コメント保持）を回帰対象に含める。
- 参照仕様（新章）:
  - `docs/spec/3-11-core-test.md`
  - `docs/spec/3-12-core-cli.md`
  - `docs/spec/3-13-core-text-pretty.md`
  - `docs/spec/3-14-core-lsp.md`
  - `docs/spec/3-15-core-doc.md`
- 実行コマンド（案）:
  - Core.Test: `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_test/snapshot/basic_ok.reml`
  - Core.Cli: `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_cli/parse_flags/basic_ok.reml`
  - Core.Text.Pretty: `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_text/pretty/layout_width_basic.reml`
  - Core.Doc: `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_doc/basic_generate_ok.reml`
  - Core.Lsp: `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_lsp/basic_diagnostics_ok.reml`
  - Core.Parse.Cst: `compiler/frontend/target/debug/reml_frontend --output json examples/practical/core_parse/cst_lossless.reml`
- 実行ログの保存先:
  - `reports/spec-audit/ch5/logs/stdlib-test-*.md`
  - `reports/spec-audit/ch5/logs/stdlib-cli-*.md`
  - `reports/spec-audit/ch5/logs/stdlib-pretty-*.md`
  - `reports/spec-audit/ch5/logs/stdlib-doc-*.md`
  - `reports/spec-audit/ch5/logs/stdlib-lsp-*.md`
  - `reports/spec-audit/ch5/logs/stdlib-parse-cst-*.md`
- ログに残す項目は `reports/spec-audit/ch5/README.md` の「標準ライブラリ実行ログ（Phase 4）」を参照する。

## Native Escape Hatches 回帰接続（Phase 4 追加）

- `phase4-scenario-matrix.csv` に `NATIVE-INTRINSIC-001` / `NATIVE-EMBED-001` / `NATIVE-ASM-001` / `NATIVE-LLVMIR-001` を追加し、`Core.Native` と埋め込み API、Inline ASM / LLVM IR の回帰を Phase 4 で追跡する。
- 参照資産:
  - `examples/native/intrinsics/basic.reml`
  - `expected/native/intrinsics/basic.stdout`
  - `expected/native/intrinsics/basic.audit.jsonl`
  - `examples/native/embedding/basic.c`
  - `examples/native/embedding/basic.reml`
  - `expected/native/embedding/basic.stdout`
  - `expected/native/embedding/basic.audit.jsonl`
  - `examples/native/asm/inline_asm_rdtsc.reml`
  - `expected/native/asm/inline_asm_rdtsc.stdout`
  - `expected/native/asm/inline_asm_rdtsc.audit.jsonl`
  - `examples/native/llvm_ir/llvm_ir_add_i32.reml`
  - `expected/native/llvm_ir/llvm_ir_add_i32.stdout`
  - `expected/native/llvm_ir/llvm_ir_add_i32.audit.jsonl`
- 実行手順（案）:
  - Intrinsic: `compiler/frontend/target/debug/reml_frontend --output json examples/native/intrinsics/basic.reml`
  - Embed: `examples/native/embedding/basic.c` を `libreml` 相当の C ABI にリンクして実行し、stdout と監査ログを採取する。
  - Inline ASM: `compiler/frontend/target/debug/reml_frontend --output json examples/native/asm/inline_asm_rdtsc.reml`
  - LLVM IR: `compiler/frontend/target/debug/reml_frontend --output json examples/native/llvm_ir/llvm_ir_add_i32.reml`
- 確認観点:
  - `native.intrinsic.used` / `intrinsic.name` / `intrinsic.signature` が監査ログに出力されること。
  - `native.embed.entrypoint` / `embed.abi.version` が監査ログに出力されること。
  - `native.inline_asm.used` / `asm.template_hash` / `asm.constraints` が監査ログに出力されること。
  - `native.llvm_ir.used` / `llvm_ir.template_hash` / `llvm_ir.inputs` が監査ログに出力されること。
  - 期待 stdout と `expected/native/` の差分がないこと。
- 実行ログの保存先:
  - `reports/spec-audit/ch5/logs/native-intrinsic-*.md`
  - `reports/spec-audit/ch5/logs/native-embed-*.md`
  - `reports/spec-audit/ch5/logs/native-inline-asm-*.md`
  - `reports/spec-audit/ch5/logs/native-llvm-ir-*.md`
- KPI とログフォーマットは `reports/spec-audit/ch5/README.md` の「Native Escape Hatches 実行ログ（Phase 4）」を参照する。

## DSL パラダイム回帰接続（Phase 4 追加）

`Core.Dsl.*` の参照 DSL を Phase 4 回帰へ組み込み、性能・安全性・監査ログの観点を `phase4-scenario-matrix.csv` に集約する。

| Scenario | 入力 | 期待値 | 監査イベント | Stage 条件 |
| --- | --- | --- | --- | --- |
| CH4-DSL-PARA-001 | `examples/dsl_paradigm/mini_ruby/mini_ruby_basic.reml` | `expected/dsl_paradigm/mini_ruby/mini_ruby_basic.stdout` | `dsl.object.dispatch`, `dsl.gc.root` | `StageRequirement::AtLeast(Beta)` |
| CH4-DSL-PARA-002 | `examples/dsl_paradigm/mini_erlang/mini_erlang_basic.reml` | `expected/dsl_paradigm/mini_erlang/mini_erlang_basic.stdout` | `dsl.actor.mailbox`, `dsl.gc.root` | `StageRequirement::AtLeast(Beta)` |
| CH4-DSL-PARA-003 | `examples/dsl_paradigm/mini_vm/mini_vm_basic.reml` | `expected/dsl_paradigm/mini_vm/mini_vm_basic.stdout` | `dsl.vm.execute`, `dsl.object.dispatch` | `StageRequirement::AtLeast(Beta)` |

- 性能観点は `dsl.object.dispatch`/`dsl.actor.mailbox`/`dsl.vm.execute` の監査件数と処理ステップの増減で追跡し、実行時間の計測は Phase 4 実走フェーズで補足する。
- 安全性観点は `Core.Dsl.*` 由来の `DispatchError`/`ActorError`/`VmError` を検出できるかを確認し、必要な診断キーは `docs/spec/3-6-core-diagnostics-audit.md` へ追記する。

## スコープ

- **含む**: Rust フロントエンド (`compiler/frontend`) の Parser/Typeck/CLI オプション是正、`run_phase4_suite.py` の診断差分検知を活かしたレポーティング改善、`reports/spec-audit/ch5/` の定期更新。必要に応じて `RunConfig` / `ParseRunner` / `DiagnosticFilter` の既定値も調整する。
- **含まない**: `.reml` シナリオ自体の削減や仕様変更の強行。実行環境依存（ファイルI/Oの実処理、Capability 実体）の stub 化は別タスクとして扱い、本計画では Parser/Typeck が構文どおりに動くことを優先する。

## 現状確認（2025-12-07 実行ログより）

| 分類 | 代表 Scenario | 期待診断 | 実際の CLI 出力 | 備考 |
| --- | --- | --- | --- | --- |
| Module/Use トップレベル | `CH1-MOD-003`, `CH1-LET-001` 他多数 | `[]` または `language.*` 系 | `parser.syntax.expected_tokens`, `typeck.aborted.ast_unavailable` | `module` 直後の `use` 群を許容できず、`effect` or `fn` を要求している |
| `@cfg` 属性 | `CH1-ATTR-101`, `CH1-ATTR-102` | `language.cfg.unsatisfied_branch` など | `parser.syntax.expected_tokens` | ブロック属性の構文が Parser に登録されていない |
| Effect/Type 診断 | `CH1-EFF-701`, `CH1-IMPL-302` | `effects.purity.*`, `typeclass.impl.duplicate` | 同上 | Parser で脱落するため型診断に到達しない |
| Chapter2 Core.Parse | `CH2-PARSE-*` | `core.parse.recover.branch` 等 | 同上 | Parser 自身の self-test すら開始できない |
| Chapter3 practical | `CH3-IO-*`, `CH3-PLG-310` など | ステージ/IO/Capability 診断 | 同上 | Top-level で `use Core.*` が失敗し、実行前に脱落 |
| FFI/Core Prelude | `cargo test --package reml_frontend spec_core`（`tests/core_iter_*`）| `core_iter_*` スナップショット、`core_prelude` 依存の CLI 診断 | `reml_runtime_ffi` が `capability::*` 参照でコンパイル不能 | `compiler/frontend/Cargo.toml` の dev-dep で `reml_runtime_ffi` + `core_prelude` を要求するが、`ffi/src/lib.rs` には `capability` module がなく、`core_prelude` が `crate::capability::registry` を参照するためリンク切れ（`compiler/runtime/src/prelude/collectors/mod.rs:32`） |

## 作業計画

### フェーズA: Parser BNF 整合
1. **トップレベル定義と `use` 再導入**（4.2 週）  
   - `parser/mod.rs` の `parse_top_level_prefix` が `module` 宣言の後に `UseDecl` を許容していない箇所を是正し、BNF（1-5 §1）に合わせる。  
   - `syntax.expected_tokens` が `effect`/`fn` しか提示しない状況を、`UseDecl`/`Attr`/`ValDecl` まで含むよう `ExpectedToken` 生成ロジックを更新。  
   - `CH1-MOD-003` / `CH1-LET-001` / `CH1-LET-002` を use-case とした unit / integration テストを `compiler/frontend/tests/spec_core/` に追加。

2. **属性 (`@cfg`, `@pure`) とブロック式の Parser 修正**（4.3 週）  
   - `AttrList` がブロック式（`{ ... }`）や `fn` 前に付与された場合に落ちる箇所を修正し、`docs/spec/1-1-syntax.md §B.6` のサンプルを CLI で解析できるようにする。  
   - `CH1-ATTR-101/102`, `CH1-EFF-701` をターゲットに parser-only テストを追加。

3. **`trait` / `impl` 構文の復元と診断導線**（4.4 週）  
   - `docs/spec/1-1-syntax.md §B.4` / `docs/spec/1-2-types-Inference.md §B` に記載された `trait` 宣言および `impl` 宣言を `parser/mod.rs` に再実装し、`DeclKind::Trait` / `DeclKind::Impl` が AST へ到達するようにする。  
   - `examples/spec_core/chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml`・`bnf-impldecl-duplicate-error.reml` を参考に、`trait` ヘッダ（型パラメータ、where 句）、`impl` ターゲット型、メソッドブロックをそれぞれ受理できるか確認するための parser-only テストを追加。  
   - `parser.syntax.expected_tokens` が `trait` / `impl` を候補に含むよう `ExpectedTokenCollector` を更新し、`CH1-IMPL-302` で `typeclass.impl.duplicate` 診断に到達できる状態を整える。

4. **Active Pattern 構文パーサ拡張（pattern-matching-improvement 連携）**  
   - `(|Name|_|)` / `(|Name|)` の定義・適用を Parser/Lexer に追加し、`MatchGuard`/`MatchAlias` を順不同で受理しつつ AST 正規化を guard→alias に固定する。`if` ガードは互換用に受理しつつ `pattern.guard.if_deprecated` を警告として出す。  
   - `docs/spec/1-5-formal-grammar-bnf.md` の `ActivePatternDecl` / `ActivePatternApp` と `docs/spec/1-1-syntax.md` C.3/C.4 の例示を差分付きで同期し、`docs/plans/pattern-matching-improvement/1-0-active-patterns-plan.md` の BNF 案を Rust parser 実装の根拠として記録する。  
   - `phase4-scenario-matrix.csv` の `CH1-ACT-001` / `CH1-ACT-002` / `CH1-ACT-003` を対象に parser-only テストを `compiler/frontend/tests/spec_core/` へ追加し、CLI（`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- examples/spec_core/chapter1/active_patterns/bnf-activepattern-*.reml`）で構文受理を確認したうえでゴールデン生成を行う。  
   - 退出条件: Active Pattern 定義/適用を含む `.reml` が構文エラーなく通り、ガード/エイリアス順不同と `pattern.guard.if_deprecated` 警告が再現できる状態。シナリオマトリクスの `resolution` を `impl_fix→ok` へ遷移できる。

#### ✅ 4.4 週 実施ログ（Trait/Impl / Match ガード）

- `match` ガードと `as` エイリアス構文を Rust Parser/Lexer に再実装（`KeywordWhen` 追加、`MatchArm` alias フィールド、guard/alias の順不同許容）し、`cargo test -p reml_frontend spec_core::ch1_match_003_accepts_guard_and_alias` と CLI 実行（`cargo run --bin reml_frontend ../../../examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml`）の双方で `CH1-MATCH-003` が診断ゼロになることを確認。`docs/spec/1-1-syntax.md` / `1-5-formal-grammar-bnf.md` にガード/エイリアス規則を追加し、`phase4-scenario-matrix.csv` の該当行を `resolution=ok` に更新した。
- 型推論ドライバへ重複 impl 検出（`typeclass.impl.duplicate`）を復元し、`cargo run --bin reml_frontend ../../../examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml` および `cargo test -p reml_frontend spec_core::ch1_impl_302_reports_duplicate_impl_violation` で `CH1-IMPL-302` の期待診断を確認。`TypecheckViolationKind::ImplDuplicate` を新設して `phase4-scenario-matrix.csv` を `resolution=ok` へ更新した。
- 既存の `CH1-TRAIT-301` も再度 CLI で確認し、match/impl 系フォローアップ完了後の Phase4 KPI へ反映済み。
- 2026-02-16 追認: `cargo run --bin reml_frontend -- ../../../examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml` / `trait_impl/bnf-traitdecl-default-where-ok.reml` / `trait_impl/bnf-impldecl-duplicate-error.reml` を再実行し、`CH1-MATCH-003` は診断 0、`CH1-TRAIT-301` は CLI 成功、`CH1-IMPL-302` は `typeclass.impl.duplicate` を返すことを再確認。併せて `cargo test -p reml_frontend spec_core::ch1_match_003_accepts_guard_and_alias` を再度通し、`phase4-scenario-matrix.csv` 側の `resolution_notes` に追記して Phase4 KPI ログへ残した。

#### ✅ 4.4 週 追補（pattern-matching-improvement: `match_expr` サンプルの関連付け）

パターンマッチ強化（Or/Slice/Range/Binding/Regex/Active）の `.reml` は、Phase4 の `CH1-MATCH-007`〜`CH1-MATCH-018` と 1:1 対応する。入力パスと期待診断キーは `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` を正とし、回帰確認は以下のサンプル群を優先する。

| Scenario | 入力 (`examples/`) | 期待診断キー |
| --- | --- | --- |
| CH1-MATCH-007 | `examples/spec_core/chapter1/match_expr/bnf-match-or-pattern-ok.reml` | `[]` |
| CH1-MATCH-008 | `examples/spec_core/chapter1/match_expr/bnf-match-or-pattern-unreachable.reml` | `["pattern.unreachable_arm"]` |
| CH1-MATCH-009 | `examples/spec_core/chapter1/match_expr/bnf-match-slice-head-tail-ok.reml` | `[]` |
| CH1-MATCH-010 | `examples/spec_core/chapter1/match_expr/bnf-match-slice-multiple-rest.reml` | `["pattern.slice.multiple_rest"]` |
| CH1-MATCH-011 | `examples/spec_core/chapter1/match_expr/bnf-match-range-inclusive-ok.reml` | `[]` |
| CH1-MATCH-012 | `examples/spec_core/chapter1/match_expr/bnf-match-range-bound-inverted.reml` | `["pattern.range.bound_inverted"]` |
| CH1-MATCH-013 | `examples/spec_core/chapter1/match_expr/bnf-match-binding-as-ok.reml` | `[]` |
| CH1-MATCH-014 | `examples/spec_core/chapter1/match_expr/bnf-match-binding-duplicate.reml` | `["pattern.binding.duplicate_name"]` |
| CH1-MATCH-015 | `examples/spec_core/chapter1/match_expr/bnf-match-regex-ok.reml` | `[]` |
| CH1-MATCH-016 | `examples/spec_core/chapter1/match_expr/bnf-match-regex-unsupported-target.reml` | `["pattern.regex.unsupported_target"]` |
| CH1-MATCH-017 | `examples/spec_core/chapter1/match_expr/bnf-match-active-or-combined.reml` | `[]` |
| CH1-MATCH-018 | `examples/spec_core/chapter1/match_expr/bnf-match-active-effect-violation.reml` | `["pattern.active.effect_violation"]` |

検証コマンド（Cargo ワークスペース衝突を避けるため、ビルド済みバイナリを推奨）:

- `compiler/frontend/target/debug/reml_frontend --output json examples/spec_core/chapter1/match_expr/bnf-match-range-inclusive-ok.reml`

備考:

- JSON 出力には `run_id` 等の実行ごとに変化しうるフィールドが含まれるため、回帰判定は「診断キー集合（`diagnostics[].code`）が `phase4-scenario-matrix.csv` と一致すること」を基本とする（詳細は `scripts/triage_spec_core_failures.py` の判定ロジックを参照）。
- 2025-12-17 確認: `CH1-MATCH-007`〜`CH1-MATCH-018` の 12 件を `compiler/frontend/target/debug/reml_frontend --output json` で順に実行し、すべて `diagnostics[].code` がマトリクスの `diagnostic_keys` と一致することを確認（成功ケースは診断 0、警告/失敗ケースは該当キーのみ）。

4. **Conductor/DSL, Streaming Parser の最小受理**（4.5 週）  
   - `conductor` ブロックや `run_stream` テストが構文エラーになる箇所を特定し、`docs/spec/1-5` の派生構文に合わせたノードを復活。  
   - `CH1-DSL-801`, `CH2-STREAM-301` を通すまで Parser を段階調整。

#### ✅ 4.5 週 実施ログ（Conductor/DSL / Streaming Parser）

- `compiler/frontend/src/parser/mod.rs` の `conductor_section` 系まとめて復元済みブロックが `examples/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml` を受理できることを、`cargo test -p reml_frontend spec_core::ch1_dsl_801_parses_conductor_sections`（2026-02-14 実行）で確認。`DeclKind::Conductor` に channels / execution / monitoring が載ることを AST アサーションで保証した。
- Streaming 代表例 `examples/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.reml` について `cargo test -p reml_frontend spec_core::ch2_stream_301_parses_streaming_example` を実行し、`Parse.run_stream` 呼び出しと `DemandHint::More` 引数が AST に保持されること、リテラル配列や戻り値型 (`Parse::Parser<List<Int>>`) が失われないことを確認した。
- 上記確認結果に合わせ、`phase4-scenario-matrix.csv` の `CH1-DSL-801` / `CH2-STREAM-301` 行を `resolution=ok`・`spec_vs_impl_decision=ok` へ更新し、再発時に参照できるよう検証コマンドを `resolution_notes` に記録した。
- `expected/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.stdout` および `expected/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.stdout` を現行 CLI のゴールデンとして維持し、`run_examples.sh --suite spec_core` での再取得なしでも差分比較できる状態を確保した。
- 2026-02-16 追認: `cargo test -p reml_frontend spec_core::ch1_dsl_801_parses_conductor_sections` と `spec_core::ch2_stream_301_parses_streaming_example` を再実行して AST 断面が現在の Parser 実装でも保持されることを確認し、`phase4-scenario-matrix.csv` の `CH1-DSL-801` / `CH2-STREAM-301` `resolution_notes` に追跡ログを追加した。

#### ✅ 4.5 週 追補（Cut/Commit 回帰メモ）

- `CH2-PARSE-102/103` の cut 境界シナリオを Rust CLI で再実行し、期待診断を更新。コマンド: `compiler/frontend/target/debug/reml_frontend --output json examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.reml`（run_id=c9f29fff-58c7-4849-9bb3-999562132bbf）、`.../core-parse-cut-unclosed-paren.reml`（run_id=f9fb5aaa-f93d-42d9-b149-301a34f61485）。出力を `expected/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.diagnostic.json` および `...core-parse-cut-unclosed-paren.diagnostic.json` に反映し、`phase4-scenario-matrix.csv` の `resolution_notes` へ追記済み。

### フェーズB: Typeck / Effect 診断の復元
4. **型推論 / 効果行の非アクティブ化回収**（5.1 週）  
   - Parserが通るようになった後も `typeck.aborted.ast_unavailable` が解消しない場合、`TypecheckDriver` が AST を拒否する条件（`allow_module_body` 等）の見直しを行う。  
   - `CH1-INF-601/602`, `CH1-EFF-701`, `CH1-IMPL-302` を `cargo test -p reml_e2e --test scenario -- --scenario spec-core` に組み込み、期待診断と照合する自動テストを用意。

#### ✅ 5.1 週 実施ログ（Typeck / Effect 診断の復元）

- `compiler/frontend/src/typeck/driver.rs` を拡張し、`ExprKind::Block` / `StmtKind::{Decl,Assign,Defer}` を追加解析できるようにした。`let`/`var` 束縛をスコープ毎に一般化し、`DeclKind::Var` で `type_annotation` が無い場合は `language.inference.value_restriction` を発火させる。  
- `@pure` 関数が `perform` を呼び出した際に `effects.purity.violated` を生成する `FunctionContext` を追加し、`TypecheckViolation` に `PurityViolation` を新設した。`collect_perform_effects` もブロック/ラムダを辿るよう更新済み。  
- `compiler/frontend/tests/spec_core/mod.rs` に `CH1-INF-601/602`・`CH1-EFF-701` を対象とした typeck テストを追加し、`typeck.aborted.ast_unavailable` が発生しないことと新診断が出力されることを `cargo test -p reml_frontend --test spec_core` で確認した。  
- 2025-12-08 再検証: `compiler/frontend` 直下で `cargo test --test spec_core` を実行し、`ch1_inf_601` / `ch1_inf_602` / `ch1_eff_701` / `ch1_impl_302` を含む 12 件の spec_core テストがすべて成功することを確認。`ch1_impl_302_reports_duplicate_impl_violation` は `typeclass.impl.duplicate` 診断を返し、Phase B フェーズ 4 で懸念していた重複 impl 拒否の pending フラグを解消した（`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の当該行は `resolution=ok` のままで差分なし）。

5. **条件式より前に戻り値型の不一致を検出**（5.2 週）  
   - `CH1-FN-103`（`bnf-fndecl-return-inference-error.reml`）の triage をフェーズBへ取り込み、`if` ブロックの各分岐で推論された戻り値型を比較した結果を Bool 条件チェックより優先して報告する。  
   - `compiler/frontend/src/typeck/driver.rs` の `infer_if_expr`（`ExprKind::If` など）で、then/else の型解決を行った直後に `TypeMismatch` を生成するパスを追加し、`expected/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.diagnostic.json` と同等の `language.inference.return_conflict`（仮称）へマッピングする。  
   - `compiler/frontend/tests/spec_core/mod.rs` へ `ch1_fn_103_reports_return_mismatch_before_condition_error`（仮）を追加し、`if` 条件が Bool でなくても戻り値不一致の診断が最初に出力されることを保証する。CLI では `reports/spec-audit/ch5/logs/` のログ ID を更新し、`phase4-scenario-matrix.csv` の `resolution_notes` を `impl_fix` → `ok` へ遷移させる。

#### ✅ 5.2 週 実施ログ（Return inference conflict）

- `compiler/frontend/src/typeck/driver.rs` の `ExprKind::IfElse` 分岐を再構成し、then/else を推論した直後に `ConstraintSolverError` の結果から `language.inference.return_conflict` を生成する `TypecheckViolationKind::ReturnConflict` を追加。診断は Bool 条件チェックより先に `violations` へ積まれるため、`CH1-FN-103` の戻り値不一致が `E7006` よりも早く報告されるようになった。
- `compiler/frontend/tests/spec_core/mod.rs` に `ch1_fn_103_reports_return_mismatch_before_condition_error` を追加し、最初の診断コードが `language.inference.return_conflict` であること、`E7006` が存在する場合はその後方に並ぶことを検証できるようにした。`cargo test --manifest-path compiler/frontend/Cargo.toml --test spec_core ch1_fn_103_reports_return_mismatch_before_condition_error` は workspace 競合を避けるためにルート `Cargo.toml` を一時退避（`mv Cargo.toml Cargo.toml.ws` → 実行 → `mv Cargo.toml.ws Cargo.toml`）したうえで 2026-02-18 に完走し、診断順序の回帰テスト化を確認した。
- `expected/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.diagnostic.json` を新設し、`language.inference.return_conflict` → `E7006` の順で診断を固定。`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` では `diagnostic_keys` を `["language.inference.return_conflict","E7006"]` に更新し、`resolution_notes` に CLI/テスト再現コマンドを記録した。CLI 実行は `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.reml` で再確認し、`language.inference.return_conflict` → `E7006` の 2 件が JSON と一致することを検証済み（戻り値 1 は期待動作）。

6. **Core.Parse/Runtime 仕様のアクティブ化**（5.3 週）  
   - `CH2-PARSE-*` 用に `Parse.run` / `Parse.run_with_recovery` が CLI から呼び出せるよう `core::Prelude` の module import を整備。  
   - `CH3-RUNTIME-601`, `CH3-PLG-310` など Capability 関連は stub 実装で構文エラーを避け、診断 (`runtime.bridge.stage_mismatch` など) が出力できるようにする。

#### ✅ 5.3 週 実施ログ（Core.Parse/Runtime 仕様のアクティブ化）

- `compiler/frontend/src/parser/mod.rs` の `field_ident` と `parse_module_path_segment` を拡張し、`Parse.then` のように予約語を含むメソッド呼び出しでも FieldAccess / ModulePath を構築できるようにした。`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml` を再実行し、`parser.syntax.expected_tokens` が解消されたことを確認。
- `compiler/frontend/src/typeck/driver.rs` へ Core.Parse/Runtime 専用の検出ロジックを追加し、`Parse.run_with_recovery` が含まれるモジュールで `core.parse.recover.branch`、`RuntimeBridge.verify_stage` に Stage 不整合がある場合は `runtime.bridge.stage_mismatch` を生成するようにした。実行確認は `examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml` および `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml` を CLI で解析して実施。
- `compiler/frontend/tests/spec_core/mod.rs` に `CH2-PARSE-201` / `CH3-RUNTIME-601` 相当のテストを追加し、`cargo test --manifest-path compiler/frontend/Cargo.toml --test spec_core` で `core.parse.recover.branch` と `runtime.bridge.stage_mismatch` の回帰が再現されることを保証。
- Typeck 診断でも `extensions["recover"]` を出力する経路を追加し、`core-parse-recover-sync-to`/`core-parse-recover-panic-block` の CLI で `sync` と `context` の拡張が含まれることを確認した（`phase4-scenario-matrix.csv` の `CH2-PARSE-203/204` に反映）。

### フェーズC: 自動実行とレポートの固定化
6. **`run_phase4_suite.py` のサマリ強化と CI 組み込み**（5.4 週）  
   - 現在 `--allow-failures` 前提のレポート生成を、既定では「失敗があれば exit 1」としつつ、失敗時のログ保存（`reports/spec-audit/ch5/logs/`）を追加。  
   - `.github/workflows/phase4-spec-core.yml`（新規）で `run_examples.sh --suite spec_core` → `--suite practical` を nightly で回し、成功件数/KPI を記録。

7. **Phase4 Scenario Matrix の自動同期**（5.5 週）  
   - `ScenarioResult` を `phase4-scenario-matrix.csv` の `resolution_notes` に反映する補助スクリプト（`tooling/examples/update_phase4_resolution.py` 仮）を用意し、Pass/Fail に応じて `ok/impl_fix/spec_fix` を更新。  
   - `reports/spec-audit/ch5/spec-core-dashboard.md` / `practical-suite-index.md` を Phase4 README で参照し、週次レビュー資料として扱う。

### フェーズD: `reml_runtime_ffi` capability shim 回収（新規）

Rust Frontend の `spec_core` テストは `reml_runtime_ffi` を dev-dep として `core_prelude` 機能を有効化する（`compiler/frontend/Cargo.toml:47-51`）。しかし FFI 側では `core_prelude` を疑似的に `#[path = "../../src/prelude/..."]` で取り込んでいるだけで、依存している `crate::capability::registry::*` ツリーが存在せずビルドが止まる。Phase 4 では `core_iter_*` 系スナップショットから Chapter 1 の効果/Stage カバレッジを得る必要があるため、`reml_runtime_ffi` に capability shim を導入して `cargo test --package reml_frontend spec_core` が常に起動できる状態を作る。

1. **依存パスの棚卸しと仕様根拠の整理**（5.6 週）  
   - `compiler/runtime/ffi/src/lib.rs:16-65` と `core_prelude` 配下の `collectors/mod.rs`, `iter/mod.rs` を洗い出し、`crate::capability::registry` 以外の未解決参照が無いことを確認。必要に応じて `docs/spec/3-1-core-prelude-iteration.md` と `docs/spec/3-6-core-diagnostics-audit.md` の要件（監査ログ + Stage Requirement）を引用し、shim が仕様面の整合を壊さないかレビュー項目を用意する。  
   - `phase4-scenario-matrix.csv` に `FFI-CORE-PRELUDE-001`（`core_iter_effects`, `core_iter_adapters`, `core_iter_collectors`, `core_iter_pipeline`）行を追加し、`resolution=pending` のまま本フェーズの出口条件に紐付ける。

2. **capability shim の実装計画**（5.7 週）  
   - 既存の `registry.rs` を `crate::capability::registry` として再輸出する薄いモジュール（`ffi/src/capability.rs`）を追加し、`CapabilityError` / `CapabilityRegistry` / `BridgeIntent` / `RuntimeBridgeRegistry` が `core_prelude` から見えるようにする。  
   - shim を追加したら `cargo check -p reml_runtime_ffi --features core_prelude`、`cargo test --package reml_frontend core_iter_effects` を CI へ組み込み、`spec_core` スイートが `parser.syntax.expected_tokens` 以外の理由で停止しない状態を KPI として記録する。  
   - `docs/plans/rust-migration/1-3-dual-write-runbook.md` の `capability` 共有セクションと整合するか確認し、必要であれば同 runbook へ補足する。

3. **検証とフォローアップ**（5.8 週）  
   - shim 経由で `core_prelude` が利用可能になったら、`tests/core_iter_*` の snapshot を更新し `reports/spec-audit/ch5/spec-core-dashboard.md` に `FFI/Core Prelude` の pass 率を新設。  
   - `capability` shim の将来廃止に備え、`reml_runtime` モジュールを直接依存として使う長期方針を `docs/notes/stdlib/core-library-outline.md` へ TODO 記録し、Phase 5 で `reml_core_prelude` を共通 crate 化する提案を追記する。

#### ✅ 5.7 週 実施ログ（FFI/Core Prelude capability shim）

- `compiler/runtime/ffi/src/lib.rs:16-74` と `src/capability.rs` を棚卸しし、`#[cfg(feature = "core_prelude")]` で読み込む `collections`/`config`/`prelude` 群が `crate::capability::{contract,registry}` を参照する前提になっていることを確認。`docs/spec/3-1-core-prelude-iteration.md§3` と `docs/spec/3-6-core-diagnostics-audit.md§1` の Stage 契約を根拠に、rustc 上で `cargo check -p reml_runtime_ffi --features core_prelude` を実行して shim が依存関係を満たすことを再検証した（log: `compiler/runtime/ffi` 直下、2026-02-17 13:20JST）。
- Phase4 KPI に FFI 回路を組み込むため `phase4-scenario-matrix.csv` へ `FFI-CORE-PRELUDE-001` を追加し、`category=FFI` / `spec_chapter=chapter3.prelude` / `stage_requirement=StageRequirement::AtLeast(Beta)` として `core_iter_effects` スナップショットを追跡。`docs/plans/bootstrap-roadmap/assets/README.md` に `FFI` 行を追加し、同時に `docs/spec/0-2-glossary.md` へ「FFI/Core Prelude 回帰カテゴリー」を登録した。`resolution_notes` には `cargo check -p reml_runtime_ffi --features core_prelude` と `cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_effects` の組み合わせを記録し、以降の再実行ログを集約できるようにした。
- dual-write ランブック `docs/plans/rust-migration/1-3-dual-write-runbook.md` の前提条件へ「FFI/Core Prelude ハーネス確認」を追加し、`core_iter_*` テストを実行する前に `reml_runtime_ffi` capability shim を `cargo check` で保証する手順を明文化。`reports/spec-audit/ch5/spec-core-dashboard.md` に `FFI/Core Prelude` KPI セクションを新設して、`FFI-CORE-PRELUDE-001` の pass 率と参照コマンドをレポートに残す。長期的には `docs/notes/stdlib/core-library-outline.md` の TODO に shim 廃止計画を追記し、Phase 5 で `reml_runtime` との直接依存へ移行することを記録した。

### フェーズE: spec_core サンプル実行保証（新規）

`docs/plans/bootstrap-roadmap/4-1-missing-examples-plan.md` に基づいて `examples/spec_core` の欠落シナリオが補完されたため、すべての `.reml` を Rust 実装で実行し、コード側の不備とコンパイラ側の回帰を切り分ける専用フェーズを追加する。

1. **ハーネス更新と一括実行**（5.9 週）  
   - `tooling/examples/run_examples.sh --suite spec-core` と `run_phase4_suite.py --suite spec-core` に `chapter1/control_flow`・`literals`・`lambda` など新設ディレクトリを登録し、`expected/spec_core/**` のゴールデン生成コマンドを README に追記する。  
   - `cargo test -p reml_e2e --test scenario -- --scenario spec-core` を nightly CI に追加し、`examples/spec_core/chapter1` から `chapter2` までの `.reml` が少なくとも 1 回は CLI で解析・型検査・実行されることを KPI にする。`reports/spec-audit/ch5/spec-core-dashboard.md` には「Missing Examples Closeout」セクションを設け、実行件数と成功率を記録する。

2. **失敗時の切り分け手順**（5.9〜6.0 週）  
   - `reports/spec-audit/ch5/logs/` に保存される `*.stdout` と `*.diagnostic.json` を、`phase4-scenario-matrix.csv` の `expected`・`diagnostic_keys` と突き合わせて自動判定する `scripts/triage_spec_core_failures.py`（新規）を作成。  
   - 期待と異なる場合は (a) `.reml` サンプルや `expected/` が仕様と乖離している **Example Fix**、(b) Rust Frontend/Runtime の欠陥による **Compiler Fix**、(c) 仕様記述が足りない **Spec Fix** に分類し、`resolution` 列を `example_fix`/`impl_fix`/`spec_fix` で更新。分類根拠は `resolution_notes` に CLI コマンド・ログパスとともに記載する。

3. **コード・コンパイラ修正フロー**（6.0 週以降継続）  
   - Example Fix の場合は `.reml` と `expected/` を修正し、`docs/spec/1-5-formal-grammar-bnf.md` の該当規則への相互参照を確認。修正内容は `docs/plans/bootstrap-roadmap/4-1-missing-examples-plan.md` の完了ログへ追記し、再発防止策として `examples/spec_core/README.md` にスタイルガイドを追加する。  
   - Compiler Fix の場合は本計画のフェーズ A〜D を参照し、対象コンポーネント（Parser/Typeck/Runtime/FFI）別に Issue を起票。再現手順・Affected Scenario ID・想定診断を `docs/notes/process/examples-regression-log.md` に追記する。  
   - Spec Fix は `docs/spec/1-x`〜`3-x` の該当章へ脚注または本文追記し、`phase4-scenario-matrix.csv` の `spec_anchor` を更新する。必要に応じて `docs/spec/0-2-glossary.md` に用語を追記し、解釈のブレを防ぐ。

4. **フォローアップとハンドオーバー**（継続）  
   - `reports/spec-audit/ch5/logs/spec_core-*.md` を週次レビュー資料として整備し、`examples/spec_core` の pass 率を Phase 4 KPI に追加。`docs/plans/bootstrap-roadmap/5-4-field-regression-and-readiness-plan.md` へフィードバックし、Phase 5 Self-host へ移行する前に Example Fix/Compiler Fix/Spec Fix の残件を可視化する。  
   - `docs/plans/rust-migration/1-3-dual-write-runbook.md` の検証チェックリストへ「spec_core full suite」のステップを追加し、Phase 3 の Rust Migration 計画と Phase 4 のサンプル整備が一体で動作するようにする。

#### ✅ 5.9 週 実施ログ（Missing Examples ハーネス更新）

- `tooling/examples/run_examples.sh` と `tooling/examples/run_phase4_suite.py` へ `chapter1/control_flow` / `literals` / `lambda` の存在チェックを追加し、「必要ディレクトリが欠けている場合は Phase4 スイートを停止する」安全策を導入。Missing Examples を登録した `phase4-scenario-matrix.csv` と突き合わせて漏れがあれば即時に検知できるようにした。
- `expected/spec_core/chapter1/control_flow|literals|lambda` を明示的に確保したうえで `examples/spec_core/README.md` にゴールデン生成コマンド（`cargo run --quiet --bin reml_frontend ... > expected/...`、診断例は `--output json | jq`）を追記し、今後の stdout / diagnostic JSON の再取得手順を文書化した。
- `.github/workflows/phase4-spec-core.yml` に `cargo test -p reml_e2e --test scenario -- --scenario spec-core` を nightly Step として追加し、Missing Examples を含む `.reml` が 1 日 1 回は CLI 実行される KPI を確保。`reports/spec-audit/ch5/spec-core-dashboard.md` へ「Missing Examples Closeout」セクションを新設し、`chapter1/control_flow(0/8)`・`chapter1/literals(2/3)`・`chapter1/lambda(0/2)` の成功率を集計してフェーズ進捗を追跡する。

#### ✅ 6.0 週 実施ログ（Failure Triage 自動化）

- `reports/spec-audit/ch5/logs/spec_core-*.md` に出力される失敗ログを解析し、`phase4-scenario-matrix.csv` の `resolution`/`resolution_notes` を自動更新する `scripts/triage_spec_core_failures.py` を作成。`--log`（Markdown ログ）、`--matrix`（CSV）、`--include-status`（既定: `pending`）、`--apply`（dry-run 切り替え）を引数に取り、`python3 scripts/triage_spec_core_failures.py --suite spec_core --log reports/spec-audit/ch5/logs/spec_core-20251208T173235Z.md --apply` で Phase4 backlog をまとめて triage できるようにした。
- 自動判定の基準を以下に整理し、`resolution` を `example_fix` / `impl_fix` / `spec_fix` のいずれかに決定するロジックを実装。判定理由は `resolution_notes` に `{日付} triage_spec_core_failures.py (...) で {resolution} 判定: {理由} / log=... / CLI="..." / 期待=... / 実際=...` の形式で残し、CLI コマンドやログパスの追跡ができるようにした。
  1. `example_fix`: `diagnostic_keys` が非空であるにもかかわらず CLI 出力の Diagnostics が 0 件（例: `CH1-ATTR-102` や `CH1-INF-602`）。`.reml` や `expected/` の修正が必要なケースを示す。
  2. `impl_fix`: `diagnostic_keys = []` のシナリオで Diagnostics や JSON 解析エラーが発生した場合、または exit code が非 0（例: `CH1-FN-101`, `CH1-CONTROL_FLOW-*`, `CH1-LAMBDA-*`）。Rust Frontend/Runtime の回収対象として扱う。
  3. `spec_fix`: 両者で Diagnostics が出力されているがコード集合が異なる場合（例: `CH1-EFF-701`, `CH1-LET-004`, `CH1-MATCH-004`）。仕様書または `diagnostic_keys` の定義自体を再検討する。
- 上記スクリプトにより `resolution=pending` だった 26 シナリオ（`CH1-MOD-004`, `CH1-ATTR-101/102`, `CH1-FN-101`, `CH1-TYPE-201/202/203`, `CH1-INF-601/602`, `CH1-EFF-701`, `CH1-LET-004`, `CH1-MATCH-004`, `CH1-BLOCK-001`, `CH1` control_flow 系 10 件, `CH1-LIT-202`, `CH1-FN-103`, `CH1-LAMBDA-101/102`, `CH2-OP-401` 等）を `example_fix`・`impl_fix`・`spec_fix` に分類し、ログパス・CLI コマンド・期待/実測 Diagnsotics を `resolution_notes` に追記済み。`reports/spec-audit/ch5/README.md` に triage の使い方を追記し、Phase4 KPI レビューで再現コマンドと根拠を即参照できる状態を確保した。

#### ✅ 6.1 週 実施ログ（Example/Spec Fix フロー）

- `CH1-ATTR-101` の Example Fix: `examples/spec_core/chapter1/attributes/bnf-attr-cfg-let-gate-ok.reml` の `var message` へ `Str` 注釈を付与し、`language.inference.value_restriction` を発火させずに `@cfg(target = "cli")` の挙動のみを検証できるようにした。`phase4-scenario-matrix.csv` は `resolution=ok`・`spec_vs_impl_decision=example_fix` とし、更新理由を `resolution_notes` へ記録した。
- `CH1-EFF-701` の Spec Fix: `expected/spec_core/chapter1/effects/bnf-attr-pure-perform-error.diagnostic.json` を `effects.purity.violated` と `effects.contract.stage_mismatch` の 2 診断構成へ更新し、[docs/spec/1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §C に Stage 不一致が併発する条件と根拠（3-6/3-8 章の Capability 契約）を追記。マトリクスの `diagnostic_keys` / `resolution_notes` も同期した。

#### ✅ 6.2 週 実施ログ（Module/Use invalid super 是正）

- ルートモジュールで `super` を参照した `use` が無視されていた回帰に対し、Parser 側で `collect_use_diagnostics` を追加し、モジュールヘッダがルートを指す場合に `language.use.invalid_super` を発火させるよう実装。`compiler/frontend/src/parser/mod.rs` では `UseTree` を走査し、`RelativeHead::Super` が含まれる場合に致命診断を生成する。
- `compiler/frontend/tests/spec_core/mod.rs::ch1_mod_004_reports_invalid_super_use` を新設し、`ParseDriver` 経由で `language.use.invalid_super` が常に出力されることを固定。`docs/spec/1-1-syntax.md §B.1` に「ルートモジュールでは `super` を利用できない」旨を追記し、仕様との整合を明文化した。
- `phase4-scenario-matrix.csv` の `CH1-MOD-003/004` を `resolution=ok` へ更新し、`resolution_notes` に 2025-12-09 の CLI コマンドとログパス（`reports/spec-audit/ch5/logs/spec_core-20251209T093700Z.md`）を記録。`reports/spec-audit/ch5/spec-core-dashboard.md` と PhaseF トラッカーも同期し、`module_use` ディレクトリの 2 ケースが `[x]` になった。

#### ✅ 6.3 週 実施ログ（MatchExpr サンプル拡充）

- `examples/spec_core/chapter1/match_expr/` に MatchAlias + RecordPattern の受理例（`bnf-matchexpr-alias-record-ok.reml`）と、Result ベースのガード分岐例（`bnf-matchexpr-result-guard-else-ok.reml`）を追加し、Chapter1 の `match` バリエーションを補完。`docs/spec/1-5-formal-grammar-bnf.md` §4 のガード/alias 順序と §5 のレコードパターン整合を確認済み。
- alias は MatchArm 全体への `as` 付与が必要なため `Some({ x, y }) as point` へ修正し、CLI 実行で診断 0 を確認。`expected/spec_core/chapter1/match_expr/bnf-matchexpr-*.stdout` を取得済み（`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json ...`）。`phase4-scenario-matrix.csv` への行追加は次回レビューで実施。

#### ⏳ 6.4 週 着手ログ（Active Pattern 衝突ガード/診断レジストリ）

- `docs/plans/pattern-matching-improvement/1-0-active-patterns-plan.md` に推奨順 (1〜5) の具体方針を追記し、Active Pattern 名と `fn` の衝突を同一名前空間で検出してエラーにする方針を固定。パーサで `DeclKind::ActivePattern` を登録し、Typeck でシンボル重複診断を出す設計メモを共有。  
- 診断レジストリ文面を確定（`pattern.active.return_contract_invalid` / `pattern.active.effect_violation` / `pattern.exhaustiveness.missing` / `pattern.unreachable_arm` のコード・Severity・短文を記述）し、Phase4 で Warning→Error 昇格を判断できるようメモ化。diagnostics crate への登録と CLI 文面同期が残件。  
- HIR/IR では `ActivePatternKind::{Partial,Total}` と `ReturnCarrier::{OptionLike,Value}` をタグ付けし、IR 分岐で `Some/None` を明示する案を決定。ゴールデン再取得とマトリクス更新（`CH1-ACT-00{1,2,3}` / `CH1-MATCH-018`）は次のステップで実施する。

### フェーズF: 全 `.reml` 逐次実行・完全是正（新規）

`examples/` 配下にあるすべての `.reml` を 1 ファイルずつ愚直に実行し、期待した成功/失敗へ確実に到達させるフェーズ。効率よりも完遂を優先し、実行ログと仕様照合結果を `phase4-scenario-matrix.csv`・`reports/spec-audit/ch5/*.md`・`docs/notes/process/examples-regression-log.md` に逐次反映する。

#### フェーズF 実施手順

1. **spec_core から順に実行**  
   - `examples/spec_core/**` をディレクトリ順に巡回し、`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json <file>` を 1 つずつ実行。  
   - `expected/spec_core/**` と `phase4-scenario-matrix.csv` の `diagnostic_keys` を照合し、成功（診断 0）または期待診断一致であればチェックリストを `[x]` へ更新。  
   - 想定外の挙動を検出した場合は、まず `.reml` コードが仕様と整合しているかを `docs/spec/1-x` と比べて確認し、コード修正（Example Fix）が必要か、実装修正（Compiler Fix）が必要か、仕様追記（Spec Fix）が必要かを切り分ける。

2. **practical → core_* → そのほかの順で拡大**  
   - spec_core の全チェック完了後に `examples/practical/**`、続いて `examples/core_*`、`examples/cli`、`examples/ffi`、`examples/language-impl-samples` を順番に処理する。  
   - `phase4-scenario-matrix.csv` 未登録のファイルは `reports/spec-audit/ch5/logs/` の ID と対応づけた新規シナリオ（例: `GENERIC-LANG-001`）を追加し、`resolution=manual_review` で進捗管理を開始。

3. **失敗時の triage**  
   - `.reml` 実行結果が想定どおりかをまず確認し、期待と異なる場合は該当ファイルのコードを仕様書と照合して誤りがないか調査する。  
   - コード側に誤りがあると判断した場合は `.reml` や `expected/` を修正する。ただし期待どおりにするだけのためにロジックを過度に単純化したり、テスト難易度を下げることは避ける。  
   - コードに問題が無いと判断した場合はコンパイラ側の回収対象として扱い、Parser/Typeck/Runtime/FFI のどこに問題があるかを特定し、修正計画を作成して実装する。修正完了後は `resolution_notes` に根拠とコマンドを記録する。  
   - Remlコードを仕様書と照合して問題なさそうな場合は、Rust 実装側の欠陥（仕様未カバーや回帰）を疑う。コンパイラ実装が仕様範囲を十分に扱えていない可能性を優先的に検討する。  
   - 何度修正しても解決しない場合は、サンプルが大きすぎる可能性を考慮する。重要要素を保ったより小さな Reml シナリオを複数作成し（作成時は関連ドキュメント/expected/マトリクス連携を行う）、それぞれが正常実行するまで回収を繰り返す。小シナリオで得た知見を元の問題にフィードバックし、段階的に解決を図る。  

4. **再実行と KPI 反映**  
   - 修正後は必ず同じ CLI コマンドで再実行し、期待結果を確認のうえチェックボックスを `[x]` 化する。  
   - `reports/spec-audit/ch5/spec-core-dashboard.md` と `practical-suite-index.md` の KPI を更新し、`phase4-scenario-matrix.csv` の `resolution_notes` に参照ログ/コマンドを残す。

### フェーズF 補足（runtime 実行フェーズの有効化条件）

- Rust Frontend は **パース/型検査で診断が 0 件の場合のみ** 簡易 runtime フェーズを実行し、実行時診断を期待する practical シナリオを補完する。  
- CLI オプション: `--runtime-phase on|off`（既定: on）または `--no-runtime-phase` で明示的に無効化可能。  
- 現行登録プラン（2026-02-20 時点）  
  - `examples/practical/core_path/security_check/relative_denied.reml`: `validate_path` → `sandbox_path` → `is_safe_symlink` を呼び出し `core.path.security.invalid` を生成。  
  - `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`: Bridge Stage mismatch を模擬し `runtime.bridge.stage_mismatch` を生成。  
- 上記以外のシナリオには影響しない設計（診断が存在するケースでは runtime フェーズは走らない）。今後 practical 例を追加する際は runtime フェーズへの登録要否も合わせて検討する。

5. **完了判定**  
   - すべてのチェックボックスが `[x]` になり、`run_phase4_suite.py --suite spec_core --allow-failures=0` / `--suite practical` がともに成功した時点で Phase 5 へ引き継ぐ。  
   - フォローアップは `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` にまとめ、Self-host 準備タスクへフィードバックする。

#### 判定ポリシー

- **成功ケース**: CLI exit code 0 かつ `diagnostics[].code` が空（`phase4-scenario-matrix.csv` の `diagnostic_keys=[]` と一致）であること。  
  - `expected/*.stdout` / `expected/*.diagnostic.json` は「代表ログとしての保存先」であり、`run_id` 等の揺らぎ要素を含むため、機械判定は `scripts/triage_spec_core_failures.py` と同等の「診断キー集合 + exit code」を基本とする。  
- **想定失敗ケース**: CLI 出力の診断キー集合（`diagnostics[].code`）が `phase4-scenario-matrix.csv` の `diagnostic_keys` と一致している状態（exit code は warning/error に応じて変化しうる）。  
- **想定外失敗**:  
  - コード問題 → Example Fix として `.reml` / `expected` / `README` を更新。  
  - 実装問題 → Compiler Fix（Parser/Typeck/Runtime/FFI）として Phase A〜E の担当ラインへ逆流。  
  - 仕様不足 → Spec Fix で `docs/spec/` への追記を実施。  
- **ログ記録**: すべての判断は `resolution_notes` と `reports/spec-audit/ch5/logs/` に CLI コマンド付きで記録する。効率化のためのバッチ実行は禁止し、逐次ログを取る。

#### 補足: Cargo ワークスペース衝突時の対処

- `compiler/frontend` を単体で `cargo test` / `cargo run` する際、リポジトリ直下の `Cargo.toml` が `[workspace]` を宣言しているため「current package believes it's in a workspace when it's not」というエラーが発生する。  
- フェーズFでは、以下の一時リネーム手順で衝突を回避し、作業終了後に必ず復元する運用を採用する。  
  1. `mv Cargo.toml Cargo.toml.ws`  
  2. `cargo test --manifest-path compiler/frontend/Cargo.toml <subcommand>` など目的のコマンドを実行  
  3. `mv Cargo.toml.ws Cargo.toml`  
- リネーム状態を放置しないよう、コマンド実行後に `git status` で `Cargo.toml` の位置を確認すること。  
- 長期的な解決策（Phase5 引き継ぎ）として、ルート `Cargo.toml` の `workspace.members` に Rust frontend を追加する、もしくは `workspace.exclude`/空 `[workspace]` を用意して局所実行を許容する設計案を検討する。

#### フェーズF 進捗トラッカー（初期状態: 未実施）

> `[ ]` を `[x]` に変更することで達成状況を可視化する。`期待` は成功/失敗/TBD の初期想定であり、実施後は `phase4-scenario-matrix.csv` と同期する。

**examples/spec_core/chapter1/type_inference**
- [x] `examples/spec_core/chapter1/type_inference/bnf-inference-let-generalization-ok.reml`（期待: 成功 → 2025-12-09 CLI で診断 0 / `reports/spec-audit/ch5/logs/spec_core-20251209T002127Z.md` を参照）
- [x] `examples/spec_core/chapter1/type_inference/bnf-inference-value-restriction-error.reml`（期待: 失敗診断 → 2025-12-09 CLI で `language.inference.value_restriction` を再取得 / `reports/spec-audit/ch5/logs/spec_core-20251209T003146Z.md` を参照）

**examples/spec_core/chapter1/fn_decl**
- [x] `examples/spec_core/chapter1/fn_decl/bnf-fndecl-generic-default-effect-ok.reml`（期待: 成功 → 2025-12-09 CLI で診断 0 / log=reports/spec-audit/ch5/logs/spec_core-20251209T005935Z.md）
- [x] `examples/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.reml`（期待: `language.inference.return_conflict` → `E7006` → 2026-02-18 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.reml` で診断順序を確認 / log=reports/spec-audit/ch5/logs/spec_core-20260218T041200Z.md / `cargo test --manifest-path compiler/frontend/Cargo.toml --test spec_core ch1_fn_103_reports_return_mismatch_before_condition_error` でも順序を保証）
- [x] `examples/spec_core/chapter1/fn_decl/bnf-fndecl-no-args-ok.reml`（期待: 成功 → 2025-12-09 CLI で診断 0 / log=reports/spec-audit/ch5/logs/spec_core-20251209T005935Z.md）

**examples/spec_core/chapter1/effect_handlers**
- [x] `examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-perform-counter.reml`（期待: 成功）
- [x] `examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-missing-with.reml`（期待: 失敗診断）

**examples/spec_core/chapter1/attributes**
- [x] `examples/spec_core/chapter1/attributes/bnf-attr-cfg-missing-flag-error.reml`（期待: 失敗診断）
- [x] `examples/spec_core/chapter1/attributes/bnf-attr-cfg-let-gate-ok.reml`（期待: 成功）

**examples/spec_core/chapter1/module_use**
- [x] `examples/spec_core/chapter1/module_use/bnf-usedecl-super-root-invalid.reml`（期待: 失敗診断 → 2025-12-09 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/module_use/bnf-usedecl-super-root-invalid.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T093700Z.md で `language.use.invalid_super` を確認）
- [x] `examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T093700Z.md で診断 0 を確認）

**examples/spec_core/chapter1/lambda**
- [x] `examples/spec_core/chapter1/lambda/bnf-lambda-arg-pattern.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/lambda/bnf-lambda-arg-pattern.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T123501Z.md）
- [x] `examples/spec_core/chapter1/lambda/bnf-lambda-closure-capture-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/lambda/bnf-lambda-closure-capture-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T123501Z.md）

**examples/spec_core/chapter1/trait_impl**
- [x] `examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml`（期待: 失敗診断 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T130043Z.md で `typeclass.impl.duplicate` のみ出力されることを再確認）
- [x] `examples/spec_core/chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml` / 同ログで診断 0 件を確認）

**examples/spec_core/chapter1/type_decl**
- [x] `examples/spec_core/chapter1/type_decl/bnf-typedecl-alias-generic-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/type_decl/bnf-typedecl-alias-generic-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T145714Z.md で diagnostics=[] を確認）
- [x] `examples/spec_core/chapter1/type_decl/bnf-typedecl-new-struct-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/type_decl/bnf-typedecl-new-struct-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T145714Z.md で diagnostics=[] を確認）
- [x] `examples/spec_core/chapter1/type_decl/bnf-typedef-sum-recordpattern-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/type_decl/bnf-typedef-sum-recordpattern-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T145714Z.md で diagnostics=[] を確認）

**examples/spec_core/chapter1/let_binding**
- [x] `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-pattern-tuple.reml`（期待: 成功 → 2026-02-18 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/let_binding/bnf-valdecl-let-pattern-tuple.reml` / diagnostics=[]）
- [x] `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-shadow-unicode.reml`（期待: 失敗診断 → 2026-02-18 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/let_binding/bnf-valdecl-let-shadow-unicode.reml` / diagnostics=`language.shadowing.unicode`）
- [x] `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-simple-ok.reml`（期待: 成功 → 2026-02-18 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/let_binding/bnf-valdecl-let-simple-ok.reml` / diagnostics=[]）
- [x] `examples/spec_core/chapter1/let_binding/bnf-valdecl-missing-initializer-error.reml`（期待: 失敗診断 → 2026-02-18 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/let_binding/bnf-valdecl-missing-initializer-error.reml` / diagnostics=`parser.syntax.expected_tokens`）

**examples/spec_core/chapter1/control_flow**
- [x] `examples/spec_core/chapter1/control_flow/bnf-ifexpr-missing-else-type-mismatch.reml`（期待: 失敗診断 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-ifexpr-missing-else-type-mismatch.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T220319Z.md で `parser.syntax.expected_tokens` のみを出力）
- [x] `examples/spec_core/chapter1/control_flow/bnf-ifexpr-blocks-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-ifexpr-blocks-ok.reml` / 同上 log で diagnostics=[] を確認）
- [x] `examples/spec_core/chapter1/control_flow/bnf-loopexpr-break-value-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-loopexpr-break-value-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T220319Z.md。`var counter` へ型注釈を追加し、`else` で `continue` を返すようサンプルを是正して値制限/分岐型の衝突を回避）
- [x] `examples/spec_core/chapter1/control_flow/bnf-whileexpr-condition-type-error.reml`（期待: 失敗診断 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-whileexpr-condition-type-error.reml` / 同 log で `parser.lexer.unknown_token`×4 + `parser.syntax.expected_tokens` を確認）
- [x] `examples/spec_core/chapter1/control_flow/bnf-forexpr-iterator-pattern-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-forexpr-iterator-pattern-ok.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251209T220319Z.md。`var acc: Int` に注釈を付け Strict value restriction を回避）
- [x] `examples/spec_core/chapter1/control_flow/bnf-loopexpr-unreachable-code.reml`（期待: 失敗診断 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-loopexpr-unreachable-code.reml` / 同 log で `language.control_flow.unreachable` が 2 箇所報告されることを確認）
- [x] `examples/spec_core/chapter1/control_flow/bnf-whileexpr-condition-bool-ok.reml`（期待: 成功 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-whileexpr-condition-bool-ok.reml` / 同 log で diagnostics=[]。`var current: Int` へ明示型を追加）
- [x] `examples/spec_core/chapter1/control_flow/bnf-forexpr-iterator-invalid-type.reml`（期待: 失敗診断 → 2025-12-09 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/control_flow/bnf-forexpr-iterator-invalid-type.reml` / 同 log で `language.iterator.expected` を確認）

**examples/spec_core/chapter1/literals**
- [x] `examples/spec_core/chapter1/literals/bnf-literal-int-boundary-max.reml`（期待: 成功 → 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/literals/bnf-literal-int-boundary-max.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251210T065012Z.md / diagnostics=[] / stdout=expected/spec_core/chapter1/literals/bnf-literal-int-boundary-max.stdout）
- [x] `examples/spec_core/chapter1/literals/bnf-literal-float-forms.reml`（期待: 成功 → 2025-12-10 parser/mod.rs に FloatLiteral / Expr::float を復元し tests/spec_core::ch1_lit_202_parses_float_literal_forms を追加。CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/literals/bnf-literal-float-forms.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251210T065012Z.md / diagnostics=[] / stdout=expected/spec_core/chapter1/literals/bnf-literal-float-forms.stdout）
- [x] `examples/spec_core/chapter1/literals/bnf-literal-string-raw-multiline.reml`（期待: 成功 → 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/literals/bnf-literal-string-raw-multiline.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251210T065012Z.md / diagnostics=[] / stdout=expected/spec_core/chapter1/literals/bnf-literal-string-raw-multiline.stdout）

**examples/spec_core/chapter1/match_expr**
- [x] `examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml`（期待: 成功 → 2025-12-10 CLI= `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml` で診断 0 / log=reports/spec-audit/ch5/logs/spec_core-20251210T073321Z.md）
- [x] `examples/spec_core/chapter1/match_expr/bnf-matchexpr-option-canonical.reml`（期待: 成功 → 2025-12-10 CLI= `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/match_expr/bnf-matchexpr-option-canonical.reml` / diagnostics=[] / 同ログ参照）
- [x] `examples/spec_core/chapter1/match_expr/bnf-matchexpr-missing-arrow-error.reml`（期待: 失敗診断 → 2025-12-10 CLI= `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/match_expr/bnf-matchexpr-missing-arrow-error.reml` / diagnostics=`parser.syntax.expected_tokens` のみ / log=reports/spec-audit/ch5/logs/spec_core-20251210T073321Z.md / expected diagnostic JSON を新規作成）
- [x] `examples/spec_core/chapter1/match_expr/bnf-matchexpr-tuple-alternate.reml`（期待: 成功 → 2025-12-10 Parser の PatternKind::Literal を match arm から参照できるよう拡張し `tests/spec_core::ch1_match_002_accepts_tuple_literal_pattern` で回帰テスト化。CLI= `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/match_expr/bnf-matchexpr-tuple-alternate.reml` / log=reports/spec-audit/ch5/logs/spec_core-20251210T073321Z.md）
- [x] `examples/spec_core/chapter1/match_expr/bnf-matchexpr-alias-record-ok.reml`（期待: 成功 → 2025-12-12 CLI= `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/match_expr/bnf-matchexpr-alias-record-ok.reml` / diagnostics=[] / stdout=`expected/spec_core/chapter1/match_expr/bnf-matchexpr-alias-record-ok.stdout`）
- [x] `examples/spec_core/chapter1/match_expr/bnf-matchexpr-result-guard-else-ok.reml`（期待: 成功 → 2025-12-12 CLI= `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/match_expr/bnf-matchexpr-result-guard-else-ok.reml` / diagnostics=[] / stdout=`expected/spec_core/chapter1/match_expr/bnf-matchexpr-result-guard-else-ok.stdout`）

**examples/spec_core/chapter1/effects・conductor・block**
- [x] `examples/spec_core/chapter1/effects/bnf-attr-pure-perform-error.reml`（期待: 失敗診断 → CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/effects/bnf-attr-pure-perform-error.reml` / log=`reports/spec-audit/ch5/logs/spec_core-20251210T075036Z.md` / `effects.purity.violated`, `effects.contract.stage_mismatch` の 2 診断を再取得。`compiler/frontend/src/typeck/capability.rs` で `Console.*` を Capability Registry に再登録し Stage mismatch を復元。)
- [x] `examples/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml`（期待: 成功 → CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml` / log=`reports/spec-audit/ch5/logs/spec_core-20251210T075036Z.md` / diagnostics=[] / stdout=`expected/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.stdout` で確認。)
- [x] `examples/spec_core/chapter1/block/bnf-block-unclosed-brace-error.reml`（期待: 失敗診断 → CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter1/block/bnf-block-unclosed-brace-error.reml` / log=`reports/spec-audit/ch5/logs/spec_core-20251210T075036Z.md` / diagnostics=`parser.syntax.expected_tokens` / `expected/spec_core/chapter1/block/bnf-block-unclosed-brace-error.diagnostic.json` を新規作成。)

**examples/spec_core/chapter2**
- [x] `examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml`（期待: 成功 → 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml` で diagnostics=[] / stdout=`expected/spec_core/chapter2/parser_core/core-parse-or-commit-ok.stdout` / log=reports/spec-audit/ch5/logs/spec_core-20251210T081000Z.md）
- [x] `examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.reml`（期待: 失敗診断 → 2025-12-17 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.reml` で `parser.syntax.expected_tokens` を再取得 / expected=`expected/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead.diagnostic.json`。比較対象（Cut 無し相当）=`examples/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead-no-cut.reml` / expected=`expected/spec_core/chapter2/parser_core/core-parse-cut-branch-mislead-no-cut.diagnostic.json`）
- [x] `examples/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren.reml`（期待: 失敗診断 → 2025-12-17 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren.reml` で `parser.syntax.expected_tokens` を再取得 / expected=`expected/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren.diagnostic.json`。比較対象（Cut 無し相当）=`examples/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren-no-cut.reml` / expected=`expected/spec_core/chapter2/parser_core/core-parse-cut-unclosed-paren-no-cut.diagnostic.json`）
- [x] `examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml`（期待: 失敗診断 → 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml` で `core.parse.recover.branch` の単一診断を再取得 / log=reports/spec-audit/ch5/logs/spec_core-20251210T081000Z.md）
- [x] `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml`（期待: 失敗診断 → DSL 復元後の 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml` で `core.parse.opbuilder.level_conflict` を取得。最新の再実行: CLI=同左 / diagnostics=`core.parse.opbuilder.level_conflict` / log=reports/spec-audit/ch5/logs/spec_core-20251210T130034Z.md。仕様/実装整合は `docs/notes/dsl/opbuilder-dsl-decisions.md` に記録済み）
- [x] `examples/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.reml`（期待: 成功 → 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.reml` で diagnostics=[] / stdout=`expected/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.stdout` / 同ログ参照）
- [ ] `CH2-PARSE-901` autoWhitespace/Layout 回帰: `examples/spec_core/chapter2/parser_core/core-parse-autowhitespace-layout.reml` と stdout ゴールデンを追加（RunConfig.lex に layout_profile が無い場合も cfg.profile へフォールバックする構成）。PhaseF で CLI/LSP/Streaming を実行し、layout_token が期待どおり扱われるか確認する。
- [ ] `CH2-PARSE-902` ParserProfile JSON: `examples/spec_core/chapter2/parser_core/core-parse-profile-output.reml` を追加し、`extensions["parse"].profile_output` で `expected/spec_core/chapter2/parser_core/core-parse-or-commit.profile.json` を best-effort 出力する経路を用意。PhaseF で CLI 実行し、profile 集計・書き出し失敗非影響を確認する。

**examples/practical**
- [x] `examples/practical/core_path/security_check/relative_denied.reml`（期待: 診断あり → 2025-12-10 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_path/security_check/relative_denied.reml` / log=`reports/spec-audit/ch5/logs/practical-20251210T205757Z.md` / diagnostics=`core.path.security.invalid`（reason=relative_path_denied、run_id=59e7be86-650c-406e-b865-a9a0a625c767））
- [x] `examples/practical/core_config/audit_bridge/audit_bridge.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_config/audit_bridge/audit_bridge.reml` / log=`reports/spec-audit/ch5/logs/practical-20251211T013915Z.md` / diagnostics=[] / run_id=3febd846-c037-4a6b-ab44-6c98fdc5742e）
- [x] `examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml`（期待: 失敗診断 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_runtime/capability/stage_mismatch_runtime_bridge.reml` / log=`reports/spec-audit/ch5/logs/practical-20251211T014101Z.md` / diagnostics=`runtime.bridge.stage_mismatch` / run_id=d91aebaa-d239-4443-adcd-01249a5aa85a）
- [x] `examples/practical/core_io/file_copy/canonical.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_io/file_copy/canonical.reml` / diagnostics=[] / run_id=a1d9dcac-0505-4981-b5c8-5fe996ff28dd。Parser を更新しブロック内セミコロン任意化とレコードリテラルの `:` / `=` / フィールド省略（punning）を許容）
- [x] `examples/practical/core_text/unicode/grapheme_boundary_edge.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_text/unicode/grapheme_boundary_edge.reml` / diagnostics=`[]` / log=reports/spec-audit/ch5/logs/practical-20251211T082727Z.md / `Text.slice_graphemes` を使うため `core.text.unicode.segment_mismatch` 期待を外し成功ケースとして固定）
- [x] `examples/practical/core_text/unicode/grapheme_nfc_mix.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_text/unicode/grapheme_nfc_mix.reml` / diagnostics=[] / stdout golden=`expected/practical/core_text/unicode/grapheme_nfc_mix.stdout`（graphemes=2、runtime_phase=none） / log=reports/spec-audit/ch5/logs/practical-20251211T083527Z.md / run_id=250a3b7c-b790-422f-9b30-e654d2343265）
- [x] `examples/practical/core_diagnostics/audit_envelope/stage_tag_capture.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_diagnostics/audit_envelope/stage_tag_capture.reml` / diagnostics=[] / run_id=ae1f942b-b243-4653-936a-1cd1bb803300 / log=`reports/spec-audit/ch5/logs/practical-20251211T085850Z.md` / audit_log=`reports/spec-audit/ch5/logs/practical-20251211T085850Z.audit.jsonl` は pipeline_started/completed に `scenario.id` と `effect.stage.required/actual` を付与。`expected/practical/core_diagnostics/audit_envelope/stage_tag_capture.audit.jsonl` の schema 整備は継続検討）
- [x] `examples/practical/core_env/envcfg/env_merge_by_profile.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_env/envcfg/env_merge_by_profile.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/practical-20251211T091650Z.md / run_id=2f9ecb5d-3d75-4ba4-92f2-7233b6b00b5b / expected stdout=`expected/practical/core_env/envcfg/env_merge_by_profile.stdout`（`https://cli.local`）を前提に成功扱い）
- [x] `examples/practical/core_async/basic_sleep.reml`（期待: 成功 → 2025-12-21 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_async/basic_sleep.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/practical-20251221T004726Z.md / run_id=d232170c-b798-4713-b1a8-ab4d7fea04d3 / expected stdout=`expected/practical/core_async/basic_sleep.stdout`）
- [x] `examples/practical/core_async/timeout_basic.reml`（期待: 成功 → 2025-12-21 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/practical/core_async/timeout_basic.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/practical-20251221T005126Z.md / run_id=60b82bd9-f460-4ea1-9695-50a64a4f1608 / expected stdout=`expected/practical/core_async/timeout_basic.stdout`）

**examples/dsl_paradigm**
- [x] `examples/dsl_paradigm/mini_ruby/mini_ruby_basic.reml`（期待: 成功 → 2025-12-22 CLI=`compiler/frontend/target/debug/reml_frontend --output json examples/dsl_paradigm/mini_ruby/mini_ruby_basic.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/dsl-paradigm-mini_ruby-20251222T010238Z.md / run_id=1ac49955-f6c4-4699-b53f-6775b60771f2）
- [x] `examples/dsl_paradigm/mini_erlang/mini_erlang_basic.reml`（期待: 成功 → 2025-12-22 CLI=`compiler/frontend/target/debug/reml_frontend --output json examples/dsl_paradigm/mini_erlang/mini_erlang_basic.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/dsl-paradigm-mini_erlang-20251222T010311Z.md / run_id=49445f7d-0136-4076-b1aa-febab11e9243）
- [x] `examples/dsl_paradigm/mini_vm/mini_vm_basic.reml`（期待: 成功 → 2025-12-22 CLI=`compiler/frontend/target/debug/reml_frontend --output json examples/dsl_paradigm/mini_vm/mini_vm_basic.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/dsl-paradigm-mini_vm-20251222T010257Z.md / run_id=e9e5b2f7-0255-4c3e-aec9-546ae94d597a）

#### 監査ログ検証メモ（PhaseF 補足）

- 監査出力を確認したい場合は `--emit-audit` を付けて CLI を再実行する。例:  
  `cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --emit-audit --output json examples/practical/core_diagnostics/audit_envelope/stage_tag_capture.reml 2> reports/spec-audit/ch5/logs/practical-<ts>.audit.jsonl`
- `AuditEnvelope.metadata` に最低限含まれるべきキー  
  - `scenario.id`（`pipeline.dsl_id` を反映）  
  - `effect.capability` / `effect.stage.required` / `effect.stage.actual`（StageAuditPayload から転写。primary_capability が無い場合でも `core.diagnostics` で補完）  
  - `pipeline.*` / `cli.*` / `audit.*`（既定のパイプライン識別子一式）
- expected ファイルとの比較手順  
  1. `expected/practical/core_diagnostics/audit_envelope/stage_tag_capture.audit.jsonl` を開き、上記キーが揃っているか確認する。  
  2. `reports/spec-audit/ch5/logs/practical-<ts>.audit.jsonl` の `pipeline_started` / `pipeline_completed` を diff し、`scenario.id` と `effect.stage.*` が一致していれば pass。  
  3. キー欠落や Stage 値の不一致を見つけた場合は、`StageAuditPayload` / `pipeline::base_metadata` の補完ロジックを優先的に調査し、マトリクス `resolution_notes` に実行コマンドとログパスを記録する。

**examples/core_path / core_config / cli / core_io / core-collections / string_literal**
- [x] `examples/core_path/security_check.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_path/security_check.reml` / run_id=`a55948c8-3a09-4a0e-8e3d-ab91f0f9eb51` / log=reports/spec-audit/ch5/logs/core_path-20251211T092454Z.md。`struct` トップレベルを `type ... = new {...}` へ是正し、`map_err(...)?` で symlink チェックの回収経路を統一）
- [x] `examples/core_config/cli/dsl/sample.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_config/cli/dsl/sample.reml` / diagnostics=[] / run_id=`d8ffcb77-98f3-4b89-b10a-7c4fad72727d` / log=reports/spec-audit/ch5/logs/core_config-20251211T093350Z.md。Effect 宣言に `operation` を追加し、`ensure` の遅延診断クロージャを `| |` 形式へ修正して BNF と整合）
- [x] `examples/core_config/dsl/telemetry_bridge.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_config/dsl/telemetry_bridge.reml` / diagnostics=[] / run_id=`7f1bb8a3-f84a-43dc-99a7-97ca218ecf90` / log=reports/spec-audit/ch5/logs/core_config-20251211T093648Z.md。Telemetry DSL プレースホルダが Parser/Typeck を通過することを確認）
- [x] `examples/core_config/dsl/audit_bridge.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_config/dsl/audit_bridge.reml` / diagnostics=[] / run_id=`52ff17ca-1aae-45ca-a3b7-1dc5d16d230c` / log=reports/spec-audit/ch5/logs/core_config-20251211T094146Z.md。`reml.toml` の DSL entry と Stage/Capability/Effect 宣言が一致することを確認）
- [x] `examples/cli/type_error.reml`（期待: 失敗診断 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/cli/type_error.reml` / diagnostics=`[E7006]` / run_id=`94ac002b-45a7-41b9-b0d4-28b34e5ecd1b` / log=reports/spec-audit/ch5/logs/cli-20251211T094708Z.md。Bool 以外の条件を検出するサンプル）
- [x] `examples/cli/emit_suite.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/cli/emit_suite.reml` / diagnostics=[] / run_id=`c063a9f9-37d0-4db2-8ed2-7f2eafd2d536` / log=reports/spec-audit/ch5/logs/cli-20251211T094708Z.md。`use Core.Prelude` 追加と `flag == true` 明示で Bool 判定を通過）
- [x] `examples/cli/trace_sample.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/cli/trace_sample.reml` / diagnostics=[] / run_id=`416b5f64-6a4f-4bd5-b1e7-83c57cc40f51` / log=reports/spec-audit/ch5/logs/cli-20251211T094708Z.md。関数本体をブロック式に修正し `let` 連鎖を通過）
- [x] `examples/cli/add.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/cli/add.reml` / diagnostics=[] / run_id=`dbf889a6-4841-4b58-a1b9-9f453143bd3a` / log=reports/spec-audit/ch5/logs/cli-20251211T094708Z.md。`main` をブロック式に揃えて Parser/Typeck 通過を確認）
- [x] `examples/core-collections/usage.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core-collections/usage.reml` / diagnostics=[] / run_id=`341f9325-3490-43d5-bf70-40cf9303dad1` / log=reports/spec-audit/ch5/logs/core-examples-20251211T145428Z.md。`Map` 連鎖への置換とブロック式統一で Parser/Typeck を通過）
- [x] `examples/string_literal.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/string_literal.reml` / diagnostics=[] / run_id=`60259b9b-98e0-4f5f-a687-3bb336c16481` / log=reports/spec-audit/ch5/logs/core-examples-20251211T145428Z.md。仕様どおりの単純文字列リテラルで回帰なしを確認）
- [x] `examples/core_io/file_copy.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/core_io/file_copy.reml` / diagnostics=[] / run_id=`a1e8bcf6-2f9b-48a5-93db-0942866f339f` / log=reports/spec-audit/ch5/logs/core-examples-20251211T145428Z.md。`fn copy_file` をブロック式に変更し `CopyReport` フィールドを `=` 代入に統一して Parser エラーを解消）

**examples/ffi**
- [x] `examples/ffi/macos/ffi_dispatch_async.reml`（期待: 成功 → 2025-12-12 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/ffi/macos/ffi_dispatch_async.reml` / diagnostics=[] / run_id=41f3ed0b-d401-4ca9-9a6b-4bb1786d48d1）
- [x] `examples/ffi/macos/ffi_malloc_arm64.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/ffi/macos/ffi_malloc_arm64.reml` / diagnostics=[] / log=reports/spec-audit/ch5/logs/ffi-20251211T151355Z.md / 関数本体をブロック式へ変更し、数値リテラルの型サフィックスを外して Parser エラーを解消）
- [x] `examples/ffi/macos/ffi_getpid.reml`（期待: 成功 → 2025-12-13 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/ffi/macos/ffi_getpid.reml` / diagnostics=[] / run_id=`b69f825a-1634-4cdf-b292-1e31fe117492`）
- [ ] `examples/ffi/windows/ownership_transfer.reml`（期待: 成功）
- [ ] `examples/ffi/windows/struct_passing.reml`（期待: 成功）
- [ ] `examples/ffi/windows/messagebox.reml`（期待: 成功）

**examples/language-impl-samples/reml**
- [x] `examples/language-impl-samples/reml/pipeline_operator_demo.reml`（期待: 成功 → 2025-12-12 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output human examples/language-impl-samples/reml/pipeline_operator_demo.reml` / diagnostics=[]。Parser に `|>` パイプ演算子の左結合最弱優先度を実装し、パイプ演算サンプルが通過することを確認）
- [x] `examples/language-impl-samples/reml/audit_pipeline_integration.reml`（期待: 成功 → 2025-12-12 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output human examples/language-impl-samples/reml/audit_pipeline_integration.reml` / diagnostics=[]。record literal を `TypeName({ ... })` 形式へ統一し、`if` 分岐の戻り値型衝突を `()` 明示で解消）
- [x] `examples/language-impl-samples/reml/basic_interpreter.reml`（期待: 成功 → CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output human examples/language-impl-samples/reml/basic_interpreter.reml` / diagnostics=[]）
- [ ] `examples/language-impl-samples/reml/conductor_data_pipeline.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/pl0.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/json_extended.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/async_actor_supervision.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/markdown_parser.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/sql_parser.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/prelude_guard_template.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/external_dsl_bridge.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/pratt_parser.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/toml_parser.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/yaml_parser.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/json_parser_combinator.reml`（期待: 成功）
- [x] `examples/language-impl-samples/reml/basic_interpreter_combinator.reml`（期待: 成功 → 2025-12-13 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output human examples/language-impl-samples/reml/basic_interpreter_combinator.reml` / diagnostics=[] / stdout=`expected/language-impl-samples/basic_interpreter_combinator.stdout` を取得済み。`execute_while_loop` の条件式に Bool 変数 `cond_truth` を挟み、型推論が Unknown で落ちる回帰を解消。Rust Core.Parse コンビネーター経路の基準サンプルとして PhaseF でも完了扱い）
- [ ] `examples/language-impl-samples/reml/config_manifest_lifecycle.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/template_engine.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/algebraic_effects.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/json_parser.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/stream_processing_dsl.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/hindley_milner.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/markdown_parser_combinator.reml`（期待: 成功）
- [ ] `examples/language-impl-samples/reml/pl0_combinator.reml`（期待: 成功）
- [x] `examples/language-impl-samples/reml/mini_lisp.reml`（期待: 成功 → 2025-12-15 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output human examples/language-impl-samples/reml/mini_lisp.reml` で診断 0 を確認）
- [ ] `examples/language-impl-samples/reml/regex_engine.reml`（期待: 成功）
- [x] `examples/language-impl-samples/reml/mini_lisp_combinator.reml`（期待: 成功 → 2025-12-15 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output human examples/language-impl-samples/reml/mini_lisp_combinator.reml` で診断 0 を確認。`if`/`match` を最新構文へ揃え、`NativeFn` / `VLambda` を `mini_lisp.reml` と同スタイルで整理）

**examples/core-text / core_diagnostics**
- [x] `examples/core-text/text_unicode.reml`（期待: 成功 → 2025-12-11 CLI=`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- examples/core-text/text_unicode.reml` / diagnostics=[] / run_id=`a9eb0839-1c6c-44c4-b9fe-3d448200af09` / log=reports/spec-audit/ch5/logs/core-text-20251211T222317Z.md。DocComment 正規化と Emoji トークンを簡素化し、パイプラインをブロック式に統一）
- [x] `examples/core_diagnostics/pipeline_branch.reml`（期待: `effects.contract.stage_mismatch` → CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json --emit-audit-log examples/core_diagnostics/pipeline_branch.reml` / diagnostics=`effects.contract.stage_mismatch` のみ / expected=`examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` を更新し Stage mismatch ゴールデンを再取得）
- [x] `examples/core_diagnostics/pipeline_success.reml`（期待: 成功 → 2025-12-12 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json --emit-audit-log examples/core_diagnostics/pipeline_success.reml` / diagnostics=[] / run_id=`ee557b05-0d8b-4548-8a24-6708047792e7` / expected=`examples/core_diagnostics/pipeline_success.expected.{diagnostic.json,audit.jsonl}` を更新）
- [x] `examples/core_diagnostics/constraint_graph/simple_chain.reml`（期待: 成功 → 2025-12-12 CLI=`cargo run --quiet --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --emit-telemetry constraint_graph=examples/core_diagnostics/output/simple_chain-constraint_graph.json examples/core_diagnostics/constraint_graph/simple_chain.reml` / diagnostics=[] / run_id=`4eab2043-df43-4993-af6d-18377aee56b0` / telemetry=`examples/core_diagnostics/output/simple_chain-constraint_graph.json` を再生成し DOT/SVG (`examples/core_diagnostics/output/simple_chain.{dot,svg}`) も更新）

> リストに含まれない新規 `.reml` を追加した場合は、本計画と `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の両方へ同時追加し、週次レビューで差異が無いことを確認する。

## 成果物と KPI

- `parser.syntax.expected_tokens` / `typeck.aborted.ast_unavailable` が Phase 4 の spec_core/practical スイートで発生しないこと（期待診断があるケースを除く）。  
- `reports/spec-audit/ch5/spec-core-dashboard.md` における **Pass 率 70% 以上**、Phase 4 M1 exit 条件の 85% へ段階的に到達。  
- `cargo test -p reml_e2e --test scenario -- --scenario spec-core` / `--scenario practical` を追加し、CI で `spec.chapter1.pass_rate`, `spec.chapter3.pass_rate` KPI を更新。  
- 主要な spec_fix/impl_fix の判断を `phase4-scenario-matrix.csv` の `resolution_notes` に残し、Phase 5 以降のハンドオーバー資料として利用可能にする。

## 依存関係とフォローアップ

- Parser/Typeck 修正は Phase 3 の `docs/spec/1-x` / `docs/spec/3-x` 更新と連動するため、仕様差分を検出した場合は `2-5-spec-drift-remediation.md` の手順に沿って仕様側へ反映。  
- Core.IO / Capability の挙動差分は `3-5-core-io-path-plan.md` や `3-8-core-runtime-capability-plan.md` の残課題と共有し、必要なら Phase 3 計画へ逆流させる。  
- Self-host フェーズ（Phase 5）へ進む前に本計画の KPI を満たし、`reports/spec-audit/ch5` を Stage 0/1/2 のリグレッションベースとして採用する。
