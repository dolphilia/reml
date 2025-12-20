# Core.Parse CST 設計メモ

## 目的
- `docs/plans/bootstrap-roadmap/4-1-core-parse-cst-plan.md` の CST 付着ルールと表現を補足する。
- `span` の空表現を明示し、CST 収集とフォーマッタの相互運用を安定させる。

## 空Spanの定義
- **定義**: 空Spanは「位置が確定しない合成ノード」を表すための Span とし、`Span::empty` 相当の値で表現する。
- **生成規則**:
  - 子要素の span を統合できない場合は空Spanを付与する。
  - `children` が空、または `trivia` のみで構成されるノードは空Spanを採用する。
- **利用側の前提**:
  - `CstPrinter` は空Spanを「位置なし」として扱い、字句順は `children`/`trivia` の順序に従う。
  - 診断や LSP への位置提示は `CstNode.span` ではなく `ParseResult.diagnostics` を優先する。

## 付着ルールの補足
- 先頭 Trivia は `trivia_leading` に付着する。
- `autoWhitespace` 経由で消費した Trivia は直近の確定ノードの `trivia_trailing` に付着し、
  次ノード生成時に `trivia_leading` へ移送する（改行/コメントで境界を作る）。
- `Layout` トークンは `Trivia.kind=Layout` として同じ付着ルールを適用する。

## 参照
- `docs/plans/bootstrap-roadmap/4-1-core-parse-cst-plan.md`
- `docs/notes/dsl-enhancement-proposal.md`
