#!/usr/bin/env raku

=begin pod
JSON拡張版：コメント・トレーリングカンマ対応。

標準JSONからの拡張点：
1. コメント対応（C<//> 行コメント、C</* */> ブロックコメント）
2. トレーリングカンマ許可（配列・オブジェクトの最後の要素の後）
3. より詳細なエラーメッセージ

実用的な設定ファイル形式として：
- C<package.json> 風の設定ファイル
- C<.babelrc>, C<.eslintrc> など開発ツールの設定
- VS Code の C<settings.json>
=end pod

# 文法定義

grammar JsonExtended {
  token TOP { <ws-and-comments> <json-value> <ws-and-comments> }

  # 空白とコメント
  token ws-and-comments {
    [
      | <whitespace>
      | <line-comment>
      | <block-comment>
    ]*
  }

  token whitespace { <[ \s \n \t \r ]>+ }
  token line-comment { '//' \N* }
  token block-comment { '/*' .*? '*/' }

  # JSON値
  token json-value {
    | <null>
    | <bool>
    | <number>
    | <string>
    | <array>
    | <object>
  }

  token null { 'null' }
  token bool { 'true' | 'false' }

  token number {
    '-'? <digit>+ ['.' <digit>+]? [<[eE]> <[+-]>? <digit>+]?
  }

  token string {
    '"' <string-content> '"'
  }

  token string-content {
    [
      | <-[ " \\ ]>
      | '\\' <escape-char>
    ]*
  }

  token escape-char {
    | 'n'
    | 't'
    | 'r'
    | '\\'
    | '"'
    | .
  }

  # 配列（トレーリングカンマ対応）
  token array {
    '[' <ws-and-comments>
    [
      | ']'
      | <json-value> <ws-and-comments>
        [',' <ws-and-comments> <json-value> <ws-and-comments>]*
        [',' <ws-and-comments>]?  # トレーリングカンマ
        ']'
    ]
  }

  # オブジェクト（トレーリングカンマ対応）
  token object {
    '{' <ws-and-comments>
    [
      | '}'
      | <pair> <ws-and-comments>
        [',' <ws-and-comments> <pair> <ws-and-comments>]*
        [',' <ws-and-comments>]?  # トレーリングカンマ
        '}'
    ]
  }

  token pair {
    <string> <ws-and-comments> ':' <ws-and-comments> <json-value>
  }
}

# アクション

class JsonExtendedActions {
  method TOP($/) {
    make $<json-value>.made;
  }

  method json-value($/) {
    if $<null> {
      make Any;
    } elsif $<bool> {
      make $<bool>.Str eq 'true';
    } elsif $<number> {
      make $<number>.Str.Numeric;
    } elsif $<string> {
      make $<string>.made;
    } elsif $<array> {
      make $<array>.made;
    } elsif $<object> {
      make $<object>.made;
    }
  }

  method string($/) {
    my $content = $<string-content>.Str;
    $content = $content.subst('\\n', "\n", :g);
    $content = $content.subst('\\t', "\t", :g);
    $content = $content.subst('\\r', "\r", :g);
    $content = $content.subst('\\\\', "\\", :g);
    $content = $content.subst('\\"', '"', :g);
    make $content;
  }

  method array($/) {
    my @items;
    for $<json-value> -> $value {
      @items.push($value.made);
    }
    make @items;
  }

  method object($/) {
    my %pairs;
    for $<pair> -> $pair {
      my $key = $pair<string>.made;
      my $value = $pair<json-value>.made;
      %pairs{$key} = $value;
    }
    make %pairs;
  }
}

# パース関数

sub parse(Str $input) is export {
  my $match = JsonExtended.parse($input, :actions(JsonExtendedActions));
  if $match {
    return $match.made;
  } else {
    die "パースエラー: 不正なJSON拡張形式";
  }
}

# レンダリング

sub render-to-string($value, Int $indent-level = 0) is export {
  my $indent = '  ' x $indent-level;
  my $next-indent = '  ' x ($indent-level + 1);

  given $value {
    when Any { return 'null'; }
    when Bool { return $value ?? 'true' !! 'false'; }
    when Numeric { return ~$value; }
    when Str { return '"' ~ $value ~ '"'; }
    when Positional {
      if $value.elems == 0 {
        return '[]';
      }
      my @items = $value.map({ $next-indent ~ render-to-string($_, $indent-level + 1) });
      return "[\n" ~ @items.join(",\n") ~ "\n" ~ $indent ~ "]";
    }
    when Associative {
      if $value.keys.elems == 0 {
        return '{}';
      }
      my @pairs = $value.kv.map(-> $key, $val {
        $next-indent ~ '"' ~ $key ~ '": ' ~ render-to-string($val, $indent-level + 1)
      });
      return "{\n" ~ @pairs.join(",\n") ~ "\n" ~ $indent ~ "}";
    }
    default { return ''; }
  }
}

# テスト

sub test-extended-json() is export {
  my @test-cases = (
    ('コメント対応', q:to/END/),
{
  // これは行コメント
  "name": "test",
  /* これは
     ブロックコメント */
  "version": "1.0"
}
END
    ('トレーリングカンマ', q:to/END/),
{
  "items": [
    1,
    2,
    3,
  ],
  "config": {
    "debug": true,
    "port": 8080,
  }
}
END
  );

  for @test-cases -> ($name, $json-str) {
    say "--- $name ---";
    try {
      my $value = parse($json-str);
      say "パース成功:";
      say render-to-string($value, 0);
      CATCH {
        default {
          say "パースエラー: $_";
        }
      }
    }
    say "";
  }
}

# メイン（スクリプトとして実行された場合）

sub MAIN() {
  test-extended-json();
}