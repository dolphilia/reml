#!/usr/bin/env raku
# JSON パーサー - Raku 版
# Reml との比較ポイント: Grammar、宣言的パーサー定義

# JSON 構文を解析して汎用値型に変換する
role JsonValue { }

class JsonNull does JsonValue {
  method gist() { 'null' }
}

class JsonBool does JsonValue {
  has Bool $.value;
  method gist() { $.value ?? 'true' !! 'false' }
}

class JsonNumber does JsonValue {
  has Num $.value;
  method gist() { ~$.value }
}

class JsonString does JsonValue {
  has Str $.value;
  method gist() { '"' ~ $.value ~ '"' }
}

class JsonArray does JsonValue {
  has JsonValue @.items;
  method gist() { '[' ~ @.items.map(*.gist).join(', ') ~ ']' }
}

class JsonObject does JsonValue {
  has %.fields;
  method gist() {
    '{' ~ %.fields.map({ "\"$_.key()\": {$_.value.gist}" }).join(', ') ~ '}'
  }
}

# === Raku Grammar による JSON パース ===
# Reml の Core.Parse コンビネーターと比較する

grammar JSON {
  token TOP { <value> }

  token value {
    | <object>
    | <array>
    | <string>
    | <number>
    | <boolean>
    | <null>
  }

  rule object {
    '{' ~ '}' [ <pair> * % ',' ]?
  }

  rule pair {
    <string> ':' <value>
  }

  rule array {
    '[' ~ ']' [ <value> * % ',' ]?
  }

  token string {
    '"' ~ '"' <-["\\]>*  # 簡易実装（エスケープ未対応）
  }

  token number {
    '-'? \d+ ['.' \d+]? ['e' <[+-]>? \d+]?
  }

  token boolean {
    | 'true'
    | 'false'
  }

  token null {
    'null'
  }

  token ws { \s* }
}

# === Actions: AST 構築 ===

class JSONActions {
  method TOP($/) { make $<value>.made }

  method value($/) {
    make $<object>.made  // $<array>.made   // $<string>.made //
         $<number>.made  // $<boolean>.made // $<null>.made
  }

  method object($/) {
    my %fields;
    for $<pair>.list -> $pair {
      my ($key, $val) = $pair.made;
      %fields{$key} = $val;
    }
    make JsonObject.new(fields => %fields)
  }

  method pair($/) {
    make ($<string>.made.value, $<value>.made)
  }

  method array($/) {
    make JsonArray.new(items => $<value>.map(*.made).Array)
  }

  method string($/) {
    make JsonString.new(value => ~$/)
  }

  method number($/) {
    make JsonNumber.new(value => +$/)
  }

  method boolean($/) {
    make JsonBool.new(value => ~$/ eq 'true')
  }

  method null($/) {
    make JsonNull.new
  }
}

# === パース関数 ===

sub parse-json(Str $source --> JsonValue) is export {
  my $match = JSON.parse($source, :actions(JSONActions.new));
  die "JSON パースエラー" unless $match;
  $match.made
}

# === テスト ===

sub MAIN() {
  say "=== Raku JSON パーサー ===";

  try {
    my $json1 = parse-json('{"key": 123}');
    say "Parsed: {$json1.gist}";

    my $json2 = parse-json('[1, 2, 3]');
    say "Parsed: {$json2.gist}";

    my $json3 = parse-json('{"name": "Alice", "age": 30, "active": true}');
    say "Parsed: {$json3.gist}";
  }
  CATCH {
    default { say "Error: {.message}" }
  }
}

# === Reml との比較メモ ===

=begin comment

1. **Grammar vs パーサーコンビネーター**
   Raku: Grammar で宣言的にパーサーを定義
         token/rule で空白処理を制御
         - token: 空白を無視しない
         - rule: 空白を自動的にスキップ
   Reml: Core.Parse コンビネーターで宣言的に定義
         skipWs などで明示的に空白処理を制御

   - Raku の Grammar は正規表現ベースで直感的
   - Reml のコンビネーターはエラー品質の制御が細かい（cut/commit/recover）

2. **構文の比較**

   **Raku:**
   ```raku
   rule object {
     '{' ~ '}' [ <pair> * % ',' ]?
   }
   ```
   - `~` で開き・閉じの対応を明示
   - `* %` で区切り文字を指定した繰り返し

   **Reml:**
   ```reml
   let json_object: Parser<JsonValue> =
     rule("json.object",
       sym("{")
         .skipR(Parse.sepBy(pair, sym(",")))
         .skipL(sym("}"))
         .map(|pairs| JObject(pairs))
     )
   ```
   - メソッドチェーンで組み立て
   - map で AST を構築

   - Raku の方が簡潔
   - Reml の方が型安全で、エラーメッセージの制御が細かい

3. **エラーハンドリング**
   Raku: パースエラーは例外として送出
         Grammar 内でエラー回復は限定的
   Reml: Result<T, ParseError> で型安全にエラーを扱う
         cut/commit/recover でエラー回復を明示的に制御

   - Reml の方がエラー品質の制御が優れている

4. **型システム**
   Raku: 動的型付け（オプショナルな型注釈あり）
         Role（トレイト）で多相性を実現
   Reml: 静的型付け、Hindley-Milner 型推論
         型注釈をほぼ省略可能

   - Reml の方が型安全性が高い

5. **パフォーマンス**
   Raku: 正規表現エンジンベースで、バックトラックあり
         JIT コンパイルで最適化されるが、起動時間が長い
   Reml: LL(*) をベースに、Packrat メモ化や左再帰もサポート
         ネイティブコード生成で高速実行を目指す

   - Reml の方が高速な実行を期待

6. **Unicode 処理**
   Raku: .chars（Grapheme）、.codes（Codepoint）、.encode（Byte）
         Grammar 内で Grapheme 単位の処理が可能
   Reml: Grapheme、Char、Byte を型レベルで区別
         パーサーコンビネーター内で型安全に処理

   - Raku: メソッドで選択、柔軟
   - Reml: 型レベルで区別、型安全

7. **Grammar の拡張性**
   Raku: Grammar を継承して拡張可能
         ```raku
         grammar ExtendedJSON is JSON {
           token value {
             | <comment>
             | nextsame
           }
           token comment { '#' \N* }
         }
         ```
   Reml: パーサーコンビネーターを組み合わせて拡張
         関数合成で新しいパーサーを構築

   - どちらも拡張性が高い
   - Raku: 継承ベース
   - Reml: 合成ベース

**結論**:
Raku の Grammar は正規表現ベースで直感的かつ簡潔。
Reml のパーサーコンビネーターは型安全で、エラー品質の制御が細かい。
どちらも宣言的なパーサー記述が可能だが、アプローチが異なる。

Raku の Grammar は言語実装に組み込まれているため、
学習コストが低く、すぐに使える。
Reml のコンビネーターは標準ライブラリとして提供され、
言語実装に最適化されたエラーハンドリングを持つ。

=end comment