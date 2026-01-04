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
  3. Lex API 抽出後、`docs/guides/compiler/core-parse-streaming.md` のサンプルを更新し、`lexeme` / `config_trivia` を利用したストリーミング例（`resume`/`checkpoint` 付き）を追加する。
- **2025-11-26 追記**: Step1 で `Pack` レコード（`space`/`lexeme`/`symbol`/`space_id`）と `Bridge.effective_profile`/`attach_space` を設計済み。`space_id` は `Parser_diag_state` の ID 発行器を流用し `RunConfig.extensions["lex"].space_id` へ round-trip させる方針。Streaming 実装では Checkpoint 作成前に `Pack.space_id` と `RunConfig` の値が一致するか検証する。

[^lexer-step0]: `docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md` Step0 調査サマリ、および `docs/plans/bootstrap-roadmap/2-5-review-log.md` 「LEXER-002 Day1」参照。

## 2025-12-12 Recover FixIt フォローアップ
- **背景**: ERR-002 Step3 で CLI/LSP/ストリーミング各経路の `recover` 拡張と FixIt を整備し、`parser.recover_fixit_coverage = 1.0` を達成。Packrat キャッシュ経路と notes 翻訳ルールは未確定のため、Phase 2-7 へ継続課題として登録する。関連ログは [`docs/plans/bootstrap-roadmap/2-5-review-log.md`](../plans/bootstrap-roadmap/2-5-review-log.md#err-002-step3-clilsp-出力とメトリクス整備2025-12-12) を参照。
- **TODO**:
  1. `Parser_expectation.Packrat` に `recover` スナップショットを保持するハンドルを追加し、Packrat 経路で FixIt を生成しても `parser.recover_fixit_coverage` が低下しないようにする（ERR-002 Step4 で仕様・脚注を更新済み、Phase 2-7 で継続検証）。  
  2. `Diagnostic.Builder.add_note` で利用する `recover` notes の文章をローカライズ可能なテンプレートに抽象化し、CLI/LSP で locale に応じたメッセージ切替を提供する。翻訳ルールは `docs/spec/2-5-error.md` の脚注と連動させ、Phase 2-7 `Recover FixIt 継続整備` で追跡。  
  3. ストリーミング Pending → resume の往復で FixIt が重複適用されないかを監査ログ (`StreamOutcome.Pending.extensions.recover`) で確認するチェックリストを作成し、Phase 2-7 の CI に組み込む（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §4 へ登録）。
- **2025-12-15 追記**: ERR-002 Step4 で `docs/spec/2-5-error.md` と `docs/spec/3-6-core-diagnostics-audit.md` に脚注を追加し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` に共有記録を残した。上記 TODO は Phase 2-7 計画の「Recover FixIt 継続整備」に移管済みで、Packrat 経路と notes ローカライズの検証は Phase 2-7 で完了させる。
