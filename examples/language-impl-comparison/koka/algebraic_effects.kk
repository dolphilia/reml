// 代数的効果を使うミニ言語 - Koka 版
// Reml との核心的な比較: 効果システムの設計、ハンドラーの記法、型推論

module examples/language-impl-comparison/algebraic-effects

import std/core

// ミニ言語の式定義
type expr
  Lit(n: int)
  Var(name: string)
  Add(left: expr, right: expr)
  Mul(left: expr, right: expr)
  Div(left: expr, right: expr)
  Get                         // 状態を取得
  Put(e: expr)                // 状態を更新
  Fail(msg: string)           // 例外を送出
  Choose(left: expr, right: expr)  // 非決定的選択

// 環境：変数束縛
alias env = list<(string, int)>

// === 効果の定義 ===

// Koka 組み込みの効果：
// - exn: 例外効果（組み込み、throw/catch で利用）
// - div: 発散の可能性（組み込み、無限ループなど）
// - ndet: 非決定性（組み込み、ambiguous で利用可能）

// 状態効果（カスタム定義）
effect state<s>
  fun get() : s
  fun put(x : s) : ()

// 非決定性効果（カスタム定義 - より明示的に）
effect choose
  fun choice<a>(left: a, right: a) : a

// === 式の評価 ===

// 評価関数（効果を持つ）
// Koka: 効果型 <state<int>, exn, choose> が自動推論される
// Reml: with State<Int>, Except<String>, Choose と明示可能
fun eval(expr: expr, environ: env) : <state<int>, exn, choose> int
  match expr
    Lit(n) -> n

    Var(name) ->
      match environ.lookup(name)
        Just(value) -> value
        Nothing -> throw("未定義変数: " ++ name)

    Add(left, right) ->
      val l = eval(left, environ)
      val r = eval(right, environ)
      l + r

    Mul(left, right) ->
      val l = eval(left, environ)
      val r = eval(right, environ)
      l * r

    Div(left, right) ->
      val l = eval(left, environ)
      val r = eval(right, environ)
      if r == 0 then
        throw("ゼロ除算")
      else
        l / r

    Get ->
      get()

    Put(e) ->
      val v = eval(e, environ)
      put(v)
      v

    Fail(msg) ->
      throw(msg)

    Choose(left, right) ->
      // 非決定的選択: 両方の分岐を評価
      choice(eval(left, environ), eval(right, environ))

// === 効果ハンドラー ===

// 状態ハンドラー: State<s> を処理
// 状態を初期値 init から開始し、最終状態と結果をペアで返す
//
// Koka のハンドラー構文:
// - handle(action) で効果を持つ計算をハンドル
// - fun operation() -> resume(...) で効果演算をインターセプト
// - return x -> ... で正常終了時の処理
fun state-handler(init: s, action: () -> <state<s>|e> a) : e (a, s)
  var current := init
  handle(action)
    return x -> (x, current)
    fun get() -> resume(current)
    fun put(new-state) -> { current := new-state; resume(()) }

// 例外ハンドラー: exn 効果を Result 型に変換
// Koka 組み込みの try-catch を使用する方が自然だが、
// Reml との比較のため明示的なハンドラー風の関数で表現
fun except-handler(action: () -> <exn|e> a) : e maybe<a>
  try
    Just(action())
  fn(msg)
    Nothing

// 非決定性ハンドラー: すべての選択肢を試す
// Koka: リストを返すことで複数の結果を集約
fun choose-handler(action: () -> <choose|e> a) : e list<a>
  handle(action)
    return x -> [x]
    fun choice(left, right) ->
      resume(left) ++ resume(right)

// === 効果の合成例 ===

// すべての効果を処理（State → Except → Choose の順）
// Koka: ハンドラーをネストして合成
// Reml: handle ... do ... do ... で同等
pub fun run-with-all-effects(expr: expr, environ: env, init-state: int) : maybe<list<(int, int)>>
  except-handler fn()
    val results = choose-handler fn()
      state-handler(init-state) fn()
        eval(expr, environ)
    results.map(fn((value, state)) (value, state))

// State のみをハンドル（Except と Choose は伝播）
pub fun run-with-state(expr: expr, environ: env, init-state: int) : <exn, choose> (int, int)
  state-handler(init-state) fn()
    eval(expr, environ)

// === テストケース ===

// 簡易的なテスト例
pub fun example-expressions() : list<(string, expr)>
  [ ("単純な加算", Add(Lit(10), Lit(20)))
  , ("乗算と除算", Div(Mul(Lit(6), Lit(7)), Lit(2)))
  , ("状態の取得", Add(Get, Lit(5)))
  , ("状態の更新", Put(Add(Get, Lit(1))))
  , ("ゼロ除算エラー", Div(Lit(10), Lit(0)))
  , ("非決定的選択", Choose(Lit(1), Lit(2)))
  , ("複雑な例", Add(
      Choose(Lit(10), Lit(20)),
      Put(Add(Get, Lit(1)))
    ))
  ]

// 環境の検索
fun lookup(environ: env, name: string) : maybe<int>
  match environ
    Nil -> Nothing
    Cons((k, v), rest) ->
      if k == name then Just(v) else rest.lookup(name)

// === Reml との比較メモ ===

// 1. **効果の定義**
//    Koka: effect state<s> { fun get() : s; fun put(x : s) : () }
//    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
//    - 構文はほぼ同一
//    - Koka は fun キーワード、Reml は operation キーワード

// 2. **効果ハンドラー**
//    Koka: handle(action) { return x -> ...; fun get() -> resume(...) }
//    Reml: handler state_handler<A>(init) for State<S> { operation get() resume -> ...; return value -> ... }
//    - Koka: handle 式でインラインに記述、var による状態管理が自然
//    - Reml: handler 宣言で名前付き、関数型スタイルで状態を明示的に渡す

// 3. **効果の型推論**
//    Koka: <state<int>, exn, choose> が自動推論される
//    Reml: with State<Int>, Except<String>, Choose で明示可能
//    - どちらも型推論をサポート
//    - Koka は型シグネチャでの明示がより一般的
//    - Reml は with 節でより読みやすく表現

// 4. **ハンドラーの合成**
//    Koka: ネストした関数呼び出しで合成
//          except-handler fn() choose-handler fn() state-handler(init) fn() ...
//    Reml: handle ... do ... do ... で視覚的に明確
//          handle state_handler(init) do
//            handle except_handler() do
//              handle choose_handler() do
//                eval(expr, env)
//    - Reml の構文が若干読みやすい

// 5. **resumption（継続）の扱い**
//    Koka: resume(value) で継続を呼び出し
//    Reml: resume(value) で同様に継続を呼び出し
//    - ほぼ同等の意味論

// 6. **標準効果**
//    Koka: exn（例外）、div（発散）、ndet（非決定性）が組み込み
//    Reml: Except、Div、Choose をライブラリで定義
//    - Koka は組み込みが多い
//    - Reml はユーザー定義効果として統一的に扱う

// **結論**:
// Koka と Reml の効果システムは非常に類似しており、
// どちらも代数的効果とハンドラーを中心に設計されている。
// 主な違いは構文の細部（fun vs operation、handle式 vs handler宣言）と
// 標準効果の扱い（組み込み vs ライブラリ定義）。
//
// Reml は Koka の設計を参考にしつつ、以下の点で差別化:
// - より統一的な effect/handler 構文
// - with 節による効果の明示がより読みやすい
// - パーサーコンビネーターなど、言語実装に最適化された標準ライブラリ