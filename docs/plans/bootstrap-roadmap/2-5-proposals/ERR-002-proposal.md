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

### Step1 recover フックと同期トークン収集の設計（Week 32 Day3-4）
- `Core_parse.rule`／`Core_parse_stream.register_diagnostic` にフックを追加し、Menhir の `HandlingError` 到達時に `Parser_diag_state.record_recovery` を呼び出して回復状態を一元管理する。  
- `RunConfig.Recover.sync_tokens` の内容を `Parser_diag_state.recover_config` から引き出し、`Diagnostic.set_extension "recover"` で `{ "sync_tokens": [...], "strategy": Str }` を埋める変換ヘルパを設計する。  
- `Parser.MenhirInterpreter` のエラー遷移を再確認し、`checkpoint` から同期トークン候補（`;`, `}`, `end` など）を抽出する補助ロジックを PoC する。  
- 調査: `compiler/ocaml/src/parser.mly` で回復対象規則を洗い出す、`docs/spec/2-1-parser-type.md` §D（`Parse.recover` の制約）、`parser_expectation.mli` の Packrat API（同期トークン収集に利用可能なメタ情報）。

### Step2 FixIt 生成と notes 拡張（Week 32 Day5 〜 Week 33 Day1）
- `Diagnostic.Builder.add_fixits` を利用して `FixIt::Insert` / `Replace` のテンプレートを組み立て、同期トークンごとに候補を生成する。  
- `parser_diag_state.recover_notes_enabled` を参照し、`emit_notes=true` の場合に `notes` と `hints` へ `recover` 前後のコンテキスト（例: 「ここで `(` を開きました」）を追加する。  
- `type_error.ml` の FixIt 実装例を参照し、共通ヘルパ（例: `Diagnostic.Builder.insert_token`）を整備するか既存ヘルパを流用する方針を決定する。  
- 調査: `docs/spec/2-5-error.md` §D（代表エラーの FixIt パターン）、`docs/spec/3-6-core-diagnostics-audit.md` §1-§2（FixIt/Hints の必須フィールド）、`compiler/ocaml/src/diagnostic.ml` `Builder` 実装。

### Step3 CLI/LSP 出力とメトリクス整備（Week 33 Day1-3）
- `compiler/ocaml/tests/parser_recover_tests.ml`（新設）と `streaming_runner_tests.ml` を拡張し、同期トークン回復と FixIt が `ParseResult.diagnostics` に含まれることをゴールデンで検証する。  
- `scripts/validate-diagnostic-json.sh` と `tooling/ci/collect-iterator-audit-metrics.py` に `parser.recover_fixit_coverage` 指標を追加し、`reports/diagnostic-format-regression.md` へサンプル JSON を追記する。  
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に新指標を登録し、CI（Linux/macOS/Windows）で回復指標が 1.0 に到達するか確認する。
- 調査: `compiler/ocaml/tests/test_cli_diagnostics.ml`, `tooling/ci/collect-iterator-audit-metrics.py` の既存集計処理、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6（診断整合ライン）。

### Step4 ドキュメント更新とレビュー共有（Week 33 Day3-4）
- `docs/spec/2-5-error.md` / `docs/spec/3-6-core-diagnostics-audit.md` に OCaml 実装の整備状況を脚注で追記し、完了後に脚注を更新して Phase 2-7 へ周知する。  
- `docs/plans/bootstrap-roadmap/2-5-review-log.md` に実施記録を追加し、`docs/notes/core-parse-streaming-todo.md` へストリーミング経路の残課題と回復カバレッジを登録する。  
- Phase 2-7 チームへ CLI/LSP 出力差分と自動修正インターフェイスの追随点を共有し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に必要な後続タスクを記録する。  
- 調査: `docs/spec/0-1-project-purpose.md` §2.2（エラーメッセージ改善指標）、`docs/guides/ai-integration.md` FixIt 活用章、`docs/plans/bootstrap-roadmap/2-4-completion-report.md` の診断 KPI。

## 残課題
- Menhir ベースの回復戦略（同期トークン）と仕様上の `recover` API をどこまで合わせるかを Parser チームで調整する必要がある。  
- FixIt 生成の優先順位（複数候補がある場合）や CLI/LSP 表示形式を UI チームと合意したい。
