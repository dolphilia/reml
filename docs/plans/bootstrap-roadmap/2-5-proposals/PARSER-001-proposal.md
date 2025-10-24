# PARSER-001 `ParseResult` シム構築計画

## 1. 背景と症状
- 仕様は `Parser<T>` が `Reply{consumed, committed}` を返し、ランナーが `ParseResult` に診断・回復情報を集約すると定義する（docs/spec/2-1-parser-type.md:11-37）。  
- 現行 OCaml 実装は Menhir 生成コードを直接呼び出し、`Result.t`（AST または `Diagnostic` 1 件）を返している（compiler/ocaml/src/parser_driver.ml:15-43）。`DiagState`・`consumed`・`committed`・最遠エラー統計が保持されず、仕様通りの回復戦略や `recover` 診断を検証できない。  
- `Core.Parse` API と CLI/LSP の整合を確認するためのメトリクス（`parser_driver.farthest_error_offset` など）が欠落し、Phase 3 の self-host パーサ移植で互換性を評価できない。

## 2. Before / After
### Before
- Menhir のチェックポイントを即座に評価し、成功時は AST、失敗時は単一 `Diagnostic` を返却。  
- 期待集合・消費フラグ・コミットフラグを収集する仕組みが無く、`Parser<T>` / `Reply<T>` に相当するデータ構造が存在しない。

### After
- Menhir ドライバを `Core.Parse` 風のシムで包み、`State`, `Reply`, `ParseResult` を OCaml 実装に導入する。  
- `DiagState` に最遠エラー位置・期待集合・回復履歴を蓄積し、`ParseResult.diagnostics` に複数診断を連ねる。  
- ランナーは `require_eof` や `extensions["recover"]` に従って追加診断を生成し、仕様の `run`/`run_partial` 契約（docs/spec/2-6-execution-strategy.md:10-19）を満たす。

#### シム構造案
```ocaml
type state = {
  lexbuf : Lexing.lexbuf;
  diag : diag_state;
  consumed : bool;
  committed : bool;
}

type 'a reply =
  | Ok of { value : 'a; span : Span.t; consumed : bool }
  | Err of { error : Diagnostic.t; consumed : bool; committed : bool }
```
Menhir の `checkpoint` から `reply` を構築し、`ParseResult` へ畳み込む。

## 3. 影響範囲と検証
- **ユニットテスト**: `parser_driver_tests.ml` を追加し、`or` 分岐・`cut`・`recover` の消費/コミット挙動を検証。  
- **診断比較**: `reports/diagnostic-format-regression.md` に `ParseResult.diagnostics` の多件数パターンを追加し、`scripts/validate-diagnostic-json.sh` で仕様通りのフィールドが出力されることを確認。  
- **メトリクス**: `0-3-audit-and-metrics.md` に `parser.parse_result_consistency` と `parser.farthest_error_offset` を新設し、`run` と `run_partial` の結果一致・最遠エラー更新回数を CI で監視する。Phase 2-4→2-5 ハンドオーバーで指示された `diagnostic_schema.validation_pass` と連動させ、`reports/diagnostic-format-regression.md` の自動比較結果も記録する。
- **実装整合**: `compiler/ocaml/tests/parse_result_state_tests.ml` を追加し、`DiagState` が最遠エラーとコミット情報を保持するか、シナリオ型のスナップショットテストで検証する。

## 4. フォローアップ
- `docs/spec/2-1-parser-type.md` に OCaml 実装の移行段階（シム→純粋実装）を明記し、Phase 3 の self-host パーサ設計ノートへリンクを追加。  
- `Parser<T>` シム導入後、`PARSER-002`（RunConfig）・`ERR-001`（期待集合）と連動したリファクタリングを続行する。  
- CLI/LSP の JSON 出力で `ParseResult.recovered` を利用できるよう、`tooling/lsp/diagnostic_transport.ml` を更新する。
- **タイミング**: Phase 2-5 の開幕直後に最優先で導入し、同フェーズ中盤の RunConfig／期待集合整備が始まる前にシム化を完了する。

## 5. 実施ステップ（Week31）
1. **設計・ドキュメント更新**  
   - Week31 Day1-2 で `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6.2、および本計画書の型定義・メトリクス記述を最新化し、`DiagState` のフィールド一覧（`farthest_offset`, `expected_tokens`, `committed`, `recovered_spans` 等）を明文化する。  
   - `docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` に移行脚注を追加し、Phase 2-4→2-5 ハンドオーバー（`docs/plans/bootstrap-roadmap/2-4-to-2-5-handover.md`）で指定された診断レビュー手順をリンクする。
2. **Menhir シム実装**  
   - Week31 Day3 までに `compiler/ocaml/src/parser_driver.ml` へ `State`/`Reply` 抽象を導入し、`Parser.MenhirInterpreter.accept`/`offer` を監視して `consumed`/`committed` を更新する。  
   - `DiagState` を `compiler/ocaml/src/parser_diag_state.ml`（新規）へ切り出し、最遠エラー／期待集合の更新を `ERR-001` が再利用できる API として提供する。
3. **ランナー統合とメトリクス計測**  
   - Week31 Day4 に `parser_driver.run` / `run_partial` を `ParseResult` ベースへ置換し、`reports/diagnostic-format-regression.md` で比較するサンプルを更新する。  
   - `0-3-audit-and-metrics.md` の `parser.parse_result_consistency` / `parser.farthest_error_offset` エントリへしきい値（>=0.99、一致失敗時は CI fail）を記入し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` へ実施記録を残す。
4. **検証と共有**  
   - Week31 Day5 に `scripts/validate-diagnostic-json.sh`、`tooling/lsp/tests/client_compat/validate-diagnostic-json.mjs` を実行し、複数診断と `ParseResult.recovered` が JSON に出力されることを証明する。  
   - 成果を Phase 2-5 週次レビューへ提出し、`PARSER-002`・`ERR-001` チームへ API 変更点をフィードバックする。

## 6. 依存関係と連携
- **TYPE-003**: `ParseResult` で `effect_capabilities` を集計できるよう、辞書復元フロー（`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-003-proposal.md`）とメトリクス記録を同期させる。シム導入時に IR への影響範囲を共有し、型クラス辞書の `committed` 境界を壊さない。
- **DIAG-002**: `Diagnostic.audit` / `timestamp` 必須化に合わせ、`ParseResult.diagnostics` へ格納するすべての診断で `AuditEnvelope` を補完する処理を `parser_driver` レイヤに追加する。
- **ERR-001 / EXEC-001**: 期待集合とストリーミングランナーの PoC が `DiagState` を共有できるよう API を公開し、`docs/guides/core-parse-streaming.md` へサンプルを追記する。
- **技術的負債 ID 22/23**: Windows/macOS 監査ゲート（`compiler/ocaml/docs/technical-debt.md`）に影響するため、`ParseResult` の追加フィールドを LSP/CLI 双方で検証し、各プラットフォームの CI ログに `parser.parse_result_consistency` を表示する。

## 7. 残課題
- Menhir 生成コードに手を入れずに `consumed` / `committed` 情報を取得する手段（`Parser.MenhirInterpreter` の API 露出）が十分か要確認。  
- `ParseResult.span` を構築するためのスパン情報をどのレイヤで取得するか（AST ノード vs. トークン）を決める必要がある。

## 8. 進捗状況（2025-10-25）
- `Step 1`〜`Step 4` を完了し、`parser_driver.ml` のシム化・`parser_diag_state.ml` の分離・`run_string`/`run_partial` の公開を実装。`test_parser_driver.ml` と `test_parse_result_state.ml` を追加し、`dune runtest tests` で成功を確認。
- `ParseResult.diagnostics` へ複数診断を蓄積できる状態となり、`farthest_error_offset` を `DiagState` から参照可能。`legacy_result` ブリッジ経由で従来の `parse` API も維持。
- 追加対応（2025-10-25 後半）: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に parser 系メトリクスを追加し、`docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` に Phase 2-5 の移行脚注を追記。`scripts/validate-diagnostic-json.sh` でも `parse_result.recovered` 欠落を検知する検証を自動化済み。
