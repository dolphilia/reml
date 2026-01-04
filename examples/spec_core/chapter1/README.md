# Chapter 1 spec_core ケース

- `let_binding/`: `ValDecl` の BNF を対象に「正例」「境界例」「Unicode シャドーイングによるエラー」を収録。
- `match_expr/`: `MatchExpr` と `TuplePattern` の BNF を対象に canonical/alternate/guard パターンを用意。
- `effect_handlers/`: `HandleExpr` と `HandlerLiteral` の BNF を対象に `handle ... with` の必須キーワードと `perform` バリエーションを検証。
- `module_use/`: `CompilationUnit`/`UseDecl` の入れ子書式と `super` 禁止ケースを BNF どおりに再現。
- `attributes/`: `@cfg` 属性の成功例と `language.cfg.unsatisfied_branch` を返す境界例を収録。
- `fn_decl/`, `type_decl/`, `trait_impl/`: `FnDecl`/`TypeDecl`/`ImplDecl` のジェネリクス・効果注釈・重複 `impl` 診断を Chapter 1 §B.4/§B.5 と同期。
- `type_inference/`: `let` 一般化 (`CH1-INF-601`) と値制限違反 (`CH1-INF-602`) を通じて `docs/spec/1-2-types-Inference.md` の推論ルールを確認。
- `effects/`: `@pure` と `perform` の衝突を `effects.purity.violated` 診断で固定。
- `conductor/`: `B.8` の DSL 制御ブロックを最小構成で実行し、監査タグ付けを確認。

各ケースは `docs/spec/1-1-syntax.md` および `docs/spec/1-3-effects-safety.md` の該当セクションと相互参照できるよう、ファイル冒頭に BNF 規則名のコメントを付与しています。
