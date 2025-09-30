#!/usr/bin/env raku

# TOML風パーサー：Rakuの文法と正規表現を活用した実装。
#
# 対応する構文（TOML v1.0.0準拠の簡易版）：
# - キーバリューペア: `key = "value"`
# - テーブル: `[section]`
# - 配列テーブル: `[[array_section]]`
# - データ型: 文字列、整数、浮動小数点、真偽値、日時、配列、インラインテーブル
# - コメント: `# comment`
#
# 実装の特徴：
# - Rakuのgrammer機能による宣言的な構文定義
# - Actions クラスによるAST構築
# - 再帰的なテーブルマージロジック
# - トレーリングカンマのサポート
#
# 他言語との比較：
# - Reml: パーサーコンビネーターによる柔軟な構文定義と高品質なエラー処理
# - Raku: grammarとactionsによる簡潔な実装、強力な正規表現サポート
#

# TOML値の表現。
subset TomlValue where * ~~ any(TomlString, TomlInteger, TomlFloat, TomlBoolean, TomlDateTime, TomlArray, TomlInlineTable);

class TomlString {
  has Str $.value;
}

class TomlInteger {
  has Int $.value;
}

class TomlFloat {
  has Rat $.value;
}

class TomlBoolean {
  has Bool $.value;
}

class TomlDateTime {
  has Str $.value;  # ISO 8601形式の文字列として保持
}

class TomlArray {
  has TomlValue @.items;
}

class TomlInlineTable {
  has %.entries;
}

class TomlTable {
  has %.entries;
}

class TomlDocument {
  has TomlTable $.root;
  has %.tables;  # テーブルパス => テーブル
}

# TOML Grammar定義。
grammar TomlGrammar {
  token TOP { <ws> <document> }

  token document {
    [ <blank-or-comment> ]*
    [ <element> [ <blank-or-comment> ]* ]*
  }

  token element {
    | <array-table-header>
    | <table-header>
    | <key-value-pair>
  }

  token blank-or-comment {
    | <comment>
    | \n
  }

  token comment {
    '#' \N*
  }

  token ws {
    [ ' ' | \t ]*
  }

  # キー名（ベアキーまたは引用符付き）。
  token key {
    | <quoted-key>
    | <bare-key>
  }

  token bare-key {
    <[A..Z a..z 0..9 \- _]>+
  }

  token quoted-key {
    '"' <( <-["]>* )> '"'
  }

  # ドットで区切られたキーパス。
  token key-path {
    <key> [ '.' <ws> <key> ]*
  }

  # キーバリューペア。
  token key-value-pair {
    <key-path> <ws> '=' <ws> <value>
  }

  # テーブルヘッダー。
  token table-header {
    '[' <ws> <key-path> <ws> ']'
  }

  # 配列テーブルヘッダー。
  token array-table-header {
    '[[' <ws> <key-path> <ws> ']]'
  }

  # TOML値。
  token value {
    | <multiline-basic-string>
    | <multiline-literal-string>
    | <basic-string>
    | <literal-string>
    | <datetime>
    | <float>
    | <integer>
    | <boolean>
    | <array>
    | <inline-table>
  }

  # 文字列（基本）。
  token basic-string {
    '"' <( [ <-["\\\n]> | '\\' <["\\/bfnrt]> ]* )> '"'
  }

  # 文字列（リテラル）。
  token literal-string {
    "'" <( <-[']>* )> "'"
  }

  # 複数行基本文字列。
  token multiline-basic-string {
    '"""' <ws> \n? <( [ <-["]> | '"' <!before '""'> ]* )> '"""'
  }

  # 複数行リテラル文字列。
  token multiline-literal-string {
    "'''" <ws> \n? <( [ <-[']> | "'" <!before "''"> ]* )> "'''"
  }

  # 整数。
  token integer {
    '-'? \d+ [ '_' \d+ ]*
  }

  # 浮動小数点。
  token float {
    '-'? \d+ [ '_' \d+ ]* '.' \d+ [ '_' \d+ ]* [ <[eE]> <[\+\-]>? \d+ ]?
  }

  # 真偽値。
  token boolean {
    | 'true'
    | 'false'
  }

  # 日時（ISO 8601形式の簡易実装）。
  token datetime {
    \d ** 4 '-' \d ** 2 '-' \d ** 2 'T' \d ** 2 ':' \d ** 2 ':' \d ** 2 [ <[\+\-Z]> \S* ]?
  }

  # 配列。
  token array {
    '[' <ws> [ <value> <ws> [ ',' <ws> <value> <ws> ]* [ ',' <ws> ]? ]? ']'
  }

  # インラインテーブル。
  token inline-table {
    '{' <ws> [ <key> <ws> '=' <ws> <value> <ws> [ ',' <ws> <key> <ws> '=' <ws> <value> <ws> ]* [ ',' <ws> ]? ]? '}'
  }
}

# TOML Actions：文法からASTを構築。
class TomlActions {
  has TomlDocument $.document;
  has @.current-table-path;
  has Bool $.in-array-table = False;

  method TOP($/) {
    make $<document>.made;
  }

  method document($/) {
    my $root = TomlTable.new(entries => {});
    my %tables;
    my @current-path = ();
    my $in-array-table = False;

    for $<element>.list -> $elem {
      my $made = $elem.made;

      given $made {
        when Hash {
          if $made<type> eq 'table' {
            @current-path = $made<path>.list;
            $in-array-table = False;
            %tables{@current-path.join('.')} = TomlTable.new(entries => {}) unless %tables{@current-path.join('.')}:exists;
          } elsif $made<type> eq 'array-table' {
            @current-path = $made<path>.list;
            $in-array-table = True;
            %tables{@current-path.join('.')} = TomlTable.new(entries => {}) unless %tables{@current-path.join('.')}:exists;
          } elsif $made<type> eq 'key-value' {
            my @path = $made<path>.list;
            my $value = $made<value>;

            if @current-path.elems == 0 {
              # ルートテーブルに追加
              self.insert-nested($root.entries, @path, $value);
            } else {
              # 現在のテーブルに追加
              my $table = %tables{@current-path.join('.')};
              self.insert-nested($table.entries, @path, $value);
            }
          }
        }
      }
    }

    make TomlDocument.new(root => $root, tables => %tables);
  }

  method element($/) {
    if $<array-table-header> {
      make $<array-table-header>.made;
    } elsif $<table-header> {
      make $<table-header>.made;
    } elsif $<key-value-pair> {
      make $<key-value-pair>.made;
    }
  }

  method table-header($/) {
    my @path = self.extract-key-path($<key-path>);
    make { type => 'table', path => @path };
  }

  method array-table-header($/) {
    my @path = self.extract-key-path($<key-path>);
    make { type => 'array-table', path => @path };
  }

  method key-value-pair($/) {
    my @path = self.extract-key-path($<key-path>);
    my $value = $<value>.made;
    make { type => 'key-value', path => @path, value => $value };
  }

  method extract-key-path($key-path) {
    my @keys;
    for $key-path<key>.list -> $key {
      if $key<bare-key> {
        @keys.push($key<bare-key>.Str);
      } elsif $key<quoted-key> {
        @keys.push($key<quoted-key>.Str);
      }
    }
    return @keys;
  }

  method value($/) {
    if $<basic-string> {
      make TomlString.new(value => $<basic-string>.Str);
    } elsif $<literal-string> {
      make TomlString.new(value => $<literal-string>.Str);
    } elsif $<multiline-basic-string> {
      make TomlString.new(value => $<multiline-basic-string>.Str);
    } elsif $<multiline-literal-string> {
      make TomlString.new(value => $<multiline-literal-string>.Str);
    } elsif $<integer> {
      my $int-str = $<integer>.Str.subst('_', '', :g);
      make TomlInteger.new(value => $int-str.Int);
    } elsif $<float> {
      my $float-str = $<float>.Str.subst('_', '', :g);
      make TomlFloat.new(value => $float-str.Rat);
    } elsif $<boolean> {
      make TomlBoolean.new(value => $<boolean>.Str eq 'true');
    } elsif $<datetime> {
      make TomlDateTime.new(value => $<datetime>.Str);
    } elsif $<array> {
      make $<array>.made;
    } elsif $<inline-table> {
      make $<inline-table>.made;
    }
  }

  method array($/) {
    my @items;
    for $<value>.list -> $val {
      @items.push($val.made);
    }
    make TomlArray.new(items => @items);
  }

  method inline-table($/) {
    my %entries;
    for $<key> Z $<value> -> ($key, $value) {
      my $key-str = $key<bare-key> ?? $key<bare-key>.Str !! $key<quoted-key>.Str;
      %entries{$key-str} = $value.made;
    }
    make TomlInlineTable.new(entries => %entries);
  }

  # ネストしたキーパスに値を挿入する補助メソッド。
  method insert-nested(%table, @path, $value) {
    return if @path.elems == 0;

    if @path.elems == 1 {
      %table{@path[0]} = $value;
    } else {
      my $key = @path[0];
      my @rest = @path[1..*];

      unless %table{$key}:exists {
        %table{$key} = TomlInlineTable.new(entries => {});
      }

      given %table{$key} {
        when TomlInlineTable {
          self.insert-nested(%table{$key}.entries, @rest, $value);
        }
      }
    }
  }
}

# パブリックAPI：TOML文字列をパース。
sub parse-toml(Str $input) returns TomlDocument is export {
  my $actions = TomlActions.new;
  my $match = TomlGrammar.parse($input, :actions($actions));

  die "TOMLパースに失敗しました" unless $match;

  return $match.made;
}

# 簡易的なレンダリング（検証用）。
sub render-to-string(TomlDocument $doc) returns Str is export {
  my $output = '';

  # ルートテーブルをレンダリング
  $output ~= render-table($doc.root, ());

  # 各セクションをレンダリング
  for $doc.tables.kv -> $path-str, $table {
    $output ~= "\n[{$path-str}]\n";
    $output ~= render-table($table, ());
  }

  return $output;
}

sub render-table(TomlTable $table, @prefix) returns Str {
  my $output = '';

  for $table.entries.kv -> $key, $value {
    my @full-path = @prefix.elems > 0 ?? (@prefix.flat, $key) !! ($key,);
    my $full-key = @full-path.join('.');

    given $value {
      when TomlInlineTable {
        $output ~= render-table-entries($value, @full-path);
      }
      default {
        $output ~= "{$full-key} = {render-value($value)}\n";
      }
    }
  }

  return $output;
}

sub render-table-entries(TomlInlineTable $table, @prefix) returns Str {
  my $output = '';

  for $table.entries.kv -> $key, $value {
    my @full-path = (@prefix.flat, $key);
    my $full-key = @full-path.join('.');

    given $value {
      when TomlInlineTable {
        $output ~= render-table-entries($value, @full-path);
      }
      default {
        $output ~= "{$full-key} = {render-value($value)}\n";
      }
    }
  }

  return $output;
}

sub render-value(TomlValue $value) returns Str {
  given $value {
    when TomlString {
      return "\"{$value.value}\"";
    }
    when TomlInteger {
      return ~$value.value;
    }
    when TomlFloat {
      return ~$value.value;
    }
    when TomlBoolean {
      return $value.value ?? 'true' !! 'false';
    }
    when TomlDateTime {
      return $value.value;
    }
    when TomlArray {
      my $items = $value.items.map({ render-value($_) }).join(', ');
      return "[{$items}]";
    }
    when TomlInlineTable {
      my $entries = $value.entries.kv.map(-> $k, $v { "{$k} = {render-value($v)}" }).join(', ');
      return "\{ {$entries} \}";
    }
  }
}

# テスト例。
sub test-examples() is export {
  my $example-toml = q:to/END/;
    # Reml パッケージ設定

    [package]
    name = "my_project"
    version = "0.1.0"
    authors = ["Author Name"]

    [dependencies]
    core = "1.0"

    [dev-dependencies]
    test_framework = "0.5"

    [[plugins]]
    name = "system"
    version = "1.0"

    [[plugins]]
    name = "memory"
    version = "1.0"
    END

  say "--- reml.toml 風設定のパース ---";
  try {
    my $doc = parse-toml($example-toml);
    say "パース成功:";
    say render-to-string($doc);
    CATCH {
      default {
        say "パースエラー: $_";
      }
    }
  }

  # 追加のテスト例
  my @test-cases = [
    ("simple_key_value", 'key = "value"'),
    ("integer", 'number = 42'),
    ("float", 'pi = 3.14'),
    ("boolean", 'enabled = true'),
    ("array", 'items = [1, 2, 3]'),
    ("inline_table", 'person = { name = "John", age = 30 }'),
    ("datetime", 'timestamp = 2025-09-30T12:00:00Z'),
  ];

  for @test-cases -> ($name, $toml-str) {
    say "\n--- {$name} ---";
    try {
      my $doc = parse-toml($toml-str);
      say "パース成功:";
      say render-to-string($doc);
      CATCH {
        default {
          say "パースエラー: $_";
        }
      }
    }
  }
}

# Rakuの文法システムの利点と課題：
#
# 1. **宣言的な構文定義**
#    - grammar により、BNF風の読みやすい構文定義が可能
#    - token/rule/regex の使い分けで空白処理を制御
#
# 2. **強力な正規表現**
#    - Rakuの正規表現は非常に表現力が高い
#    - 先読み・後読み、名前付きキャプチャ、再帰パターンをサポート
#
# 3. **Actions による AST 構築**
#    - 文法定義と意味処理を分離
#    - made/make によるクリーンなAST構築
#
# 4. **課題**
#    - エラーメッセージの品質が限定的
#    - バックトラック制御が自動的で、細かい制御が難しい
#    - Remlのcut/commitのような明示的なエラー位置特定が困難
#
# Remlとの比較：
#
# - **Remlの利点**:
#   - パーサーコンビネーターによる柔軟な構文定義
#   - cut/commit/recoverによる高品質なエラー処理
#   - 期待集合による詳細な診断メッセージ
#   - トレースによるデバッグサポート
#
# - **Rakuの利点**:
#   - grammarによる簡潔で宣言的な構文定義
#   - 強力な正規表現エンジン
#   - 標準機能として組み込まれている
#
# エラー品質の向上には：
# - カスタムエラーハンドリングの追加
# - より詳細な位置情報の追跡
# - 期待値の収集と報告機構の実装
# が必要となる。