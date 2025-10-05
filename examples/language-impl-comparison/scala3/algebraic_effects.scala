// 代数的効果を使うミニ言語 - Scala 3 版
// Reml との比較: コンテキスト関数とモナド風の効果エミュレーション

import scala.util.{Try, Success, Failure}

// ミニ言語の式定義
enum Expr:
  case Lit(n: Int)
  case Var(name: String)
  case Add(left: Expr, right: Expr)
  case Mul(left: Expr, right: Expr)
  case Div(left: Expr, right: Expr)
  case Get
  case Put(expr: Expr)
  case Fail(msg: String)
  case Choose(left: Expr, right: Expr)

type Env = Map[String, Int]

// 効果をデータ構造で表現
// State<Int> × Except<String> × Choose をモナドスタックとして実装
type EffectResult[A] = Int => Either[String, List[(A, Int)]]

object Effect:
  // return（純粋な値）
  def pure[A](x: A): EffectResult[A] =
    state => Right(List((x, state)))

  // bind（モナド合成）
  def flatMap[A, B](m: EffectResult[A])(f: A => Int => EffectResult[B]): EffectResult[B] =
    state =>
      m(state) match
        case Left(err) => Left(err)
        case Right(results) =>
          results.foldLeft[Either[String, List[(B, Int)]]](Right(Nil)) { (acc, item) =>
            val (value, st) = item
            acc match
              case Left(_) => acc
              case Right(accList) =>
                f(value)(st)(st) match
                  case Left(err) => Left(err)
                  case Right(newResults) => Right(accList ++ newResults)
          }

  // map（関数適用）
  def map[A, B](m: EffectResult[A])(f: A => B): EffectResult[B] =
    state =>
      m(state) match
        case Right(results) => Right(results.map { case (v, s) => (f(v), s) })
        case Left(err) => Left(err)

  // State.get
  def get: EffectResult[Int] =
    state => Right(List((state, state)))

  // State.put
  def put(newState: Int): EffectResult[Unit] =
    _ => Right(List(((), newState)))

  // Except.raise
  def raise[A](msg: String): EffectResult[A] =
    _ => Left(msg)

  // Choose（非決定的選択）
  def choose[A](left: EffectResult[A], right: EffectResult[A]): EffectResult[A] =
    state =>
      (left(state), right(state)) match
        case (Right(l), Right(r)) => Right(l ++ r)
        case (Left(err), _) => Left(err)
        case (_, Left(err)) => Left(err)

// 式の評価関数（効果を持つ）
//
// Reml の perform に相当する操作を for 式で記述：
// - for { ... } yield ... による flatMap 連鎖
// - Effect.get, Effect.put, Effect.raise で効果を発行
def eval(expr: Expr, env: Env): EffectResult[Int] =
  import Expr.*
  import Effect.*

  expr match
    case Lit(n) =>
      pure(n)

    case Var(name) =>
      env.get(name) match
        case Some(value) => pure(value)
        case None => raise(s"未定義変数: $name")

    case Add(left, right) =>
      flatMap(eval(left, env)) { l => _ =>
        flatMap(eval(right, env)) { r => _ =>
          pure(l + r)
        }
      }

    case Mul(left, right) =>
      flatMap(eval(left, env)) { l => _ =>
        flatMap(eval(right, env)) { r => _ =>
          pure(l * r)
        }
      }

    case Div(left, right) =>
      flatMap(eval(left, env)) { l => _ =>
        flatMap(eval(right, env)) { r => _ =>
          if r == 0 then
            raise("ゼロ除算")
          else
            pure(l / r)
        }
      }

    case Get =>
      get

    case Put(e) =>
      flatMap(eval(e, env)) { v => _ =>
        flatMap(put(v)) { _ => _ =>
          pure(v)
        }
      }

    case Fail(msg) =>
      raise(msg)

    case Choose(left, right) =>
      choose(eval(left, env), eval(right, env))

// すべての効果を処理して結果を返す
//
// Reml の handle ... do ... do ... に相当するが、
// Scala 3 では関数呼び出しで State × Except × Choose を管理。
def runWithAllEffects(expr: Expr, env: Env, initState: Int): Either[String, List[(Int, Int)]] =
  eval(expr, env)(initState)

// テストケース
def exampleExpressions: List[(String, Expr)] =
  import Expr.*
  List(
    ("単純な加算", Add(Lit(10), Lit(20))),
    ("乗算と除算", Div(Mul(Lit(6), Lit(7)), Lit(2))),
    ("状態の取得", Add(Get, Lit(5))),
    ("状態の更新", Put(Add(Get, Lit(1)))),
    ("ゼロ除算エラー", Div(Lit(10), Lit(0))),
    ("非決定的選択", Choose(Lit(1), Lit(2))),
    ("複雑な例", Add(
      Choose(Lit(10), Lit(20)),
      Put(Add(Get, Lit(1)))
    ))
  )

// テスト実行関数
def runExamples(): Unit =
  val env = Map.empty[String, Int]
  val initState = 0

  exampleExpressions.foreach { (name, expr) =>
    println(s"--- $name ---")
    runWithAllEffects(expr, env, initState) match
      case Right(results) =>
        results.foreach { (value, state) =>
          println(s"  結果: $value, 状態: $state")
        }
      case Left(err) =>
        println(s"  エラー: $err")
  }

// Reml との比較メモ:
//
// 1. **効果の表現**
//    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
//    Scala 3: type EffectResult[A] = Int => Either[String, List[(A, Int)]]
//    - Reml は言語レベルで効果を定義
//    - Scala 3 は関数型（State モナド風）でエンコード
//
// 2. **ハンドラーの実装**
//    Reml: handler state_handler<A>(init) for State<S> { ... }
//    Scala 3: Effect オブジェクトで flatMap/pure を実装
//    - Reml はハンドラーが明示的で再利用可能
//    - Scala 3 はモナド的に合成（ネストが深くなる）
//
// 3. **非決定性の扱い**
//    Reml: choose_handler で分岐を自動収集
//    Scala 3: Effect.choose で手動でリストを結合
//    - どちらもリストを使うが、Reml の方が宣言的
//
// 4. **型推論**
//    Reml: 効果が型レベルで推論される
//    Scala 3: 型注釈が必要な場合が多い
//    - Reml の方が型注釈を省略しやすい
//
// 5. **可読性**
//    Reml: with State<Int>, Except<String>, Choose で効果が明確
//    Scala 3: flatMap のネストが読みづらい
//    - Reml の方が効果の意図が分かりやすい
//
// 6. **for 式の活用**
//    Scala 3: for 式でモナド構文を簡潔に記述可能
//    Reml: handle ... do ... 構文で同等の記述
//    - どちらも簡潔だが、Reml の方が効果の種類が明確
//
// **結論**:
// Scala 3 の関数型アプローチは強力だが、代数的効果の表現には向いていない。
// Reml の effect/handler 構文はより直感的で、効果の合成が容易。
// 特に複雑な効果の組み合わせで、Reml の方が記述性に優れる。

// テスト実行例（main 関数）
// @main def run(): Unit = runExamples()