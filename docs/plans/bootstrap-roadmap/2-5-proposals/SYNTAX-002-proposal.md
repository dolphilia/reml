# SYNTAX-002 `use` 多段ネスト対応計画

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
- **パーサテスト**: 既存の `compiler/ocaml/tests/test_parser.ml` に `use Core.Parse.{Lex, Op.{Infix, Prefix}}` などのケースを追加し、`Ast.use_item.item_nested` が正しく構築されることを `ast_printer` ベースのスナップショットで検証する。必要に応じて `parser_driver` 経由の CLI ゴールデンも更新する。  
- **既存サンプル**: Chapter 1 のコードサンプルと `examples/` 内モジュール参照を実際にパースし、差分が生じないか確認。再エクスポート構造の変化は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に記録し Phase 2-7 と共有する。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `parser.use_nested_support` を追加し、ネスト再エクスポートが 100% 成功することを記録。`collect-iterator-audit-metrics.py` 経由で収集できるようキー名と算出手順を追記する。  
- **ドキュメント整合**: `docs/spec/1-5-formal-grammar-bnf.md` と `docs/spec/3-0-core-library-overview.md` の再エクスポート例を同期し、差分レビューで参照できるよう脚注を追加する。

## 4. フォローアップ
- `Formal BNF`（docs/spec/1-5-formal-grammar-bnf.md）にもネスト規則を反映済みか確認し、必要ならハイライトを更新。  
- `docs/notes/core-parser-migration.md`（予定）へ OCaml 実装の AST 形状を記録し、Reml 実装移行時の参照資料とする。  
- コンパイラ IR で再エクスポートをどの段階で解決するか、Phase 2-7 Parser チームと合意を取る。
- `docs/guides/runtime/runtime-bridges.md` にネスト再エクスポートの使用例を追記し、ホストアプリケーションが適切にモジュールを束ねられるよう調整する。
- **タイミング**: Phase 2-5 の前半で AST 拡張を実装し、標準ライブラリ再エクスポート検証（Phase 2-5 中盤）より前に差分吸収を完了する。

## 5. 実施ステップ

| ステップ | 目的 | 主な作業 | 成果物 | 依存・調査 |
|----------|------|----------|--------|------------|
| S1: 現状棚卸し（Week32 Day1） | 仕様と実装の差分を明確化 | `docs/spec/1-1-syntax.md` の該当節と `compiler/ocaml/src/parser.mly` / `ast.ml` を突き合わせ、`item_nested = None` 固定になっている経路を洗い出す。`docs/plans/bootstrap-roadmap/2-5-review-log.md` に症状と再現手順を追記。 | レビュー記録、修正対象リスト | `2-5-spec-drift-remediation.md` の High 優先タスク整理、`ERR-001` 計画との期待集合共有 |
| S2: AST/型付き AST 整合確認（Week32 Day1-2） | AST 構造の変更有無を判断 | `ast.ml`/`typed_ast.ml`/`parser_design.md` を読込み、`use_item` に必要なフィールドが揃っているか確認し、不足すれば型を拡張。`type_inference.ml` で `tcu_use_decls` を利用する箇所を点検し、ネスト情報の伝播経路をメモ化。 | 更新済み設計メモ、必要なら型定義差分 | `parser_design.md` の更新、`TYPE-001` で予定している束縛ロジックとの整合 |
| S3: Menhir ルール実装（Week32 Day2-3） | 多段ネストを受理できる文法へ更新 | `use_item` に `LBRACE use_item_list RBRACE` 分岐を追加し、再帰的に `item_nested` を構築。`menhir --list-errors parser.mly` と `parser.conflicts` を再生成し、新規コンフリクトが無いかレビュー。`ERR-001` と連携して期待集合の変化を報告。 | 更新された `parser.mly` / `parser.conflicts` / `parser.automaton` | `ERR-001` の同期、`Lexer` 側で追加トークンが不要か確認 |
| S4: 束縛・診断連携（Week32 Day3-4） | AST 変更が後段へ伝播することを確認 | `type_inference.ml` や将来的なモジュール解決ロジックに備え、`use` ツリーを探索するユーティリティ（例: `Module_env.flatten_use_tree`）を設計。`parser_diag_state` での期待集合出力にネスト対応が影響しないか確認し、必要なら `ERR-001` の FixIt 計画へインプット。 | 設計ノート、必要なら補助ユーティリティ | Phase 2-7 で予定している再エクスポート解決タスク |
| S5: 検証とドキュメント更新（Week32 Day4-5） | 回 regressions を防ぎ成果を固定化 | `test_parser.ml` に多段ネストケースを追加し、`dune runtest compiler/ocaml/tests/test_parser.exe` で成功を確認。`0-3-audit-and-metrics.md` へメトリクスを追加し、`docs/spec/1-5-formal-grammar-bnf.md` と `docs/spec/3-0-core-library-overview.md` に脚注を追記。必要に応じて `docs/plans/bootstrap-roadmap/README.md` のハイライトを更新。 | テスト更新、メトリクス記録、脚注 | `docs/plans/bootstrap-roadmap/2-5-review-log.md` と `docs-migrations.log` の更新ポリシーに従う |

## 6. 進捗状況（2025-10-29 更新）
- **S1 現状棚卸し**: 仕様（`docs/spec/1-1-syntax.md:68-86`、`docs/spec/1-5-formal-grammar-bnf.md:24-33`）と実装（修正前の `compiler/ocaml/src/parser.mly:758-792`、`compiler/ocaml/src/ast.ml:372-389`）の差分を突合し、`item_nested` が常に `None` で構築される経路を特定。結果は [`../2-5-review-log.md`](../2-5-review-log.md#syntax-002-day1-調査2025-10-27) に記録。
- **S2 AST/型付き AST 整合確認**: `typed_ast.ml` と `type_inference.ml` が `tcu_use_decls = cu.uses` を保持する設計を確認し、`item_nested` を Menhir から渡せば下流がそのまま受け取れることを検証。`compiler/ocaml/docs/parser_design.md` に脚注を追記し、結果を [`../2-5-review-log.md`](../2-5-review-log.md#syntax-002-day1-2-ast型付きast整合確認2025-10-27) に記録。
- **S3 Menhir ルール実装**: `compiler/ocaml/src/parser.mly` の `use_item` を `ident` + `as` + `.{...}` の再帰展開へ更新し、`item_nested` へ `Some nested` を格納できるようにした。`menhir --list-errors parser.mly` を再実行して `parser.conflicts` に追加のコンフリクトが発生しないことを確認し、ネスト再帰による期待集合への影響が無いことを `ERR-001` チームへ共有。作業ログは [`../2-5-review-log.md`](../2-5-review-log.md#syntax-002-day2-3-menhir-ルール実装2025-10-28) に記録。
- **S4 束縛・診断連携**: `compiler/ocaml/src/module_env.ml` を追加し、`flatten_use_decls` で `use` ツリーを `binding_local`／`binding_path` へ展開。`typed_ast.ml` に `tcu_use_bindings` を追加して Typer 側で集約結果を保持し、`compiler/ocaml/tests/test_module_env.ml` でシナリオ別の展開結果を検証した。`parser_diag_state.ml` の最遠期待集合ロジックに変化が無いことも合わせて確認済み。詳細は [`../2-5-review-log.md`](../2-5-review-log.md#syntax-002-day3-4-束縛診断連携2025-10-29) に記録。
- **再現確認**: `dune exec remlc -- --emit-ast tmp/use_nested.reml`（`compiler/ocaml` カレント）で `use Core.Parse.{Lex, Op.{Infix, Prefix}}` が構文エラーになることを再現し、診断ログと監査情報を取得。
- **次ステップ準備**: S2 で `typed_ast` 伝搬と `Module_env` 連携を調査できるよう、関連ファイル（`typed_ast.ml`, `parser_design.md`）の参照位置を整理済み。
- **S5 検証とドキュメント更新**: `compiler/ocaml/tests/test_parser.ml` に多段ネスト `use` のユニットテストを追加し、`test_module_env.ml` と併せて `dune runtest` で成功を確認。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `parser.use_nested_support` 指標を追加し、`docs/spec/1-5-formal-grammar-bnf.md` と `docs/spec/3-0-core-library-overview.md` に脚注/概要を追記して実装完了と監視体制を明記した（[`../2-5-review-log.md`](../2-5-review-log.md#syntax-002-day4-5-検証ドキュメント更新2025-11-12) を参照）。

## 残課題
- `pub use` と通常の `use` で共有する AST 形状（特にモジュール可視性）をどう扱うか要確認。  
- ネスト解決時に不要な再帰による性能劣化がないか、Menhir 生成コードでの影響をレビューする。
