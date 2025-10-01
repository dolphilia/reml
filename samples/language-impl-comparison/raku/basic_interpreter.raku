#!/usr/bin/env raku

=begin pod

=head1 Basic言語インタープリタ - Raku実装

Rakuの文法の柔軟性を活かしたBasicインタープリタ実装
- 型システムとマルチメソッドディスパッチ活用
- Grammarによる構文解析（この実装ではASTを直接構築）
- 演算子オーバーロードとカスタム型

=end pod

# ============================================================================
# データ型定義
# ============================================================================

role Value {
  method truthy() returns Bool { ... }
  method to-string() returns Str { ... }
}

class NumberValue does Value {
  has Numeric $.val;

  method truthy() returns Bool { $!val != 0 }
  method to-string() returns Str { ~$!val }
}

class StringValue does Value {
  has Str $.val;

  method truthy() returns Bool { $!val ne '' }
  method to-string() returns Str { $!val }
}

class ArrayValue does Value {
  has Value @.elements;

  method truthy() returns Bool { @!elements.elems > 0 }
  method to-string() returns Str { '[Array]' }
}

# ============================================================================
# 式の定義
# ============================================================================

role Expr { }

class NumberExpr does Expr {
  has Numeric $.val;
}

class StringExpr does Expr {
  has Str $.val;
}

class VariableExpr does Expr {
  has Str $.name;
}

class ArrayAccessExpr does Expr {
  has Str $.var;
  has Expr $.index;
}

enum BinOperator <Add Sub Mul Div Eq Ne Lt Le Gt Ge And Or>;

class BinOpExpr does Expr {
  has BinOperator $.op;
  has Expr $.left;
  has Expr $.right;
}

enum UnaryOperator <Neg Not>;

class UnaryOpExpr does Expr {
  has UnaryOperator $.op;
  has Expr $.operand;
}

# ============================================================================
# 文の定義
# ============================================================================

role Statement { }

class LetStmt does Statement {
  has Str $.var;
  has Expr $.expr;
}

class PrintStmt does Statement {
  has Expr @.exprs;
}

class IfStmt does Statement {
  has Expr $.cond;
  has Statement @.then-block;
  has Statement @.else-block;
}

class ForStmt does Statement {
  has Str $.var;
  has Expr $.start;
  has Expr $.end;
  has Expr $.step;
  has Statement @.body;
}

class WhileStmt does Statement {
  has Expr $.cond;
  has Statement @.body;
}

class GotoStmt does Statement {
  has Int $.line;
}

class GosubStmt does Statement {
  has Int $.line;
}

class ReturnStmt does Statement { }

class DimStmt does Statement {
  has Str $.var;
  has Expr $.size;
}

class EndStmt does Statement { }

# ============================================================================
# プログラムとランタイムステート
# ============================================================================

class ProgramLine {
  has Int $.line-num;
  has Statement $.stmt;
}

class RuntimeState {
  has %.env;
  has Int @.call-stack;
  has Str @.output;

  method add-output(Str $line) {
    @!output.push($line);
    self;
  }

  method set-var(Str $name, Value $value) {
    %!env{$name} = $value;
    self;
  }

  method get-var(Str $name) returns Value {
    %!env{$name} // Nil;
  }

  method push-call(Int $pc) {
    @!call-stack.push($pc);
    self;
  }

  method pop-call() {
    return Nil unless @!call-stack;
    @!call-stack.pop;
  }
}

# ============================================================================
# エラー定義
# ============================================================================

class RuntimeError is Exception {
  has Str $.message;
  method message() { $!message }
}

class UndefinedVariable is RuntimeError {
  has Str $.var-name;
  method message() { "未定義変数: $!var-name" }
}

class UndefinedLabel is RuntimeError {
  has Int $.label;
  method message() { "未定義ラベル: $!label" }
}

class TypeMismatch is RuntimeError {
  has Str $.expected;
  has Str $.got;
  method message() { "型不一致: 期待={$!expected}, 実際={$!got}" }
}

class IndexOutOfBounds is RuntimeError {
  method message() { "インデックス範囲外" }
}

class DivisionByZero is RuntimeError {
  method message() { "0で割ることはできません" }
}

class StackUnderflow is RuntimeError {
  method message() { "スタックアンダーフロー" }
}

# ============================================================================
# インタープリタ本体
# ============================================================================

class BasicInterpreter {

  # プログラム実行エントリーポイント
  method run(ProgramLine @program) returns Positional {
    my $state = RuntimeState.new;
    self!execute-program(@program, 0, $state);
    return $state.output;
  }

  # プログラム実行（再帰的）
  method !execute-program(ProgramLine @program, Int $pc, RuntimeState $state) returns RuntimeState {
    return $state if $pc >= @program.elems;

    my $stmt = @program[$pc].stmt;

    given $stmt {
      when EndStmt {
        return $state;
      }

      when LetStmt {
        my $value = self!eval-expr($stmt.expr, $state.env);
        $state.set-var($stmt.var, $value);
        return self!execute-program(@program, $pc + 1, $state);
      }

      when PrintStmt {
        my @values = $stmt.exprs.map: { self!eval-expr($_, $state.env) };
        my $text = @values.map(*.to-string).join(' ');
        $state.add-output($text);
        return self!execute-program(@program, $pc + 1, $state);
      }

      when IfStmt {
        my $cond-val = self!eval-expr($stmt.cond, $state.env);
        my @branch = $cond-val.truthy ?? $stmt.then-block !! $stmt.else-block;
        my $new-state = self!execute-block(@branch, $state);
        return self!execute-program(@program, $pc + 1, $new-state);
      }

      when ForStmt {
        my $start-val = self!eval-expr($stmt.start, $state.env);
        my $end-val = self!eval-expr($stmt.end, $state.env);
        my $step-val = self!eval-expr($stmt.step, $state.env);
        return self!execute-for-loop($stmt.var, $start-val, $end-val, $step-val,
                                     $stmt.body, @program, $pc, $state);
      }

      when WhileStmt {
        return self!execute-while-loop($stmt.cond, $stmt.body, @program, $pc, $state);
      }

      when GotoStmt {
        my $new-pc = self!find-line(@program, $stmt.line);
        return self!execute-program(@program, $new-pc, $state);
      }

      when GosubStmt {
        my $new-pc = self!find-line(@program, $stmt.line);
        $state.push-call($pc + 1);
        return self!execute-program(@program, $new-pc, $state);
      }

      when ReturnStmt {
        my $return-pc = $state.pop-call;
        die StackUnderflow.new unless $return-pc.defined;
        return self!execute-program(@program, $return-pc, $state);
      }

      when DimStmt {
        my $size-val = self!eval-expr($stmt.size, $state.env);
        die TypeMismatch.new(expected => 'Number', got => 'Other') unless $size-val ~~ NumberValue;
        my $size = $size-val.val.Int;
        my @array = (NumberValue.new(val => 0.0) xx $size).Array;
        $state.set-var($stmt.var, ArrayValue.new(elements => @array));
        return self!execute-program(@program, $pc + 1, $state);
      }
    }
  }

  # ブロック実行
  method !execute-block(Statement @block, RuntimeState $state) returns RuntimeState {
    my $current-state = $state;
    for @block -> $stmt {
      $current-state = self!execute-single-statement($stmt, $current-state);
    }
    return $current-state;
  }

  # 単一文の実行
  method !execute-single-statement(Statement $stmt, RuntimeState $state) returns RuntimeState {
    given $stmt {
      when LetStmt {
        my $value = self!eval-expr($stmt.expr, $state.env);
        $state.set-var($stmt.var, $value);
        return $state;
      }

      when PrintStmt {
        my @values = $stmt.exprs.map: { self!eval-expr($_, $state.env) };
        my $text = @values.map(*.to-string).join(' ');
        $state.add-output($text);
        return $state;
      }

      default {
        return $state;
      }
    }
  }

  # FORループ実行
  method !execute-for-loop(Str $var, Value $start, Value $end, Value $step,
                           Statement @body, ProgramLine @program, Int $pc,
                           RuntimeState $state) returns RuntimeState {
    die TypeMismatch.new(expected => 'Number', got => 'Other')
      unless $start ~~ NumberValue && $end ~~ NumberValue && $step ~~ NumberValue;

    return self!for-loop-helper($var, $start.val, $end.val, $step.val,
                                @body, @program, $pc, $state);
  }

  method !for-loop-helper(Str $var, Numeric $current, Numeric $end, Numeric $step,
                          Statement @body, ProgramLine @program, Int $pc,
                          RuntimeState $state) returns RuntimeState {
    if ($step > 0 && $current > $end) || ($step < 0 && $current < $end) {
      return self!execute-program(@program, $pc + 1, $state);
    }

    $state.set-var($var, NumberValue.new(val => $current));
    my $new-state = self!execute-block(@body, $state);
    return self!for-loop-helper($var, $current + $step, $end, $step,
                                @body, @program, $pc, $new-state);
  }

  # WHILEループ実行
  method !execute-while-loop(Expr $cond, Statement @body, ProgramLine @program,
                             Int $pc, RuntimeState $state) returns RuntimeState {
    my $cond-val = self!eval-expr($cond, $state.env);
    if $cond-val.truthy {
      my $new-state = self!execute-block(@body, $state);
      return self!execute-while-loop($cond, @body, @program, $pc, $new-state);
    } else {
      return self!execute-program(@program, $pc + 1, $state);
    }
  }

  # ============================================================================
  # 式評価
  # ============================================================================

  multi method !eval-expr(NumberExpr $expr, %env) returns Value {
    NumberValue.new(val => $expr.val);
  }

  multi method !eval-expr(StringExpr $expr, %env) returns Value {
    StringValue.new(val => $expr.val);
  }

  multi method !eval-expr(VariableExpr $expr, %env) returns Value {
    my $val = %env{$expr.name};
    die UndefinedVariable.new(var-name => $expr.name) unless $val.defined;
    return $val;
  }

  multi method !eval-expr(ArrayAccessExpr $expr, %env) returns Value {
    my $arr-val = %env{$expr.var};
    die UndefinedVariable.new(var-name => $expr.var) unless $arr-val.defined;
    die TypeMismatch.new(expected => 'Array', got => 'Other') unless $arr-val ~~ ArrayValue;

    my $idx-val = self!eval-expr($expr.index, %env);
    die TypeMismatch.new(expected => 'Number', got => 'Other') unless $idx-val ~~ NumberValue;

    my $index = $idx-val.val.Int;
    die IndexOutOfBounds.new if $index < 0 || $index >= $arr-val.elements.elems;
    return $arr-val.elements[$index];
  }

  multi method !eval-expr(BinOpExpr $expr, %env) returns Value {
    my $left = self!eval-expr($expr.left, %env);
    my $right = self!eval-expr($expr.right, %env);
    return self!eval-binary-op($expr.op, $left, $right);
  }

  multi method !eval-expr(UnaryOpExpr $expr, %env) returns Value {
    my $operand = self!eval-expr($expr.operand, %env);
    return self!eval-unary-op($expr.op, $operand);
  }

  # 二項演算子評価
  method !eval-binary-op(BinOperator $op, Value $left, Value $right) returns Value {
    given $op {
      when Add {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => $left.val + $right.val);
      }

      when Sub {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => $left.val - $right.val);
      }

      when Mul {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => $left.val * $right.val);
      }

      when Div {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        die DivisionByZero.new if $right.val == 0;
        return NumberValue.new(val => $left.val / $right.val);
      }

      when Eq {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => ($left.val == $right.val ?? 1.0 !! 0.0));
      }

      when Ne {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => ($left.val != $right.val ?? 1.0 !! 0.0));
      }

      when Lt {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => ($left.val < $right.val ?? 1.0 !! 0.0));
      }

      when Le {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => ($left.val <= $right.val ?? 1.0 !! 0.0));
      }

      when Gt {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => ($left.val > $right.val ?? 1.0 !! 0.0));
      }

      when Ge {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $left ~~ NumberValue && $right ~~ NumberValue;
        return NumberValue.new(val => ($left.val >= $right.val ?? 1.0 !! 0.0));
      }

      when And {
        return NumberValue.new(val => ($left.truthy && $right.truthy ?? 1.0 !! 0.0));
      }

      when Or {
        return NumberValue.new(val => ($left.truthy || $right.truthy ?? 1.0 !! 0.0));
      }
    }
  }

  # 単項演算子評価
  method !eval-unary-op(UnaryOperator $op, Value $operand) returns Value {
    given $op {
      when Neg {
        die TypeMismatch.new(expected => 'Number', got => 'Other')
          unless $operand ~~ NumberValue;
        return NumberValue.new(val => -$operand.val);
      }

      when Not {
        return NumberValue.new(val => ($operand.truthy ?? 0.0 !! 1.0));
      }
    }
  }

  # ============================================================================
  # ユーティリティ
  # ============================================================================

  method !find-line(ProgramLine @program, Int $target) returns Int {
    my $idx = @program.first(:k, *.line-num == $target);
    die UndefinedLabel.new(label => $target) unless $idx.defined;
    return $idx;
  }
}

# ============================================================================
# テスト例
# ============================================================================

sub MAIN() {
  # 10 LET x = 0
  # 20 LET x = x + 1
  # 30 PRINT x
  # 40 IF x < 5 THEN GOTO 20
  # 50 END

  my @program = (
    ProgramLine.new(
      line-num => 10,
      stmt => LetStmt.new(var => 'x', expr => NumberExpr.new(val => 0.0))
    ),
    ProgramLine.new(
      line-num => 20,
      stmt => LetStmt.new(
        var => 'x',
        expr => BinOpExpr.new(
          op => Add,
          left => VariableExpr.new(name => 'x'),
          right => NumberExpr.new(val => 1.0)
        )
      )
    ),
    ProgramLine.new(
      line-num => 30,
      stmt => PrintStmt.new(exprs => [VariableExpr.new(name => 'x')])
    ),
    ProgramLine.new(
      line-num => 40,
      stmt => IfStmt.new(
        cond => BinOpExpr.new(
          op => Lt,
          left => VariableExpr.new(name => 'x'),
          right => NumberExpr.new(val => 5.0)
        ),
        then-block => [GotoStmt.new(line => 20)],
        else-block => []
      )
    ),
    ProgramLine.new(
      line-num => 50,
      stmt => EndStmt.new
    )
  );

  my $interpreter = BasicInterpreter.new;

  try {
    my @output = $interpreter.run(@program);
    say "実行結果:";
    .say for @output;

    CATCH {
      default {
        say "エラー: {.message}";
      }
    }
  }
}
