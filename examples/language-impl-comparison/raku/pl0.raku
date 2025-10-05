#!/usr/bin/env raku
# PL/0 風ミニ言語インタプリタ - Raku 版
# Reml との比較ポイント: Grammar、代数的データ型の模倣

# === AST ===

enum Op <Add Sub Mul Div>;

role Expr { }

class Number does Expr {
  has Int $.value;
  method gist() { ~$.value }
}

class Var does Expr {
  has Str $.name;
  method gist() { $.name }
}

class Binary does Expr {
  has Op $.op;
  has Expr $.lhs;
  has Expr $.rhs;
  method gist() {
    my $op-str = $.op.Str.lc;
    "($.lhs.gist() $op-str $.rhs.gist())"
  }
}

# === Stmt ===

role Stmt { }

class Assign does Stmt {
  has Str $.name;
  has Expr $.expr;
  method gist() { "$.name := $.expr.gist()" }
}

class While does Stmt {
  has Expr $.cond;
  has Stmt @.body;
  method gist() {
    "while $.cond.gist() { @.body.map(*.gist).join('; ') }"
  }
}

class Write does Stmt {
  has Expr $.expr;
  method gist() { "write $.expr.gist()" }
}

# === Runtime ===

class Runtime {
  has %.vars = ();
  has Int @.output = ();

  method set-var(Str $name, Int $value) {
    %.vars{$name} = $value;
  }

  method get-var(Str $name --> Int) {
    %.vars{$name} // fail "未定義変数: $name";
  }

  method write-output(Int $value) {
    @.output.push($value);
  }
}

# === Evaluator ===

sub eval-expr(Runtime $rt, Expr $expr --> Int) {
  given $expr {
    when Number { .value }
    when Var { $rt.get-var(.name) }
    when Binary {
      my $l = eval-expr($rt, .lhs);
      my $r = eval-expr($rt, .rhs);
      given .op {
        when Add { $l + $r }
        when Sub { $l - $r }
        when Mul { $l * $r }
        when Div {
          fail "0 で割れません" if $r == 0;
          ($l / $r).Int
        }
      }
    }
  }
}

sub exec-stmt(Runtime $rt, Stmt $stmt) {
  given $stmt {
    when Assign {
      my $value = eval-expr($rt, .expr);
      $rt.set-var(.name, $value);
    }
    when Write {
      my $value = eval-expr($rt, .expr);
      $rt.write-output($value);
    }
    when While {
      loop {
        my $cond = eval-expr($rt, .cond);
        last if $cond == 0;
        exec-stmt($rt, $_) for .body;
      }
    }
  }
}

sub exec-program(Stmt @stmts --> Runtime) is export {
  my $rt = Runtime.new;
  try {
    exec-stmt($rt, $_) for @stmts;
    CATCH {
      default { fail "実行エラー: {.message}" }
    }
  }
  $rt
}

# === テスト ===

sub MAIN() {
  say "=== Raku PL/0 インタプリタ ===";

  # テスト1: カウントアップ
  # i := 0
  # while i - 10 do
  #   write i
  #   i := i + 1
  # end
  my @program1 = (
    Assign.new(name => 'i', expr => Number.new(value => 0)),
    While.new(
      cond => Binary.new(
        op => Sub,
        lhs => Var.new(name => 'i'),
        rhs => Number.new(value => 10)
      ),
      body => [
        Write.new(expr => Var.new(name => 'i')),
        Assign.new(
          name => 'i',
          expr => Binary.new(
            op => Add,
            lhs => Var.new(name => 'i'),
            rhs => Number.new(value => 1)
          )
        )
      ]
    )
  );

  try {
    my $rt1 = exec-program(@program1);
    say "出力: {@$rt1.output.join(', ')}";
    say "期待: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9";
  }
  CATCH {
    default { say "エラー: {.message}" }
  }

  say "";

  # テスト2: 階乗計算
  # n := 5
  # fact := 1
  # while n do
  #   fact := fact * n
  #   n := n - 1
  # end
  # write fact
  my @program2 = (
    Assign.new(name => 'n', expr => Number.new(value => 5)),
    Assign.new(name => 'fact', expr => Number.new(value => 1)),
    While.new(
      cond => Var.new(name => 'n'),
      body => [
        Assign.new(
          name => 'fact',
          expr => Binary.new(
            op => Mul,
            lhs => Var.new(name => 'fact'),
            rhs => Var.new(name => 'n')
          )
        ),
        Assign.new(
          name => 'n',
          expr => Binary.new(
            op => Sub,
            lhs => Var.new(name => 'n'),
            rhs => Number.new(value => 1)
          )
        )
      ]
    ),
    Write.new(expr => Var.new(name => 'fact'))
  );

  try {
    my $rt2 = exec-program(@program2);
    say "出力: {@$rt2.output.join(', ')}";
    say "期待: 120 (5の階乗)";
  }
  CATCH {
    default { say "エラー: {.message}" }
  }
}

# === Reml との比較メモ ===

=begin comment

1. **代数的データ型（ADT）**
   Raku: Role と Class の組み合わせで ADT を模倣
         - role は型クラス/トレイトのような役割
         - class でバリアントを実装
   Reml: 型定義で直接 `type Expr = Number(int) | Var(string) | ...` と記述
         - 構文が簡潔で、パターンマッチが自然

   - Reml の方が構文が簡潔
   - Raku は動的型付けなので、型チェックは実行時

2. **パターンマッチ**
   Raku: given/when でパターンマッチ（smart match）
         型による分岐は when Type で可能
   Reml: match 式で代数的データ型をパターンマッチ
         網羅性チェックがコンパイル時に行われる

   - Reml の方が型安全で、網羅性チェックが強力
   - Raku は柔軟だが、実行時エラーのリスクがある

3. **エラーハンドリング**
   Raku: 例外機構（try/CATCH）と fail（遅延例外）
         fail は呼び出し側でキャッチされるまで伝播
   Reml: Result<T, E> で型安全にエラーを扱う
         ? 演算子で簡潔にエラー伝播

   - Reml の方が関数型スタイルで統一されている
   - Raku は例外が主流だが、fail で遅延エラーも可能

4. **型システム**
   Raku: 動的型付け（gradual typing）
         型注釈はオプショナル、実行時にチェック
   Reml: 静的型付け、Hindley-Milner 型推論
         型注釈をほぼ省略可能、コンパイル時にチェック

   - Reml の方が型安全性が高い
   - Raku は柔軟だが、型エラーは実行時に発見

5. **実行モデル**
   Raku: インタプリタ（MoarVM バックエンド）
         JIT コンパイルで最適化されるが、起動時間が長い
   Reml: ネイティブコード生成を目指す
         高速な実行と短い起動時間

   - Reml の方が性能面で有利
   - Raku は開発速度と柔軟性に優れる

6. **Grammar との統合**
   Raku: Grammar で構文解析とインタプリタを統合可能
         Actions で AST を構築し、そのまま評価
   Reml: パーサーコンビネーターで AST を構築
         評価は別の関数で実装

   - Raku は Grammar から評価まで一貫した記述が可能
   - Reml は分離されているが、型安全性が高い

**結論**:
Raku は動的型付けで柔軟性が高く、Grammar との統合が優れている。
Reml は静的型付けで型安全性が高く、エラー品質の制御が細かい。
どちらも代数的データ型とパターンマッチをサポートするが、アプローチが異なる。

=end comment