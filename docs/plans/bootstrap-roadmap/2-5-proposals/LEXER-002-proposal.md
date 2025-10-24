# LEXER-002 Core.Parse.Lex API 抽出提案

## 1. 背景と症状
- 仕様は `Core.Parse.Lex` が空白・コメント・lexeme 等の共有ユーティリティを提供すると定義している（docs/spec/2-3-lexer.md 全体、特に §C〜G）。  
- 実装は `parser_driver` から直接 `Lexer.token` を呼び出し、`lexeme`/`symbol`/`config_trivia` などの高級 API や `RunConfig.extensions["lex"]` と連携する層が存在しない。  
- DSL や CLI/LSP が同じ空白処理を共有できず、仕様で求める `ParserId` / `ConfigTriviaProfile` の再利用が行えない。

## 2. Before / After
### Before
- 字句ユーティリティは非公開モジュールに散在し、構文パーサは Menhir 経由で直接トークン列を消費。  
- `RunConfig.extensions["lex"]` を設定しても利用されず、`lexeme` 相当の動作を実装が行っていない。

### After
- `compiler/ocaml/src/core_parse_lex.ml`（仮称）に `lexeme` / `symbol` / `config_trivia` など仕様準拠の関数群を実装し、`RunConfig.extensions["lex"]` から共有設定を読み込むシムを提供。  
- `parser_driver` をリファクタリングし、字句処理を `Core.Parse.Lex` 経由で行うよう段階移行する。  
- 仕様には現行制限の脚注を追加し、「OCaml 実装は Lex API の抽出を進行中」と明記する。

## 3. 影響範囲と検証
- **共有設定**: CLI/LSP/テストで `config_trivia` を利用し、同じ空白・コメントプロファイルが適用されるか検証。  
- **Packrat/ParserId**: `lexeme` が `rule` と連携し、`ParserId` を安定化させるかテスト（2-3 §B〜C）。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `lexer.shared_profile_pass_rate` を追加し、`RunConfig.extensions["lex"]` の反映状況を監視。
- **単体テスト**: `compiler/ocaml/tests/core_parse_lex_tests.ml` を追加し、空白・コメント・カスタムトークンプロファイルを切り替えた際の `lexeme` / `symbol` 出力をゴールデン比較で検証する。

## 4. フォローアップ
- `PARSER-002`（RunConfig 導入）と連動し、字句設定を `RunConfig` から読み込む順序を調整する。  
- `docs/guides/core-parse-streaming.md` のサンプルを OCaml 実装で動かせるよう、Lex API を呼び出す例を追加する。  
- Unicode プロファイル（LEXER-001）の対応と並行して、字句 API のテストを整備する。
- `docs/notes/core-parse-streaming-todo.md` に Lex API 抽出の進捗を追記し、Streaming PoC（EXEC-001）との依存関係を明確化する。
- **タイミング**: Phase 2-5 の中盤で RunConfig シムと並行して着手し、EXEC-001 ストリーミング PoC を開始するまでに Lex API 抽出を完了させる。

## 確認事項
- `Core.Parse.Lex` と `Lexer.token` の責務分離（どこまでをシムで巻き取るか）について Parser チームと合意が必要。  
- コメント・空白処理を共有する際の性能影響（特に大型入力でのオーバーヘッド）を事前に評価したい。
