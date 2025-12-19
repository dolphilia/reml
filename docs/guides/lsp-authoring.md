# Reml LSP 作成ガイド（Core.Lsp）

> `Core.Lsp` を用いて DSL 向け LSP サーバーを構築するための最小ガイド。

## 1. 基本フロー
1. `Core.Lsp` の型で位置情報と診断を構築
2. JSON-RPC メッセージをエンコードして送信

参照: [3-14 Core Lsp](../spec/3-14-core-lsp.md)

## 2. 最小例

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

## 3. 診断ブリッジ
- `Core.Diagnostics` から LSP へ変換する場合は `diagnostic.code` を維持する。
- `Stage`/`Capability` の警告は `Warning` 以上で通知する。
