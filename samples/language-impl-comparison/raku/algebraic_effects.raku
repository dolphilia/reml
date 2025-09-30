#!/usr/bin/env raku

# 代数的効果を使うミニ言語 - Raku 版
# Reml との比較: 動的型と例外機構による効果のエミュレーション

# ミニ言語の式定義
enum ExprKind <Lit Var Add Mul Div Get Put Fail Choose>;

class Expr {
  has ExprKind $.kind;
  has $.value;
}

sub lit(Int $n) { Expr.new(kind => Lit, value => $n) }
sub var(Str $name) { Expr.new(kind => Var, value => $name) }
sub add(Expr $l, Expr $r) { Expr.new(kind => Add, value => [$l, $r]) }
sub mul(Expr $l, Expr $r) { Expr.new(kind => Mul, value => [$l, $r]) }
sub divide(Expr $l, Expr $r) { Expr.new(kind => Div, value => [$l, $r]) }
sub get-state() { Expr.new(kind => Get, value => Nil) }
sub put-state(Expr $e) { Expr.new(kind => Put, value => $e) }
sub fail(Str $msg) { Expr.new(kind => Fail, value => $msg) }
sub choose(Expr $l, Expr $r) { Expr.new(kind => Choose, value => [$l, $r]) }

# 環境：変数束縛
class Env {
  has %.bindings;

  method lookup(Str $name) {
    %.bindings{$name}:exists ?? %.bindings{$name} !! Nil
  }
}

# 効果の結果型
# State<Int> × Except<String> × Choose をハッシュで表現
class EffectResult {
  has Bool $.success;
  has Str $.error;
  has @.results; # Array of (value, state) pairs
}

# 式の評価関数（効果を持つ）
#
# Reml の perform に相当する操作を手動で記述：
# - State: state を引数で渡して結果と共に返す
# - Except: EffectResult.success = False で表現
# - Choose: results をリストで収集
sub eval(Expr $expr, Env $env, Int $state) returns EffectResult {
  given $expr.kind {
    when Lit {
      EffectResult.new(success => True, error => '', results => [($expr.value, $state)])
    }

    when Var {
      my $value = $env.lookup($expr.value);
      if $value.defined {
        EffectResult.new(success => True, error => '', results => [($value, $state)])
      } else {
        EffectResult.new(success => False, error => "未定義変数: $expr.value()", results => [])
      }
    }

    when Add {
      my ($left, $right) = $expr.value.list;
      my $left-result = eval($left, $env, $state);
      return $left-result unless $left-result.success;

      my @all-results;
      for $left-result.results.list -> ($l-value, $l-state) {
        my $right-result = eval($right, $env, $l-state);
        return $right-result unless $right-result.success;
        for $right-result.results.list -> ($r-value, $r-state) {
          @all-results.push(($l-value + $r-value, $r-state));
        }
      }
      EffectResult.new(success => True, error => '', results => @all-results)
    }

    when Mul {
      my ($left, $right) = $expr.value.list;
      my $left-result = eval($left, $env, $state);
      return $left-result unless $left-result.success;

      my @all-results;
      for $left-result.results.list -> ($l-value, $l-state) {
        my $right-result = eval($right, $env, $l-state);
        return $right-result unless $right-result.success;
        for $right-result.results.list -> ($r-value, $r-state) {
          @all-results.push(($l-value * $r-value, $r-state));
        }
      }
      EffectResult.new(success => True, error => '', results => @all-results)
    }

    when Div {
      my ($left, $right) = $expr.value.list;
      my $left-result = eval($left, $env, $state);
      return $left-result unless $left-result.success;

      my @all-results;
      for $left-result.results.list -> ($l-value, $l-state) {
        my $right-result = eval($right, $env, $l-state);
        return $right-result unless $right-result.success;
        for $right-result.results.list -> ($r-value, $r-state) {
          if $r-value == 0 {
            return EffectResult.new(success => False, error => 'ゼロ除算', results => []);
          }
          @all-results.push(($l-value div $r-value, $r-state));
        }
      }
      EffectResult.new(success => True, error => '', results => @all-results)
    }

    when Get {
      EffectResult.new(success => True, error => '', results => [($state, $state)])
    }

    when Put {
      my $e = $expr.value;
      my $result = eval($e, $env, $state);
      return $result unless $result.success;
      my @all-results = $result.results.map(-> ($v, $) { ($v, $v) });
      EffectResult.new(success => True, error => '', results => @all-results)
    }

    when Fail {
      EffectResult.new(success => False, error => $expr.value, results => [])
    }

    when Choose {
      my ($left, $right) = $expr.value.list;
      my $left-result = eval($left, $env, $state);
      return $left-result unless $left-result.success;
      my $right-result = eval($right, $env, $state);
      return $right-result unless $right-result.success;
      my @combined = |$left-result.results, |$right-result.results;
      EffectResult.new(success => True, error => '', results => @combined)
    }
  }
}

# すべての効果を処理して結果を返す
#
# Reml の handle ... do ... do ... に相当するが、
# Raku では手動で Result を検査して分岐。
sub run-with-all-effects(Expr $expr, Env $env, Int $init-state) returns EffectResult {
  eval($expr, $env, $init-state)
}

# テストケース
sub example-expressions() {
  [
    ("単純な加算", add(lit(10), lit(20))),
    ("乗算と除算", divide(mul(lit(6), lit(7)), lit(2))),
    ("状態の取得", add(get-state(), lit(5))),
    ("状態の更新", put-state(add(get-state(), lit(1)))),
    ("ゼロ除算エラー", divide(lit(10), lit(0))),
    ("非決定的選択", choose(lit(1), lit(2))),
    ("複雑な例", add(
      choose(lit(10), lit(20)),
      put-state(add(get-state(), lit(1)))
    ))
  ]
}

# テスト実行関数
sub run-examples() {
  my @examples = example-expressions();
  my $env = Env.new(bindings => {});
  my $init-state = 0;

  for @examples -> ($name, $expr) {
    say "--- $name ---";
    my $result = run-with-all-effects($expr, $env, $init-state);
    if $result.success {
      for $result.results.list -> ($value, $state) {
        say "  結果: $value, 状態: $state";
      }
    } else {
      say "  エラー: $result.error()";
    }
  }
}

# Reml との比較メモ:
#
# 1. **効果の表現**
#    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
#    Raku: class EffectResult { has Bool $.success; ... }
#    - Reml は言語レベルで効果を定義
#    - Raku は動的型でクラスを使って手動管理
#
# 2. **ハンドラーの実装**
#    Reml: handler state_handler<A>(init) for State<S> { ... }
#    Raku: eval 関数内で state を明示的に渡す
#    - Reml はハンドラーが宣言的
#    - Raku は手続き的でエラーハンドリングが煩雑
#
# 3. **非決定性の扱い**
#    Reml: choose_handler で分岐を自動収集
#    Raku: results をリストで手動管理
#    - Reml は分岐が自然に追跡される
#    - Raku は明示的なリスト操作が必要
#
# 4. **型安全性**
#    Reml: 効果が型レベルで強制される
#    Raku: 動的型のため実行時エラーのリスク
#    - Reml の方が型安全
#
# 5. **可読性**
#    Reml: with State<Int>, Except<String>, Choose で効果が明確
#    Raku: given/when による分岐が多い
#    - Reml の方が効果の意図が分かりやすい
#
# 6. **パフォーマンス**
#    Reml: 効果はコンパイル時に最適化可能
#    Raku: 動的型のオーバーヘッドが常に発生
#    - Reml の方が高速
#
# **結論**:
# Raku の動的型システムは柔軟だが、代数的効果の表現には向いていない。
# Reml の effect/handler 構文はより宣言的で、型安全性が高い。

# テスト実行例
# run-examples();