(* Type_inference — Type Inference Engine for Reml (Phase 2)
 *
 * このファイルはHindley-Milner型推論エンジンの実装を提供する。
 * 仕様書 1-2 §C（型推論）に従い、制約ベース推論を実装する。
 *
 * 設計原則:
 * - 制約収集と制約解決の分離
 * - let多相の一般化とインスタンス化
 * - 型注釈の統合
 *)

open Types
open Type_env
open Constraint
open Type_error
open Ast
open Typed_ast

(* ========== 型注釈の変換 ========== *)

(** AST型注釈をTypes.tyに変換
 *
 * Phase 2では基本型のみサポート（型クラスは後半で実装）
 *)
let rec convert_type_annot (tannot: type_annot) : ty =
  match tannot.ty_kind with
  | TyIdent id ->
      (* 組み込み型の識別 *)
      (match id.name with
       (* 整数型 *)
       | "i8" -> ty_i8
       | "i16" -> ty_i16
       | "i32" -> ty_i32
       | "i64" -> ty_i64
       | "isize" -> ty_isize
       | "u8" -> ty_u8
       | "u16" -> ty_u16
       | "u32" -> ty_u32
       | "u64" -> ty_u64
       | "usize" -> ty_usize
       (* 浮動小数型 *)
       | "f32" -> ty_f64
       | "f64" -> ty_f64
       (* 基本型 *)
       | "Bool" -> ty_bool
       | "Char" -> ty_char
       | "String" -> ty_string
       | "()" -> ty_unit
       | "Never" -> ty_never
       (* ユーザ定義型 *)
       | name -> TCon (TCUser name))

  | TyApp (base_id, args) ->
      (* 型適用: Vec<T>, Option<T> など *)
      let base_ty = TCon (TCUser base_id.name) in
      List.fold_left (fun acc arg ->
        TApp (acc, convert_type_annot arg)
      ) base_ty args

  | TyTuple tys ->
      (* タプル型 *)
      TTuple (List.map convert_type_annot tys)

  | TyRecord fields ->
      (* レコード型 *)
      TRecord (List.map (fun (field_id, field_ty) ->
        (field_id.name, convert_type_annot field_ty)
      ) fields)

  | TyFn (arg_tys, ret_ty) ->
      (* 関数型: (A, B) -> C を A -> B -> C に変換 *)
      List.fold_right (fun arg_ty acc ->
        TArrow (convert_type_annot arg_ty, acc)
      ) arg_tys (convert_type_annot ret_ty)

(* ========== 一般化とインスタンス化 ========== *)

(** 型の一般化: generalize(env, τ)
 *
 * 仕様書 1-2 §C.1: let束縛で自由型変数を量化
 *)
let generalize (env: env) (ty: ty) : type_scheme =
  let env_vars = ftv_env env in
  let ty_vars = ftv_ty ty in
  (* 環境に出現しない自由変数を量化 *)
  let quantified = List.filter (fun tv ->
    not (List.exists (fun env_tv -> env_tv.tv_id = tv.tv_id) env_vars)
  ) ty_vars in
  { quantified; body = ty }

(** 型スキームのインスタンス化: instantiate(scheme)
 *
 * 量化変数を新鮮な型変数で置き換え
 *)
let instantiate (scheme: type_scheme) : ty =
  if scheme.quantified = [] then
    scheme.body
  else
    (* 量化変数 → 新鮮な型変数のマッピングを作成 *)
    let subst = List.map (fun qtv ->
      let fresh_var = TypeVarGen.fresh None in
      (qtv, Types.TVar fresh_var)
    ) scheme.quantified in
    apply_subst subst scheme.body

(* ========== 型推論エンジン ========== *)

(** 推論結果: 型付き式、推論された型、代入 *)
type infer_result = typed_expr * ty * substitution

(** let* 演算子（Result モナド） *)
let (let*) = Result.bind

(** リテラルの型推論 *)
let infer_literal (lit: literal) (_span: span) : ty =
  match lit with
  | Int (_, _) -> ty_i64                        (* Phase 2: デフォルト i64 *)
  | Float _ -> ty_f64                           (* Phase 2: デフォルト f64 *)
  | Char _ -> ty_char
  | String (_, _) -> ty_string
  | Bool _ -> ty_bool
  | Unit -> ty_unit
  | Tuple _ | Array _ | Record _ ->
      (* 複合リテラルはPhase 2後半で実装 *)
      failwith "Composite literals not yet implemented"

(** 式の型推論: infer_expr(env, expr)
 *
 * Phase 2 Week 2-3: 基本的な式の推論を実装
 *)
let rec infer_expr (env: env) (expr: expr) : (infer_result, type_error) result =
  match expr.expr_kind with
  | Literal lit ->
      let ty = infer_literal lit expr.expr_span in
      let texpr = make_typed_expr (TLiteral lit) ty expr.expr_span in
      Ok (texpr, ty, empty_subst)

  | Var id ->
      (* 変数参照: 型環境から検索してインスタンス化 *)
      (match lookup id.name env with
       | Some scheme ->
           let ty = instantiate scheme in
           let texpr = make_typed_expr (Typed_ast.TVar (id, scheme)) ty expr.expr_span in
           Ok (texpr, ty, empty_subst)
       | None ->
           Error (unbound_variable_error id.name expr.expr_span))

  | Call (fn_expr, args) ->
      (* 関数適用の型推論
       *
       * 1. 関数式を推論
       * 2. 引数を推論
       * 3. 関数型を構築して単一化
       * 4. 返り値型を返す
       *)
      let* (tfn, fn_ty, s1) = infer_expr env fn_expr in

      (* 引数を推論 *)
      let* (targs, arg_tys, s2) = infer_args (apply_subst_env s1 env) args s1 in

      (* 返り値型用の新鮮な型変数 *)
      let ret_var = TypeVarGen.fresh None in
      let ret_ty = Types.TVar ret_var in

      (* 関数型を構築: arg1 -> arg2 -> ... -> ret *)
      let expected_fn_ty = List.fold_right (fun arg_ty acc ->
        TArrow (arg_ty, acc)
      ) arg_tys ret_ty in

      (* 関数型と単一化 *)
      let fn_ty' = apply_subst s2 fn_ty in
      let* s3 = unify s2 fn_ty' expected_fn_ty expr.expr_span in

      (* 返り値型に代入を適用 *)
      let final_ret_ty = apply_subst s3 ret_ty in

      (* 型付き式を構築 *)
      let texpr = make_typed_expr (TCall (tfn, targs)) final_ret_ty expr.expr_span in
      Ok (texpr, final_ret_ty, s3)

  | Lambda (params, ret_ty_annot, body) ->
      (* ラムダ式の型推論
       *
       * 1. パラメータの型を決定（注釈あれば変換、なければ新鮮な型変数）
       * 2. 型環境にパラメータを追加
       * 3. 本体を推論
       * 4. 返り値型注釈があれば単一化
       * 5. 関数型を構築
       *)
      (* パラメータの型推論 *)
      let* (tparams, param_tys, param_env, s1) = infer_params env params empty_subst in

      (* 本体式を推論 *)
      let env' = apply_subst_env s1 param_env in
      let* (tbody, body_ty, s2) = infer_expr env' body in

      (* 返り値型注釈があれば単一化 *)
      let* (final_body_ty, s3) = match ret_ty_annot with
        | Some annot ->
            let expected_ret_ty = convert_type_annot annot in
            let* s = unify s2 body_ty expected_ret_ty expr.expr_span in
            Ok (apply_subst s body_ty, s)
        | None ->
            Ok (body_ty, s2)
      in

      (* 関数型を構築: param1 -> param2 -> ... -> body_ty *)
      let fn_ty = List.fold_right (fun param_ty acc ->
        TArrow (param_ty, acc)
      ) param_tys final_body_ty in

      (* 型付き式を構築 *)
      let ret_ty_opt = Option.map convert_type_annot ret_ty_annot in
      let texpr = make_typed_expr (TLambda (tparams, ret_ty_opt, tbody)) fn_ty expr.expr_span in
      Ok (texpr, fn_ty, s3)

  | Binary (_op, _e1, _e2) ->
      (* 二項演算: Phase 2 Week 3 で実装 *)
      failwith "Binary operators not yet implemented"

  | If (cond, then_e, else_e) ->
      (* if式の型推論
       *
       * 1. 条件式を推論してBool型と単一化
       * 2. then分岐を推論
       * 3. else分岐を推論してthen分岐と統一
       *)
      (* 条件式を推論 *)
      let* (tcond, cond_ty, s1) = infer_expr env cond in

      (* 条件式をBool型と単一化 *)
      let* s2 = unify s1 cond_ty ty_bool cond.expr_span in

      (* then分岐を推論 *)
      let env' = apply_subst_env s2 env in
      let* (tthen, then_ty, s3) = infer_expr env' then_e in

      (* else分岐を推論 *)
      let* (telse_opt, final_ty, s4) = match else_e with
        | Some else_expr ->
            let env'' = apply_subst_env s3 env' in
            let* (telse, else_ty, s) = infer_expr env'' else_expr in
            (* then分岐とelse分岐の型を統一 *)
            let* s' = unify s (apply_subst s then_ty) else_ty else_expr.expr_span in
            let unified_ty = apply_subst s' then_ty in
            Ok (Some telse, unified_ty, s')
        | None ->
            (* else分岐がない場合、then分岐はUnit型でなければならない *)
            let* s = unify s3 then_ty ty_unit then_e.expr_span in
            Ok (None, ty_unit, s)
      in

      (* 型付き式を構築 *)
      let texpr = make_typed_expr (TIf (tcond, tthen, telse_opt)) final_ty expr.expr_span in
      Ok (texpr, final_ty, s4)

  | Match (_scrutinee, _arms) ->
      (* match式: Phase 2 Week 4 で実装 *)
      failwith "Match expression not yet implemented"

  | Block _stmts ->
      (* ブロック式: Phase 2 Week 4 で実装 *)
      failwith "Block expression not yet implemented"

  | _ ->
      (* その他の式は Phase 2 で順次実装 *)
      failwith "Expression not yet implemented"

(** 引数リストの型推論
 *
 * 位置引数と名前付き引数の両方をサポート
 *)
and infer_args (env: env) (args: arg list) (subst: substitution)
    : (typed_arg list * ty list * substitution, type_error) result =
  List.fold_left (fun acc arg ->
    match acc with
    | Error e -> Error e
    | Ok (targs, arg_tys, s) ->
        let env' = apply_subst_env s env in
        match arg with
        | PosArg expr ->
            (match infer_expr env' expr with
             | Ok (texpr, ty, s') ->
                 let s'' = compose_subst s' s in
                 Ok (targs @ [TPosArg texpr], arg_tys @ [ty], s'')
             | Error e -> Error e)
        | NamedArg (id, expr) ->
            (match infer_expr env' expr with
             | Ok (texpr, ty, s') ->
                 let s'' = compose_subst s' s in
                 Ok (targs @ [TNamedArg (id, texpr)], arg_tys @ [ty], s'')
             | Error e -> Error e)
  ) (Ok ([], [], subst)) args

(** パラメータリストの型推論
 *
 * パラメータの型を決定し、型環境に追加
 *)
and infer_params (env: env) (params: param list) (subst: substitution)
    : (typed_param list * ty list * env * substitution, type_error) result =
  List.fold_left (fun acc param ->
    match acc with
    | Error e -> Error e
    | Ok (tparams, param_tys, param_env, s) ->
        (* パラメータの型を決定 *)
        let param_ty = match param.ty with
          | Some annot -> convert_type_annot annot
          | None -> Types.TVar (TypeVarGen.fresh None)
        in

        (* パターンから変数名を抽出（簡易版：変数パターンのみ）*)
        let (param_name, param_id) = match param.pat.pat_kind with
          | PatVar id -> (id.name, id)
          | _ -> failwith "Complex parameter patterns not yet implemented"
        in

        (* デフォルト値があれば推論（Phase 2後半で実装）*)
        let tdefault = match param.default with
          | Some _expr -> failwith "Default parameters not yet implemented"
          | None -> None
        in

        (* 型付きパターンを構築 *)
        let tpat = make_typed_pattern
          (TPatVar param_id)
          param_ty
          [(param_name, param_ty)]
          param.param_span in

        (* 型付きパラメータを構築 *)
        let tparam = {
          tpat = tpat;
          tty = param_ty;
          tdefault = tdefault;
          tparam_span = param.param_span;
        } in

        (* 型環境に追加 *)
        let param_env' = extend param_name (mono_scheme param_ty) param_env in

        Ok (tparams @ [tparam], param_tys @ [param_ty], param_env', s)
  ) (Ok ([], [], env, subst)) params

(** パターンの型推論: infer_pattern(env, pat, expected_ty)
 *
 * Phase 2 Week 4 で実装
 *)
let infer_pattern (_env: env) (_pat: pattern) (_expected_ty: ty)
    : (typed_pattern * env, type_error) result =
  failwith "Pattern inference not yet implemented"

(** 宣言の型推論: infer_decl(env, decl)
 *
 * Phase 2 Week 3-4 で実装
 *)
let infer_decl (env: env) (decl: decl)
    : (typed_decl * env, type_error) result =
  match decl.decl_kind with
  | LetDecl (pat, ty_annot, expr) ->
      (* let束縛の型推論
       *
       * 1. 式を推論
       * 2. 型注釈があれば単一化
       * 3. パターンの型推論
       * 4. 一般化してスキームを生成
       * 5. 型環境に追加
       *)
      (* 式を推論 *)
      let* (texpr, expr_ty, s1) = infer_expr env expr in

      (* 型注釈があれば単一化 *)
      let* (final_ty, s2) = match ty_annot with
        | Some annot ->
            let expected_ty = convert_type_annot annot in
            let* s = unify s1 expr_ty expected_ty expr.expr_span in
            Ok (apply_subst s expr_ty, s)
        | None ->
            Ok (expr_ty, s1)
      in

      (* パターンから変数名と識別子を抽出（簡易版：変数パターンのみ）*)
      let (pat_name, pat_id) = match pat.pat_kind with
        | PatVar id -> (id.name, id)
        | _ -> failwith "Complex let patterns not yet implemented"
      in

      (* 型付きパターンを構築 *)
      let tpat = make_typed_pattern
        (TPatVar pat_id)
        final_ty
        [(pat_name, final_ty)]
        pat.pat_span in

      (* 一般化してスキームを生成 *)
      let env' = apply_subst_env s2 env in
      let scheme = generalize env' final_ty in

      (* 型付き宣言を構築 *)
      let tdecl = make_typed_decl
        decl.decl_attrs
        decl.decl_vis
        (TLetDecl (tpat, texpr))
        scheme
        decl.decl_span in

      (* 型環境に追加 *)
      let new_env = extend pat_name scheme env' in

      Ok (tdecl, new_env)

  | FnDecl _fn ->
      (* 関数宣言: Phase 2 Week 4 で実装 *)
      failwith "Function declaration not yet implemented"

  | _ ->
      (* その他の宣言 *)
      failwith "Declaration not yet implemented"

(** コンパイル単位の型推論 *)
let infer_compilation_unit (_cu: compilation_unit)
    : (typed_compilation_unit, type_error) result =
  failwith "Compilation unit inference not yet implemented"

(* ========== デバッグ用 ========== *)

(** 推論結果の文字列表現 *)
let string_of_infer_result (texpr, ty, subst) =
  Printf.sprintf "%s : %s [%s]"
    (string_of_typed_expr texpr)
    (string_of_ty ty)
    (string_of_subst subst)
