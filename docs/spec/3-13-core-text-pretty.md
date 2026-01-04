# 3.13 Core Text Pretty

> 目的：Wadler-Leijen 系のプリティプリンタを標準化し、DSL のフォーマッタやコード生成を支援する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {none}` |
| 依存モジュール | `Core.Text`, `Core.Text.Unicode` |
| 相互参照 | [3-3 Core Text & Unicode](3-3-core-text-unicode.md), Guides: [formatter-authoring](../guides/dsl/formatter-authoring.md) |

## 1. 基本概念

`Core.Text.Pretty` は `Doc` 抽象を通じてレイアウトを遅延構築し、レンダリング時に幅に応じた改行を選択する。

## 2. 型と API

```reml
pub type Doc

fn text(value: Str) -> Doc
fn line() -> Doc
fn softline() -> Doc
fn group(doc: Doc) -> Doc
fn nest(indent: Int, doc: Doc) -> Doc
fn concat(left: Doc, right: Doc) -> Doc

fn render(doc: Doc, width: Int) -> Str

type CstPrinter
type CstNode
fn cst_printer() -> CstPrinter
fn cst_doc(printer: CstPrinter, node: CstNode) -> Doc
```

## 3. レイアウト規則

- `group` は可能なら改行を潰し、`softline` を空白へ置換する。
- `width` を超える場合は `softline` を改行へ変換する。
- 文字幅は `Core.Text.Unicode` の計測ルールを使用する。

## 3.1 CST Printer（Phase 4）

`CstPrinter` は `Core.Parse.Cst` が返す CST を `Doc` に変換する標準プリンタであり、空白・改行・コメントを**入力どおりに再現**することを最優先とする。

- `cst_doc` は `CstNode.trivia_leading` / `children` / `trivia_trailing` を入力順に連結する。
- `TriviaKind` は種別に関わらず `Trivia.text` をそのまま出力する（改行を含む場合はそのままレンダリングされる）。
- `CstChild.Token` は `Token.text` を出力し、追加の整形は行わない。

## 4. 例

```reml
use Core.Text.Pretty

fn main() -> Str {
  let doc = Pretty.group(
    Pretty.concat(
      Pretty.text("let"),
      Pretty.concat(Pretty.softline(), Pretty.text("x = 1"))
    )
  )
  Pretty.render(doc, 10)
}
```

```reml
use Core.Parse
use Core.Text.Pretty

type Parser<T>
type Ast

fn format_with_cst(parser: Parser<Ast>, input: Str) -> Str {
  let result = Parse.run_with_cst(parser, input, RunConfig{})
  match result.value with
  | Some(output) -> {
      let printer = Pretty.cst_printer()
      let doc = Pretty.cst_doc(printer, output.cst)
      Pretty.render(doc, 80)
    }
  | None -> ""
}
```
