# Reml ドキュメント作成ガイド（Core.Doc）

> `Core.Doc` を利用して DSL の API ドキュメントを生成するための最小ガイド。

## 1. 基本フロー
1. `///` コメントを含むソースを用意
2. `Doc.extract` で抽出
3. `Doc.render_markdown` / `Doc.render_html` で出力

参照: [3-15 Core Doc](../../spec/3-15-core-doc.md)

## 2. 最小例

```reml
use Core.Doc

fn main() -> Str {
  let source = "/// add\nfn add(a: Int, b: Int) -> Int { a + b }"
  match Doc.extract(source) {
    Ok(page) => Doc.render_markdown(page)
    Err(_) => "doc:error"
  }
}
```

## 3. Doctest
- 例コードが失敗した場合は `doc.doctest.failed` を返す。
- 実行ログは監査イベントとして記録する。
