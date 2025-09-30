(* 代数的効果を使うミニ言語 - OCaml 5 版 *)
(* Reml との比較: 言語レベルの代数的効果サポート *)

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

(* === 効果の定義 === *)

(* 状態効果：可変状態の読み書き *)
effect Get : int
effect Put : int -> unit

(* 例外効果：エラーの送出 *)
effect Raise : string -> 'a

(* 非決定性効果：複数の選択肢を生成 *)
effect Choose : int * int -> int

(* 環境から変数を検索 *)
let lookup_env (name : string) (env : env) : int option =
  List.assoc_opt name env

(* 式の評価関数（効果を持つ） *)
(*
   Reml の perform に相当する操作を perform で記述：
   - perform Get: 状態を取得
   - perform (Put v): 状態を更新
   - perform (Raise msg): 例外を送出
   - perform (Choose (l, r)): 非決定的選択
*)
let rec eval (expr : expr) (env : env) : int =
  match expr with
  | Lit n ->
    n

  | Var name ->
    (match lookup_env name env with
     | Some value -> value
     | None -> perform (Raise (Printf.sprintf "未定義変数: %s" name)))

  | Add (left, right) ->
    let l = eval left env in
    let r = eval right env in
    l + r

  | Mul (left, right) ->
    let l = eval left env in
    let r = eval right env in
    l * r

  | Div (left, right) ->
    let l = eval left env in
    let r = eval right env in
    if r = 0 then
      perform (Raise "ゼロ除算")
    else
      l / r

  | Get ->
    perform Get

  | Put e ->
    let v = eval e env in
    perform (Put v);
    v

  | Fail msg ->
    perform (Raise msg)

  | Choose (left, right) ->
    let l = eval left env in
    let r = eval right env in
    perform (Choose (l, r))

(* === 効果ハンドラー === *)

(* 状態ハンドラー: State を処理 *)
(* 状態を初期値 init_state から開始し、最終状態と結果をペアで返す *)
let state_handler (init_state : int) (f : unit -> 'a) : 'a * int =
  let state = ref init_state in
  let result =
    match f () with
    | value -> value
    | effect Get k ->
      continue k !state
    | effect (Put new_state) k ->
      state := new_state;
      continue k ()
  in
  (result, !state)

(* 例外ハンドラー: Raise を処理 *)
(* 例外を捕捉して result 型に変換 *)
let except_handler (f : unit -> 'a) : ('a, string) result =
  match f () with
  | value -> Ok value
  | effect (Raise msg) _k ->
    Error msg

(* 非決定性ハンドラー: Choose を処理（リスト収集版） *)
(* すべての選択肢を試し、結果のリストを返す *)
let choose_handler (f : unit -> 'a) : 'a list =
  match f () with
  | value -> [value]
  | effect (Choose (left, right)) k ->
    let left_results = continue k left in
    let right_results = continue k right in
    left_results @ right_results

(* === 効果ハンドラーの合成例 === *)

(* すべての効果を処理（State → Except → Choose の順） *)
(* 最終的に result<(int * int) list, string> を返す *)
let run_with_all_effects (expr : expr) (env : env) (init_state : int) : ((int * int) list, string) result =
  try
    except_handler (fun () ->
      let results = choose_handler (fun () ->
        state_handler init_state (fun () ->
          eval expr env
        )
      ) in
      List.map (fun (value, state) -> (value, state)) results
    )
  with
  | _ -> Error "予期しないエラー"

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
 * 1. **効果の定義**
 *    Reml: effect State<S> { operation get() -> S; operation put(s: S) -> () }
 *    OCaml 5: effect Get : int; effect Put : int -> unit
 *    - 構文がほぼ同一
 *    - Reml は operation キーワード、OCaml 5 は effect 宣言で直接
 *
 * 2. **効果ハンドラー**
 *    Reml: handler state_handler<A>(init) for State<S> { operation get() resume -> ...; return value -> ... }
 *    OCaml 5: match f () with | value -> ... | effect Get k -> continue k ...
 *    - Reml: handler 宣言で名前付き、より構造化された記法
 *    - OCaml 5: match式でインライン記述、より直接的だが冗長
 *
 * 3. **効果の型推論**
 *    Reml: with State<Int>, Except<String>, Choose で明示可能（省略も可）
 *    OCaml 5: 効果の型推論は限定的（明示的な型注釈が推奨される）
 *    - Reml の方が型注釈を省略しやすい
 *
 * 4. **ハンドラーの合成**
 *    Reml: handle state_handler(init) do handle except_handler() do ...
 *    OCaml 5: ネストした関数呼び出しで合成
 *    - Reml の構文が視覚的に明確
 *
 * 5. **resumption（継続）の扱い**
 *    Reml: resume(value) で継続を呼び出し
 *    OCaml 5: continue k value で継続を呼び出し
 *    - ほぼ同等の意味論
 *
 * 6. **標準効果**
 *    Reml: Except、Choose をライブラリで定義
 *    OCaml 5: ユーザーが全て定義（標準効果なし）
 *    - Reml の方が一貫したエコシステム
 *
 * 7. **エラーメッセージ**
 *    Reml: 効果システムと統合されたエラー報告
 *    OCaml 5: 標準的なコンパイラエラー
 *    - Reml の方が効果関連のエラーが分かりやすい
 *
 * **結論**:
 * OCaml 5 と Reml の効果システムは非常に類似しているが、
 * Reml の方が以下の点で優れている：
 * - handler 宣言による構造化された記法
 * - より自然な型推論
 * - with 節による効果の明示的な表現
 * - パーサーコンビネーターなど、言語実装に最適化された標準ライブラリ
 *
 * OCaml 5 は代数的効果の先駆的な実装だが、
 * Reml はその設計を参考にしつつ、より洗練された構文と
 * 言語実装用途に特化した機能を提供する。
 *)

(* テスト実行例 *)
(* let () = run_examples () *)