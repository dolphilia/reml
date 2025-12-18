# Phase4: Core.Parse Lex Helpers 実装計画

## 背景と目的
- `docs/plans/core-parse-improvement/1-2-lex-helpers-plan.md`（WS3）は Step0-2 まででプリセット範囲・RunConfig 共有キー・ラベル語彙を確定済み。Step3（サンプル整備と回帰接続）が未消化のため、Phase4 側で実装・ゴールデン化の導線を用意する。
- 目標は「サンプル側の自前 `lexeme/symbol/identifier/number/string` を LexPreset で置換し、`RunConfig.extensions["lex"]` とラベル付き期待を統一」すること。`CP-WS3-001`（lexeme/space 共有）を Phase4 シナリオへ昇格させ、`CH2-PARSE-901/902`（autoWhitespace/Profile）と衝突しない形で運用する。
- 仕様参照: `docs/spec/2-3-lexer.md`（LexPreset/lex_pack）、`docs/spec/2-2-core-combinator.md`（with_space/autoWhitespace と space_id 共有）、`docs/spec/2-5-error.md`（Expectation/label）。

## スコープ
- 対象: Rust フロントエンドで実行するサンプル（`examples/spec_core/chapter2/parser_core/`、`examples/language-impl-comparison/reml/`）、およびそれに紐づく `expected/` ゴールデンと Phase4 シナリオマトリクス登録。
- RunConfig: `extensions["lex"]` への `space_id/profile/identifier_profile/layout_profile/safety` 書き戻しを必須化し、`keyword_ci`/`reserved`/`identifier(label付き)` をサンプルで利用する。診断キーは既存 `parser.syntax.expected_tokens` を維持（新設しない）。
- シナリオ: 追加 `CP-WS3-001`（lexeme/space 共有と label 付与）、既存 `CH2-PARSE-901/902` への接続確認。Layout token 生成は Phase9 以降のため本計画では扱わない。

## 成果物
- 実装/ヘルパ: `lex_pack(profile, identifier_profile, layout_profile, safety)` をサンプル共通入口にし、`keyword_ci` と `IdentifierProfile::toml_key` / `ConfigTriviaProfile::toml_relaxed` を opt-in で選べる形に整理。`RunConfig.extensions["lex"]` に全フィールドを書き戻す。
- サンプル整備:  
  - `examples/spec_core/chapter2/parser_core/core-parse-lexpack-basic.reml`（識別子+数値+`=`+`;`）と expected（stdout/diagnostic）を追加し、LexPack が空白/コメント混在で安定することを示す。  
  - `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml`: 自前 `lexeme/symbol` を LexPreset へ置換し、`identifier/number/string` にラベルを残したまま `Parse.fail` 自由文を排除。  
  - `.../sql_parser.reml`: `keyword_ci` + `reserved` で予約語拒否を統一し、`identifier_profile` 共有を導入。  
  - `.../toml_parser.reml`: `lex_pack_toml`（`ConfigTriviaProfile::toml_relaxed` + `IdentifierProfile::toml_key`）へ切替し、複数行文字列をプリセット版へ移行。  
  - `.../yaml_parser.reml`: Layout token 生成は据え置きつつ `space_id` 共有と `lexeme/symbol` 置換だけ先行。
- 回帰登録: Phase4 マトリクスへ `CP-WS3-001` を追加し、`core-parse-lexpack-basic.reml` の期待（whitespace/comment を変えても AST/診断が同一、`label("identifier"|"number"|"string")` を保持）を固定。`tooling/examples/run_phase4_suite.py` に実行経路を追加。
- 記録: 実装変更メモを `docs/notes/core-parse-api-evolution.md` または `docs/plans/core-parse-improvement/2-0-integration-with-regression.md` に追記。

## 実装ステップ（優先順）
1. **LexPack 共通入口と RunConfig 共有の整備**
   - `lex_pack` の record に `space/lexeme/symbol/keyword/keyword_ci/identifier/number/string/profile/identifier_profile/layout_profile/safety/space_id` を揃え、`RunConfig.extensions["lex"]` へ書き戻すヘルパを作成（サンプル側の共通関数）。
   - `keyword_ci` と `reserved` を `identifier_profile` 境界と共有し、`Expectation::Keyword` / `label("identifier")` を保持することをサンプル側で確認。
2. **サンプル置換（自前 lexeme/symbol の削減）**
   - `basic_interpreter_combinator.reml`: `lexeme/symbol/keyword` 再定義を LexPreset へ置換し、`identifier/number/string` をラベル付きプリセットに揃える。`Parse.fail` の自由文エラーを 2-5 準拠の構造化診断へ移行。
   - `sql_parser.reml`: `keyword_ci` で境界判定を置換し、`reserved(profile, set)` に集約。`RunConfig.extensions["lex"]`（space/profile/identifier_profile/safety）を反映。
   - `toml_parser.reml`: `lex_pack_toml` でトリビアとベアキーを吸収し、複数行文字列をプリセット版へ切替。
   - `yaml_parser.reml`: Layout 由来の token 生成は据え置き、`lexeme/symbol` を共通プリセットに寄せ `space_id` を共有。
3. **新規サンプルと期待ゴールデンの追加**
   - `core-parse-lexpack-basic.reml` を追加し、空白/コメント混在入力の AST/診断を `expected/spec_core/chapter2/parser_core/core-parse-lexpack-basic.{stdout,diagnostic.json}` で固定。
   - 期待条件: `label("identifier"|"number"|"string")` を保持し、`lexeme` が `with_space` と二重スキップしない（`space_id` 共有）。
4. **回帰登録と実行パイプライン接続**
   - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CP-WS3-001` を追加し、`resolution_notes` に CLI/LSP 実行コマンドと `RunConfig.extensions["lex"]` のキーを書き残す。
   - `tooling/examples/run_phase4_suite.py` に CP-WS3-001 の経路を追加し、`CH2-PARSE-901/902` と競合しないことを一度 Phase4 スイートで確認。

## 進捗状況
- 2025-12-18: 本計画書を起票し、WS3 Step3 の Phase4 側タスクを整理。実装・サンプル更新は未着手。

## 依存関係
- 計画: `docs/plans/core-parse-improvement/1-2-lex-helpers-plan.md`、`docs/plans/core-parse-improvement/2-0-integration-with-regression.md`。
- 仕様: `docs/spec/2-3-lexer.md`（LexPreset/IdentifierProfile/LayoutProfile）、`docs/spec/2-2-core-combinator.md`（with_space/autoWhitespace/Expectation）、`docs/spec/2-5-error.md`（期待集合・ラベル縮約）。
- 関連シナリオ: `CH2-PARSE-901/902`（autoWhitespace/Profile）、`CP-WS3-001`（本計画で追加）。

## リスクと対策
- **サンプルの教材意図を損なう**: 置換範囲を最小化し、過剰な抽象化を避ける。必要なら旧実装をコメントや補足として併記。
- **Layout 連携で regressions が出る**: YAML 系は `space_id` 共有のみに留め、Layout token 生成は Phase9 まで触らない。
- **診断揺れ**: ラベル付与と `Expectation` 種別を WS2 推奨語彙に固定し、`parser.syntax.expected_tokens` 以外のキーを増やさない。

## 完了判定
- `lex_pack`（RunConfig 書き戻し込み）が各サンプル入口で利用され、`keyword_ci`/`reserved`/`identifier(label付き)` がプリセット経由で動作する。
- `core-parse-lexpack-basic.reml` を含むサンプル/expected が追加され、`CP-WS3-001` が Phase4 マトリクスに登録・緑化。
- `basic_interpreter_combinator.reml`/`sql_parser.reml`/`toml_parser.reml`/`yaml_parser.reml` の自前 lexeme/symbol/identifier 等が LexPreset へ置換され、`RunConfig.extensions["lex"]` の共有キーが記録されている。
