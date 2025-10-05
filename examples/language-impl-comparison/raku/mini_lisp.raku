#!/usr/bin/env raku
# ミニ Lisp 評価機 - Raku 版
# Reml との比較ポイント: Grammar、Unicode 3層モデル、動的型付け

# S 式構文を持つ式を解析して評価する
class Expr {
  multi method eval($env) { die "eval not implemented for {self.^name}" }
}

class Number is Expr {
  has Num $.value;
  method eval($env) { $.value }
}

class Symbol is Expr {
  has Str $.name;
  method eval($env) {
    $env{$.name} // die "未定義シンボル: $.name"
  }
}

class ExprList is Expr {
  has Expr @.items;
  method eval($env) {
    return $.items[0] unless @.items;
    die "空のリストは評価できません" unless @.items;

    my $callee = @.items[0].eval($env);
    my @args = @.items[1..*].map(*.eval($env));

    given $callee {
      when Callable { $callee.(@args) }
      when Number { die "数値を関数としては適用できません" }
      default { die "評価できない値: {$callee.^name}" }
    }
  }
}

# === Raku Grammar によるパース ===
# Reml のパーサーコンビネーターと比較する重要な機能

grammar LispGrammar {
  token TOP { <expr> }

  token expr {
    | <list>
    | <number>
    | <symbol>
  }

  token list {
    '(' ~ ')' <expr>*
  }

  token number {
    '-'? \d+ ['.' \d+]?
  }

  token symbol {
    <[a..zA..Z+\-*/]>+
  }

  token ws { \s* }
}

class LispActions {
  method TOP($/) { make $<expr>.made }

  method expr($/) {
    make $<list>.made // $<number>.made // $<symbol>.made
  }

  method list($/) {
    make ExprList.new(items => $<expr>.map(*.made).Array)
  }

  method number($/) {
    make Number.new(value => +$/)
  }

  method symbol($/) {
    make Symbol.new(name => ~$/)
  }
}

# === パース関数 ===

sub parse-lisp(Str $source --> Expr) {
  my $match = LispGrammar.parse($source, :actions(LispActions.new));
  die "パースエラー" unless $match;
  $match.made
}

# === デフォルト環境 ===

sub builtin-numeric(&op) {
  return sub (@args) {
    die "数値演算は 2 引数のみ対応します" unless @args.elems == 2;
    die "数値以外を演算できません" unless @args.all ~~ Num;
    return op(@args[0], @args[1]);
  }
}

sub default-env() {
  return %(
    '+' => builtin-numeric(* + *),
    '-' => builtin-numeric(* - *),
    '*' => builtin-numeric(* * *),
    '/' => builtin-numeric(* / *),
  );
}

# === メイン評価関数 ===

sub eval-lisp(Str $source --> Num) is export {
  my $expr = parse-lisp($source);
  my $env = default-env();
  $expr.eval($env)
}

# === テスト ===

sub MAIN() {
  say "=== Raku ミニ Lisp 評価機 ===";

  try {
    my $result1 = eval-lisp("(+ 40 2)");
    say "Result: $result1";  # => 42

    my $result2 = eval-lisp("(* (+ 1 2) (- 5 3))");
    say "Result: $result2";  # => 6
  }
  CATCH {
    default { say "Error: {.message}" }
  }
}

# === Unicode 3層モデルのデモ（Raku の強み） ===

# Raku は Grapheme/Codepoint/Byte を明示的に区別できる唯一の主流言語
sub demo-unicode-layers() {
  my $text = "Hello 🇯🇵 café";

  say "\n=== Unicode 3層モデルのデモ ===";

  # Grapheme（書記素クラスター）単位
  say "Grapheme count: {$text.chars}";           # => 11

  # Codepoint（Unicodeコードポイント）単位
  say "Codepoint count: {$text.codes}";          # => 12 (🇯🇵は2つのコードポイント)

  # Byte（UTF-8バイト）単位
  say "Byte count: {$text.encode.elems}";        # => 16
}

# === Reml との比較メモ ===

=begin comment

1. **Grammar（文法定義）**
   Raku: Grammar で宣言的にパーサーを定義
         正規表現ベースで、PEG風のバックトラック
   Reml: Core.Parse コンビネーターで宣言的に定義
         LL(*) をベースに、Packrat や左再帰もサポート
   - Raku の Grammar は正規表現との統合が自然
   - Reml のコンビネーターはエラー品質の制御（cut/commit）が明示的

2. **Unicode 3層モデル**
   Raku: .chars（Grapheme）、.codes（Codepoint）、.encode（Byte）で明示的に区別
         型レベルでは区別されず、メソッドで選択
   Reml: Grapheme、Char、Byte を型レベルで区別
         型安全性により、混同によるバグを防止
   - Raku: 既存の主流言語で唯一の3層区別
   - Reml: 型安全性でさらに強化

3. **動的型付け vs 静的型付け**
   Raku: 動的型付け（オプショナルな型注釈あり）
         ダックタイピング、実行時の型チェック
   Reml: 静的型付け、Hindley-Milner 型推論
         コンパイル時に型エラーを検出
   - Reml の方が型安全性が高い
   - Raku の方が柔軟で、プロトタイピングが速い

4. **パフォーマンス**
   Raku: MoarVM（仮想マシン）上で動作、JITコンパイル
         起動時間が長く、実行速度は中程度
   Reml: ネイティブコード生成を想定、高速実行を目指す
   - Reml の方が高速な実行を期待

5. **多様なパラダイム**
   Raku: 手続き型・オブジェクト指向・関数型・並行処理を統合
         非常に多機能だが、学習コストが高い
   Reml: 関数型と手続き型のハイブリッド
         シンプルで学習コストが低い
   - Raku は多機能で柔軟
   - Reml はシンプルで明快

6. **正規表現とパーサーの統合**
   Raku: Grammar は正規表現の文法的拡張
         正規表現エンジンそのものが拡張可能
   Reml: パーサーコンビネーターと正規表現は分離
         正規表現は標準ライブラリの一部として提供
   - Raku の統合は非常にユニーク

**結論**:
Raku は Unicode 3層モデルを実装した唯一の主流言語で、
Reml の設計哲学に大きな影響を与えている。
Reml は Raku の Unicode 設計を参考にしつつ、
静的型付けと高性能を両立することを目指す。

=end comment