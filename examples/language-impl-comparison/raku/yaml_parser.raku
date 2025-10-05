#!/usr/bin/env raku

# YAML風パーサー：インデント管理が重要な題材。
#
# 対応する構文（簡易版）：
# - スカラー値: 文字列、数値、真偽値、null
# - リスト: `- item1`
# - マップ: `key: value`
# - ネストしたインデント構造
#
# インデント処理の特徴：
# - Rakuの文法とルールを活用したパーサー実装
# - エラー回復機能でインデントミスを報告しつつ継続

# YAML値の表現。
subset YamlValue where * ~~ any(Scalar, YList, YMap, YNull);

class Scalar {
  has Str $.value;
}

class YList {
  has YamlValue @.items;
}

class YMap {
  has %.entries;
}

class YNull {
}

class Document {
  has YamlValue $.root;
}

# パーサークラス
class Parser {
  has Str $.input;
  has Int $.pos = 0;

  method peek() {
    return self.pos < self.input.chars ?? self.input.substr(self.pos, 1) !! '';
  }

  method advance() {
    self.pos++ if self.pos < self.input.chars;
  }

  method is-eof() {
    self.pos >= self.input.chars;
  }

  method expect(Str $expected) {
    die "期待された文字 '$expected' が見つかりません" unless self.peek eq $expected;
    self.advance;
  }

  method expect-string(Str $expected) {
    for $expected.comb -> $c {
      self.expect($c);
    }
  }

  # 水平空白のみをスキップ（改行は含まない）。
  method hspace() {
    while self.peek ~~ / <[ \t ]> / {
      self.advance;
    }
  }

  # 改行をスキップ。
  method newline() {
    if self.peek eq "\n" {
      self.advance;
    } elsif self.peek eq "\r" {
      self.advance;
      self.advance if self.peek eq "\n";
    }
  }

  # コメントのスキップ（`#` から行末まで）。
  method comment() {
    if self.peek eq '#' {
      self.advance;
      while !self.is-eof && self.peek ne "\n" {
        self.advance;
      }
    }
  }

  # 空行またはコメント行をスキップ。
  method blank-or-comment() {
    self.hspace;
    self.comment;
    self.newline;
  }

  # 特定のインデントレベルを期待する。
  method expect-indent(Int $level) {
    my $spaces = 0;
    while self.peek eq ' ' {
      $spaces++;
      self.advance;
    }

    die "インデント不一致: 期待 $level, 実際 $spaces" unless $spaces == $level;
  }

  # 現在よりも深いインデントを検出。
  method deeper-indent(Int $current) returns Int {
    my $spaces = 0;
    while self.peek eq ' ' {
      $spaces++;
      self.advance;
    }

    die "深いインデントが期待されます: 現在 $current, 実際 $spaces" unless $spaces > $current;

    return $spaces;
  }

  # スカラー値のパース。
  method scalar-value() returns YamlValue {
    # null
    if self.input.substr(self.pos, 4) eq 'null' {
      self.expect-string('null');
      return YNull.new;
    }

    if self.peek eq '~' {
      self.advance;
      return YNull.new;
    }

    # 真偽値
    if self.input.substr(self.pos, 4) eq 'true' {
      self.expect-string('true');
      return Scalar.new(value => 'true');
    }

    if self.input.substr(self.pos, 5) eq 'false' {
      self.expect-string('false');
      return Scalar.new(value => 'false');
    }

    # 数値（簡易実装）
    my $num-str = '';
    while self.peek ~~ / <[0..9]> / {
      $num-str ~= self.peek;
      self.advance;
    }

    return Scalar.new(value => $num-str) if $num-str.chars > 0;

    # 文字列（引用符付き）
    if self.peek eq '"' {
      self.advance;
      my $str = '';
      while self.peek ne '"' && !self.is-eof {
        $str ~= self.peek;
        self.advance;
      }
      self.expect('"');
      return Scalar.new(value => $str);
    }

    # 文字列（引用符なし：行末または `:` まで）
    my $str = '';
    while !self.is-eof && self.peek !~~ / <[\n : #]> / {
      $str ~= self.peek;
      self.advance;
    }

    return Scalar.new(value => $str.trim);
  }

  # リスト項目のパース（`- value` 形式）。
  method parse-list-item(Int $indent) returns YamlValue {
    self.expect-indent($indent);
    self.expect('-');
    self.hspace;
    return self.parse-value($indent + 2);
  }

  # リスト全体のパース。
  method parse-list(Int $indent) returns YamlValue {
    my @items;

    loop {
      my $saved-pos = self.pos;
      try {
        my $item = self.parse-list-item($indent);
        @items.push($item);

        self.newline if self.peek eq "\n";
        CATCH {
          default {
            self.pos = $saved-pos;
            last;
          }
        }
      }
    }

    die "リストが空です" if @items.elems == 0;

    return YList.new(items => @items);
  }

  # マップのキーバリューペアのパース（`key: value` 形式）。
  method parse-map-entry(Int $indent) returns Pair {
    self.expect-indent($indent);

    my $key = '';
    while !self.is-eof && self.peek !~~ / <[: \n]> / {
      $key ~= self.peek;
      self.advance;
    }

    $key = $key.trim;
    self.expect(':');
    self.hspace;

    my $value;

    # 同じ行に値があるか、次の行にネストされているか
    if self.peek eq "\n" {
      self.newline;
      $value = self.parse-value($indent + 2);
    } else {
      $value = self.parse-value($indent);
    }

    return $key => $value;
  }

  # マップ全体のパース。
  method parse-map(Int $indent) returns YamlValue {
    my %entries;

    loop {
      my $saved-pos = self.pos;
      try {
        my $entry = self.parse-map-entry($indent);
        %entries{$entry.key} = $entry.value;

        self.newline if self.peek eq "\n";
        CATCH {
          default {
            self.pos = $saved-pos;
            last;
          }
        }
      }
    }

    die "マップが空です" if %entries.elems == 0;

    return YMap.new(entries => %entries);
  }

  # YAML値のパース（再帰的）。
  method parse-value(Int $indent) returns YamlValue {
    my $saved-pos = self.pos;

    # リストを試行
    try {
      return self.parse-list($indent);
      CATCH {
        default {
          self.pos = $saved-pos;
        }
      }
    }

    # マップを試行
    try {
      return self.parse-map($indent);
      CATCH {
        default {
          self.pos = $saved-pos;
        }
      }
    }

    # スカラー
    return self.scalar-value;
  }

  # ドキュメント全体のパース。
  method document() returns Document {
    # 空行やコメントをスキップ
    while !self.is-eof {
      my $saved-pos = self.pos;
      try {
        self.blank-or-comment;
        CATCH {
          default {
            self.pos = $saved-pos;
            last;
          }
        }
      }
    }

    my $doc = self.parse-value(0);

    # 末尾の空行やコメントをスキップ
    while !self.is-eof {
      try {
        self.blank-or-comment;
        CATCH {
          default {
            last;
          }
        }
      }
    }

    die "ドキュメントの終端が期待されます" unless self.is-eof;

    return Document.new(root => $doc);
  }
}

# パブリックAPI：YAML文字列をパース。
sub parse-yaml(Str $input) returns Document is export {
  my $parser = Parser.new(input => $input);
  return $parser.document;
}

# 簡易的なレンダリング（検証用）。
sub render-to-string(Document $doc) returns Str is export {
  sub render-value(YamlValue $value, Int $indent) returns Str {
    my $indent-str = ' ' x $indent;

    given $value {
      when Scalar {
        return $value.value;
      }
      when YNull {
        return 'null';
      }
      when YList {
        return $value.items.map({ $indent-str ~ '- ' ~ render-value($_, $indent + 2) }).join("\n");
      }
      when YMap {
        my @lines;
        for $value.entries.kv -> $key, $val {
          given $val {
            when Scalar | YNull {
              @lines.push($indent-str ~ $key ~ ': ' ~ render-value($val, 0));
            }
            default {
              @lines.push($indent-str ~ $key ~ ":\n" ~ render-value($val, $indent + 2));
            }
          }
        }
        return @lines.join("\n");
      }
    }
  }

  return render-value($doc.root, 0);
}

# テスト例。
sub test-examples() is export {
  my @examples = [
    ("simple_scalar", "hello"),
    ("simple_list", "- item1\n- item2\n- item3"),
    ("simple_map", "key1: value1\nkey2: value2"),
    ("nested_map", "parent:\n  child1: value1\n  child2: value2"),
    ("nested_list", "items:\n  - item1\n  - item2"),
    ("mixed", "name: John\nage: 30\nhobbies:\n  - reading\n  - coding")
  ];

  for @examples -> ($name, $yaml-str) {
    say "--- $name ---";
    try {
      my $doc = parse-yaml($yaml-str);
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

# インデント処理の課題と解決策：
#
# 1. **インデントレベルの追跡**
#    - パーサー引数としてインデントレベルを渡す
#    - Rakuのオブジェクト指向スタイルでパーサー状態を管理
#
# 2. **エラー回復**
#    - try/CATCHでバックトラックを制御
#    - 例外で分かりやすいエラーメッセージを提供
#
# 3. **空白の扱い**
#    - hspaceで水平空白のみをスキップ（改行は構文の一部）
#    - newlineでCR/LF/CRLFを正規化
#
# Remlとの比較：
#
# - **Rakuの利点**:
#   - 強力な文法とルールシステム
#   - 柔軟なオブジェクト指向機能
#
# - **Rakuの課題**:
#   - パーサーコンビネーターライブラリがRemlほど充実していない
#   - 手動のバックトラック管理が煩雑
#
# - **Remlの利点**:
#   - 字句レイヤの柔軟性により、インデント処理が自然に表現できる
#   - cut/commitによるエラー品質の向上
#   - recoverによる部分的なパース継続が可能