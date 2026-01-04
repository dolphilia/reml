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

## AST ノード生成時の紐付け規約（暫定）
- **ノード境界**: AST ノードを生成するパーサー単位で CST ノード境界を確定する。`rule("name", ...)` を基本境界とし、`label` は診断向けのため CST の `kind` には使わない。
- **kind の命名**: `CstNode.kind` は `rule` 名を使用する。短く安定した命名を優先し、リネーム時は AST/仕様側の用語更新と同時に行う。
- **子要素の確定**: 同一 `rule` 内で成功したサブパーサーの結果を `children` に順序通りに格納する。Token と子ノードの順序は入力順に一致させる。
- **スパンの算定**: AST ノードの span が確定する場合は `CstNode.span` と一致させる。span を合成できない場合は空Spanを許容する。
- **フォールバック**: ルール境界が曖昧な場合（例: `map` のみで AST を生成する）でも CST はトークン列を保持し、後続の整形器は `children` の順序を信頼して復元する。

## 参照
- `docs/plans/bootstrap-roadmap/4-1-core-parse-cst-plan.md`
- `docs/notes/dsl/dsl-enhancement-proposal.md`

## フェーズE完了メモ（簡易）
- formatter-authoring に CST 連携の最小記述を追加。
- regression plan に `CH2-PARSE-930` の回帰条件とログ保存先を追記。
- `reports/spec-audit/ch5/logs/stdlib-parse-cst-*.md` を運用対象に追加。
