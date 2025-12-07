# Chapter 1 spec_core ケース

- `let_binding/`: `ValDecl` の BNF を対象に「正例」「境界例」「Unicode シャドーイングによるエラー」を収録。
- `match_expr/`: `MatchExpr` と `TuplePattern` の BNF を対象に canonical/alternate の 2 パターンを用意。
- `effect_handlers/`: `HandleExpr` と `HandlerLiteral` の BNF を対象に `handle ... with` の必須キーワードと `perform` バリエーションを検証。

各ケースは `docs/spec/1-1-syntax.md` および `docs/spec/1-3-effects-safety.md` の該当セクションと相互参照できるよう、ファイル冒頭に BNF 規則名のコメントを付与しています。
