(* Hindley-Milner 型推論器実装 *)
(* Algorithm W による単一化ベース型推論 *)

(* 型変数の生成カウンター *)
let tv_counter = ref 0

let reset_counter () = tv_counter := 0

let fresh_tyvar () =
  let n = !tv_counter in
  tv_counter := n + 1;
  n

(* 型の定義 *)
type ty =
  | TVar of tyvar ref        (* 型変数（参照による破壊的単一化） *)
  | TInt                     (* 整数型 *)
  | TBool                    (* 真偽値型 *)
  | TFun of ty * ty          (* 関数型 *)

and tyvar =
  | Unbound of int * int     (* 未束縛型変数（id, level） *)
  | Link of ty               (* 他の型へのリンク *)

(* 式の定義 *)
type expr =
  | EVar of string
  | EInt of int
  | EBool of bool
  | ELam of string * expr
  | EApp of expr * expr
  | ELet of string * expr * expr
  | EIf of expr * expr * expr
  | EBinOp of binop * expr * expr

and binop = Add | Sub | Mul | Eq | Lt

(* 型環境 *)
module Env = Map.Make(String)
type env = ty Env.t

(* 型スキーム（多相型） *)
type scheme = Forall of int list * ty

(* 型の文字列化 *)
let rec string_of_ty ty =
  match ty with
  | TVar {contents = Link ty'} -> string_of_ty ty'
  | TVar {contents = Unbound (id, _)} -> "'t" ^ string_of_int id
  | TInt -> "Int"
  | TBool -> "Bool"
  | TFun (t1, t2) ->
      let s1 = match t1 with
        | TFun _ -> "(" ^ string_of_ty t1 ^ ")"
        | _ -> string_of_ty t1
      in
      s1 ^ " -> " ^ string_of_ty t2

(* 型変数の出現チェック（無限型防止） *)
let rec occurs tvr level ty =
  match ty with
  | TVar tvr' when tvr == tvr' -> true
  | TVar ({contents = Unbound (id, other_level)} as other_tvr) ->
      (* レベル調整（let多相のため） *)
      let min_level = min level other_level in
      other_tvr := Unbound (id, min_level);
      false
  | TVar {contents = Link ty'} -> occurs tvr level ty'
  | TFun (t1, t2) -> occurs tvr level t1 || occurs tvr level t2
  | TInt | TBool -> false

(* 単一化 *)
let rec unify ty1 ty2 =
  match ty1, ty2 with
  | TVar {contents = Link ty1'}, ty2 -> unify ty1' ty2
  | ty1, TVar {contents = Link ty2'} -> unify ty1 ty2'
  | TVar ({contents = Unbound (id1, _)} as tvr1),
    TVar {contents = Unbound (id2, _)} when id1 = id2 ->
      () (* 同じ型変数 *)
  | TVar ({contents = Unbound (_, level)} as tvr), ty
  | ty, TVar ({contents = Unbound (_, level)} as tvr) ->
      if occurs tvr level ty then
        failwith "Occurs check failed: infinite type"
      else
        tvr := Link ty
  | TInt, TInt -> ()
  | TBool, TBool -> ()
  | TFun (t1, t2), TFun (t3, t4) ->
      unify t1 t3;
      unify t2 t4
  | _ ->
      failwith ("Cannot unify " ^ string_of_ty ty1 ^ " and " ^ string_of_ty ty2)

(* 型の一般化（多相化） *)
let generalize level ty =
  let rec collect_vars ty =
    match ty with
    | TVar {contents = Unbound (id, other_level)} when other_level > level ->
        [id]
    | TVar {contents = Link ty'} -> collect_vars ty'
    | TFun (t1, t2) -> collect_vars t1 @ collect_vars t2
    | _ -> []
  in
  let vars = collect_vars ty in
  let unique_vars = List.sort_uniq compare vars in
  Forall (unique_vars, ty)

(* 型の具体化（多相型のインスタンス化） *)
let instantiate level (Forall (vars, ty)) =
  let subst = List.fold_left (fun acc id ->
    (id, TVar (ref (Unbound (fresh_tyvar (), level)))) :: acc
  ) [] vars in
  let rec apply ty =
    match ty with
    | TVar {contents = Unbound (id, _)} ->
        (try List.assoc id subst with Not_found -> ty)
    | TVar {contents = Link ty'} -> apply ty'
    | TFun (t1, t2) -> TFun (apply t1, apply t2)
    | _ -> ty
  in
  apply ty

(* 型推論（Algorithm W） *)
let rec infer env level expr =
  match expr with
  | EVar name ->
      (try
        let scheme = Env.find name env in
        instantiate level scheme
      with Not_found ->
        failwith ("Unbound variable: " ^ name))

  | EInt _ -> TInt
  | EBool _ -> TBool

  | ELam (param, body) ->
      let param_ty = TVar (ref (Unbound (fresh_tyvar (), level))) in
      let param_scheme = Forall ([], param_ty) in
      let env' = Env.add param param_scheme env in
      let body_ty = infer env' level body in
      TFun (param_ty, body_ty)

  | EApp (func, arg) ->
      let func_ty = infer env level func in
      let arg_ty = infer env level arg in
      let result_ty = TVar (ref (Unbound (fresh_tyvar (), level))) in
      unify func_ty (TFun (arg_ty, result_ty));
      result_ty

  | ELet (name, value, body) ->
      (* let多相のためレベルを上げる *)
      let value_ty = infer env (level + 1) value in
      let value_scheme = generalize level value_ty in
      let env' = Env.add name value_scheme env in
      infer env' level body

  | EIf (cond, then_br, else_br) ->
      let cond_ty = infer env level cond in
      unify cond_ty TBool;
      let then_ty = infer env level then_br in
      let else_ty = infer env level else_br in
      unify then_ty else_ty;
      then_ty

  | EBinOp (op, e1, e2) ->
      let t1 = infer env level e1 in
      let t2 = infer env level e2 in
      match op with
      | Add | Sub | Mul ->
          unify t1 TInt;
          unify t2 TInt;
          TInt
      | Eq | Lt ->
          unify t1 TInt;
          unify t2 TInt;
          TBool

(* トップレベル推論 *)
let infer_expr expr =
  reset_counter ();
  let ty = infer Env.empty 0 expr in
  generalize (-1) ty

(* 式の文字列化 *)
let rec string_of_expr = function
  | EVar x -> x
  | EInt n -> string_of_int n
  | EBool b -> string_of_bool b
  | ELam (x, e) -> "(λ" ^ x ^ ". " ^ string_of_expr e ^ ")"
  | EApp (e1, e2) -> "(" ^ string_of_expr e1 ^ " " ^ string_of_expr e2 ^ ")"
  | ELet (x, e1, e2) ->
      "(let " ^ x ^ " = " ^ string_of_expr e1 ^ " in " ^ string_of_expr e2 ^ ")"
  | EIf (e1, e2, e3) ->
      "(if " ^ string_of_expr e1 ^ " then " ^ string_of_expr e2 ^ " else " ^ string_of_expr e3 ^ ")"
  | EBinOp (op, e1, e2) ->
      let op_str = match op with
        | Add -> "+" | Sub -> "-" | Mul -> "*" | Eq -> "==" | Lt -> "<"
      in
      "(" ^ string_of_expr e1 ^ " " ^ op_str ^ " " ^ string_of_expr e2 ^ ")"

(* テスト実行 *)
let test name expr expected =
  try
    let Forall (_, ty) = infer_expr expr in
    let ty_str = string_of_ty ty in
    if ty_str = expected then
      Printf.printf "PASS: %s : %s\n" name ty_str
    else
      Printf.printf "FAIL: %s : %s (expected: %s)\n" name ty_str expected
  with Failure msg ->
    Printf.printf "ERROR: %s : %s\n" name msg

let () =
  print_endline "=== Hindley-Milner Type Inference Tests ===";

  (* 基本型 *)
  test "int literal" (EInt 42) "Int";
  test "bool literal" (EBool true) "Bool";

  (* 関数型 *)
  test "identity" (ELam ("x", EVar "x")) "'t0 -> 't0";
  test "const" (ELam ("x", ELam ("y", EVar "x"))) "'t0 -> 't1 -> 't0";

  (* 関数適用 *)
  test "application"
    (EApp (ELam ("x", EVar "x"), EInt 42))
    "Int";

  (* let多相 *)
  test "let polymorphism"
    (ELet ("id", ELam ("x", EVar "x"),
      EApp (EVar "id", EInt 42)))
    "Int";

  test "let polymorphism (multiple use)"
    (ELet ("id", ELam ("x", EVar "x"),
      EIf (EApp (EVar "id", EBool true),
           EApp (EVar "id", EInt 1),
           EApp (EVar "id", EInt 2))))
    "Int";

  (* 二項演算子 *)
  test "addition"
    (EBinOp (Add, EInt 1, EInt 2))
    "Int";

  test "comparison"
    (EBinOp (Lt, EInt 1, EInt 2))
    "Bool";

  (* 高階関数 *)
  test "compose"
    (ELam ("f", ELam ("g", ELam ("x",
      EApp (EVar "f", EApp (EVar "g", EVar "x"))))))
    "('t2 -> 't3) -> ('t1 -> 't2) -> 't1 -> 't3";

  test "apply"
    (ELam ("f", ELam ("x", EApp (EVar "f", EVar "x"))))
    "('t1 -> 't2) -> 't1 -> 't2";

  (* 再帰的let（Y-combinator風） *)
  test "factorial type"
    (ELet ("fac", ELam ("n",
      EIf (EBinOp (Eq, EVar "n", EInt 0),
           EInt 1,
           EBinOp (Mul, EVar "n",
             EApp (EVar "fac", EBinOp (Sub, EVar "n", EInt 1))))),
      EApp (EVar "fac", EInt 5)))
    "Int";

  (* エラーケース *)
  print_endline "\n=== Error Cases ===";
  test "type mismatch"
    (EIf (EInt 1, EInt 2, EInt 3))
    "Error";

  test "occurs check"
    (ELam ("x", EApp (EVar "x", EVar "x")))
    "Error";
