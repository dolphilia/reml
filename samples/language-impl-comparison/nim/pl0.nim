# PL/0 風トイ言語コンパイラ断片 - Nim 版
# Reml との比較ポイント: 代数的データ型、再帰的評価、状態管理

import std/[tables, strutils, options]

# PL/0 風サブセットの抽象構文木
type
  Stmt = ref object
    case kind: StmtKind
    of skAssign:
      assignName: string
      assignExpr: Expr
    of skWhile:
      whileCond: Expr
      whileBody: seq[Stmt]
    of skWrite:
      writeExpr: Expr

  StmtKind = enum
    skAssign, skWhile, skWrite

  # 式は 4 則演算のみ対応
  Expr = ref object
    case kind: ExprKind
    of ekNumber: numVal: int
    of ekVar: varName: string
    of ekBinary:
      op: Op
      lhs, rhs: Expr

  ExprKind = enum
    ekNumber, ekVar, ekBinary

  Op = enum
    opAdd, opSub, opMul, opDiv

  # 実行時の状態
  Runtime = object
    vars: Table[string, int]
    output: seq[int]

  ParseError = object of CatchableError
  ExecError = object of CatchableError

# === コンストラクタヘルパー ===

proc newNumber(n: int): Expr =
  Expr(kind: ekNumber, numVal: n)

proc newVar(name: string): Expr =
  Expr(kind: ekVar, varName: name)

proc newBinary(op: Op, lhs, rhs: Expr): Expr =
  Expr(kind: ekBinary, op: op, lhs: lhs, rhs: rhs)

proc newAssign(name: string, expr: Expr): Stmt =
  Stmt(kind: skAssign, assignName: name, assignExpr: expr)

proc newWhile(cond: Expr, body: seq[Stmt]): Stmt =
  Stmt(kind: skWhile, whileCond: cond, whileBody: body)

proc newWrite(expr: Expr): Stmt =
  Stmt(kind: skWrite, writeExpr: expr)

# === プログラムのパース（簡略化された疑似実装） ===

proc parseProgram*(source: string): seq[Stmt] =
  # 実装のシンプルさを優先し、ハードコードされた例を返す
  @[
    newAssign("x", newNumber(5)),
    newWhile(
      newVar("x"),
      @[
        newWrite(newVar("x")),
        newAssign("x", newBinary(opSub, newVar("x"), newNumber(1)))
      ]
    )
  ]

# === 式の評価 ===

proc evalExpr(expr: Expr, vars: Table[string, int]): int =
  case expr.kind
  of ekNumber:
    return expr.numVal
  of ekVar:
    if not vars.hasKey(expr.varName):
      raise newException(ExecError, "未定義変数: " & expr.varName)
    return vars[expr.varName]
  of ekBinary:
    let l = evalExpr(expr.lhs, vars)
    let r = evalExpr(expr.rhs, vars)
    case expr.op
    of opAdd: return l + r
    of opSub: return l - r
    of opMul: return l * r
    of opDiv:
      if r == 0:
        raise newException(ExecError, "0 で割ることはできません")
      return l div r

# === 文の実行 ===

proc execStmt(stmt: Stmt, runtime: var Runtime) =
  case stmt.kind
  of skAssign:
    let value = evalExpr(stmt.assignExpr, runtime.vars)
    runtime.vars[stmt.assignName] = value

  of skWhile:
    while true:
      let condValue = evalExpr(stmt.whileCond, runtime.vars)
      if condValue == 0:
        break
      for s in stmt.whileBody:
        execStmt(s, runtime)

  of skWrite:
    let value = evalExpr(stmt.writeExpr, runtime.vars)
    runtime.output.add(value)

# === プログラム全体の実行 ===

proc exec*(program: seq[Stmt]): Runtime =
  result = Runtime(vars: initTable[string, int](), output: @[])
  for stmt in program:
    execStmt(stmt, result)

# === テスト ===

when isMainModule:
  echo "=== Nim PL/0 風コンパイラ ==="

  try:
    let program = parseProgram("")
    let runtime = exec(program)
    echo "Output: ", runtime.output  # => [5, 4, 3, 2, 1]
  except ExecError as e:
    echo "Exec error: ", e.msg

# === Reml との比較メモ ===

# 1. **代数的データ型（ADT）**
#    Nim: object variant (case object) で ADT 風に記述
#         `case kind: StmtKind` で型タグを明示的に持つ
#    Reml: 型定義で直接 `type Stmt = Assign {...} | While {...} | Write {...}` と記述
#         型タグは暗黙的に管理される
#    - Reml の方が構文が簡潔で、パターンマッチがより自然

# 2. **状態管理**
#    Nim: var パラメータで可変状態を明示的に渡す
#         Table[string, int] で変数環境を管理
#    Reml: Map<Text, i64> で同様に管理
#         var と不変変数の両方をサポート
#    - どちらも明示的な状態管理が可能
#    - Nim は var が必須、Reml は不要な場合は省略可能

# 3. **エラーハンドリング**
#    Nim: 例外機構（try-except）が主流
#         Option 型もサポート
#    Reml: Result<T, E> を標準で提供し、? 演算子で簡潔に記述
#    - Reml の方が関数型スタイルに統一

# 4. **性能**
#    Nim: C バックエンドにより、非常に高速
#         ゼロコスト抽象化が可能
#    Reml: 実装次第だが、同等の性能を目指す
#    - Nim は既に成熟した高性能言語

# 5. **マクロによる AST 操作**
#    Nim: マクロで AST を直接操作可能
#         compile-time execution で強力なメタプログラミング
#         例: AST を変換して最適化や DSL 構築
#    Reml: マクロは標準では提供せず、パーサーコンビネーターで記述
#    - Nim の優位性: コンパイル時の AST 操作が非常に強力
#    - Reml の優位性: 構文が単純で、学習コストが低い

# 6. **メモリ管理**
#    Nim: ARC（Automatic Reference Counting）+ ORC（Owned References Counting）
#         ガーベジコレクションなしで決定論的なメモリ管理
#    Reml: 実装方式は仕様では規定しない（処理系依存）
#         ARC を想定
#    - どちらも決定論的なメモリ管理を目指す

# **結論**:
# Nim は成熟した高性能言語で、マクロシステムが非常に強力。
# Reml はマクロなしでも、パーサーコンビネーターと型システムで
# 同等の表現力を目指し、言語実装に最適化された設計。