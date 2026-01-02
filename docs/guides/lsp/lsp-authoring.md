# Reml LSP 作成ガイド（Core.Lsp）

> `Core.Lsp` を用いて DSL 向け LSP サーバーを構築するための最小ガイド。

## 1. 基本フロー
1. `Core.Lsp` の型で位置情報と診断を構築
2. JSON-RPC メッセージをエンコードして送信

参照: [3-14 Core Lsp](../../spec/3-14-core-lsp.md)

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

## 4. Auto-LSP 導出（with_doc）

`Core.Lsp.Derive` は `Core.Parse` の `rule`/`keyword`/`symbol`/`token` と Doc comment を収集し、補完/アウトライン/ホバーを自動導出する。

```reml
use Core.Parse
use Core.Lsp.Derive

let expr =
  rule("expr",
    keyword(sc, "let")
      .then(ident)
      .with_doc("変数定義を表す式")
  )

let model = Derive.collect(expr)
let caps = Derive.standard_capabilities(model)
```

## 5. CLI での導出結果確認（--output lsp-derive）

LSP サーバーを起動せずに導出結果を確認する場合は、`--output lsp-derive` を使って JSON を取得する。

```bash
reml_frontend --output lsp-derive examples/practical/core_lsp/auto_derive_basic.reml
```
