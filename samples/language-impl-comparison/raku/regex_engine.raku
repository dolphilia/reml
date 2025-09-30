#!/usr/bin/env raku

# 正規表現エンジン：パース + 評価の両方を実装。
#
# 対応する正規表現構文（簡易版）：
# - リテラル: `abc`
# - 連結: `ab`
# - 選択: `a|b`
# - 繰り返し: `a*`, `a+`, `a?`, `a{2,5}`
# - グループ: `(abc)`
# - 文字クラス: `[a-z]`, `[^0-9]`, `\d`, `\w`, `\s`
# - アンカー: `^`, `$`
# - ドット: `.` (任意の1文字)

# 正規表現のAST
class Regex {
    method match-at($text, $pos) { ... }
}

class Literal is Regex {
    has Str $.value;

    method match-at($text, $pos) {
        return $text.substr($pos, $.value.chars) eq $.value;
    }
}

class CharClass is Regex {
    has $.charset;

    method match-at($text, $pos) {
        return False if $pos >= $text.chars;
        my $char = $text.substr($pos, 1);
        return $.charset.matches($char);
    }
}

class Dot is Regex {
    method match-at($text, $pos) {
        return $pos < $text.chars;
    }
}

class Concat is Regex {
    has @.terms;

    method match-at($text, $pos) {
        my $current-pos = $pos;
        for @.terms -> $term {
            if $term.match-at($text, $current-pos) {
                $current-pos++;
            } else {
                return False;
            }
        }
        return True;
    }
}

class Alternation is Regex {
    has @.alternatives;

    method match-at($text, $pos) {
        for @.alternatives -> $alt {
            return True if $alt.match-at($text, $pos);
        }
        return False;
    }
}

class Repeat is Regex {
    has Regex $.inner;
    has $.kind;

    method match-at($text, $pos) {
        given $.kind {
            when 'zero-or-more' { return self.match-loop($text, $pos, 0, 0, 999999); }
            when 'one-or-more' {
                return False unless $.inner.match-at($text, $pos);
                return self.match-loop($text, $pos + 1, 1, 1, 999999);
            }
            when 'zero-or-one' { return $.inner.match-at($text, $pos) || True; }
            when 'exactly' {
                my $n = $.kind<n>;
                return self.match-loop($text, $pos, 0, $n, $n);
            }
            when 'range' {
                my $min = $.kind<min>;
                my $max = $.kind<max> // 999999;
                return self.match-loop($text, $pos, 0, $min, $max);
            }
        }
    }

    method match-loop($text, $pos, $count, $min, $max) {
        return True if $count == $max;
        return True if $count >= $min && !$.inner.match-at($text, $pos);
        if $.inner.match-at($text, $pos) {
            return self.match-loop($text, $pos + 1, $count + 1, $min, $max);
        }
        return $count >= $min;
    }
}

class Group is Regex {
    has Regex $.inner;

    method match-at($text, $pos) {
        return $.inner.match-at($text, $pos);
    }
}

class Anchor is Regex {
    has $.kind;

    method match-at($text, $pos) {
        given $.kind {
            when 'start' { return $pos == 0; }
            when 'end' { return $pos >= $text.chars; }
        }
    }
}

# 文字集合
class CharSet {
    method matches($char) { ... }
}

class CharRange is CharSet {
    has Str $.start;
    has Str $.end;

    method matches($char) {
        return $.start le $char le $.end;
    }
}

class CharList is CharSet {
    has @.chars;

    method matches($char) {
        return $char (elem) @.chars;
    }
}

class PredefinedClass is CharSet {
    has Str $.class-name;

    method matches($char) {
        given $.class-name {
            when 'digit' { return $char ~~ /<[0..9]>/; }
            when 'word' { return $char ~~ /<[a..zA..Z0..9_]>/; }
            when 'whitespace' { return $char ~~ /\s/; }
            when 'not-digit' { return $char !~~ /<[0..9]>/; }
            when 'not-word' { return $char !~~ /<[a..zA..Z0..9_]>/; }
            when 'not-whitespace' { return $char !~~ /\s/; }
        }
    }
}

class NegatedCharSet is CharSet {
    has CharSet $.inner;

    method matches($char) {
        return !$.inner.matches($char);
    }
}

class UnionCharSet is CharSet {
    has @.sets;

    method matches($char) {
        for @.sets -> $set {
            return True if $set.matches($char);
        }
        return False;
    }
}

# パーサー
class Parser {
    has Str $.input;
    has Int $.pos = 0;

    method peek() {
        return Nil if $.pos >= $.input.chars;
        return $.input.substr($.pos, 1);
    }

    method advance() {
        $!pos++ if $.pos < $.input.chars;
    }

    method match-string($str) {
        if $.input.substr($.pos, $str.chars) eq $str {
            $!pos += $str.chars;
            return True;
        }
        return False;
    }

    method parse-regex() {
        return self.parse-alternation();
    }

    method parse-alternation() {
        my @alts = self.parse-concat();

        while self.match-string('|') {
            @alts.push(self.parse-concat());
        }

        return @alts.elems == 1 ?? @alts[0] !! Alternation.new(alternatives => @alts);
    }

    method parse-concat() {
        my @terms;

        while my $term = self.parse-postfix() {
            @terms.push($term);
            last if self.peek() ~~ any('|', ')', Nil);
        }

        return @terms.elems == 1 ?? @terms[0] !! Concat.new(terms => @terms);
    }

    method parse-postfix() {
        my $base = self.parse-atom();
        return Nil unless $base;

        my $kind;
        if self.match-string('*') {
            $kind = 'zero-or-more';
        } elsif self.match-string('+') {
            $kind = 'one-or-more';
        } elsif self.match-string('?') {
            $kind = 'zero-or-one';
        } elsif self.peek() eq '{' {
            $kind = self.parse-braced-repeat();
        }

        return $kind ?? Repeat.new(inner => $base, kind => $kind) !! $base;
    }

    method parse-braced-repeat() {
        return Nil unless self.match-string('{');

        my $n = self.parse-integer();
        my $kind;

        if self.match-string(',') {
            my $m = self.parse-integer();
            $kind = { kind => 'range', min => $n, max => $m };
        } else {
            $kind = { kind => 'exactly', n => $n };
        }

        self.match-string('}');
        return $kind;
    }

    method parse-integer() {
        my $num = '';
        while self.peek() ~~ /<[0..9]>/ {
            $num ~= self.peek();
            self.advance();
        }
        return $num.Int;
    }

    method parse-atom() {
        given self.peek() {
            when '(' {
                self.advance();
                my $inner = self.parse-alternation();
                self.match-string(')');
                return Group.new(inner => $inner);
            }
            when '^' {
                self.advance();
                return Anchor.new(kind => 'start');
            }
            when '$' {
                self.advance();
                return Anchor.new(kind => 'end');
            }
            when '.' {
                self.advance();
                return Dot.new();
            }
            when '[' {
                return self.parse-char-class();
            }
            when '\\' {
                return self.parse-escape();
            }
            when any('|', ')', Nil) {
                return Nil;
            }
            default {
                my $char = self.peek();
                self.advance();
                return Literal.new(value => $char);
            }
        }
    }

    method parse-char-class() {
        return Nil unless self.match-string('[');

        my $negated = self.match-string('^');
        my @sets;

        while self.peek() ne ']' {
            my $start = self.peek();
            self.advance();

            if self.match-string('-') && self.peek() ne ']' {
                my $end = self.peek();
                self.advance();
                @sets.push(CharRange.new(start => $start, end => $end));
            } else {
                @sets.push(CharList.new(chars => [$start]));
            }
        }

        self.match-string(']');

        my $union = UnionCharSet.new(sets => @sets);
        my $cs = $negated ?? NegatedCharSet.new(inner => $union) !! $union;

        return CharClass.new(charset => $cs);
    }

    method parse-escape() {
        return Nil unless self.match-string('\\');

        given self.peek() {
            when 'd' {
                self.advance();
                return CharClass.new(charset => PredefinedClass.new(class-name => 'digit'));
            }
            when 'w' {
                self.advance();
                return CharClass.new(charset => PredefinedClass.new(class-name => 'word'));
            }
            when 's' {
                self.advance();
                return CharClass.new(charset => PredefinedClass.new(class-name => 'whitespace'));
            }
            when 'D' {
                self.advance();
                return CharClass.new(charset => PredefinedClass.new(class-name => 'not-digit'));
            }
            when 'W' {
                self.advance();
                return CharClass.new(charset => PredefinedClass.new(class-name => 'not-word'));
            }
            when 'S' {
                self.advance();
                return CharClass.new(charset => PredefinedClass.new(class-name => 'not-whitespace'));
            }
            default {
                my $char = self.peek();
                self.advance();
                my $lit = $char eq 'n' ?? "\n" !! $char eq 't' ?? "\t" !! $char eq 'r' ?? "\r" !! $char;
                return Literal.new(value => $lit);
            }
        }
    }
}

# テスト例
sub test-examples() {
    my @examples = (
        ('a+', 'aaa', True),
        ('a+', 'b', False),
        ('[0-9]+', '123', True),
        ('[0-9]+', 'abc', False),
        ('\\d{2,4}', '12', True),
        ('\\d{2,4}', '12345', True),
        ('(abc)+', 'abcabc', True),
        ('a|b', 'a', True),
        ('a|b', 'b', True),
        ('a|b', 'c', False),
        ('^hello$', 'hello', True),
        ('^hello$', 'hello world', False),
    );

    for @examples -> ($pattern, $text, $expected) {
        my $parser = Parser.new(input => $pattern);
        my $regex = $parser.parse-regex();

        if $regex {
            my $result = $regex.match-at($text, 0);
            my $status = $result == $expected ?? '✓' !! '✗';
            say "$status パターン: '$pattern', テキスト: '$text', 期待: $expected, 結果: $result";
        } else {
            say "✗ パーサーエラー: $pattern";
        }
    }
}

# 実行
test-examples();