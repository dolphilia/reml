# Phase4: DSL Composability Standard 計画

## 背景と決定事項
- `docs/notes/dsl/dsl-enhancement-proposal.md` の提案「3.6 DSL Composability Standard」を Phase 4 の実装計画へ落とし込む。
- `conductor` を DSL 協働の司令塔として扱い、解析・実行・診断・LSP を一貫した契約で束ねる（[docs/spec/1-1-syntax.md](../../spec/1-1-syntax.md) B.8.3）。
- ストリーミング時のバックプレッシャ協調は [docs/guides/compiler/core-parse-streaming.md](../../guides/core-parse-streaming.md) 4.1 を準拠し、親子 DSL で同一ポリシーを共有する。

## 目的
1. 埋め込み DSL の共通インターフェース（境界トークン、復帰位置、回復規約、診断境界）を仕様化する。
2. `conductor` と Capability/Effect 契約、監査ログ、LSP 委譲を統合し、実行/診断/可観測性を一貫させる。
3. Rust 実装へ `embedded_dsl` 経路を追加し、Phase 4 の回帰シナリオへ接続する。

## スコープ
- **含む**: `embedded_dsl` 仕様、コンテキスト継承、診断境界、`conductor` 連携（execution/resource limits）、LSP 委譲の最小規約。
- **含まない**: フル LSP サーバー統合、複数 DSL の自動判別（Speculative Parsing の高度化）、外部プラグイン配布。

## 成果物
- `docs/spec/1-1-syntax.md` B.8.3 に `embedded_dsl` 契約と `conductor` の DSL 合成ルールを追記。
- `docs/spec/2-2-core-combinator.md` に `embedded_dsl` の API 仕様と `EmbeddedMode`/`ContextBridge` を追記。
- `docs/spec/3-6-core-diagnostics-audit.md` に `Diagnostic.source_dsl`/`AuditEnvelope.metadata["dsl.id"]` の標準キーを追記。
- `docs/guides/dsl/conductor-pattern.md` に埋め込み DSL の運用パターン（境界/回復/委譲）を追記。
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に composability シナリオを追加。
- Rust 実装（`compiler/runtime/src/parse/embedded.rs` など）と回帰サンプル。

## 仕様ドラフト（最小構成）

```reml
let code_block =
  embedded_dsl(
    dsl_id = "reml",
    start = "```reml",
    end = "```",
    parser = Reml.Parser.main,
    lsp = Reml.Lsp.server,
    mode = EmbeddedMode::ParallelSafe,
    context = ContextBridge::inherit(["scope", "type_env"])
  )

conductor docs_pipeline {
  markdown: Markdown.Parser.main
    |> with_embedded([code_block])
    |> with_capabilities(["core.parse", "core.lsp"])

  execution {
    parallel markdown
  }
}
```

### 最小契約（案）
- `embedded_dsl` は `dsl_id` と境界（`start`/`end`）を必須とし、診断で `Diagnostic.source_dsl` を必ず付与する。
- 親 DSL は `ContextBridge` を通じて `scope`/`type_env`/`config` の一部を継承できる。
- `EmbeddedMode::ParallelSafe` が有効な場合、親 DSL は並列実行の可否を `ExecutionPlan` に反映する。
- `embedded_dsl` が CST を返した場合は Trivia を保持して親 DSL の CST に統合する（CST は `4-1-core-parse-cst-plan.md` を参照）。

## 作業ステップ

### フェーズA: 仕様整理
1. `docs/spec/1-1-syntax.md` B.8.3 に `embedded_dsl`/`with_embedded` の契約、`dsl_id` の必須性、診断境界を追記する。
2. `docs/spec/2-2-core-combinator.md` に `embedded_dsl` の型シグネチャと `EmbeddedMode`/`ContextBridge` の定義を追加する。
3. `docs/spec/3-6-core-diagnostics-audit.md` に `Diagnostic.source_dsl` と `AuditEnvelope.metadata["dsl.id"]` を標準キーとして追記する。
4. `docs/guides/dsl/conductor-pattern.md` に「Markdown + Reml の埋め込み」例と運用チェックリスト（境界・復帰・回復）を追加する。

### フェーズB: 実行/委譲契約の整理
1. `docs/spec/3-8-core-runtime-capability.md` の `verify_conductor_contract` と `dsl_id` の対応ルールを整理する。
2. `docs/spec/3-9-core-async-ffi-unsafe.md` の `ExecutionPlan` に `EmbeddedMode` の反映ルール（並列/直列/優先度）を追記する。
3. `docs/guides/compiler/core-parse-streaming.md` 4.1 と整合するバックプレッシャ共有ルールを明記する。

### フェーズC: Rust 実装追加
1. `compiler/runtime/src/parse/embedded.rs` を追加し、基礎型を定義する。
   - `pub struct EmbeddedDslSpec<T> { dsl_id: String, boundary: EmbeddedBoundary, parser: Parser<T>, lsp: Option<LspServer>, mode: EmbeddedMode, context: ContextBridge }`
   - `pub struct EmbeddedBoundary { start: String, end: String }`
   - `pub enum EmbeddedMode { ParallelSafe, SequentialOnly, Exclusive }`
   - `pub enum ContextBridge { Inherit(Vec<String>), Custom(ContextBridgeHandler) }`
   - `pub struct EmbeddedNode<T> { dsl_id: String, span: Span, ast: T, cst: Option<CstNode>, diagnostics: Vec<ParseError> }`
2. `compiler/runtime/src/parse/mod.rs` に `pub mod embedded;` を追加し、`EmbeddedDslSpec`/`EmbeddedMode`/`ContextBridge`/`EmbeddedNode` を re-export する。
3. `compiler/runtime/src/parse/combinator.rs` に `pub fn embedded_dsl<T>(spec: EmbeddedDslSpec<T>) -> Parser<EmbeddedNode<T>>` を追加する。
   - 境界検出: `EmbeddedBoundary::match_start` / `match_end` を用意し、`Input` のサブスライスを抽出する。
   - 実行: `run_embedded_parser(spec, slice, state)` を追加し、子パーサの `ParseResult` を `EmbeddedNode` に格納する。
4. `ParseState` に DSL スコープ情報を追加する。
   - 追加フィールド: `dsl_stack: Vec<String>`, `context_bridge: Option<ContextBridge>`
   - 追加メソッド: `enter_dsl(dsl_id: &str)`, `exit_dsl()`, `current_dsl_id() -> Option<&str>`
5. `ParseError` に `source_dsl: Option<String>` を追加し、`ParseError::with_source_dsl` を実装する。
   - `ParseState::push_diagnostic` で `current_dsl_id` を自動付与する。
6. `compiler/runtime/src/diagnostics/dsl.rs` を追加し、`apply_dsl_metadata(diag: &mut Diagnostic, dsl_id: &str, parent_id: Option<&str>, span: Span)` を実装する。
   - `AuditEnvelope.metadata["dsl.id"]` / `["dsl.parent_id"]` / `["dsl.embedding.span"]` を共通キーとして埋め込む。
7. `compiler/runtime/src/lsp/embedded.rs` を追加し、`EmbeddedLspRoute` と `EmbeddedLspRegistry` を実装する。
   - `register_route(span, dsl_id, server)` と `resolve_route(position)` を用意する。
8. `compiler/runtime/src/parse/embedded.rs` と `compiler/runtime/src/lsp/embedded.rs` を `compiler/runtime/src/lsp/mod.rs` に接続し、`Core.Lsp` 側の委譲情報として取得できるようにする。
9. `compiler/frontend/src/output/cli.rs` に composability 監査ログ出力を追加する。
   - `CliDiagnosticEnvelope.summary.dsl_embeddings` に `dsl_id`/`span`/`mode` を出力する。
10. `compiler/runtime/src/parse/embedded.rs` の単体テスト（境界検出・空入力・`end` 欠落）を `compiler/runtime/tests/parse_embedded.rs` に追加する。

### フェーズD: サンプル/回帰接続
1. `examples/practical/embedded_dsl/` に Markdown + Reml の複合サンプルを追加する。
2. `expected/practical/embedded_dsl/` に診断・CST 出力の期待値を追加する。
3. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CH4-DSL-COMP-*` シナリオを登録する。
4. `reports/spec-audit/ch5/logs/` に実行ログテンプレートを追加する。

#### 期待出力テンプレート（命名規則）

- `expected/practical/embedded_dsl/{case_id}.audit.jsonl`
- `expected/practical/embedded_dsl/{case_id}.diagnostic.json`

例:
- `expected/practical/embedded_dsl/markdown_reml_basic.audit.jsonl`
- `expected/practical/embedded_dsl/markdown_reml_error.diagnostic.json`

## 進捗チェックリスト

### フェーズA: 仕様整理
- [x] `docs/spec/1-1-syntax.md` に `embedded_dsl`/`with_embedded` の契約と診断境界を追記
- [x] `docs/spec/2-2-core-combinator.md` に `embedded_dsl` API と `EmbeddedMode`/`ContextBridge` を追記
- [x] `docs/spec/3-6-core-diagnostics-audit.md` に `Diagnostic.source_dsl` と `dsl.*` 監査キーを追記
- [x] `docs/guides/dsl/conductor-pattern.md` に埋め込み DSL の運用例を追記

### フェーズB: 実行/委譲契約の整理
- [x] `docs/spec/3-8-core-runtime-capability.md` に `dsl_id` 連携ルールを追記
- [x] `docs/spec/3-9-core-async-ffi-unsafe.md` に `EmbeddedMode` と `ExecutionPlan` の対応を追記
- [x] `docs/guides/compiler/core-parse-streaming.md` にバックプレッシャ共有ルールを追記

### フェーズC: Rust 実装追加
- [x] `compiler/runtime/src/parse/embedded.rs` を追加し `EmbeddedDslSpec` などを定義
- [x] `compiler/runtime/src/parse/mod.rs` に `embedded` を追加して公開
- [x] `compiler/runtime/src/parse/combinator.rs` に `embedded_dsl` を実装
- [x] `ParseState` に `dsl_stack`/`context_bridge` を追加
- [x] `ParseError` に `source_dsl` を追加し自動付与
- [x] `compiler/runtime/src/diagnostics/dsl.rs` を追加し `dsl.*` を監査へ反映
- [x] `compiler/runtime/src/lsp/embedded.rs` を追加し委譲ルートを管理
- [x] `compiler/runtime/src/lsp/mod.rs` へ埋め込み LSP を接続
- [x] `compiler/frontend/src/output/cli.rs` に composability 監査ログを出力
- [x] `compiler/runtime/tests/parse_embedded.rs` に単体テストを追加

### フェーズD: サンプル/回帰接続
- [x] `examples/practical/embedded_dsl/` に複合サンプルを追加
- [x] `expected/practical/embedded_dsl/` に期待出力を追加
- [x] `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CH4-DSL-COMP-*` を登録
- [x] `reports/spec-audit/ch5/logs/` に実行ログテンプレートを追加

## Rust 実装の現状と追加案

### 既存実装の範囲
- `compiler/runtime/src/parse/` に `combinator.rs` と `cst.rs` が存在するが、埋め込み DSL を扱う共通インターフェースは未定義。
- `Diagnostic` には `source_dsl` の格納規約があるが、埋め込み DSL の境界情報はまだ接続されていない。

### 追加 API 案（Rust 側）
- `embedded_dsl(spec: EmbeddedDslSpec) -> Parser<EmbeddedNode>`
- `EmbeddedDslSpec { dsl_id, start, end, parser, lsp, mode, context }`
- `EmbeddedNode { dsl_id, span, ast, cst, diagnostics }`
- `EmbeddedMode::{ParallelSafe, SequentialOnly, Exclusive}`
- `ContextBridge::{inherit, custom}`（`scope`/`type_env`/`config` の受け渡し）

### モジュール分割案
- `compiler/runtime/src/parse/embedded.rs`: `EmbeddedDslSpec` と境界検出ロジック。
- `compiler/runtime/src/parse/combinator.rs`: `embedded_dsl` コンビネーターの公開 API。
- `compiler/runtime/src/diagnostics/dsl.rs`: `Diagnostic.source_dsl` の付与と `AuditEnvelope` の補助。
- `compiler/runtime/src/lsp/embedded.rs`: Span から LSP 委譲先を引くルーティングモデル。

## 依存関係
- `docs/plans/bootstrap-roadmap/4-1-core-parse-cst-plan.md`（CST/Trivia 連携）
- `docs/plans/bootstrap-roadmap/4-1-core-lsp-derive-plan.md`（LSP 導出の最小モデル）
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md`（Core.Lsp/Diagnostics 実装）

## リスクと緩和策
| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| 境界検出の揺れ | 誤パース/誤診断が増加 | `start`/`end` を厳密化し、`EmbeddedMode` の既定を `SequentialOnly` にする |
| LSP 委譲の不一致 | 補完/診断が誤表示 | Span → LSP ルートの変換を `EmbeddedLspRoute` に集約し、CLI で監査ログ化する |
| 並列実行の競合 | 監査/診断の順序が不安定 | `ExecutionPlan` に優先度と `resource_limit` を明示し、監査ログに DSL ID を記録する |

## 参照
- `docs/notes/dsl/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/1-1-syntax.md`
- `docs/spec/2-2-core-combinator.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/guides/dsl/conductor-pattern.md`
- `docs/guides/compiler/core-parse-streaming.md`
- `docs/plans/bootstrap-roadmap/4-1-core-parse-cst-plan.md`
