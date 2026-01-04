# Phase4: Auto-LSP Derivation 計画（Core.Lsp.Derive）

## 背景と決定事項
- `docs/notes/dsl/dsl-enhancement-proposal.md` の提案「3.2 Auto-LSP Derivation (`Core.Lsp.Derive`)」を Phase 4 の実装計画へ落とし込む。
- 既存の `Core.Lsp` は診断エンコード中心で、補完/アウトライン/ハイライトは DSL 側で手動実装が必要になっている。
- `docs/spec/0-1-project-purpose.md` の「分かりやすいエラーメッセージ」「エコシステム統合とDSLファースト」を満たすため、LSP導出を最小コアとして標準化する。

## 目的
1. `Core.Parse` コンビネーターから LSP 情報（補完/アウトライン/ハイライト/ホバー）を自動導出する仕組みを定義する。
2. 仕様・ガイド・実装を同期し、DSL 作者が最小構成で LSP を利用できる状態にする。
3. Rust 実装へ導出ロジックを追加し、Phase 4 の回帰シナリオへ接続する。

## スコープ
- **含む**: `keyword`/`symbol`/`rule`/`token` メタデータからの補完・アウトライン・セマンティックトークン導出、Doc comment の Hover 化、CLI での導出アーティファクト出力。
- **含まない**: LSP サーバーのフル実装、AST/型推論ベースの高度補完、増分パース/ワークスペース管理。

## 成果物
- `docs/spec/3-14-core-lsp.md` に `Core.Lsp.Derive` の API と導出規則を追記。
- `docs/spec/2-2-core-combinator.md` に LSP 向けメタデータ（`rule`/`keyword`/`symbol`/`token`/Doc comment）の収集規約を追記。
- `docs/guides/lsp/lsp-authoring.md` に Auto-LSP 導出の使い方と最小例を追記。
- `examples/practical/core_lsp/` の導出サンプル、および `expected/practical/core_lsp/` の期待出力。
- Rust 実装 (`compiler/runtime/src/lsp/derive.rs` など) の導入と CLI 出力の拡張。

## 導出 API/CLI 出力仕様（確定）

### Core.Lsp.Derive API 命名
- **モジュール名**: `Core.Lsp.Derive`
- **導出モデル**: `DeriveModel`
- **主要 API**:
  - `Derive.collect(parser: Parser<T>) -> DeriveModel`
  - `Derive.standard_capabilities(model: DeriveModel) -> LspCapabilities`
  - `Derive.apply_standard_capabilities(model: DeriveModel, server: LspServer) -> LspServer`

### CLI 出力フォーマット命名
- **OutputFormat**: `LspDerive`
- **CLI フラグ**: `--output lsp-derive`
- **JSON ルート**: `format = "lsp-derive"` / `version = 1`
- **出力用途**: 回帰サンプル・LSP 側の静的検証（エディタ非依存）

### CLI 出力 JSON（最小スキーマ）
```json
{
  "format": "lsp-derive",
  "version": 1,
  "source": "examples/practical/core_lsp/auto_derive_basic.reml",
  "capabilities": {
    "completion": true,
    "outline": true,
    "semantic_tokens": true,
    "hover": true
  },
  "completions": [
    { "label": "let", "kind": "keyword" }
  ],
  "outline": [
    { "name": "expr", "kind": "rule", "children": [] }
  ],
  "semantic_tokens": [
    { "kind": "keyword", "range": { "start": { "line": 1, "character": 1 }, "end": { "line": 1, "character": 3 } } }
  ],
  "hovers": [
    { "name": "expr", "doc": "式" }
  ]
}
```

## 仕様ドラフト（最小構成）

```reml
use Core.Lsp.Derive

let model = Derive.collect(my_parser)
let caps = Derive.standard_capabilities(model)

conductor my_dsl_server {
  serve my_parser
    |> Derive.apply_standard_capabilities(model)
}
```

### 導出ルール（案）
- **Completion**: `keyword`/`symbol` の文字列を補完候補として収集する。
- **Outline**: `rule(name, ...)` の `name` を Document Symbol として整理し、内部で参照する `rule` を階層に反映する。
- **Semantic Tokens**: `token(kind, ...)` の `kind` と `Span` をトークンとして生成する（`token` が無い場合は `keyword`/`symbol` を最低限の `keyword`/`operator` として扱う）。
- **Hover**: `rule`/`token` に付与された Doc comment を Hover として返す。

## 作業ステップ

### フェーズA: 仕様整理
1. [x] `docs/spec/3-14-core-lsp.md` に `Core.Lsp.Derive` の型と導出ルールを追加する。
2. [x] `docs/spec/2-2-core-combinator.md` に `ParserMeta` と Doc comment 収集規約を追記する。
3. [x] `docs/guides/lsp/lsp-authoring.md` に `Derive` の最小導入例と落とし穴（`rule`/`keyword` を使わない場合の補完不足）を追記する。

### フェーズB: メタデータ設計
1. [x] `ParserMetaKind`/`ParserMeta` の最小構造（`kind`/`name`/`doc`/`children`/`token_kind`）を確定する。
2. [x] `ParserId -> ParserMeta` の登録ストレージ（`ParseMetaRegistry`）のライフサイクルを定義する（生成/共有/破棄）。
3. [x] `rule` が内部パーサーの `ParserId` を収集し、`children` に保持する規約を確定する。
4. [x] `keyword`/`symbol`/`token` の `name` 表記（文字列そのままか正規化か）を決める。
5. [x] Doc comment の付与 API（`with_doc`/`parser.with_doc`）と上書き規則を確定する。
6. [x] `label` は `ParserMeta` を生成しない方針を明記する。
7. [x] 完了条件: `docs/spec/2-2-core-combinator.md` の規約と Rust 実装の API 名が一致していることをレビューで確認。
   - `ParserMeta`/`ParseMetaRegistry`/`Parser::with_doc`/`with_doc`/`token` の命名と挙動が Rust 実装と整合していることを確認済み。

### フェーズC: Rust 実装追加
1. [x] `compiler/runtime/src/parse/meta.rs` を追加し、`ParserMeta`/`ParserMetaKind` と登録 API を実装する。
2. [x] `ParseState` に `ParseMetaRegistry` を保持し、`run`/`run_with_default` で初期化されるようにする。
3. [x] `rule` で `ParserMeta` を登録し、子 `ParserId` の収集フックを追加する。
4. [x] `keyword`/`symbol`/`token` の生成時に `ParserMeta` を登録する。
5. [x] `Parser::with_doc`（および関数版 `with_doc`）を追加し、`ParserMeta.doc` を更新する。
6. [x] `compiler/runtime/src/lsp/derive.rs` を追加し、`Derive.collect` を実装する。
7. [x] `Derive.standard_capabilities` を実装し、`DeriveModel` の内容に応じた boolean を返す。
8. [x] `compiler/frontend/src/output/cli.rs` に `OutputFormat::LspDerive` を追加し、`--output lsp-derive` を受理する。
9. [x] CLI の JSON 出力に `format="lsp-derive"` / `version=1` を付与し、`DeriveModel` をエンコードする。
10. [x] 完了条件: `examples/practical/core_lsp/auto_derive_basic.reml` の導出結果が `expected/practical/core_lsp/auto_derive_basic.stdout` と一致することを確認。
    - `auto_derive_basic.reml` は `rule`/`keyword`/`symbol`/`token` を含まず、導出モデルが空になる前提で期待出力と一致していることを確認済み。

### フェーズD: サンプル/回帰接続
1. [x] `examples/practical/core_lsp/auto_derive_basic.reml` と `expected/practical/core_lsp/auto_derive_basic.stdout` を追加する。
2. [x] `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に Auto-LSP 導出シナリオを登録する。
3. [x] `reports/spec-audit/ch5/logs/` に実行ログのテンプレートを追加する。

## Rust 実装の現状と追加案

### 既存実装の範囲
- `compiler/runtime/src/lsp/mod.rs` で `Core.Lsp` の最小 API（`diagnostic`, `encode_publish`, `decode_message`）を実装済み。
- `compiler/runtime/src/parse/combinator.rs` では `rule`/`keyword`/`symbol` などの字句系コンビネーターとメタデータ収集が実装済み。

### 追加 API 案（Rust 側）
- `ParserMeta` 型（`kind`, `name`, `doc`, `children`, `token_kind`）を導入し、`ParserId` で参照できるようにする。
- `Parser::with_doc` / `Parser::with_token_kind` のような補助メソッドを追加し、LSP 導出に必要な情報を付与する。
- `Core.Lsp.Derive` で `DeriveModel`（補完候補/Outline/トークン/ホバー）を生成し、CLI へ渡す。

### モジュール分割案
- `compiler/runtime/src/parse/meta.rs`: `ParserMeta` と登録用レジストリ。
- `compiler/runtime/src/parse/combinator.rs`: `rule`/`keyword`/`symbol`/`token` のメタデータ登録。
- `compiler/runtime/src/lsp/derive.rs`: `DeriveModel` と導出ロジック。
- `compiler/frontend/src/output/cli.rs`: `OutputFormat::LspDerive` と JSON 出力。

## 依存関係
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md` の Core.Lsp 改善タスクと整合。
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` の Parser API 仕様と整合。
- `docs/spec/3-14-core-lsp.md` / `docs/spec/2-2-core-combinator.md` の更新が前提。

## リスクと緩和策
| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| メタデータが不足し導出が空になる | 補完/Outline が生成されず UX が低下 | `rule`/`keyword`/`symbol` の利用規約をガイドに明記し、検出時に警告診断を出す |
| 既存パーサーとの互換性 | 既存 DSL の導出結果が不安定 | 既存 API を壊さず、`Derive` は opt-in で導入 |
| CLI 出力形式の乱立 | 回帰サンプル管理が複雑化 | `OutputFormat::LspDerive` を追加し、`reports/spec-audit/ch5/logs/` で統一管理 |

## 参照
- `docs/notes/dsl/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/2-2-core-combinator.md`
- `docs/spec/3-14-core-lsp.md`
- `docs/guides/lsp/lsp-authoring.md`
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md`
