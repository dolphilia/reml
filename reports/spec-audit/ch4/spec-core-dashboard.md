# spec_core スイート実行レポート

- 実行時刻: 2025-12-08 20:00:34Z
- 対象シナリオ: 44 件 / 成功 0 件 / 失敗 44 件
- 入力ソース: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

| Scenario | File | 期待 Diagnostics | 実際 Diagnostics | Exit | 判定 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| `CH1-LET-001` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-simple-ok.reml` | — | — | 101 | ❌ fail | 基本的な let 束縛。Result/Option を含まない純粋ケースで、Phase4TestCase における基準入力。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LET-002` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-pattern-tuple.reml` | — | — | 101 | ❌ fail | タプルパターン束縛。仕様上は許容だが OCaml 実装では過去に制約があったため検証対象。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LET-003` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-let-shadow-unicode.reml` | `language.shadowing.unicode` | — | 101 | ❌ fail | Unicode 識別子とシャドーイング境界。`αβ` のような識別子を1回だけ再束縛する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-MATCH-001` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-option-canonical.reml` | — | — | 101 | ❌ fail | `Option` に対する `match ... with` の正準例。`Some`/`None` 分岐で BNF の最小形を固定。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-MATCH-002` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-tuple-alternate.reml` | — | — | 101 | ❌ fail | TuplePattern と `_` を組み合わせた `match` バリエーション。分岐順序とフォールバック処理を検証。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-EFFECT-004` | `examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-missing-with.reml` | `effects.handler.missing_with` | — | 101 | ❌ fail | `with` 節を省略する意地悪ケース。診断キーとメッセージを固定化。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-EFF-006` | `examples/spec_core/chapter1/effect_handlers/bnf-handleexpr-perform-counter.reml` | — | — | 101 | ❌ fail | `perform` と `handle ... with handler` を同時に提示する効果ハンドラの派生記法。`resume` を介した戻り値の合成を確認。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-MOD-003` | `examples/spec_core/chapter1/module_use/bnf-compilationunit-module-use-alias-ok.reml` | — | — | 101 | ❌ fail | `module spec_core.match_guard` と再帰 `use` を同一 `.reml` で実行し Prelude 名寄せのゴールデンを作成する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-MOD-004` | `examples/spec_core/chapter1/module_use/bnf-usedecl-super-root-invalid.reml` | `language.use.invalid_super` | — | 101 | ❌ fail | ルート直下で `super` を参照した際の拒否診断を明示し Chapter1 §B.1 の禁止事項をテスト化する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-ATTR-101` | `examples/spec_core/chapter1/attributes/bnf-attr-cfg-let-gate-ok.reml` | — | — | 101 | ❌ fail | `@cfg(target = \"cli\")` で `let` ブロックが有効化される既定経路を `RunConfig` と連携して検証する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-ATTR-102` | `examples/spec_core/chapter1/attributes/bnf-attr-cfg-missing-flag-error.reml` | `language.cfg.unsatisfied_branch` | — | 101 | ❌ fail | 未定義ターゲットを指定した `@cfg` が `language.cfg.unsatisfied_branch` を返すことを Chapter1 §B.6 準拠で確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-FN-101` | `examples/spec_core/chapter1/fn_decl/bnf-fndecl-generic-default-effect-ok.reml` | — | — | 101 | ❌ fail | ジェネリック/デフォルト引数/効果注釈を組み合わせた `fn` 宣言を Chapter1 §B.4 の要件通り通過させる。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-TYPE-201` | `examples/spec_core/chapter1/type_decl/bnf-typedef-sum-recordpattern-ok.reml` | — | — | 101 | ❌ fail | SumType と Record パターンの束縛を同一 `.reml` にまとめ BNF 通り受理されることを確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-TRAIT-301` | `examples/spec_core/chapter1/trait_impl/bnf-traitdecl-default-where-ok.reml` | — | — | 101 | ❌ fail | `trait Show<T> where T: Copy` とデフォルトメソッドの辞書生成ログを Chapter1 §B.1 の通り固定する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-IMPL-302` | `examples/spec_core/chapter1/trait_impl/bnf-impldecl-duplicate-error.reml` | `typeclass.impl.duplicate` | — | 101 | ❌ fail | 同一型への重複 `impl` を禁止する診断を B.2 の整合性規則に沿ってゴールデン化する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-INF-601` | `examples/spec_core/chapter1/type_inference/bnf-inference-let-generalization-ok.reml` | — | — | 101 | ❌ fail | `let id = fn x => x` が多相化され `Vec<i64>` と `Vec<Text>` で共有できることを Chapter1 §H.1 に基づき確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-INF-602` | `examples/spec_core/chapter1/type_inference/bnf-inference-value-restriction-error.reml` | `language.inference.value_restriction` | — | 101 | ❌ fail | `var cell = []` の一般化拒否を `language.inference.value_restriction` 診断で再現し C.3 の値制限をテストする。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-EFF-701` | `examples/spec_core/chapter1/effects/bnf-attr-pure-perform-error.reml` | `effects.purity.violated` | — | 101 | ❌ fail | `@pure fn` 内で `perform Console.log` を呼び出した際の純粋性違反を Chapter1 §B と Stage 要件に沿って診断する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-DSL-801` | `examples/spec_core/chapter1/conductor/bnf-conductor-basic-pipeline-ok.reml` | — | — | 101 | ❌ fail | `conductor telemetry { channels { ... } execution { ... } }` の DSL 制御ブロックを Chapter1 §B.8 に沿って再現し監査タグ連携を確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-MATCH-003` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-when-guard-ok.reml` | — | — | 101 | ❌ fail | `Some(x) when x > 10 as large` のような `when`/`as` パターンを Chapter1 §C.3 通り受理することを確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH2-PARSE-101` | `examples/spec_core/chapter2/parser_core/core-parse-or-commit-ok.reml` | — | — | 101 | ❌ fail | `Core.Parse.or` + `commit` の組合せで Backtracking コストを制御する基準ケースを作成する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH2-PARSE-201` | `examples/spec_core/chapter2/parser_core/core-parse-recover-diagnostic.reml` | `core.parse.recover.branch` | — | 101 | ❌ fail | `Parse.recover` が `core.parse.recover.branch` 診断を生成し Diagnostics chapter と同期することを確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH2-STREAM-301` | `examples/spec_core/chapter2/streaming/core-parse-runstream-demandhint-ok.reml` | — | — | 101 | ❌ fail | `run_stream` と `DemandHint::More` の協調を示す Streaming API 基準ケースを chapter2 §C-1 に基づき追加する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH2-OP-401` | `examples/spec_core/chapter2/op_builder/core-opbuilder-level-conflict-error.reml` | `core.parse.opbuilder.level_conflict` | — | 101 | ❌ fail | 同一レベルへ異なる fixity を登録した際の `core.parse.opbuilder.level_conflict` 診断をゴールデン化し優先度規則を確認する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LET-004` | `examples/spec_core/chapter1/let_binding/bnf-valdecl-missing-initializer-error.reml` | `parser.syntax.expected_tokens` | — | 101 | ❌ fail | 初期化式を欠落させ parser.syntax.expected_tokens を誘発する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-MATCH-004` | `examples/spec_core/chapter1/match_expr/bnf-matchexpr-missing-arrow-error.reml` | `parser.syntax.expected_tokens` | — | 101 | ❌ fail | `->` 欠落により構文エラーを明示する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-BLOCK-001` | `examples/spec_core/chapter1/block/bnf-block-unclosed-brace-error.reml` | `parser.syntax.expected_tokens` | — | 101 | ❌ fail | `}` が無いブロック終端エラーを再現する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-IF-101` | `examples/spec_core/chapter1/control_flow/bnf-ifexpr-blocks-ok.reml` | — | — | 101 | ❌ fail | if-then-else でブロック値が返る基本例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-IF-102` | `examples/spec_core/chapter1/control_flow/bnf-ifexpr-missing-else-type-mismatch.reml` | — | — | 101 | ❌ fail | else 省略で非 Unit を返そうとした型不一致の境界例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LOOP-101` | `examples/spec_core/chapter1/control_flow/bnf-loopexpr-break-value-ok.reml` | — | — | 101 | ❌ fail | loop から break 値を返す最小例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LOOP-102` | `examples/spec_core/chapter1/control_flow/bnf-loopexpr-unreachable-code.reml` | — | — | 101 | ❌ fail | break 後に到達不能な式が残る診断例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-WHILE-101` | `examples/spec_core/chapter1/control_flow/bnf-whileexpr-condition-bool-ok.reml` | — | — | 101 | ❌ fail | Bool 条件で while を評価するケース。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-WHILE-102` | `examples/spec_core/chapter1/control_flow/bnf-whileexpr-condition-type-error.reml` | — | — | 101 | ❌ fail | Bool 以外を条件に使った拒否診断。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-FOR-101` | `examples/spec_core/chapter1/control_flow/bnf-forexpr-iterator-pattern-ok.reml` | — | — | 101 | ❌ fail | Pattern でイテレータ要素を分解する for 式。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-FOR-102` | `examples/spec_core/chapter1/control_flow/bnf-forexpr-iterator-invalid-type.reml` | — | — | 101 | ❌ fail | 非イテレータ値を `for` に渡す診断例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LIT-201` | `examples/spec_core/chapter1/literals/bnf-literal-int-boundary-max.reml` | — | — | 101 | ❌ fail | i64 最大値を下線区切りで表す境界テスト。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LIT-202` | `examples/spec_core/chapter1/literals/bnf-literal-float-forms.reml` | — | — | 101 | ❌ fail | 指数表記や大文字 E を含む浮動小数リテラル例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LIT-203` | `examples/spec_core/chapter1/literals/bnf-literal-string-raw-multiline.reml` | — | — | 101 | ❌ fail | raw 文字列と複数行文字列を並記する。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-TYPE-202` | `examples/spec_core/chapter1/type_decl/bnf-typedecl-alias-generic-ok.reml` | — | — | 101 | ❌ fail | ジェネリック alias を入れ子で定義する例。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-TYPE-203` | `examples/spec_core/chapter1/type_decl/bnf-typedecl-new-struct-ok.reml` | — | — | 101 | ❌ fail | `type Name = new { ... }` の構造体ラッパを示す。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-FN-102` | `examples/spec_core/chapter1/fn_decl/bnf-fndecl-no-args-ok.reml` | — | — | 101 | ❌ fail | 引数無し関数の宣言と呼び出しを示す。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-FN-103` | `examples/spec_core/chapter1/fn_decl/bnf-fndecl-return-inference-error.reml` | — | — | 101 | ❌ fail | 戻り値注釈無しで分岐戻り型が衝突する推論エラー。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LAMBDA-101` | `examples/spec_core/chapter1/lambda/bnf-lambda-closure-capture-ok.reml` | — | — | 101 | ❌ fail | 外側変数を捕捉するラムダを示す。 (error: CLI から JSON 出力が得られませんでした) |
| `CH1-LAMBDA-102` | `examples/spec_core/chapter1/lambda/bnf-lambda-arg-pattern.reml` | — | — | 101 | ❌ fail | パターン引数を用いたラムダの短縮形。 (error: CLI から JSON 出力が得られませんでした) |
