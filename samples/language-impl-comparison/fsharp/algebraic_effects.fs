// 代数的効果を使うミニ言語 - F# 版
// Reml との比較: コンピュテーション式による効果のエミュレーション

namespace AlgebraicEffects

open System

// ミニ言語の式定義
type Expr =
    | Lit of int
    | Var of string
    | Add of Expr * Expr
    | Mul of Expr * Expr
    | Div of Expr * Expr
    | Get
    | Put of Expr
    | Fail of string
    | Choose of Expr * Expr

type Env = (string * int) list

/// 効果をデータ構造で表現
/// State<Int> × Except<String> × Choose をモナドスタックとして実装
type EffectResult<'a> = int -> Result<('a * int) list, string>

module Effect =
    /// return（純粋な値）
    let ret (x: 'a) : EffectResult<'a> =
        fun state -> Ok [(x, state)]

    /// bind（モナド合成）
    let bind (m: EffectResult<'a>) (f: 'a -> int -> EffectResult<'b>) : EffectResult<'b> =
        fun state ->
            match m state with
            | Error err -> Error err
            | Ok results ->
                let folder acc (value, st) =
                    match acc with
                    | Error _ -> acc
                    | Ok accList ->
                        match f value st st with
                        | Error err -> Error err
                        | Ok newResults -> Ok (accList @ newResults)
                List.fold folder (Ok []) results

    /// map（関数適用）
    let map (f: 'a -> 'b) (m: EffectResult<'a>) : EffectResult<'b> =
        fun state ->
            match m state with
            | Ok results -> Ok (List.map (fun (v, s) -> (f v, s)) results)
            | Error err -> Error err

    /// State.get
    let get : EffectResult<int> =
        fun state -> Ok [(state, state)]

    /// State.put
    let put (newState: int) : EffectResult<unit> =
        fun _ -> Ok [((), newState)]

    /// Except.raise
    let raise (msg: string) : EffectResult<'a> =
        fun _ -> Error msg

    /// Choose（非決定的選択）
    let choose (left: EffectResult<'a>) (right: EffectResult<'a>) : EffectResult<'a> =
        fun state ->
            match left state, right state with
            | Ok l, Ok r -> Ok (l @ r)
            | Error err, _ -> Error err
            | _, Error err -> Error err

/// コンピュテーション式（モナド構文）
type EffectBuilder() =
    member _.Return(x) = Effect.ret x
    member _.Bind(m, f) = Effect.bind m (fun v _ -> f v)
    member _.ReturnFrom(m) = m
    member _.Zero() = Effect.ret ()

let effect = EffectBuilder()

module Eval =
    /// 環境から変数を検索
    let lookupEnv (name: string) (env: Env) : int option =
        List.tryFind (fun (k, _) -> k = name) env
        |> Option.map snd

    /// 式の評価関数（効果を持つ）
    ///
    /// Reml の perform に相当する操作をコンピュテーション式で記述：
    /// - let! による bind
    /// - Effect.get, Effect.put, Effect.raise で効果を発行
    let rec eval (expr: Expr) (env: Env) : EffectResult<int> =
        match expr with
        | Lit n ->
            Effect.ret n

        | Var name ->
            match lookupEnv name env with
            | Some value -> Effect.ret value
            | None -> Effect.raise $"未定義変数: {name}"

        | Add (left, right) ->
            effect {
                let! l = eval left env
                let! r = eval right env
                return l + r
            }

        | Mul (left, right) ->
            effect {
                let! l = eval left env
                let! r = eval right env
                return l * r
            }

        | Div (left, right) ->
            effect {
                let! l = eval left env
                let! r = eval right env
                if r = 0 then
                    return! Effect.raise "ゼロ除算"
                else
                    return l / r
            }

        | Get ->
            Effect.get

        | Put e ->
            effect {
                let! v = eval e env
                do! Effect.put v
                return v
            }

        | Fail msg ->
            Effect.raise msg

        | Choose (left, right) ->
            Effect.choose (eval left env) (eval right env)

    /// すべての効果を処理して結果を返す
    ///
    /// Reml の handle ... do ... do ... に相当するが、
    /// F# ではコンピュテーション式で State × Except × Choose を管理。
    let runWithAllEffects (expr: Expr) (env: Env) (initState: int) : Result<(int * int) list, string> =
        eval expr env initState

    /// テストケース
    let exampleExpressions : (string * Expr) list =
        [ ("単純な加算", Add (Lit 10, Lit 20))
          ("乗算と除算", Div (Mul (Lit 6, Lit 7), Lit 2))
          ("状態の取得", Add (Get, Lit 5))
          ("状態の更新", Put (Add (Get, Lit 1)))
          ("ゼロ除算エラー", Div (Lit 10, Lit 0))
          ("非決定的選択", Choose (Lit 1, Lit 2))
          ("複雑な例", Add (
              Choose (Lit 10, Lit 20),
              Put (Add (Get, Lit 1))
          )) ]

    /// テスト実行関数
    let runExamples () =
        let env = []
        let initState = 0

        exampleExpressions
        |> List.iter (fun (name, expr) ->
            printfn "--- %s ---" name
            match runWithAllEffects expr env initState with
            | Ok results ->
                results
                |> List.iter (fun (value, state) ->
                    printfn "  結果: %d, 状態: %d" value state
                )
            | Error err ->
                printfn "  エラー: %s" err
        )

// Reml との比較メモ:
//
// 1. **効果の表現**
//    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
//    F#: type EffectResult<'a> = int -> Result<('a * int) list, string>
//    - Reml は言語レベルで効果を定義
//    - F# は関数型（State モナド風）でエンコード
//
// 2. **ハンドラーの実装**
//    Reml: handler state_handler<A>(init) for State<S> { ... }
//    F#: コンピュテーション式（let! / do!）で bind を連鎖
//    - Reml はハンドラーが明示的で再利用可能
//    - F# は暗黙的な bind でモナド的に合成
//
// 3. **非決定性の扱い**
//    Reml: choose_handler で分岐を自動収集
//    F#: Effect.choose で手動でリストを結合
//    - どちらもリストを使うが、Reml の方が宣言的
//
// 4. **型推論**
//    Reml: 効果が型レベルで推論される
//    F#: EffectResult<'a> の型注釈が必要な場合が多い
//    - Reml の方が型注釈を省略しやすい
//
// 5. **可読性**
//    Reml: with State<Int>, Except<String>, Choose で効果が明確
//    F#: コンピュテーション式でモナド的に記述（慣れが必要）
//    - Reml の方が効果の意図が分かりやすい
//
// **結論**:
// F# のコンピュテーション式は強力だが、代数的効果の表現には向いていない。
// Reml の effect/handler 構文はより直感的で、効果の合成が容易。

// テスト実行例（コメントアウト）
// Eval.runExamples()