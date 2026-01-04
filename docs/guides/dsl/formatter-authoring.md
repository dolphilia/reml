# Reml フォーマッタ作成ガイド（Core.Text.Pretty）

> DSL フォーマッタを `Core.Text.Pretty` で組み立てるための最小ガイド。

## 1. 基本フロー
1. `Doc` を組み合わせて AST を表現
2. `Pretty.render` で幅に応じた出力を生成

参照: [3-13 Core Text Pretty](../../spec/3-13-core-text-pretty.md)

## 2. 最小例

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

## 3. 幅と Unicode
- 幅計測は `Core.Text.Unicode` のルールと一致させる。
- 絵文字や結合文字を含む場合は `display_width` の計測を優先する。

## 4. CST 連携（Core.Parse.Cst）

CST を利用する場合は `Parse.run_with_cst` と `Pretty.cst_printer` を組み合わせ、**入力の空白・改行・コメントを維持**したまま `Doc` を生成する。

```reml
use Core.Parse
use Core.Text.Pretty

fn format_with_cst(parser: Parser<Ast>, input: Str) -> Str {
  let result = Parse.run_with_cst(parser, input, RunConfig.default())
  match result.value with
    | Some(output) ->
        Pretty.render(
          Pretty.cst_doc(Pretty.cst_printer(), output.cst),
          80
        )
    | None -> ""
}
```

- CST は `RunConfig.extensions["parse"].cst` が opt-in の場合のみ収集される。
- `CstPrinter` は `Trivia.text` をそのまま出力するため、既定では整形を加えないロスレス経路となる。
