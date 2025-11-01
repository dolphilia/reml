# ERR-002 `recover` / FixIt 情報拡張計画

## 1. 背景と症状
- 仕様は `Parse.recover` が同期トークン・FixIt・notes を生成し、`ParseResult.diagnostics` に回復情報を残すと定義している（docs/spec/2-5-error.md:161-318）。  
- 現行 OCaml 実装は `Result.Error` で単一診断を返すのみで、`recover` の同期トークンや FixIt を構築していない（compiler/ocaml/src/parser_driver.ml:15-43）。`scripts/validate-diagnostic-json.sh` で期待されるフィールドも欠落する。  
- Phase 2-7 の診断タスクで CLI テキスト出力刷新を予定しているが、FixIt/notes が未整備のままだと効果を発揮できない。

## 2. Before / After
### Before
- `Parse.recover` が呼び出されず、`Diagnostic.fixits` や `Diagnostic.hints` が常に空。  
- CLI/LSP の自動修正や同期トークン解析が機能せず、仕様上の `recover` 契約を満たしていない。

### After
- `ParseResult` シム導入と併せて `recover` ポイントを定義し、Menhir のエラー回復（同期トークン）を `Diagnostic.hints` と `fixits` へ変換する。  
- `RunConfig.extensions["recover"].sync_tokens` を参照し、仕様どおり同期トークン集合を `Diagnostic.extensions["recover"]` に出力。  
- CLI/LSP ゴールデンを更新し、FixIt と notes が JSON / テキスト出力に含まれることを確認。

## 3. 影響範囲と検証
- **テスト**: 新規 `recover` テストケースを追加し、典型的な欠落記号（例: `;`）で FixIt が生成されるか確認。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `parser.recover_fixit_coverage` を追加し、CI で FixIt 生成率を監視。  
- **監査**: `reports/diagnostic-format-regression.md` に回復付き診断の JSON を追加し、`scripts/validate-diagnostic-json.sh` で必須フィールドを検証。
- **実装**: `compiler/ocaml/tests/parser_recover_tests.ml` を新設し、同期トークンと FixIt のペアが `ParseResult.diagnostics` に出力されるか Golden テストで確認する。

## 4. フォローアップ
- Phase 2-7 診断タスクと協力して、CLI/LSP のテキスト出力・自動修正インターフェイスを更新する。  
- 仕様書の脚注に「OCaml 実装は `recover`/FixIt の整備中」と明記し、実装完了時に脚注を削除。  
- `docs/guides/ai-integration.md` に FixIt 情報を活用する運用例を追記する。
- `docs/notes/core-parse-streaming-todo.md` にストリーミング解析での `recover` 活用状況と残課題を追記し、Phase 3 へ引き継ぐ。
- **タイミング**: PARSER-001/002 の反映後、Phase 2-5 中盤〜後半にかけて実装し、Phase 2-7 の CLI/LSP 刷新に先行して FixIt 支援を提供する。

## 5. 実施ステップ
### Step0 現状棚卸しと仕様突合（Week 32 Day1-2）
- `parser_driver.ml` / `core_parse_streaming.ml` / `parser_diag_state.ml` の回復フローを棚卸しし、`Parser_diag_state.record_recovery` が呼ばれていない経路を特定する。  
- `parser_expectation.ml` と既存ゴールデン（`compiler/ocaml/tests/golden/diagnostics/parser/*.json.golden`）で同期トークン集合がどこまで露出しているか確認し、欠落メタデータを洗い出す。  
- 調査: `docs/spec/2-5-error.md` §B-11 〜 §E（`recover` の API と FixIt 仕様）、`docs/plans/bootstrap-roadmap/2-5-review-log.md` 「PARSER-003 Step1/Step3」の指摘箇所、`compiler/ocaml/src/parser_run_config.ml` `Recover` モジュールの既存フィールド。

#### Step0 実施記録（Week 32 Day2 完了）
- `Parser_diag_state.record_recovery` は定義のみで呼び出しがなく、Menhir の `HandlingError` 分岐でも回復フラグを更新しないため `ParseResult.recovered` は常に既定値の `false` のままとなる（compiler/ocaml/src/parser_diag_state.ml:68, compiler/ocaml/src/parser_driver.ml:82-233）。  
- `Run_config.Recover.of_run_config` で同期トークンと `emit_notes` を取り出しているが、`Core_parse_streaming.create_session` 以降でこれらを診断へ反映する処理が存在せず、`recover_sync_tokens` / `recover_notes_enabled` も未使用である（compiler/ocaml/src/core_parse_streaming.ml:24-86, compiler/ocaml/src/parser_run_config.ml:264-291）。  
- 既存ゴールデンは RunConfig 側の `extensions.recover` を記録しているものの、診断出力には `extensions["recover"]` や FixIt が含まれず、`recovered` フラグも立っていないことを確認（compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden:1-82）。  
- 期待集合シムは Menhir の受理可能トークンを列挙するのみで、同期トークンや `recover` メタデータを収集する仕組みが無いため、仕様で求められる `Diagnostic.hints`・FixIt・notes の前提が欠落している（compiler/ocaml/src/parser_expectation.ml:384-420, docs/spec/2-5-error.md:170-335）。  
- 既存レビューでも `recover` 経路が未配線であることが指摘されており、今回の棚卸しで該当箇所が最新の実装でも解消されていないことを再確認した（docs/plans/bootstrap-roadmap/2-5-review-log.md:6-39）。

### Step1 recover フックと同期トークン収集の設計（Week 32 Day3-4）
- `Core_parse.rule`／`Core_parse_stream.register_diagnostic` にフックを追加し、Menhir の `HandlingError` 到達時に `Parser_diag_state.record_recovery` を呼び出して回復状態を一元管理する。  
- `RunConfig.Recover.sync_tokens` の内容を `Parser_diag_state.recover_config` から引き出し、`Diagnostic.set_extension "recover"` で `{ "sync_tokens": [...], "strategy": Str }` を埋める変換ヘルパを設計する。  
- `Parser.MenhirInterpreter` のエラー遷移を再確認し、`checkpoint` から同期トークン候補（`;`, `}`, `end` など）を抽出する補助ロジックを PoC する。  
- 調査: `compiler/ocaml/src/parser.mly` で回復対象規則を洗い出す、`docs/spec/2-1-parser-type.md` §D（`Parse.recover` の制約）、`parser_expectation.mli` の Packrat API（同期トークン収集に利用可能なメタ情報）。

#### Step1 実施記録（Week 32 Day4 完了）
- **エントリポイントの確定**: `parser_driver.ml:175-185` の `I.HandlingError` 分岐で Menhir チェックポイントと `Lexing.lexeme_start_p`／`lex_curr_p` を取得できるため、ここから `Core_parse_streaming.begin_recovery`（新設）を呼び出し、`Parser_diag_state.record_recovery` を同時に実行する設計とした。`Core_parse.rule` で返却される `Core_reply.Err` は単一路線なので、回復フラグの更新と拡張メタ生成をこのタイミングへ集中させる（compiler/ocaml/src/parser_driver.ml:156-233）。  
- **診断状態に保持するスナップショット**: `Parser_diag_state.t` に `mutable pending_recovery : recovery_snapshot option` を追加し、`sync_tokens`（RunConfig 設定）、`sample_tokens`（Menhir 期待集合）、`summary`（`Parser_expectation.collect` が返す `Diagnostic.expectation_summary`）、`start_pos`/`end_pos`（FixIt 用）を保存する。`record_recovery` では `recovered <- true` に加えて `pending_recovery` を差し替え、複数回復が発生した場合は最新のものだけを保持する方針とする（compiler/ocaml/src/parser_diag_state.ml:1-90, compiler/ocaml/src/parser_run_config.ml:264-291）。  
- **同期トークン整形ヘルパ**: `Parser_expectation.collect` が返す `collection.sample_tokens` から `RunConfig.Recover.sync_tokens` と一致するものを抽出し、`Diagnostic.Extensions` に `{ "sync_tokens": List<Str>, "hits": List<Str>, "strategy": "token-set", "notes": Bool }` を生成するヘルパを `Core_parse_streaming.Recovery` サブモジュールとして定義する。`sync_tokens` は `Namespace` での上書き値を優先しつつ重複を除去し、`hits` は将来の FixIt 生成で候補を優先付けするために利用する（compiler/ocaml/src/core_parse_streaming.ml:24-82, compiler/ocaml/src/parser_expectation.ml:360-420）。  
- **診断登録フック**: `Core_parse_streaming.register_diagnostic` は `pending_recovery` を消費する際に `Diagnostic.set_extension "recover"` を適用し、`emit_notes=true` の場合は `Diagnostic.add_hint`/`add_note` のテンプレートを返すところまでを共通化する。拡張適用後に `pending_recovery` を `None` へ戻すことで複数診断への二重適用を防ぎ、`merge_warnings=true` のままでも回復イベントを Error として保持する（compiler/ocaml/src/core_parse_streaming.ml:80-128）。  
- **ストリーミング経路との整合**: ストリーミング ランナーは同じ `session`／`diag_state` を共有するため、`begin_recovery` を `Core_parse_streaming.Stream` からも利用できるよう公開し、`parser.stream_extension_field_coverage` 指標で `recover` 拡張が欠落していないか監視する。`collect-iterator-audit-metrics.py` に登録済みのストリーミング項目へ `parser.recover_sync_success_rate` を統合する準備を行い、Phase 2-7 の Pending 遷移監査と連携させる（compiler/ocaml/tests/streaming_runner_tests.ml:1-124, tooling/ci/collect-iterator-audit-metrics.py:1654-1684）。  
- **フォローアップ**: Step2 で `FixIt::Insert` を生成する際には `pending_recovery` に保持した `start_pos`/`summary.alternatives` を利用し、括弧対やステートメント終端など代表的ケースを網羅する。`Diagnostic.extensions["recover"]` の JSON スキーマ定義は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の監視指標と同期させ、CLI/LSP ゴールデン更新時に `sync_tokens` と `hits` が確認できるようゴールデンファイルを追加する。

### Step2 FixIt 生成と notes 拡張（Week 32 Day5 〜 Week 33 Day1）
- `Diagnostic.Builder.add_fixits` を利用して `FixIt::Insert` / `Replace` のテンプレートを組み立て、同期トークンごとに候補を生成する。  
- `parser_diag_state.recover_notes_enabled` を参照し、`emit_notes=true` の場合に `notes` と `hints` へ `recover` 前後のコンテキスト（例: 「ここで `(` を開きました」）を追加する。  
- `type_error.ml` の FixIt 実装例を参照し、共通ヘルパ（例: `Diagnostic.Builder.insert_token`）を整備するか既存ヘルパを流用する方針を決定する。  
- 調査: `docs/spec/2-5-error.md` §D（代表エラーの FixIt パターン）、`docs/spec/3-6-core-diagnostics-audit.md` §1-§2（FixIt/Hints の必須フィールド）、`compiler/ocaml/src/diagnostic.ml` `Builder` 実装。

#### Step2 実施記録（Week 33 Day1 完了）
- **FixIt テンプレートの整理**: `Diagnostic.fixit` 列挙（`compiler/ocaml/src/diagnostic.ml:122-146`）と `Builder.add_fixits`（`compiler/ocaml/src/diagnostic.ml:1236-1241`）を確認し、`recover` 向けには `Insert` を基本、欠落トークンが既存文字に重なっている場合のみ `Replace` を生成する方針を採用した。`Builder` に軽量ヘルパ `insert_token` / `replace_token` を追加する設計を固め、`publish_recover_fixits`（仮称）で `pending_recovery.start_pos`〜`end_pos` をスパンとして利用する計画を明文化。括弧・セミコロン・`end` など代表同期トークンは `Run_config.Recover.sync_tokens`（`compiler/ocaml/src/parser_run_config.ml:264-291`）から優先順を取得し、`summary.alternatives` のうち一致したものを `hits` として扱う。
- **回復スナップショットの活用**: Step1 で整理した `Parser_diag_state.pending_recovery` に `start_pos` / `end_pos` / `summary` / `sample_tokens` を保持する前提を再確認し、FixIt 生成フェーズでは `Parser_diag_state.recover_config.emit_notes` と組み合わせて `Diagnostic.Builder.add_fixits`・`add_hint` を呼び出す流れを定義。`pending_recovery` 消費後は `Core_parse_streaming.register_diagnostic` 内で `None` に戻し、多重適用を防ぐ。
- **notes / hints の文面**: 仕様の補足文例（`docs/spec/2-5-error.md:190-238`）を下敷きに、`emit_notes=true` の場合は「同期トークン `<token>` を挿入すると構文を継続できます」といったテンプレートを `Diagnostic.Builder.add_hint`（`compiler/ocaml/src/diagnostic.ml:1243-1251`）で追加する方針を決定。notes は `Diagnostic.Builder.add_note` で一次テキストを残し、CLI テキスト出力と LSP データに同じ文章が現れるよう JSON ゴールデンの更新計画を立てた。
- **`extensions["recover"]` の更新形**: Step1 で定義した `{ sync_tokens, hits, strategy, notes }` に加え、FixIt 生成有無を示す `has_fixits: Bool` を追加し、監査ログやメトリクスから回復が成功したかを判別できるようにする。`Diagnostic.set_extension "recover"` を適用する箇所で `Yojson.Basic` の `List.map` を使い JSON 配列へ変換するコードスケッチを用意し、`scripts/validate-diagnostic-json.sh` で `has_fixits=true` のサンプルを検証する準備を完了。
- **テストとメトリクスへの布石**: Step3 以降で追加する `parser_recover_tests.ml` と CLI/LSP ゴールデンでは、欠落セミコロンと未閉背括弧の 2 ケースを最低ラインとし、FixIt/notes/`extensions["recover"]` 全てが出力されるかを確認する。メトリクスは `parser.recover_fixit_coverage` を `collect-iterator-audit-metrics.py`（`tooling/ci/collect-iterator-audit-metrics.py:1654-1684`）へ追加し、JSON ゴールデン（`compiler/ocaml/tests/golden/diagnostics/parser/`）に `has_fixits` を含む新ファイルを配置する段取りを設定した。
- **フォローアップ**: Step3 では CLI/LSP/GH Actions で FixIt 出力を確認し、`docs/spec/3-6-core-diagnostics-audit.md` 更新と `docs/notes/core-parse-streaming-todo.md` への残タスク登録を行う。`recover` 拡張の JSON スキーマ更新は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の監視指標登録と同時に進め、Phase 2-7 の CLI テキスト刷新へ連結する。

### Step3 CLI/LSP 出力とメトリクス整備（Week 33 Day1-3）
- `compiler/ocaml/tests/parser_recover_tests.ml`（新設）と `streaming_runner_tests.ml` を拡張し、同期トークン回復と FixIt が `ParseResult.diagnostics` に含まれることをゴールデンで検証する。  
- `scripts/validate-diagnostic-json.sh` と `tooling/ci/collect-iterator-audit-metrics.py` に `parser.recover_fixit_coverage` 指標を追加し、`reports/diagnostic-format-regression.md` へサンプル JSON を追記する。  
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に新指標を登録し、CI（Linux/macOS/Windows）で回復指標が 1.0 に到達するか確認する。
- 調査: `compiler/ocaml/tests/test_cli_diagnostics.ml`, `tooling/ci/collect-iterator-audit-metrics.py` の既存集計処理、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6（診断整合ライン）。

#### Step3 実施記録（Week 33 Day3 完了）
- **CLI/LSP ゴールデン更新と新規テスト**: `compiler/ocaml/tests/parser_recover_tests.ml` を追加し、欠落セミコロンと未閉括弧の 2 ケースで `extensions["recover"].sync_tokens`・`hits`・`has_fixits` を検証するゴールデン（`compiler/ocaml/tests/golden/diagnostics/parser/recover-missing-semicolon.json.golden` / `compiler/ocaml/tests/golden/diagnostics/parser/recover-unclosed-block.json.golden`）を作成。`streaming_runner_tests.ml` / `test_cli_diagnostics.ml` 側でも同じ入力を共有して CLI/LSP 出力が FixIt と notes を表示することを確認した。  
- **スクリプトとメトリクスの配線**: `scripts/validate-diagnostic-json.sh` に `recover` 拡張のスキーマ検査（`sync_tokens` / `hits` / `strategy` / `has_fixits` / `notes`）を追加し、`tooling/ci/collect-iterator-audit-metrics.py` へ `parser.recover_fixit_coverage` を組み込んで CI 成功条件へ設定。Linux/macOS/Windows の nightly ビルドで 1.0 を記録し、欠落時は `--require-success` で失敗することを確認。  
- **監査サンプルとドキュメント整備**: `reports/diagnostic-format-regression.md` に回復付き診断の JSON 例を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標表へ `parser.recover_fixit_coverage` を追加。さらに `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6.2 に CLI/LSP 経路の整合確認結果を反映し、Phase 2-7 の監査タスクに参照先を共有した。  
- **フォローアップ整理**: `docs/plans/bootstrap-roadmap/2-5-review-log.md` へ Step3 の調査ログと CI チェックリストを追記し、Phase 2-7 へ引き継ぐ残課題として Packrat 経路の FixIt カバレッジと `recover` notes の翻訳整備を登録。`docs/plans/bootstrap-roadmap/2-5-proposals/README.md` でも Step3 完了を報告し、関連タスクの依存関係を更新した。

### Step4 ドキュメント更新とレビュー共有（Week 33 Day3-4）
- `docs/spec/2-5-error.md` / `docs/spec/3-6-core-diagnostics-audit.md` に OCaml 実装の整備状況を脚注で追記し、完了後に脚注を更新して Phase 2-7 へ周知する。  
- `docs/plans/bootstrap-roadmap/2-5-review-log.md` に実施記録を追加し、`docs/notes/core-parse-streaming-todo.md` へストリーミング経路の残課題と回復カバレッジを登録する。  
- Phase 2-7 チームへ CLI/LSP 出力差分と自動修正インターフェイスの追随点を共有し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に必要な後続タスクを記録する。  
- 調査: `docs/spec/0-1-project-purpose.md` §2.2（エラーメッセージ改善指標）、`docs/guides/ai-integration.md` FixIt 活用章、`docs/plans/bootstrap-roadmap/2-4-completion-report.md` の診断 KPI。

#### Step4 実施記録（Week 33 Day4 完了）
- **仕様脚注の更新**: `docs/spec/2-5-error.md` と `docs/spec/3-6-core-diagnostics-audit.md` に ERR-002 Step3/Step4 の整備状況を示す脚注を追加し、`parser.recover_fixit_coverage = 1.0` を維持していることを明記した。  
- **レビュー記録と TODO 整理**: `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Step4 の共有ログを追加し、`docs/notes/core-parse-streaming-todo.md` を更新して Packrat 経路・notes ローカライズ・ストリーミング重複検証の継続タスクを Phase 2-7 へ引き継いだ。  
- **後続フェーズへの登録**: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §3.4 に Recover FixIt 継続整備を追記し、Packrat スナップショット整備と CLI/LSP ローカライズ対応を Phase 2-7 の作業ブレークダウンへ正式登録した。

## 残課題
- Menhir ベースの回復戦略（同期トークン）と仕様上の `recover` API をどこまで合わせるかを Parser チームで調整する必要がある。  
- FixIt 生成の優先順位（複数候補がある場合）や CLI/LSP 表示形式を UI チームと合意したい。
