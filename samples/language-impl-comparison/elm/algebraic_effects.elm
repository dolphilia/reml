module AlgebraicEffects exposing (..)

{-| 代数的効果を Elm でエミュレートする実装

Reml との比較:
- Reml: 言語レベルで effect/handler をサポート
- Elm: モナド風の Result/State を手動で管理
- Elm は純粋関数型のため、効果は明示的にデータ構造で表現

-}

-- ミニ言語の式定義


type Expr
    = Lit Int
    | Var String
    | Add Expr Expr
    | Mul Expr Expr
    | Div Expr Expr
    | Get
    | Put Expr
    | Fail String
    | Choose Expr Expr


type alias Env =
    List ( String, Int )


{-| 効果をデータ構造で表現

State: (結果, 状態) のペア
Except: Result 型
Choose: リスト（複数の結果）

-}
type alias EffectResult a =
    -- State<Int> × Except<String> × Choose
    Result String (List ( a, Int ))


{-| 式の評価関数（効果を持つ）

Reml の perform に相当する操作を手動で記述：
- State.get → 状態を引数から取得
- State.put → 新しい状態を返す
- Except.raise → Err を返す
- Choose → リストを flatMap で結合

-}
eval : Expr -> Env -> Int -> EffectResult Int
eval expr env state =
    case expr of
        Lit n ->
            Ok [ ( n, state ) ]

        Var name ->
            case lookupEnv name env of
                Just value ->
                    Ok [ ( value, state ) ]

                Nothing ->
                    Err ("未定義変数: " ++ name)

        Add left right ->
            eval left env state
                |> Result.andThen
                    (\leftResults ->
                        flatMapResults
                            (\( l, s1 ) ->
                                eval right env s1
                                    |> Result.map
                                        (List.map (\( r, s2 ) -> ( l + r, s2 )))
                            )
                            leftResults
                    )

        Mul left right ->
            eval left env state
                |> Result.andThen
                    (\leftResults ->
                        flatMapResults
                            (\( l, s1 ) ->
                                eval right env s1
                                    |> Result.map
                                        (List.map (\( r, s2 ) -> ( l * r, s2 )))
                            )
                            leftResults
                    )

        Div left right ->
            eval left env state
                |> Result.andThen
                    (\leftResults ->
                        flatMapResults
                            (\( l, s1 ) ->
                                eval right env s1
                                    |> Result.andThen
                                        (\rightResults ->
                                            flatMapResults
                                                (\( r, s2 ) ->
                                                    if r == 0 then
                                                        Err "ゼロ除算"

                                                    else
                                                        Ok [ ( l // r, s2 ) ]
                                                )
                                                rightResults
                                        )
                            )
                            leftResults
                    )

        Get ->
            Ok [ ( state, state ) ]

        Put e ->
            eval e env state
                |> Result.map (List.map (\( v, _ ) -> ( v, v )))

        Fail msg ->
            Err msg

        Choose left right ->
            Result.map2 (++)
                (eval left env state)
                (eval right env state)


{-| 環境から変数を検索
-}
lookupEnv : String -> Env -> Maybe Int
lookupEnv name env =
    case env of
        [] ->
            Nothing

        ( k, v ) :: rest ->
            if k == name then
                Just v

            else
                lookupEnv name rest


{-| リストの結果を flatMap する補助関数
-}
flatMapResults : (( a, Int ) -> Result String (List ( b, Int ))) -> List ( a, Int ) -> Result String (List ( b, Int ))
flatMapResults f results =
    List.foldl
        (\item acc ->
            Result.map2 (++)
                acc
                (f item)
        )
        (Ok [])
        results


{-| すべての効果を処理して結果を返す

Reml の handle ... do ... do ... に相当するが、
Elm では型を合わせるために手動で Result/List を管理。

-}
runWithAllEffects : Expr -> Env -> Int -> EffectResult Int
runWithAllEffects expr env initState =
    eval expr env initState


{-| テストケース
-}
exampleExpressions : List ( String, Expr )
exampleExpressions =
    [ ( "単純な加算", Add (Lit 10) (Lit 20) )
    , ( "乗算と除算", Div (Mul (Lit 6) (Lit 7)) (Lit 2) )
    , ( "状態の取得", Add Get (Lit 5) )
    , ( "状態の更新", Put (Add Get (Lit 1)) )
    , ( "ゼロ除算エラー", Div (Lit 10) (Lit 0) )
    , ( "非決定的選択", Choose (Lit 1) (Lit 2) )
    , ( "複雑な例"
      , Add
            (Choose (Lit 10) (Lit 20))
            (Put (Add Get (Lit 1)))
      )
    ]


{-| テスト実行関数（デバッグ用）

Elm は副作用が制限されているため、実際の実行は
Main モジュールで Html.text 経由で表示する必要がある。

-}
runExamples : List String
runExamples =
    let
        env =
            []

        initState =
            0
    in
    List.map
        (\( name, expr ) ->
            case runWithAllEffects expr env initState of
                Ok results ->
                    "--- "
                        ++ name
                        ++ " ---\n"
                        ++ String.join "\n"
                            (List.map
                                (\( value, st ) ->
                                    "  結果: "
                                        ++ String.fromInt value
                                        ++ ", 状態: "
                                        ++ String.fromInt st
                                )
                                results
                            )

                Err err ->
                    "--- " ++ name ++ " ---\n  エラー: " ++ err
        )
        exampleExpressions


{-| Reml との比較メモ

1.  **効果の表現**
    Reml: effect State<S> { operation get() -> S; ... }
    Elm: Result String (List ( a, Int )) で手動エンコード

      - Reml は言語が効果を追跡
      - Elm は型を明示的に積み重ねる（Result × List × タプル）

2.  **ハンドラーの実装**
    Reml: handler state\_handler<A>(init) for State<S> { ... }
    Elm: eval 関数内で Result.andThen と List.map を手動合成

      - Reml はハンドラーが宣言的で再利用可能
      - Elm は毎回手動で Result/List を処理

3.  **非決定性の扱い**
    Reml: choose\_handler でリストを自動収集
    Elm: Result.map2 (++) で手動結合

      - Reml は分岐が自動的に追跡される
      - Elm は明示的なリスト操作が必要

4.  **可読性**
    Reml: 効果の意図が型シグネチャで明確
    Elm: ネストした Result.andThen が読みづらい

5.  **型安全性**
    Reml: 効果が型レベルで強制される
    Elm: Result/List の型で安全だが、ボイラープレートが多い

**結論**:
Elm の純粋関数型アプローチは安全だが、代数的効果の表現には向いていない。
Reml の effect/handler 構文はより宣言的で、複雑な効果の合成を簡潔に記述できる。

-}