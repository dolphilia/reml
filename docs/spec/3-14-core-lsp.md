# 3.14 Core Lsp

> 目的：DSL 作者が最小構成の LSP サーバーを構築できるよう、プロトコル型と JSON-RPC ループを標準化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}` |
| 依存モジュール | `Core.IO`, `Core.Diagnostics`, `Core.Text` |
| 相互参照 | [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [2-7 Core Parse Streaming](2-7-core-parse-streaming.md), [2-2 Core Combinator](2-2-core-combinator.md), Guides: [lsp-authoring](../guides/lsp/lsp-authoring.md) |

## 1. 基本型

```reml
pub type Position = { line: Int, character: Int }
pub type Range = { start: Position, end: Position }

pub enum DiagnosticSeverity = Error | Warning | Information | Hint

pub type LspDiagnostic = {
  range: Range,
  severity: DiagnosticSeverity,
  message: Str,
  code: Option<Str>,
}

pub type JsonRpcMessage = {
  method: Str,
  params: Map<Str, Str>,
}

pub type LspError = { kind: LspErrorKind, message: Str }

pub enum LspErrorKind = DecodeFailed | UnsupportedMethod
```

## 2. JSON-RPC ヘルパ

```reml
fn position(line: Int, character: Int) -> Position
fn range(start_line: Int, start_char: Int, end_line: Int, end_char: Int) -> Range
fn diagnostic(range: Range, severity: DiagnosticSeverity, message: Str) -> LspDiagnostic

fn encode_publish(uri: Str, diagnostics: List<LspDiagnostic>) -> Str
fn decode_message(payload: Str) -> Result<JsonRpcMessage, LspError>
```

- `encode_publish` は `textDocument/publishDiagnostics` の JSON を返す。
- `decode_message` は不正な JSON で `LspErrorKind::DecodeFailed` を返す。

## 3. 診断ブリッジ

- `Core.Diagnostics` の `Diagnostic` を `LspDiagnostic` へ変換する `to_lsp` を提供する。
- 変換時に `code` を `diagnostic.code` へ同期する。

## 4. Core.Lsp.Derive

> 目的：`Core.Parse` のメタデータから LSP 機能を自動導出する。

```reml
pub type DeriveModel = {
  completions: List<CompletionItem>,
  outline: List<OutlineNode>,
  semantic_tokens: List<SemanticToken>,
  hovers: List<HoverEntry>,
}

pub type CompletionItem = { label: Str, kind: Str }

pub type OutlineNode = {
  name: Str,
  kind: Str,
  children: List<OutlineNode>,
}

pub type SemanticToken = { kind: Str, range: Range }

pub type HoverEntry = { name: Str, doc: Str }

fn Derive.collect<T>(parser: Parser<T>) -> DeriveModel
fn Derive.standard_capabilities(model: DeriveModel) -> LspCapabilities
fn Derive.apply_standard_capabilities(model: DeriveModel, server: LspServer) -> LspServer
```

- `Derive.collect` は `keyword`/`symbol`/`rule`/`token` と Doc comment を収集し、`DeriveModel` を構築する。
- `Derive.standard_capabilities` は補完/アウトライン/セマンティックトークン/ホバーの有効化フラグを生成する。
- `Derive.apply_standard_capabilities` は `DeriveModel` を LSP サーバーへ接続する。
- Doc comment の付与は `with_doc` を用いる（[2-2 コア・コンビネータ](2-2-core-combinator.md#G-3-doc-comment-の付与)）。
- Layout 由来の仮想トークンは `layout_token` を通じて扱い、字句連携は [2-2 コア・コンビネータ](2-2-core-combinator.md#b-2-a-layout_tokenlayout-連携) を参照する。

## 5. LspDerive 出力仕様

CLI で `--output lsp-derive` を指定した場合、`DeriveModel` を JSON で出力する。

```reml
pub type LspDeriveEnvelope = {
  format: Str,    // "lsp-derive"
  version: Int,   // 1
  source: Str,
  capabilities: LspDeriveCapabilities,
  completions: List<CompletionItem>,
  outline: List<OutlineNode>,
  semantic_tokens: List<SemanticToken>,
  hovers: List<HoverEntry>,
}

pub type LspDeriveCapabilities = {
  completion: Bool,
  outline: Bool,
  semantic_tokens: Bool,
  hover: Bool,
}
```

- `format` は常に `"lsp-derive"` とする。
- `version` は互換性管理のため `1` を固定する。
- `source` は入力ファイルのパスまたは URI を格納する。

## 6. 例

```reml
use Core.Lsp

fn main() -> Str {
  let diag = Lsp.diagnostic(
    range = Lsp.range(0, 0, 0, 1),
    severity = Lsp.DiagnosticSeverity::Warning,
    message = "demo"
  )
  Lsp.encode_publish("file:///demo.reml", [diag])
}
```
