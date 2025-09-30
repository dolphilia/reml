# F# 実装サンプル

このディレクトリには、F# と FParsec を使用した Reml 比較用の小規模言語実装が含まれています。

## F# と FParsec の特徴

- **.NET 上の関数型言語**: ML ファミリーの型推論と関数型プログラミング
- **FParsec**: 産業実用レベルのパーサーコンビネーターライブラリ
- **型安全**: 静的型付けと型推論のバランスが優れている
- **パイプライン演算子**: `|>` による可読性の高いコード

## Reml との比較ポイント

### 1. **パーサーコンビネーターの設計**

**FParsec の特徴:**
```fsharp
// FParsec: 演算子ベースのコンビネーター
let parser = pstring "hello" >>. spaces >>. pint32
```

**Reml の特徴:**
```reml
// Reml: メソッドチェーンとパイプライン
let parser = string("hello").skipR(spaces).skipR(int32)
```

- FParsec は演算子 (`>>.`, `.>>`, `.>>.`) が豊富だが、初学者には難解
- Reml はメソッド名が明示的で、読みやすさを重視

### 2. **エラーハンドリング**

**FParsec:**
```fsharp
let parser = attempt p1 <|> p2 <?> "expected value"
```

- `<?>` でカスタムエラーメッセージを付与
- `attempt` でバックトラック制御

**Reml:**
```reml
let parser = Parse.attempt(p1).or(p2).label("expected value")
```

- メソッド名が明示的で、意図が明確
- cut/commit の区別が明確

### 3. **型推論**

**F#:**
- ML系の型推論で、ほとんどの場合型注釈不要
- ただし、ジェネリクスの制約は明示が必要な場合あり

**Reml:**
- Hindley-Milner型推論で、F#と同等の推論能力
- 効果システムも統合され、副作用の追跡も自動

### 4. **Unicode 処理**

**F#:**
- .NET の String は UTF-16 ベース
- Grapheme 処理には `System.Globalization.StringInfo` が必要
- Reml の3層モデル（Byte/Char/Grapheme）に相当する明示的な区別はない

**Reml:**
- UTF-8 ベースで、3層モデルが型レベルで区別される
- 絵文字・結合文字の扱いが安全で明示的

## 実装予定のサンプル

このディレクトリには以下のサンプルを追加予定：

1. **Markdown風パーサー** (`markdown_parser.fs`)
   - FParsec を使用したMarkdown構文解析
   - Reml との Grapheme 処理の比較

2. **SQL風パーサー** (`sql_parser.fs`)
   - FParsec の `OperatorPrecedenceParser` を使用
   - Reml の演算子優先度ビルダーとの比較

3. **JSON パーサー** (`json_parser.fs`)
   - FParsec の基本的なコンビネーター活用例

## 参考資料

- [FParsec 公式ドキュメント](https://www.quanttec.com/fparsec/)
- [F# 公式サイト](https://fsharp.org/)
- [FParsec チュートリアル](https://www.quanttec.com/fparsec/tutorial.html)

## ビルド方法

```bash
# .NET SDK をインストール後
dotnet new console -lang F# -n markdown_parser
cd markdown_parser
dotnet add package FParsec
dotnet build
```

> **注記**: このディレクトリの実装はドキュメント用途のため、完全なプロジェクトファイルは含まれていません。必要に応じて上記の手順でプロジェクトを作成してください。