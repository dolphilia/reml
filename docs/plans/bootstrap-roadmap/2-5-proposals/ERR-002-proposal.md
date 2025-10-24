# ERR-002 `recover` / FixIt 情報拡張提案

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

## 確認事項
- Menhir ベースの回復戦略（同期トークン）と仕様上の `recover` API をどこまで合わせるかを Parser チームで調整する必要がある。  
- FixIt 生成の優先順位（複数候補がある場合）や CLI/LSP 表示形式を UI チームと合意したい。
