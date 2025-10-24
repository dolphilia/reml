# PARSER-001 `ParseResult` シム構築提案

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
- **メトリクス**: `0-3-audit-and-metrics.md` に `parser.parse_result_consistency` を新設し、`run` と `run_partial` の結果一致を CI で保証。
- **実装整合**: `compiler/ocaml/tests/parse_result_state_tests.ml` を追加し、`DiagState` が最遠エラーとコミット情報を保持するか、シナリオ型のスナップショットテストで検証する。

## 4. フォローアップ
- `docs/spec/2-1-parser-type.md` に OCaml 実装の移行段階（シム→純粋実装）を明記し、Phase 3 の self-host パーサ設計ノートへリンクを追加。  
- `Parser<T>` シム導入後、`PARSER-002`（RunConfig）・`ERR-001`（期待集合）と連動したリファクタリングを続行する。  
- CLI/LSP の JSON 出力で `ParseResult.recovered` を利用できるよう、`tooling/lsp/diagnostic_transport.ml` を更新する。

## 確認事項
- Menhir 生成コードに手を入れずに `consumed` / `committed` 情報を取得する手段（`Parser.MenhirInterpreter` の API 露出）が十分か要確認。  
- `ParseResult.span` を構築するためのスパン情報をどのレイヤで取得するか（AST ノード vs. トークン）を決める必要がある。
