# Raku 実装サンプル

このディレクトリには、Raku を使用した Reml 比較用の小規模言語実装が含まれています。

## Raku の特徴

- **Perl の後継言語**: Perl 6 として開発され、2019年に Raku に改名
- **Unicode 第一級市民**: Grapheme/Codepoint/Byte の明示的な型区別
- **グラマー（Grammar）**: 組み込みのパーサージェネレーター機能
- **多様なパラダイム**: 手続き型・オブジェクト指向・関数型・並行処理

## Reml との比較ポイント

### 1. **Unicode 3層モデルの比較**

**Raku の文字列型:**
```raku
my $text = "Hello 🇯🇵 café";

# Grapheme（書記素クラスター）単位
say $text.chars;           # => 11 (Grapheme単位)

# Codepoint（Unicodeコードポイント）単位
say $text.codes;           # => 12 (🇯🇵は2つのコードポイント)

# Byte（UTF-8バイト）単位
say $text.encode.elems;    # => 16 (バイト数)
```

**Reml の文字列処理:**
```reml
let text = "Hello 🇯🇵 café"

// Grapheme（書記素クラスター）単位
Grapheme.count(text)       // => 11

// Char（Unicodeコードポイント）単位
String.char_len(text)      // => 12

// Byte（UTF-8バイト）単位
String.byte_len(text)      // => 16
```

**比較のポイント:**
- **Raku**: メソッドで明示的に選択（`.chars`, `.codes`, `.encode`）
- **Reml**: 型レベルで区別（`Grapheme`, `Char`, `Byte`）
- **Raku の優位性**: 既存の主流言語で唯一、3層の区別を持つ
- **Reml の優位性**: 型安全性により、混同によるバグを防止

### 2. **Grammar（文法定義）**

**Raku の Grammar:**
```raku
grammar JSON {
    token TOP { <value> }

    token value {
        | <object>
        | <array>
        | <string>
        | <number>
        | 'true'
        | 'false'
        | 'null'
    }

    token object {
        '{' ~ '}' <pair>* % ','
    }

    token pair {
        <string> ':' <value>
    }

    token array {
        '[' ~ ']' <value>* % ','
    }

    token string {
        '"' ~ '"' <-["\\]>* # 簡易実装
    }

    token number {
        '-'? \d+ [ '.' \d+ ]?
    }
}

# 使用例
my $result = JSON.parse('{"key": 123}');
```

**Reml のパーサーコンビネーター:**
```reml
let json_object: Parser<JsonValue> =
  rule("json.object",
    sym("{")
      .skipR(Parse.sepBy(pair, sym(",")))
      .skipL(sym("}"))
      .map(|pairs| JObject(pairs))
  )
```

**比較のポイント:**
- **Raku**: 宣言的な文法定義で、正規表現風の構文
- **Reml**: コンビネーターによる組み立て式
- **Raku の優位性**: バックトラック・正規表現との統合が自然
- **Reml の優位性**: エラー品質の制御（cut/commit）が明示的

### 3. **正規表現とパーサーの統合**

**Raku:**
```raku
# 正規表現とGrammarが統合されている
grammar SimpleRegex {
    token TOP {
        <literal> | <alternation> | <repetition>
    }

    token literal { \w+ }
    token alternation { <literal> '|' <literal> }
    token repetition { <literal> <[*+?]> }
}
```

- 正規表現エンジンそのものが拡張可能
- Grammar は正規表現の文法的拡張

**Reml:**
- パーサーコンビネーターと正規表現は分離
- 正規表現は標準ライブラリの一部として提供

### 4. **パフォーマンス**

**Raku:**
- 動的言語で、起動時間が長い
- Just-In-Time（JIT）コンパイルで実行時最適化
- MoarVM（仮想マシン）上で動作

**Reml:**
- 静的型付けで、コンパイル時最適化
- ネイティブコード生成を想定
- より高速な実行を期待

## 実装予定のサンプル

このディレクトリには以下のサンプルを追加予定：

1. **Markdown風パーサー** (`markdown_parser.raku`)
   - Grammar を使用した Markdown 構文解析
   - Grapheme/Codepoint の使い分けを明示

2. **正規表現エンジン** (`regex_engine.raku`)
   - Raku の組み込み正規表現との比較
   - Unicode文字クラスの扱い

3. **JSON パーサー** (`json_parser.raku`)
   - Grammar の基本的な使用例

## 参考資料

- [Raku 公式サイト](https://raku.org/)
- [Raku ドキュメント](https://docs.raku.org/)
- [Raku Grammar チュートリアル](https://docs.raku.org/language/grammars)
- [Unicode in Raku](https://docs.raku.org/language/unicode)

## 実行方法

```bash
# Raku (Rakudo) をインストール後
raku markdown_parser.raku

# または
rakudo markdown_parser.raku
```

## Raku の独自性

Raku は Reml の Unicode 3層モデルの妥当性を検証する上で、最も重要な比較対象です：

1. **既存の主流言語で唯一**: Grapheme/Codepoint/Byte を型レベルで区別
2. **実証された有用性**: 絵文字・結合文字の処理が安全で直感的
3. **パフォーマンストレードオフ**: 動的言語のため、Reml より低速だが、表現力は高い

Reml は Raku の Unicode 設計を参考にしつつ、静的型付けと高性能を両立することを目指しています。

> **注記**: Raku は動的言語であり、Reml の静的型システムとは大きく異なりますが、Unicode 処理の設計哲学は Reml に大きな影響を与えています。