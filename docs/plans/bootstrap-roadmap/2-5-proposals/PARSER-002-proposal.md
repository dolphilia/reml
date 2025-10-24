# PARSER-002 RunConfig 導入ロードマップ提案

## 1. 背景と症状
- 仕様では `RunConfig` によって Packrat / 左再帰 / recover / lex / stream などの挙動を切り替えると定義している（docs/spec/2-1-parser-type.md:90-188）。  
- 現行 `parser_driver` は Menhir ランナーを直接呼び出し、設定やメモテーブルを受け取る仕組みがない（compiler/ocaml/src/parser_driver.ml:15-43）。  
- `RunConfig.extensions["lex"]` / `["recover"]` / `["stream"]` などの契約を参照できず、Chapter 2 の実行戦略やガイドとの整合が崩れている。

## 2. Before / After
### Before
- `parse` 関数は `Lexer.token` を呼び出すだけで Packrat や左再帰を制御できない。  
- `RunConfig` との連携が無いため、`extensions` に設定しても効果がなく、DSL/CLI/LSP 間で挙動を統一できない。

### After
- `parser_driver` に `run : parser -> RunConfig -> ParseResult` を新設し、`State` 初期化時に `RunConfig` を保持。  
- Packrat/左再帰/trace 等の設定を段階導入し、`RunConfig` を通して CLI/LSP と連携できるようシム層を構築する。  
- 仕様に暫定脚注を追記し、「OCaml 実装は RunConfig 移行中」と明記する。

## 3. 影響範囲と検証
- **メトリクス**: `0-3-audit-and-metrics.md` に `parser.runconfig_coverage` を追加し、`require_eof` `packrat` `left_recursion` など主要スイッチがテスト経由で確認されるかを記録。  
- **CLI/LSP**: `tooling` 側で `RunConfig` を生成している箇所を更新し、空白・recover・stream 等の設定が OCaml 実装へ届くことを確認。  
- **ストリーミング**: EXEC-001（run_stream PoC）と並行して、`extensions["stream"]` を受け取る経路を確立する。
- **単体テスト**: `compiler/ocaml/tests/runconfig_tests.ml` を追加し、設定値ごとの挙動（Packrat/左再帰/require_eof）が `ParseResult` に反映されるかパラメトリックテストで保証する。

## 4. フォローアップ
- Packrat/左再帰実装は `PARSER-003`（コンビネーター抽出）と密接に関係するため、同じロードマップで管理する。  
- RunConfig シムが整った段階で CLI フラグ・LSP 設定ファイルを更新し、ユーザーが仕様通りに設定できるようドキュメントを調整する。  
- `docs/guides/core-parse-streaming.md` と `docs/spec/2-6-execution-strategy.md` に OCaml 実装の進行状況を脚注で追加する。
- `docs/notes/core-parser-migration.md`（未作成なら新規）へ RunConfig 移行ステップと既知の制限を記録し、Phase 3 での Reml 実装への写像に備える。

## 確認事項
- Packrat/左再帰の段階導入順序（`require_eof` → `packrat` → `extensions`）を Parser チームと調整したい。  
- RunConfig の共有方法（不変構造体 vs. 可変レコード）とメモリコストについて、実装方針を再確認する必要がある。
