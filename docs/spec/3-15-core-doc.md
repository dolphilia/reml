# 3.15 Core Doc

> 目的：ドキュメントコメントの抽出と HTML/Markdown 生成を標準化し、Doctest による整合性確認を支援する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}` |
| 依存モジュール | `Core.Text`, `Core.Diagnostics` |
| 相互参照 | [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [doc-authoring](../guides/dsl/doc-authoring.md) |

## 1. 基本概念

`Core.Doc` は `///` ドキュメントコメントを抽出し、API 要素ごとにドキュメントノードを構築する。Doctest は抽出されたコード片を実行し、結果を検証する。

## 2. 型と API

```reml
pub type DocItem = {
  name: Str,
  summary: Str,
  body: Str,
  examples: List<Str>,
}

pub type DocPage = { items: List<DocItem> }

pub type DocError = { kind: DocErrorKind, message: Str }

pub enum DocErrorKind =
  | ParseFailed
  | RenderFailed
  | DoctestFailed

fn extract(source: Str) -> Result<DocPage, DocError>
fn render_markdown(page: DocPage) -> Str
fn render_html(page: DocPage) -> Str
fn run_doctest(page: DocPage) -> Result<(), DocError>
```

## 3. Doctest ポリシー

- 失敗時は `DocErrorKind::DoctestFailed` を返し、`Diagnostic.code = "doc.doctest.failed"` を使用する。
- Doctest 実行ログは `AuditEvent::DocTest` として記録する。

## 4. 例

```reml
use Core.Doc

fn main() -> Str {
  let source = "/// add\nfn add(a: Int, b: Int) -> Int { a + b }"
  match Doc.extract(source) with
  | Ok(page) -> Doc.render_markdown(page)
  | Err(_) -> "doc:error"
}
```
