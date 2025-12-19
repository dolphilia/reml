# Reml フォーマッタ作成ガイド（Core.Text.Pretty）

> DSL フォーマッタを `Core.Text.Pretty` で組み立てるための最小ガイド。

## 1. 基本フロー
1. `Doc` を組み合わせて AST を表現
2. `Pretty.render` で幅に応じた出力を生成

参照: [3-13 Core Text Pretty](../spec/3-13-core-text-pretty.md)

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
