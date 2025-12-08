# spec_core スイート実行レポート

- 実行時刻: 2025-12-07 10:46:58Z
- 対象シナリオ: 24 件 / 成功 0 件 / 失敗 24 件
- 入力ソース: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

## FFI/Core Prelude KPI（Phase4 追補）

- 監視対象: `FFI-CORE-PRELUDE-001`（`category=FFI`, `spec_chapter=chapter3.prelude`）。Rust Frontend の `core_iter_effects.rs`／`core_iter_collectors.rs`／`core_iter_pipeline.rs` を dev-dep `reml_runtime_ffi` + `core_prelude` で再生し、`docs/spec/3-1-core-prelude-iteration.md` と `docs/spec/3-6-core-diagnostics-audit.md` の Stage/Capability 契約を満たすかを確認する。
- コマンド（2026-02-17 実行）:
  - `cd compiler/rust/runtime/ffi && cargo check --features core_prelude`
  - `cd compiler/rust/frontend && cargo test core_iter_effects`
- 成果物: `compiler/rust/frontend/tests/snapshots/core_iter_effects__core_iter_effect_labels.snap` をアルファベット順キーへ更新し、`cargo test core_iter_effects` が snapshot 差分ゼロで完走。`phase4-scenario-matrix.csv` の `FFI-CORE-PRELUDE-001` `resolution_notes` と同じログを本レポートに残し、Run 毎に StageMismatch が再発しないことを確認できるようにした。
- 判定: ✅ pass（`StageRequirement::AtLeast(Beta)` / `capability = core.iter.core` を再確認）。次回以降も spec_core/practical スイートに追加する再生タスクは本 GitHub Actions nightly から参照する。

| Scenario | File | 期待 Diagnostics | 実際 Diagnostics | Exit | 判定 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| `CH1-LET-001` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-simple-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | 基本的な let 束縛。Result/Option を含まない純粋ケースで、Phase4TestCase における基準入力。 |
| `CH1-LET-002` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-pattern-tuple.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | タプルパターン束縛。仕様上は許容だが OCaml 実装では過去に制約があったため検証対象。 |
| `CH1-LET-003` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-shadow-unicode.reml` | `language.shadowing.unicode` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | Unicode 識別子とシャドーイング境界。`αβ` のような識別子を1回だけ再束縛する。 |
| `CH1-MATCH-001` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-option-canonical.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `Option` に対する `match ... with` の正準例。`Some`/`None` 分岐で BNF の最小形を固定。 |
| `CH1-MATCH-002` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-tuple-alternate.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | TuplePattern と `_` を組み合わせた `match` バリエーション。分岐順序とフォールバック処理を検証。 |
| `CH1-EFFECT-004` | `examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-missing-with.reml` | `effects.handler.missing_with` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `with` 節を省略する意地悪ケース。診断キーとメッセージを固定化。 |
| `CH1-EFF-006` | `examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-perform-counter.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `perform` と `handle ... with handler` を同時に提示する効果ハンドラの派生記法。`resume` を介した戻り値の合成を確認。 |
| `CH1-MOD-003` | `examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `module spec_core.match_guard` と再帰 `use` を同一 `.reml` で実行し Prelude 名寄せのゴールデンを作成する。 |
| `CH1-MOD-004` | `examples/spec_core/chapter1/module_use/bnf-usedecl-super-root-invalid.reml` | `language.use.invalid_super` | — | 0 | ❌ fail | ルート直下で `super` を参照した際の拒否診断を明示し Chapter1 §B.1 の禁止事項をテスト化する。 |
| `CH1-ATTR-101` | `examples/spec_core/chapter1/attributes/bnf-attr-cfg-let-gate-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `@cfg(target = \"cli\")` で `let` ブロックが有効化される既定経路を `RunConfig` と連携して検証する。 |
| `CH1-ATTR-102` | `examples/spec_core/chapter1/attributes/bnf-attr-cfg-missing-flag-error.reml` | `language.cfg.unsatisfied_branch` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | 未定義ターゲットを指定した `@cfg` が `language.cfg.unsatisfied_branch` を返すことを Chapter1 §B.6 準拠で確認する。 |
| `CH1-FN-101` | `examples/spec_core/chapter1/fn_decl/bnf-fndecl-generic-default-effect-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | ジェネリック/デフォルト引数/効果注釈を組み合わせた `fn` 宣言を Chapter1 §B.4 の要件通り通過させる。 |
| `CH1-TYPE-201` | `examples/spec_core/chapter1/type_decl/bnf-typedef-sum-recordpattern-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | SumType と Record パターンの束縛を同一 `.reml` にまとめ BNF 通り受理されることを確認する。 |
| `CH1-TRAIT-301` | `examples/spec_core/chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `trait Show<T> where T: Copy` とデフォルトメソッドの辞書生成ログを Chapter1 §B.1 の通り固定する。 |
| `CH1-IMPL-302` | `examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml` | `typeclass.impl.duplicate` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | 同一型への重複 `impl` を禁止する診断を B.2 の整合性規則に沿ってゴールデン化する。 |
| `CH1-INF-601` | `examples/spec_core/chapter1/type_inference/bnf-inference-let-generalization-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `let id = fn x => x` が多相化され `Vec<i64>` と `Vec<Text>` で共有できることを Chapter1 §H.1 に基づき確認する。 |
| `CH1-INF-602` | `examples/spec_core/chapter1/type_inference/bnf-inference-value-restriction-error.reml` | `language.inference.value_restriction` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `var cell = []` の一般化拒否を `language.inference.value_restriction` 診断で再現し C.3 の値制限をテストする。 |
| `CH1-EFF-701` | `examples/spec_core/chapter1/effects/bnf-attr-pure-perform-error.reml` | `effects.purity.violated` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `@pure fn` 内で `perform Console.log` を呼び出した際の純粋性違反を Chapter1 §B と Stage 要件に沿って診断する。 |
| `CH1-DSL-801` | `examples/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `conductor telemetry { channels { ... } execution { ... } }` の DSL 制御ブロックを Chapter1 §B.8 に沿って再現し監査タグ連携を確認する。 |
| `CH1-MATCH-003` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `Some(x) when x > 10 as large` のような `when`/`as` パターンを Chapter1 §C.3 通り受理することを確認する。 |
| `CH2-PARSE-101` | `examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `Core.Parse.or` + `commit` の組合せで Backtracking コストを制御する基準ケースを作成する。 |
| `CH2-PARSE-201` | `examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml` | `core.parse.recover.branch` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `Parse.recover` が `core.parse.recover.branch` 診断を生成し Diagnostics chapter と同期することを確認する。 |
| `CH2-STREAM-301` | `examples/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.reml` | — | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | `run_stream` と `DemandHint::More` の協調を示す Streaming API 基準ケースを chapter2 §C-1 に基づき追加する。 |
| `CH2-OP-401` | `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml` | `core.parse.opbuilder.level_conflict` | `parser.syntax.expected_tokens`<br>`typeck.aborted.ast_unavailable` | 1 | ❌ fail | 同一レベルへ異なる fixity を登録した際の `core.parse.opbuilder.level_conflict` 診断をゴールデン化し優先度規則を確認する。 |
