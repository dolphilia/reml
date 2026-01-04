# Phase4: Core.Parse Lex Helpers 実装計画

## 背景と目的
- `docs/plans/core-parse-improvement/1-2-lex-helpers-plan.md`（WS3）は Step0-2 まででプリセット範囲・RunConfig 共有キー・ラベル語彙を確定済み。Step3（サンプル整備と回帰接続）が未消化のため、Phase4 側で実装・ゴールデン化の導線を用意する。
- 目標は「サンプル側の自前 `lexeme/symbol/identifier/number/string` を LexPreset で置換し、`RunConfig.extensions["lex"]` とラベル付き期待を統一」すること。`CP-WS3-001`（lexeme/space 共有）を Phase4 シナリオへ昇格させ、`CH2-PARSE-901/902`（autoWhitespace/Profile）と衝突しない形で運用する。
- 仕様参照: `docs/spec/2-3-lexer.md`（LexPreset/lex_pack）、`docs/spec/2-2-core-combinator.md`（with_space/autoWhitespace と space_id 共有）、`docs/spec/2-5-error.md`（Expectation/label）。

## 今回の実装で手を加える領域（抜粋）
- **ヘルパ/API**: `lex_pack`（RunConfig 書き戻し込み）、`keyword_ci`、`IdentifierProfile::toml_key`、`ConfigTriviaProfile::toml_relaxed` をサンプル入口で利用できるよう整理し、ラベル付き `identifier/number/string` をプリセット経由で固定。
- **サンプル**: `basic_interpreter_combinator.reml` / `sql_parser.reml` / `toml_parser.reml` / `yaml_parser.reml` の自前 `lexeme/symbol/...` を LexPreset に置換し、自由文エラーを 2-5 準拠の期待集合へ統一。新規 `core-parse-lexpack-basic.reml` を追加。
- **回帰/ツール**: Phase4 シナリオマトリクスに `CP-WS3-001` を追加し、`tooling/examples/run_phase4_suite.py` 経由でゴールデンを固定。`RunConfig.extensions["lex"]` キー（`space_id/profile/identifier_profile/layout_profile/safety`）を回帰で再構成できるよう記録。
- **実装（Rust CLI）**: `compiler/frontend/src/bin/reml_frontend.rs` の `--parse-driver` を拡張し、`core-parse-lexpack-basic.reml` の失敗入力（`Parse.run("...")` 由来）で `parser.syntax.expected_tokens` が `identifier|number|string` を返すことを回帰で検証できるようにする。

## スコープ
- 対象: Rust フロントエンドで実行するサンプル（`examples/spec_core/chapter2/parser_core/`、`examples/language-impl-samples/reml/`）、およびそれに紐づく `expected/` ゴールデンと Phase4 シナリオマトリクス登録。
- RunConfig: `extensions["lex"]` への `space_id/profile/identifier_profile/layout_profile/safety` 書き戻しを必須化し、`keyword_ci`/`reserved`/`identifier(label付き)` をサンプルで利用する。診断キーは既存 `parser.syntax.expected_tokens` を維持（新設しない）。
- シナリオ: 追加 `CP-WS3-001`（lexeme/space 共有と label 付与）、既存 `CH2-PARSE-901/902` への接続確認。Layout token 生成は Phase9 以降のため本計画では扱わない。

## 成果物
- 実装/ヘルパ: `lex_pack(profile, identifier_profile, layout_profile, safety)` をサンプル共通入口にし、`keyword_ci` と `IdentifierProfile::toml_key` / `ConfigTriviaProfile::toml_relaxed` を opt-in で選べる形に整理。`RunConfig.extensions["lex"]` に全フィールドを書き戻す。
- サンプル整備:  
  - `examples/spec_core/chapter2/parser_core/core-parse-lexpack-basic.reml`（識別子+数値+`=`+`;`）と expected（stdout/diagnostic）を追加し、LexPack が空白/コメント混在で安定することを示す。  
  - `examples/language-impl-samples/reml/basic_interpreter_combinator.reml`: 自前 `lexeme/symbol` を LexPreset へ置換し、`identifier/number/string` にラベルを残したまま `Parse.fail` 自由文を排除。  
  - `.../sql_parser.reml`: `keyword_ci` + `reserved` で予約語拒否を統一し、`identifier_profile` 共有を導入。  
  - `.../toml_parser.reml`: `lex_pack_toml`（`ConfigTriviaProfile::toml_relaxed` + `IdentifierProfile::toml_key`）へ切替し、複数行文字列をプリセット版へ移行。  
  - `.../yaml_parser.reml`: Layout token 生成は据え置きつつ `space_id` 共有と `lexeme/symbol` 置換だけ先行。
- 回帰登録: Phase4 マトリクスへ `CP-WS3-001` を追加し、`core-parse-lexpack-basic.reml` の期待（whitespace/comment を変えても AST/診断が同一、`label("identifier"|"number"|"string")` を保持）を固定。`tooling/examples/run_phase4_suite.py` に実行経路を追加。
- 記録: 実装変更メモを `docs/notes/parser/core-parse-api-evolution.md` または `docs/plans/core-parse-improvement/2-0-integration-with-regression.md` に追記。

## 実装ステップ（優先順）
1. **LexPack 共通入口と RunConfig 共有の整備**
   - `lex_pack` の record に `space/lexeme/symbol/keyword/keyword_ci/identifier/number/string/profile/identifier_profile/layout_profile/safety/space_id` を揃え、`RunConfig.extensions["lex"]` へ書き戻すヘルパを作成（サンプル側の共通関数）。
   - `keyword_ci` と `reserved` を `identifier_profile` 境界と共有し、`Expectation::Keyword` / `label("identifier")` を保持することをサンプル側で確認。
   - 具体タスク（Step1 完了条件）:
     - `LexPack` record の項目名と型を明文化し、`lex_pack(profile, identifier_profile, layout_profile, safety)` / `lex_pack_toml()` のシグネチャと返却フィールドを決める（RunConfig への書き戻しキー: `space_id`, `profile`, `identifier_profile`, `layout_profile`, `safety`）。
     - `keyword_ci(space, kw)` の境界判定を `identifier_profile` 由来の `is_identifier_continue` に合わせ、`Expectation::Keyword(kw)` を返すことを確認するチェックリストを作成。
     - RunConfig 書き戻しの処理順（`space_id` 採番 → `profile`/`identifier_profile`/`layout_profile`/`safety` 設定 → `lexeme/symbol/keyword` 提供）を決め、サンプル共通ヘルパに適用する段取りを記載。
   - 実施順（Step1 内の詳細手順）:
     1) `docs/spec/2-3-lexer.md` と `docs/spec/2-2-core-combinator.md` の `lex_pack`/`with_space`/`autoWhitespace` 記述を再確認し、`space_id` 採番と書き戻し順序を箇条書きで確定する。
     2) `lex_pack`/`lex_pack_toml` の返却フィールド表を作り、デフォルト値（profile/identifier_profile/layout_profile/safety）を明示。`identifier/number/string` のラベル同梱を表に含める。
     3) `keyword_ci` 境界チェックのテスト観点をリスト化（大文字小文字揺れ、予約語拒否との共存、返却値は原文 `kw`、Expectation は `Keyword(kw)`）。必要なら `docs/notes/parser/core-parse-api-evolution.md` にチェックリストを転記。
     4) RunConfig 書き戻しキーを JSON 例で示し、`extensions["lex"]` に最低限必要なフィールドセット（`space_id/profile/identifier_profile/layout_profile/safety`）を固定する。
   - Step1 実施メモ（結果）
     - `space_id` 採番と書き戻し順序（確定）:
       - ① `space` 構築直後に `space_id` を採番し `extensions["lex"].space_id` に保存  
       - ② `profile`/`identifier_profile`/`layout_profile`/`safety` を既定値含めて書き戻し  
       - ③ `lexeme`/`symbol`/`keyword`/`keyword_ci`/`identifier`/`number`/`string` を `space_id` 共有で生成  
       - ④ `reserved(profile, set)` が `identifier_profile` と同じ境界を使うことをチェックリストに含める
     - `lex_pack`/`lex_pack_toml` 返却フィールド（デフォルト値込み）:
       - `space: Parser<()>`（ConfigTriviaProfile に基づく空白/コメント）
       - `lexeme: Parser<T> -> Parser<T>`（space_id 共有、後続 skip なし）
       - `symbol: string -> Parser<string>`
       - `keyword: string -> Parser<string>`（境界判定あり, Expectation::Keyword）
       - `keyword_ci: string -> Parser<string>`（境界判定に identifier_profile を利用, Expectation::Keyword）
       - `identifier: Parser<Identifier>`（`label("identifier")` 内蔵, IdentifierProfile 既定 `default`）
       - `number: Parser<Number>`（`label("number")` 内蔵）
       - `string: Parser<StringLiteral>`（`label("string")` 内蔵）
       - `profile: ConfigTriviaProfile`（デフォルト `strict_json`）
       - `identifier_profile: IdentifierProfile`（デフォルト `default`）
       - `layout_profile: LayoutProfile option`（デフォルト `None`）
       - `safety: LexSafetyProfile`（デフォルト `strict`）
       - `space_id: ParserId`
       - `lex_pack_toml() = lex_pack(ConfigTriviaProfile::toml_relaxed, IdentifierProfile::toml_key, None, LexSafetyProfile::strict)` としてショートカット
     - `keyword_ci` 境界チェック観点:
       - 大文字小文字揺れで一致しつつ返却値は原文 `kw` を返す
       - 予約語拒否（`reserved(profile, set)}`）と同じ境界判定（`identifier_profile.is_identifier_continue`）で競合しない
       - `Expectation::Keyword(kw)` を維持し、Rule 名に依存しない
       - `space_id` が `with_space`/`autoWhitespace` と一致し二重スキップしない
     - RunConfig への書き戻し例（最低限フィールドセット）:
       ```json
       {
         "extensions": {
           "lex": {
             "space_id": "<parser_id>",
             "profile": "strict_json",
             "identifier_profile": "default",
             "layout_profile": null,
             "safety": "strict"
           }
         }
       }
       ```
2. **サンプル置換（自前 lexeme/symbol の削減）**
   - `basic_interpreter_combinator.reml`: `lexeme/symbol/keyword` 再定義を LexPreset へ置換し、`identifier/number/string` をラベル付きプリセットに揃える。`Parse.fail` の自由文エラーを 2-5 準拠の構造化診断へ移行。
   - `sql_parser.reml`: `keyword_ci` で境界判定を置換し、`reserved(profile, set)` に集約。`RunConfig.extensions["lex"]`（space/profile/identifier_profile/safety）を反映。
   - `toml_parser.reml`: `lex_pack_toml` でトリビアとベアキーを吸収し、複数行文字列をプリセット版へ切替。
   - `yaml_parser.reml`: Layout 由来の token 生成は据え置き、`lexeme/symbol` を共通プリセットに寄せ `space_id` を共有。
   - 実施順（Step2 内の詳細手順）:
     1) `lex_pack` 共有ヘルパをサンプル共通入口に導入し、`RunConfig.extensions["lex"]` へ `space_id/profile/identifier_profile/layout_profile/safety` を書き戻す雛形を作成（既定値は Step1 に従う）。
     2) `basic_interpreter_combinator.reml`: 自前 `lexeme/symbol/keyword` を LexPreset に置換し、`identifier/number/string` のラベル付き版へ統一。`Parse.fail` の自由文を 2-5 構造化診断（Expectation ベース）に差し替え。
     3) `sql_parser.reml`: `keyword_ci` + `reserved(profile, set)` へ置換し、予約語拒否ロジックをプリセットへ集約。`identifier_profile` 共有を導入し、RunConfig を書き戻す。
     4) `toml_parser.reml`: `lex_pack_toml` に切替え、ベアキーを `IdentifierProfile::toml_key` に寄せる。複数行文字列をプリセット版（`stringMultiline` 相当）へ移行。
     5) `yaml_parser.reml`: Layout token 生成は触らず、`space_id` 共有と `lexeme/symbol` 置換のみ先行。`RunConfig` への書き戻しで layout_profile を None のまま共有する。
     6) 影響確認: 各サンプルの expected（stdout/diagnostic）が揺れる場合は Step3 で再生成予定とし、差分の理由をメモ（`docs/notes/parser/core-parse-api-evolution.md`）に追記。
3. **新規サンプルと期待ゴールデンの追加**
   - `core-parse-lexpack-basic.reml` を追加し、空白/コメント混在入力の AST/診断を `expected/spec_core/chapter2/parser_core/core-parse-lexpack-basic.{stdout,diagnostic.json}` で固定。
   - 期待条件: `label("identifier"|"number"|"string")` を保持し、`lexeme` が `with_space` と二重スキップしない（`space_id` 共有）。
   - 実施順（Step3 内の詳細手順）:
     1) サンプル新規作成: `examples/spec_core/chapter2/parser_core/core-parse-lexpack-basic.reml` を追加し、`lex_pack` 入口を使って「識別子 + 数値 + `=` + `;`」をパースする最小 DSL を記述。空白・コメント（行/ブロック）の混在を許容する入力例をサンプル末尾に含める。
     2) 期待ゴールデン生成: CLI で `--output stdout` と `--output json` を取得し、`expected/spec_core/chapter2/parser_core/core-parse-lexpack-basic.stdout` と `.diagnostic.json` に保存。`label("identifier"|"number"|"string")` が humanized/JSON のいずれでも保持されることを確認する。
     3) `CP-WS3-001` と紐付け: 期待ファイルの取得コマンドと前提（`RunConfig.extensions["lex"]` の profile/safety/space_id）を `phase4-scenario-matrix.csv` `resolution_notes` にメモし、空白/コメントを変えたバリエーションでも AST/診断が揺れないことを確認する。
     4) 影響メモ: 生成したゴールデンの取得方法（コマンド/入力ファイル名）と、`label` 維持を確認した観点を `docs/notes/parser/core-parse-api-evolution.md` に追記する。
   - 実施状況（2025-12-18 時点）:
     - 1) サンプル作成済み。成功入力 + 数値欠落の失敗入力を同居させ、LexPack 入口を使用。
     - 2) `--parse-driver --output json` で `.diagnostic.json` を再生成し、`expected_tokens` に `identifier/number/string` が含まれることを確認（サンプル冒頭コメントの `Parse.run("alpha = ;")` から失敗入力を抽出する）。
     - stdout ゴールデンも更新し、失敗ケースでは `identifier/number/string` のラベルが humanized に出ることを固定した。
     - 3)〜4) 実施済み。`CP-WS3-001` を Phase4 マトリクスへ登録し、取得コマンドを `resolution_notes` に記録。影響メモは `docs/notes/parser/core-parse-api-evolution.md` に追記。
4. **回帰登録と実行パイプライン接続**
   - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CP-WS3-001` を追加し、`resolution_notes` に CLI/LSP 実行コマンドと `RunConfig.extensions["lex"]` のキーを書き残す。
   - `tooling/examples/run_phase4_suite.py` に CP-WS3-001 の経路を追加し、`CH2-PARSE-901/902` と競合しないことを一度 Phase4 スイートで確認。

### Step1 実施記録（LexPack 共通入口と RunConfig 共有）
- **LexPack 標準形の決定**: `space: Parser<()>`（`with_space` と併用する空白/コメントトリビア）、`lexeme: Parser<T> -> Parser<T>`、`symbol: string -> Parser<string>`、`keyword: string -> Parser<string>`、`keyword_ci: string -> Parser<string>`、`identifier: Parser<Identifier>`（`label("identifier")` 付与済み）、`number: Parser<Number>`（`label("number")` 付与済み）、`string: Parser<StringLiteral>`（`label("string")` 付与済み）、`profile: ConfigTriviaProfile`、`identifier_profile: IdentifierProfile`、`layout_profile: LayoutProfile option`、`safety: LexSafetyProfile`、`space_id: ParserId` を保持する record として固定。`identifier/number/string` は WS2 推奨ラベルを組込み、サンプル側で追加ラップを要求しない。
- **ヘルパシグネチャの整理**: `lex_pack(profile, identifier_profile, layout_profile, safety)` はすべての引数をオプション指定可能にし、未指定は `profile=ConfigTriviaProfile::strict_json`、`identifier_profile=IdentifierProfile::default`、`layout_profile=None`、`safety=LexSafetyProfile::strict` を既定とする。`lex_pack_toml()` は `lex_pack(ConfigTriviaProfile::toml_relaxed, IdentifierProfile::toml_key, None, LexSafetyProfile::strict)` のショートカットとして扱い、返却 record のフィールドは上記標準形と一致させる。
- **keyword_ci 境界チェックリスト**:
  - `keyword_ci(space, kw)` は `identifier_profile.is_identifier_continue` に基づく境界判定を必須とし、大小文字変換はマッチングのみに限定して **返却値は原文 `kw` を維持**する。
  - 期待集合は `Expectation::Keyword(kw)` を返し、`identifier_profile` と共有する境界判定が `reserved(profile, set)` と衝突しないことを確認する（予約語拒否と同じ境界を共有）。
  - `space_id` が一致する `space` を内部で利用し、`with_space`/`autoWhitespace` と二重スキップしないことをサンプル側のチェックリストに含める。
- **RunConfig 書き戻し順序の固定**:
  1. `space` を構築した時点で `space_id` を採番し、`RunConfig.extensions["lex"].space_id` に格納する。
  2. `profile` / `identifier_profile` / `layout_profile` / `safety` を `extensions["lex"]` に書き戻し、未指定は既定値を明示する。
  3. `lexeme`/`symbol`/`keyword`/`keyword_ci`/`identifier`/`number`/`string` のクロージャを構築し、`space_id` を共有した状態でサンプルへ渡す。
  4. `reserved(profile, set)` を利用するサンプルでは、上記 `identifier_profile` を境界判定に再利用することをチェック項目に含める。

## 進捗状況
- 2025-12-18: 本計画書を起票し、WS3 Step3 の Phase4 側タスクを整理。
- 2025-12-18: Step1 実施。`LexPack` 標準フィールド・`lex_pack/lex_pack_toml` シグネチャ・`keyword_ci` 境界チェックリスト・RunConfig 書き戻し順を確定し、共通ヘルパに含める内容を明文化。
- 2025-12-18: Step3 実施。`examples/spec_core/chapter2/parser_core/core-parse-lexpack-basic.reml` と `expected/.../core-parse-lexpack-basic.{stdout,diagnostic.json}` を更新し、`--parse-driver` 経由で `identifier/number/string` のラベル付き `parser.syntax.expected_tokens` をゴールデン化。
- 2025-12-18: Step4 実施。`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CP-WS3-001` を登録し、`tooling/examples/run_phase4_suite.py --suite spec_core` で `CP-WS3-001` が `diagnostic_keys` と一致すること（= 緑化）を確認。
- 2025-12-18: Rust 側の `--parse-driver` 経路を調整し、`core-parse-lexpack-basic.reml` から抽出した失敗入力に対して `expected_tokens` が `identifier|number|string` で揃うようにした（`Parser::or` が期待集合を自動で union しないため、失敗時の期待集合を明示的に構築）。
- 2025-12-18: Step2-1 実施。`examples/language-impl-samples/reml/basic_interpreter_combinator.reml` の LexPreset 置換（`identifier/number/string` の label 化、自由文 `Parse.fail` 排除）と `expected/language-impl-samples/basic_interpreter_combinator.stdout` の更新、`CH2-PARSE-501` の回帰（`language_impl_samples` スイート追加）まで完了。

## 現在の達成状況（チェックリスト）
- [x] `core-parse-lexpack-basic.reml` を追加/整備し、stdout と診断ゴールデンを固定した
- [x] `CP-WS3-001` を Phase4 マトリクスへ登録し、Phase4 スイートで緑化した
- [x] `label("identifier"|"number"|"string")` 相当のラベルが `parser.syntax.expected_tokens` に残ることを `--parse-driver` で確認できる
- [x] `basic_interpreter_combinator.reml` を LexPreset 置換し、`CH2-PARSE-501` を Phase4 マトリクスに紐付けて回帰化した（`language_impl_samples` スイートで pass）
- [ ] 既存サンプル（`sql_parser/toml_parser/yaml_parser`）の LexPreset 置換（Step2 残タスク）は未着手/未完了

## 次に着手する作業（推奨順）
1. **Step2-2: `sql_parser.reml` の置換**
   - 対象: `examples/language-impl-samples/reml/sql_parser.reml`
   - 作業: `keyword_ci` 導入（境界判定を identifier_profile と共有）、予約語拒否を `reserved(profile, set)` へ集約、RunConfig.lex の書き戻しを導入
   - 完了条件: 予約語/境界の期待表示が `Expectation::Keyword` / `label("identifier")` の語彙で統一される
2. **Step2-3: `toml_parser.reml` の置換**
   - 対象: `examples/language-impl-samples/reml/toml_parser.reml`
   - 作業: `lex_pack_toml`（`ConfigTriviaProfile::toml_relaxed` + `IdentifierProfile::toml_key`）へ切替、複数行文字列をプリセット版へ移行
   - 完了条件: トリビア/ベアキーの揺れが RunConfig.lex で再現でき、期待表示が自由文に依存しない
3. **Step2-4: `yaml_parser.reml` の最小置換**
   - 対象: `examples/language-impl-samples/reml/yaml_parser.reml`
   - 作業: Layout token 生成は維持しつつ、`space_id` 共有と `lexeme/symbol` の置換のみ先行
   - 完了条件: Layout 由来の挙動差を増やさずに LexPreset へ寄せられる
4. **回帰の拡張**
   - 作業: Step2 の各サンプルに対応する Phase4 シナリオ（必要なら新規）を `phase4-scenario-matrix.csv` に追加し、`tooling/examples/run_phase4_suite.py` 経由で緑化まで持っていく
   - 補足: `basic_interpreter_combinator.reml` は `python3 tooling/examples/run_phase4_suite.py --suite language_impl_samples` で回せる状態になっている

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
