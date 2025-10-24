# SYNTAX-002 `use` 多段ネスト対応提案

## 1. 背景と症状
- 仕様は `use Core.Parse.{Lex, Op.{Infix, Prefix}}` のように中括弧内で多段ネストを許容すると定義している（docs/spec/1-1-syntax.md:78-83）。  
- 現行パーサは `use_item` 生成時に `item_nested = None` を固定しており（compiler/ocaml/src/parser.mly:784-788）、ネストした再エクスポートを解析できない。結果として Chapter 1 のサンプルが OCaml 実装で失敗し、Phase 3 のセルフホスト計画で想定するモジュール再エクスポートが再現できない。

## 2. Before / After
### Before
- 中括弧内の `.` 区切りを `use_brace_prefix` で解釈するが、最終的な `use_item` に子要素を渡さず、単一階層のみ受理。  
- `pub use` でも同様にネスト不可で、標準ライブラリが仕様通りの再エクスポート構造を記述できない。

### After
- `use_item` に `item_nested : use_item list option` を保持させ、`Op.{Infix, Prefix}` のような子リストを再帰的に構築する。  
- `use_item_list` を再帰化して `item_nested` に差し込み、`use Core.Parse.{Lex, Op.{Infix}}` などネストした再エクスポートを AST へ反映する。  
- 仕様とのギャップを解消するまで `docs/spec/1-1-syntax.md` へ暫定脚注を追記し、実装進捗を記録する。

## 3. 影響範囲と検証
- **パーサテスト**: `compiler/ocaml/tests/module_use_tests.ml`（新設）で多段ネストケースを追加し、AST が期待通りであることをスナップショット比較。  
- **既存サンプル**: Chapter 1 のコードサンプルと `examples/` 内モジュール参照を実際にパースし、差分が生じないか確認。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `parser.use_nested_support` を追加し、ネスト再エクスポートが 100% 成功することを記録。
- **ドキュメント整合**: `docs/spec/1-5-formal-grammar-bnf.md` と `docs/spec/3-0-core-library-overview.md` の再エクスポート例を同期し、差分レビューで参照できるよう脚注を追加する。

## 4. フォローアップ
- `Form al BNF`（docs/spec/1-5-formal-grammar-bnf.md）にもネスト規則を反映済みか確認し、必要ならハイライトを更新。  
- `docs/notes/core-parser-migration.md`（予定）へ OCaml 実装の AST 形状を記録し、Reml 実装移行時の参照資料とする。  
- コンパイラ IR で再エクスポートをどの段階で解決するか、Phase 2-7 Parser チームと合意を取る。
- `docs/guides/runtime-bridges.md` にネスト再エクスポートの使用例を追記し、ホストアプリケーションが適切にモジュールを束ねられるよう調整する。
- **タイミング**: Phase 2-5 の前半で AST 拡張を実装し、標準ライブラリ再エクスポート検証（Phase 2-5 中盤）より前に差分吸収を完了する。

## 確認事項
- `pub use` と通常の `use` で共有する AST 形状（特にモジュール可視性）をどう扱うか要確認。  
- ネスト解決時に不要な再帰による性能劣化がないか、Menhir 生成コードでの影響をレビューする。
