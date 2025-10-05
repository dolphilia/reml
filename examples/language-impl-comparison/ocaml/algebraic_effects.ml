(* 代数的効果を使うミニ言語 - OCaml 4.x 版 *)
(* Reml との比較: 例外と Result 型による効果のエミュレーション *)
(* 注: OCaml 4.x には代数的効果がないため、モナド風に実装 *)

(* ミニ言語の式定義 *)
type expr =
  | Lit of int
  | Var of string
  | Add of expr * expr
  | Mul of expr * expr
  | Div of expr * expr
  | Get
  | Put of expr
  | Fail of string
  | Choose of expr * expr

type env = (string * int) list

(* 効果をデータ構造で表現 *)
(* State<Int> × Except<String> × Choose *)
type 'a effect_result = int -> (('a * int) list, string) result

(* 効果モナドの操作 *)
module Effect = struct
  (* return（純粋な値） *)
  let return (x : 'a) : 'a effect_result =
    fun state -> Ok [(x, state)]

  (* bind（モナド合成） *)
  let bind (m : 'a effect_result) (f : 'a -> int -> 'b effect_result) : 'b effect_result =
    fun state ->
      match m state with
      | Error err -> Error err
      | Ok results ->
        List.fold_left
          (fun acc (value, st) ->
            match acc with
            | Error _ -> acc
            | Ok acc_list ->
              match f value st st with
              | Error err -> Error err
              | Ok new_results -> Ok (acc_list @ new_results))
          (Ok [])
          results

  (* map（関数適用） *)
  let map (f : 'a -> 'b) (m : 'a effect_result) : 'b effect_result =
    fun state ->
      match m state with
      | Ok results -> Ok (List.map (fun (v, s) -> (f v, s)) results)
      | Error err -> Error err

  (* State.get *)
  let get : int effect_result =
    fun state -> Ok [(state, state)]

  (* State.put *)
  let put (new_state : int) : unit effect_result =
    fun _ -> Ok [((), new_state)]

  (* Except.raise *)
  let raise (msg : string) : 'a effect_result =
    fun _ -> Error msg

  (* Choose（非決定的選択） *)
  let choose (left : 'a effect_result) (right : 'a effect_result) : 'a effect_result =
    fun state ->
      match (left state, right state) with
      | (Ok l, Ok r) -> Ok (l @ r)
      | (Error err, _) -> Error err
      | (_, Error err) -> Error err
end

(* 環境から変数を検索 *)
let lookup_env (name : string) (env : env) : int option =
  List.assoc_opt name env

(* 式の評価関数（効果を持つ） *)
(*
   Reml の perform に相当する操作をモナド操作で記述：
   - let* による bind（OCaml 4.08+）
   - Effect.get, Effect.put, Effect.raise で効果を発行
*)
let rec eval (expr : expr) (env : env) : int effect_result =
  let ( let* ) = Effect.bind in
  match expr with
  | Lit n ->
    Effect.return n

  | Var name ->
    (match lookup_env name env with
     | Some value -> Effect.return value
     | None -> Effect.raise (Printf.sprintf "未定義変数: %s" name))

  | Add (left, right) ->
    let* l = eval left env in
    let* r = eval right env in
    Effect.return (l + r)

  | Mul (left, right) ->
    let* l = eval left env in
    let* r = eval right env in
    Effect.return (l * r)

  | Div (left, right) ->
    let* l = eval left env in
    let* r = eval right env in
    if r = 0 then
      Effect.raise "ゼロ除算"
    else
      Effect.return (l / r)

  | Get ->
    Effect.get

  | Put e ->
    let* v = eval e env in
    let* () = Effect.put v in
    Effect.return v

  | Fail msg ->
    Effect.raise msg

  | Choose (left, right) ->
    Effect.choose (eval left env) (eval right env)

(* すべての効果を処理して結果を返す *)
(*
   Reml の handle ... do ... do ... に相当するが、
   OCaml 4.x ではモナド操作で State × Except × Choose を管理。
*)
let run_with_all_effects (expr : expr) (env : env) (init_state : int) : ((int * int) list, string) result =
  eval expr env init_state

(* テストケース *)
let example_expressions : (string * expr) list =
  [ ("単純な加算", Add (Lit 10, Lit 20))
  ; ("乗算と除算", Div (Mul (Lit 6, Lit 7), Lit 2))
  ; ("状態の取得", Add (Get, Lit 5))
  ; ("状態の更新", Put (Add (Get, Lit 1)))
  ; ("ゼロ除算エラー", Div (Lit 10, Lit 0))
  ; ("非決定的選択", Choose (Lit 1, Lit 2))
  ; ("複雑な例", Add (
      Choose (Lit 10, Lit 20),
      Put (Add (Get, Lit 1))
    ))
  ]

(* テスト実行関数 *)
let run_examples () =
  let env = [] in
  let init_state = 0 in
  List.iter
    (fun (name, expr) ->
      Printf.printf "--- %s ---\n" name;
      match run_with_all_effects expr env init_state with
      | Ok results ->
        List.iter
          (fun (value, state) ->
            Printf.printf "  結果: %d, 状態: %d\n" value state)
          results
      | Error err ->
        Printf.printf "  エラー: %s\n" err)
    example_expressions

(* Reml との比較メモ:
 *
 * 1. **効果の表現**
 *    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
 *    OCaml 4.x: type 'a effect_result = int -> (('a * int) list, string) result
 *    - Reml は言語レベルで効果を定義
 *    - OCaml 4.x はモナド風の関数型でエンコード
 *
 * 2. **ハンドラーの実装**
 *    Reml: handler state_handler<A>(init) for State<S> { ... }
 *    OCaml 4.x: Effect モジュールで bind/return を実装
 *    - Reml はハンドラーが明示的で再利用可能
 *    - OCaml 4.x は let* 構文でモナド的に合成
 *
 * 3. **非決定性の扱い**
 *    Reml: choose_handler で分岐を自動収集
 *    OCaml 4.x: Effect.choose で手動でリストを結合
 *    - どちらもリストを使うが、Reml の方が宣言的
 *
 * 4. **型推論**
 *    Reml: 効果が型レベルで推論される
 *    OCaml 4.x: 型注釈が必要な場合が多い
 *    - Reml の方が型注釈を省略しやすい
 *
 * 5. **可読性**
 *    Reml: with State<Int>, Except<String>, Choose で効果が明確
 *    OCaml 4.x: let* によるモナド構文（慣れが必要）
 *    - Reml の方が効果の意図が分かりやすい
 *
 * 6. **OCaml 5 との比較**
 *    OCaml 5: 代数的効果が言語レベルでサポートされる
 *    OCaml 4.x: モナド風のエミュレーションが必要
 *    - Reml は OCaml 5 と同様の設計（より洗練された構文）
 *
 * **結論**:
 * OCaml 4.x のモナドアプローチは安全だが、代数的効果の表現には向いていない。
 * Reml の effect/handler 構文はより直感的で、効果の合成が容易。
 * OCaml 5 の代数的効果に近いが、Reml の方が構文が洗練されている。
 *)

(* テスト実行例 *)
(* let () = run_examples () *)