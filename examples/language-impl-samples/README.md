# 言語実装サンプル（Reml）

このディレクトリは Reml の記述性や読みやすさを示すための小規模言語実装サンプルをまとめています。現在は Reml 実装のみを収録しています。

## 収録対象

- Reml サンプル: `reml/`

## 実装する小規模言語

1. **ミニ Lisp 評価機** (`mini_lisp.*`)
2. **JSON パーサー** (`json_parser.*`)
3. **PL/0 風トイ言語コンパイラ断片** (`pl0.*`)
4. **Markdown風軽量マークアップパーサー** (`markdown_parser.*`)
5. **SQL風クエリ言語パーサー** (`sql_parser.*`)
6. **代数的効果を使うミニ言語** (`algebraic_effects.*`)
7. **正規表現エンジン** (`regex_engine.*`)
8. **YAML風パーサー** (`yaml_parser.*`)
9. **TOML風設定ファイルパーサー** (`toml_parser.*`)
10. **テンプレート言語** (`template_engine.*`)
11. **JSON拡張版** (`json_extended.*`)
12. **Basic言語インタープリタ** (`basic_interpreter.*`)
13. **Pratt Parser実装** (`pratt_parser.*`)
14. **Hindley-Milner型推論器** (`hindley_milner.*`)
15. **Prelude Guard テンプレート DSL** (`prelude_guard_template.*`): `ensure` と `ensure_not_null` によるテンプレート検証と診断出力の例

Reml 版は仕様記述に合わせて `reml` 言語タグのコードブロックを用いています。

Reml ディレクトリには手続き的な解析器に加え、`Core.Parse` コンビネーターを用いた実装も収録しており、Chapter 2 の標準 API を利用した記述スタイルを確認できます。

> **注記**: これらの実装はドキュメント用途のため、依存管理やビルドスクリプトは付属しません。必要に応じて利用者側で補完してください。
