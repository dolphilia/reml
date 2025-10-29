# Core.Parse.Streaming TODO

## 2025-11-25 Lex API 抽出連携メモ
- **背景**: LEXER-002 Step0 調査で、`Core.Parse.Lex` 公開 API が未抽出であること、`RunConfig.extensions["lex"]` がランナーやストリーミング経路へ伝播していないことを確認した[^lexer-step0]。
- **依存関係整理**:
  - `PARSER-002` で導入済みの `parser_run_config` を Lex 層から参照し、`ConfigTriviaProfile` と `ParserId` を共有するフックを追加する必要がある。
  - `PARSER-003` が計画する Packrat/左再帰シムは `ParserId` と `lexeme` 共有を前提とするため、Lex API 抽出時に `space`/`symbol` 生成で安定 ID を割り当てる設計を決定しておく。
  - `EXEC-001`（Streaming PoC）では CLI/ストリーミングの両経路が同一 RunConfig を用いるため、`Core.Parse.Lex` 側で `config_trivia` → `RunConfig.extensions["lex"]` の round-trip を保証し、Checkpoint/Resume 時にも同じ空白プロファイルを再構築できるようにする。
- **TODO（共有タスク案）**:
  1. `core_parse_lex.{mli,ml}` 設計時に `Run_config.Lex` から `ConfigTriviaProfile` へ変換するヘルパを用意し、Streaming ランナーが `lex.space_id()` を検証できるよう `ParserId` 生成ポリシーを定義する。
  2. Streaming `run_stream` 初期化で `RunConfig.extensions["lex"]` を必須化し、未設定時は `parser.runconfig.lex_unset` 警告を返す。CLI 側で fallback を注入する場合は脚注で仕様に明記する。
  3. Lex API 抽出後、`docs/guides/core-parse-streaming.md` のサンプルを更新し、`lexeme` / `config_trivia` を利用したストリーミング例（`resume`/`checkpoint` 付き）を追加する。

[^lexer-step0]: `docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md` Step0 調査サマリ、および `docs/plans/bootstrap-roadmap/2-5-review-log.md` 「LEXER-002 Day1」参照。

