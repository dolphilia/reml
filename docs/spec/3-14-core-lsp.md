# 3.14 Core Lsp

> 目的：DSL 作者が最小構成の LSP サーバーを構築できるよう、プロトコル型と JSON-RPC ループを標準化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}` |
| 依存モジュール | `Core.IO`, `Core.Diagnostics`, `Core.Text` |
| 相互参照 | [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [2-7 Core Parse Streaming](2-7-core-parse-streaming.md), Guides: [lsp-authoring](../guides/lsp-authoring.md) |

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

## 4. 例

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
