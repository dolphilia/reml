# Phase4: CST サポートとロスレスパース計画（Core.Parse.Cst）

## 背景と決定事項
- `docs/notes/dsl/dsl-enhancement-proposal.md` の提案「3.3 CST Support & Lossless Parsing」を Phase 4 の実装・回帰計画へ具体化する。
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` の `autoWhitespace` 完了を前提に、空白・コメント（Trivia）を CST へ保持する。
- `docs/spec/0-1-project-purpose.md` の性能・安全性・診断明瞭性を維持し、既存 AST パスを破壊しない形で opt-in を提供する。

## 目的
1. CST ノードと Trivia 収集の最小仕様を定義し、Core.Parse にロスレスパース経路を追加する。
2. `Core.Text.Pretty` と連携し、CST から `Doc` への標準プリンタ導出経路を整備する。
3. Phase 4 のシナリオマトリクスに CST/Formatter の回帰シナリオを登録し、運用可能な検証基盤を作る。

## スコープ
- **含む**: CST ノード構造、Trivia 付着ルール、`run_with_cst` などの新規 API、Rust 実装追加、最小のプリンタ導出、サンプルと回帰登録、仕様・ガイド更新。
- **含まない**: すべての構文の完全ロスレス復元、LSP/Refactor の自動更新、ストリーミング解析の CST 対応（必要に応じて Phase 5 へ繰り越し）。

## 成果物
- 仕様追記: `docs/spec/2-2-core-combinator.md`, `docs/spec/2-0-parser-api-overview.md`
- Pretty 連携: `docs/spec/3-13-core-text-pretty.md`, `docs/guides/dsl/formatter-authoring.md`
- 実装メモ: `docs/notes/parser/core-parse-cst-design.md`（CST 付着ルールと実装判断の根拠）
- 回帰資産: `examples/`, `expected/`, `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`

## 仕様ドラフト（最小構成）

### CST ノードと Trivia
- `CstNode { kind, children, trivia_leading, trivia_trailing, span }`（`span` は必須。合成ノードは空Spanで表現する）
- `Trivia { kind, text, span }`（`kind`: `Whitespace | Comment | Layout`）
- `CstChild = Node(CstNode) | Token(Token)` とし、`Token` は Lex/Parse の既存トークン表現に従う。

### ロスレスパース API（確定）
```reml
use Core.Parse

let result = run_with_cst(my_parser, input, config)
match result.value {
  Some(output) => output.cst
  None => Cst.empty
}
```

- **命名**: `run_with_cst` に固定する（`run` / `run_with_recovery` 系と並列の位置付け）。
- **戻り値**: `run_with_cst` は `ParseResult<CstOutput<T>>` を返す。
- **CstOutput 形状**: `CstOutput { ast: T, cst: CstNode }` に固定する。追加メタデータは `CstNode.span` と `ParseResult` 側の `diagnostics` に集約する。
- **共有入力**: `run_shared` と同様の用途があるため、`run_with_cst_shared` を追加する（`Arc<str>` を受け取り、余計なコピーを避ける）。
- Trivia 収集は `autoWhitespace` が消費したトークンを入力とし、`RunConfig` で opt-in（デフォルト OFF）。

### Trivia 付着ルール（初期案）
- 先頭 Trivia は `trivia_leading` に付着。
- `autoWhitespace` で消費した Trivia は直近の「確定ノード」の `trivia_trailing` に付着し、次ノード生成時に先頭へ移送する（改行/コメントで区切る）。
- `Layout` トークンは `Trivia.kind=Layout` として同じルールで扱う。

### Printer 導出
- `CstNode -> Doc` の標準プリンタ（`CstPrinter`）を `Core.Text.Pretty` で提供。
- 既存フォーマッタは `CstOutput` を opt-in で利用し、`Ast` のみ利用する経路は維持する。

## 作業ステップ

### フェーズA: 仕様整理と用語統一
1. `docs/spec/2-2-core-combinator.md` に CST/Trivia の基本構造、`run_with_cst` の契約を追記する。
2. `docs/spec/2-0-parser-api-overview.md` に `RunConfig.extensions["parse"].cst` の設定方針（opt-in、デフォルト OFF）を追加する。
3. `docs/notes/parser/core-parse-cst-design.md` に付着ルールと `autoWhitespace` 連携の判断根拠を記録する。

### フェーズB: Rust 実装の追加
1. `compiler/runtime/src/parse/cst.rs` を新設し、`CstNode` / `CstChild` / `Trivia` / `CstOutput` を定義する。
2. `compiler/runtime/src/parse/combinator.rs` に CST 収集フラグを追加し、`ParseState` で Trivia バッファを保持する。
3. `run_with_cst` を追加し、`ParseResult<CstOutput<T>>` を返す経路を `run` 系に並列で用意する。
4. `autoWhitespace` 経由で消費された Trivia を収集し、確定ノードに付着する最小ロジックを追加する。

### フェーズC: Core.Text.Pretty 連携
1. `compiler/runtime/src/text/pretty.rs` に `CstPrinter`（標準プリンタ）を追加する。
2. `Doc` 生成時に Trivia を保持する既定スタイル（空白/改行/コメントの再現）を定義する。
3. `docs/spec/3-13-core-text-pretty.md` に `CstPrinter` と入力前提（CST 形状）を追記する。

### フェーズD: サンプルと回帰接続
1. `examples/practical/` に CST/Formatter の最小サンプルを追加する（`autoWhitespace` の挙動が分かる入力）。
2. `expected/` に CST から生成した Doc をゴールデン化して保存する。
3. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に CST シナリオを登録する（例: `CH2-PARSE-930`）。

### フェーズE: ドキュメントと運用整理
1. `docs/guides/dsl/formatter-authoring.md` に CST 利用時の注意点（Trivia 付着/空白戦略）を追加する。
2. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に CST の回帰条件を追記する。
3. `reports/spec-audit/ch5/logs/` に実行ログテンプレートを追加し、更新方針を明記する。

## 進捗チェックリスト

### フェーズA: 仕様整理と用語統一
- [x] `docs/spec/2-2-core-combinator.md` に CST/Trivia と `run_with_cst` 契約を追記した
- [x] `docs/spec/2-0-parser-api-overview.md` に `RunConfig.extensions["parse"].cst` 方針を追記した
- [x] `docs/notes/parser/core-parse-cst-design.md` に付着ルールと判断根拠を記録した

### フェーズB: Rust 実装の追加
- [x] `compiler/runtime/src/parse/cst.rs` を追加し CST 型を定義した
- [x] `ParseState` に CST 収集フラグと Trivia バッファを追加した
- [x] `run_with_cst` / `run_with_cst_shared` を追加した
- [x] `autoWhitespace` 由来の Trivia 収集を実装した

### フェーズC: Core.Text.Pretty 連携
- [x] `compiler/runtime/src/text/pretty.rs` に `CstPrinter` を追加した
- [x] `CstPrinter` の既定スタイル（空白/改行/コメント）を定義した
- [x] `docs/spec/3-13-core-text-pretty.md` に `CstPrinter` を追記した

### フェーズD: サンプルと回帰接続
- [x] `examples/practical/` に CST/Formatter サンプルを追加した
- [x] `expected/` に Doc 出力のゴールデンを保存した
- [x] `phase4-scenario-matrix.csv` に CST シナリオを登録した

### フェーズE: ドキュメントと運用整理
- [x] `docs/guides/dsl/formatter-authoring.md` に CST 利用注意点を追記した
- [x] `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に回帰条件を追記した
- [x] `reports/spec-audit/ch5/logs/` に実行ログテンプレートを追加した

## Rust 実装の現状と追加案

### 既存実装の範囲
- `compiler/runtime/src/parse/` は `Parser<T>` と `ParseResult<T>` を中心に AST 指向のパースを提供。
- `autoWhitespace` は `Core.Parse` の拡張として計画済みであり、トリビア収集は未実装。

### 追加 API（Rust 側 / 確定）
- `CstOutput<T> { ast: T, cst: CstNode }` を新設（`CstNode` に span/trivia を保持）。
- `run_with_cst(parser, input, config) -> ParseResult<CstOutput<T>>` を追加（既存 `run` 系の互換は維持）。
- `run_with_cst_shared(parser, input: Arc<str>, config) -> ParseResult<CstOutput<T>>` を追加（`run_shared` と同等の位置付け）。
- `ParseState` に `cst_mode` と `trivia_buffer` を追加し、`autoWhitespace` が収集したトークンをバッファへ蓄積。
- `CstBuilder` を `combinator.rs` もしくは `cst.rs` に配置し、ノード確定タイミングで Trivia を付着。

### モジュール分割案
- `parse/mod.rs`: `cst` モジュールの公開と `run_with_cst` / `run_with_cst_shared` の再公開。
- `parse/cst.rs`: CST 型、付着ルール、`CstBuilder` を実装。
- `text/pretty.rs`: `CstPrinter` を実装し、`Doc` 生成の標準経路を提供。

## 依存関係
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` の `autoWhitespace` 実装完了が前提。
- `docs/spec/3-13-core-text-pretty.md` と `docs/guides/dsl/formatter-authoring.md` の仕様整合。
- `docs/plans/bootstrap-roadmap/4-1-core-parse-lex-helpers-impl-plan.md` の Lex プロファイル定義。

## リスクと緩和策
| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| Trivia 付着ルールの曖昧化 | フォーマッタが揺れる | 付着ルールを `docs/notes/parser/core-parse-cst-design.md` に固定し、変更時はログを残す |
| CST 収集の性能劣化 | パース性能低下 | `RunConfig` で opt-in とし、既定は OFF を維持 |
| Formatter との二重責務 | AST/CST 両方の導線が複雑化 | `CstOutput` の標準経路を明記し、AST-only ルートは互換維持 |

## 参照
- `docs/notes/dsl/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/2-2-core-combinator.md`
- `docs/spec/2-0-parser-api-overview.md`
- `docs/spec/3-13-core-text-pretty.md`
- `docs/guides/dsl/formatter-authoring.md`
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md`
