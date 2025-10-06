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

  | Binary (op, e1, e2) ->
      (* 二項演算の型推論
       *
       * 仕様書 1-2 §C.5: 演算子はトレイトで解決
       * Phase 2 MVP: 基本演算子の組み込みトレイトのみ（i64, f64, Bool, String対応）
       *
       * 1. 左辺と右辺を推論
       * 2. 演算子に応じた型制約を生成
       * 3. 返り値型を決定
       *)
      (* 左辺を推論 *)
      let* (te1, ty1, s1) = infer_expr env e1 in

      (* 右辺を推論 *)
      let env' = apply_subst_env s1 env in
      let* (te2, ty2, s2) = infer_expr env' e2 in

      (* 演算子に応じた型推論 *)
      let* (ret_ty, s3) = infer_binary_op op ty1 ty2 s2 e1.expr_span e2.expr_span in

      (* 型付き式を構築 *)
      let texpr = make_typed_expr (TBinary (op, te1, te2)) ret_ty expr.expr_span in
      Ok (texpr, ret_ty, s3)

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

  | Match (scrutinee, arms) ->
      (* match式の型推論
       *
       * 1. スクラティニー（検査対象）式の型推論
       * 2. 各アームのパターン推論と型環境更新
       * 3. ガード条件の型推論（Bool型）
       * 4. 各アームのボディを推論
       * 5. 全アームの型を統一
       *)
      (* スクラティニー式を推論 *)
      let* (tscrutinee, scrutinee_ty, s1) = infer_expr env scrutinee in

      (* アームが空の場合はエラー *)
      if arms = [] then
        Error (type_error_with_message
          "Match expression must have at least one arm"
          expr.expr_span)
      else
        (* 最初のアームを処理 *)
        let first_arm = List.hd arms in
        let* (first_tarm, first_body_ty, s2) = infer_match_arm
          (apply_subst_env s1 env) first_arm scrutinee_ty s1 in

        (* 残りのアームを処理して型を統一 *)
        let* (rest_tarms, final_ty, s_final) =
          List.fold_left (fun acc arm ->
            match acc with
            | Error e -> Error e
            | Ok (tarms, unified_ty, s_acc) ->
                let env' = apply_subst_env s_acc env in
                let scrutinee_ty' = apply_subst s_acc scrutinee_ty in
                let* (tarm, arm_body_ty, s_new) = infer_match_arm env' arm scrutinee_ty' s_acc in

                (* 型を統一 *)
                let* s_unified = unify s_new (apply_subst s_new unified_ty) arm_body_ty arm.arm_span in
                let new_unified_ty = apply_subst s_unified unified_ty in

                Ok (tarms @ [tarm], new_unified_ty, s_unified)
          ) (Ok ([first_tarm], first_body_ty, s2)) (List.tl arms)
        in

        (* 型付き式を構築 *)
        let texpr = make_typed_expr (TMatch (tscrutinee, rest_tarms)) final_ty expr.expr_span in
        Ok (texpr, final_ty, s_final)

  | Block stmts ->
      (* ブロック式: Phase 2 Week 5 で実装
       *
       * 仕様書 1-1 §C.6: ブロックの最後の式が値
       * - 空のブロック → Unit型
       * - 文のリストを順次処理し、型環境を更新
       * - 最後の要素が式文なら、その式の型がブロック全体の型
       * - 最後の要素が宣言文・代入文・defer文なら Unit型
       *)
      if stmts = [] then
        (* 空のブロック *)
        let texpr = make_typed_expr (TBlock []) ty_unit expr.expr_span in
        Ok (texpr, ty_unit, empty_subst)
      else
        (* 文を順次処理 *)
        let* (tstmts, final_ty, s_final) = infer_stmts env stmts empty_subst in
        let texpr = make_typed_expr (TBlock tstmts) final_ty expr.expr_span in
        Ok (texpr, final_ty, s_final)

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
 * パターンを推論し、束縛変数を型環境に追加する
 *
 * @param env 現在の型環境
 * @param pat パターン（AST）
 * @param expected_ty パターンの期待される型
 * @return (型付きパターン, 束縛変数を追加した型環境)
 *)
and infer_pattern (env: env) (pat: pattern) (expected_ty: ty)
    : (typed_pattern * env, type_error) result =
  match pat.pat_kind with
  | PatLiteral lit ->
      (* リテラルパターン: リテラルの型と expected_ty を単一化 *)
      let lit_ty = infer_literal lit pat.pat_span in
      let* _subst = unify empty_subst lit_ty expected_ty pat.pat_span in
      let tpat = make_typed_pattern (TPatLiteral lit) expected_ty [] pat.pat_span in
      Ok (tpat, env)

  | PatVar id ->
      (* 変数パターン: 変数を環境に追加 *)
      let bindings = [(id.name, expected_ty)] in
      let env' = extend id.name (mono_scheme expected_ty) env in
      let tpat = make_typed_pattern (TPatVar id) expected_ty bindings pat.pat_span in
      Ok (tpat, env')

  | PatWildcard ->
      (* ワイルドカードパターン: 任意の型にマッチ、束縛なし *)
      let tpat = make_typed_pattern TPatWildcard expected_ty [] pat.pat_span in
      Ok (tpat, env)

  | PatTuple pats ->
      (* タプルパターン: タプル型を構築して単一化 *)
      (* expected_ty がタプル型でない場合はエラー *)
      (match expected_ty with
       | TTuple expected_tys when List.length pats = List.length expected_tys ->
           (* 各要素パターンを推論 *)
           let* (tpats, env', all_bindings) =
             List.fold_left2 (fun acc pat expected_elem_ty ->
               match acc with
               | Error e -> Error e
               | Ok (tpats, env_acc, bindings_acc) ->
                   let* (tpat, env_new) = infer_pattern env_acc pat expected_elem_ty in
                   Ok (tpats @ [tpat], env_new, bindings_acc @ tpat.tpat_bindings)
             ) (Ok ([], env, [])) pats expected_tys
           in
           let tpat = make_typed_pattern (TPatTuple tpats) expected_ty all_bindings pat.pat_span in
           Ok (tpat, env')

       | TTuple _ ->
           (* タプルの要素数が不一致 *)
           Error (type_error_with_message
             (Printf.sprintf "Tuple pattern has %d elements, but type has different arity"
               (List.length pats))
             pat.pat_span)

       | _ ->
           (* expected_ty がタプル型でない場合は新しいタプル型を作成して単一化 *)
           let elem_vars = List.map (fun _ -> Types.TVar (TypeVarGen.fresh None)) pats in
           let tuple_ty = TTuple elem_vars in
           let* _subst = unify empty_subst expected_ty tuple_ty pat.pat_span in
           (* 再帰的に推論 *)
           infer_pattern env pat tuple_ty)

  | PatConstructor (id, arg_pats) ->
      (* コンストラクタパターン: コンストラクタの型スキームを取得してインスタンス化 *)
      (match lookup id.name env with
       | Some scheme ->
           (* 型スキームをインスタンス化 *)
           let ctor_ty = instantiate scheme in

           (* コンストラクタ型から引数型と結果型を抽出 *)
           let (arg_tys, result_ty) = extract_function_args ctor_ty in

           (* 引数の数が一致するか確認 *)
           if List.length arg_pats <> List.length arg_tys then
             Error (type_error_with_message
               (Printf.sprintf "Constructor %s expects %d arguments, but got %d"
                 id.name (List.length arg_tys) (List.length arg_pats))
               pat.pat_span)
           else
             (* 結果型と expected_ty を単一化 *)
             let* _subst = unify empty_subst result_ty expected_ty pat.pat_span in

             (* 各引数パターンを推論 *)
             let* (targ_pats, env', all_bindings) =
               List.fold_left2 (fun acc arg_pat arg_ty ->
                 match acc with
                 | Error e -> Error e
                 | Ok (tpats, env_acc, bindings_acc) ->
                     let* (tpat, env_new) = infer_pattern env_acc arg_pat arg_ty in
                     Ok (tpats @ [tpat], env_new, bindings_acc @ tpat.tpat_bindings)
               ) (Ok ([], env, [])) arg_pats arg_tys
             in

             let tpat = make_typed_pattern
               (TPatConstructor (id, targ_pats))
               expected_ty
               all_bindings
               pat.pat_span in
             Ok (tpat, env')

       | None ->
           Error (unbound_variable_error id.name pat.pat_span))

  | PatRecord (fields, has_rest) ->
      (* レコードパターン: Phase 2 では基本実装のみ *)
      (match expected_ty with
       | TRecord expected_fields ->
           (* 各フィールドパターンを推論 *)
           let* (tfield_pats, env', all_bindings) =
             List.fold_left (fun acc (field_id, field_pat_opt) ->
               match acc with
               | Error e -> Error e
               | Ok (tfields, env_acc, bindings_acc) ->
                   (* expected_fields からフィールド型を検索 *)
                   (match List.assoc_opt field_id.name expected_fields with
                    | Some field_ty ->
                        (* フィールドパターンがある場合は推論、ない場合は変数束縛 *)
                        (match field_pat_opt with
                         | Some field_pat ->
                             let* (tpat, env_new) = infer_pattern env_acc field_pat field_ty in
                             Ok (tfields @ [(field_id, Some tpat)], env_new,
                                 bindings_acc @ tpat.tpat_bindings)
                         | None ->
                             (* フィールド名を変数として束縛 *)
                             let bindings = [(field_id.name, field_ty)] in
                             let env_new = extend field_id.name (mono_scheme field_ty) env_acc in
                             let tpat = make_typed_pattern
                               (TPatVar field_id) field_ty bindings pat.pat_span in
                             Ok (tfields @ [(field_id, Some tpat)], env_new,
                                 bindings_acc @ bindings))
                    | None ->
                        Error (type_error_with_message
                          (Printf.sprintf "Field %s not found in record type" field_id.name)
                          pat.pat_span))
             ) (Ok ([], env, [])) fields
           in

           (* rest (..) がない場合、全フィールドをカバーしているか確認 *)
           if not has_rest then
             let pattern_fields = List.map (fun (id, _) -> id.name) fields in
             let type_fields = List.map fst expected_fields in
             let missing_fields = List.filter (fun f ->
               not (List.mem f pattern_fields)
             ) type_fields in
             if missing_fields <> [] then
               Error (type_error_with_message
                 (Printf.sprintf "Missing fields in record pattern: %s"
                   (String.concat ", " missing_fields))
                 pat.pat_span)
             else
               let tpat = make_typed_pattern
                 (TPatRecord (tfield_pats, has_rest))
                 expected_ty
                 all_bindings
                 pat.pat_span in
               Ok (tpat, env')
           else
             let tpat = make_typed_pattern
               (TPatRecord (tfield_pats, has_rest))
               expected_ty
               all_bindings
               pat.pat_span in
             Ok (tpat, env')

       | _ ->
           Error (type_error_with_message
             "Record pattern requires a record type"
             pat.pat_span))

  | PatGuard (inner_pat, guard_expr) ->
      (* ガード付きパターン: 内部パターンを推論後、ガード式を Bool 型として推論 *)
      let* (tinner_pat, env') = infer_pattern env inner_pat expected_ty in

      (* ガード式を Bool 型として推論 *)
      let* (tguard_expr, guard_ty, _) = infer_expr env' guard_expr in
      let* _ = unify empty_subst guard_ty ty_bool guard_expr.expr_span in

      let tpat = make_typed_pattern
        (TPatGuard (tinner_pat, tguard_expr))
        expected_ty
        tinner_pat.tpat_bindings  (* 束縛は内部パターンから継承 *)
        pat.pat_span in
      Ok (tpat, env')

(** 関数型から引数型のリストと結果型を抽出
 *
 * TArrow (A, TArrow (B, C)) → ([A; B], C)
 *)
and extract_function_args (ty: ty) : (ty list * ty) =
  match ty with
  | TArrow (arg_ty, rest_ty) ->
      let (args, result) = extract_function_args rest_ty in
      (arg_ty :: args, result)
  | _ ->
      ([], ty)

(** 二項演算子の型推論
 *
 * Phase 2 MVP: 基本演算子の組み込みトレイトのみ（i64, f64, Bool, String対応）
 *
 * @param op 演算子
 * @param ty1 左辺の型
 * @param ty2 右辺の型
 * @param subst 現在の代入
 * @param span1 左辺のSpan
 * @param span2 右辺のSpan
 * @return (返り値型, 新しい代入)
 *)
and infer_binary_op (op: Ast.binary_op) (ty1: ty) (ty2: ty)
    (subst: substitution) (span1: span) (span2: span)
    : (ty * substitution, type_error) result =
  match op with
  (* 算術演算子: + - * / % ^ *)
  | Add | Sub | Mul | Div | Mod | Pow ->
      (* 仕様書 1-2 §C.5: 数値型（i64, f64）のみサポート *)
      (* ty1 と ty2 を単一化し、返り値型も同じ *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in
      let* s1 = unify subst ty1' ty2' span2 in
      (* 単一化された型を返す *)
      let unified_ty = apply_subst s1 ty1' in
      Ok (unified_ty, s1)

  (* 比較演算子: == != < <= > >= *)
  | Eq | Ne | Lt | Le | Gt | Ge ->
      (* 左辺と右辺を単一化し、返り値は Bool *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in
      let* s1 = unify subst ty1' ty2' span2 in
      Ok (ty_bool, s1)

  (* 論理演算子: && || *)
  | And | Or ->
      (* 左辺と右辺をBool型と単一化 *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in
      let* s1 = unify subst ty1' ty_bool span1 in
      let* s2 = unify s1 ty2' ty_bool span2 in
      Ok (ty_bool, s2)

  (* パイプ演算子: |> *)
  | PipeOp ->
      (* x |> f は f(x) に等価
       * ty1 : A, ty2 : A -> B のとき、返り値は B
       *)
      let ty1' = apply_subst subst ty1 in
      let ty2' = apply_subst subst ty2 in
      let ret_var = TypeVarGen.fresh None in
      let ret_ty = Types.TVar ret_var in
      let expected_fn_ty = TArrow (ty1', ret_ty) in
      let* s1 = unify subst ty2' expected_fn_ty span2 in
      let final_ret_ty = apply_subst s1 ret_ty in
      Ok (final_ret_ty, s1)

(** match アームの型推論
 *
 * @param env 型環境
 * @param arm match アーム
 * @param scrutinee_ty スクラティニー式の型
 * @param subst 現在の代入
 * @return (型付きアーム, ボディの型, 新しい代入)
 *)
and infer_match_arm (env: env) (arm: match_arm) (scrutinee_ty: ty) (subst: substitution)
    : (typed_match_arm * ty * substitution, type_error) result =
  (* パターンを推論 *)
  let* (tpat, pat_env) = infer_pattern env arm.arm_pattern scrutinee_ty in

  (* ガード条件があれば推論 *)
  let* (tguard_opt, s1) = match arm.arm_guard with
    | Some guard_expr ->
        let* (tguard, guard_ty, s) = infer_expr pat_env guard_expr in
        let* s' = unify s guard_ty ty_bool guard_expr.expr_span in
        Ok (Some tguard, s')
    | None ->
        Ok (None, subst)
  in

  (* ボディを推論 *)
  let env' = apply_subst_env s1 pat_env in
  let* (tbody, body_ty, s2) = infer_expr env' arm.arm_body in

  (* 型付きアームを構築 *)
  let tarm = {
    tarm_pattern = tpat;
    tarm_guard = tguard_opt;
    tarm_body = tbody;
    tarm_span = arm.arm_span;
  } in

  Ok (tarm, body_ty, s2)

(** 関数本体の型推論: infer_fn_body(env, body)
 *
 * Phase 2 Week 5: FnExpr（式）とFnBlock（文のリスト）の両方に対応
 *
 * @param env 型環境（パラメータで拡張済み）
 * @param body 関数本体（AST）
 * @return (型付き関数本体, 本体の型, 代入)
 *)
and infer_fn_body (env: env) (body: fn_body)
    : (typed_fn_body * ty * substitution, type_error) result =
  match body with
  | FnExpr expr ->
      (* 式の場合: 直接推論 *)
      let* (texpr, ty, s) = infer_expr env expr in
      Ok (TFnExpr texpr, ty, s)

  | FnBlock stmts ->
      (* ブロックの場合: 文のリストを推論 *)
      let* (tstmts, ty, s) = infer_stmts env stmts empty_subst in
      Ok (TFnBlock tstmts, ty, s)

(** 文リストの型推論: infer_stmts(env, stmts, subst)
 *
 * Phase 2 Week 5: ブロック式のための文リスト型推論
 *
 * @param env 現在の型環境
 * @param stmts 文のリスト
 * @param subst 現在の代入
 * @return (型付き文リスト, 最終型, 最終代入)
 *)
and infer_stmts (env: env) (stmts: stmt list) (subst: substitution)
    : (typed_stmt list * ty * substitution, type_error) result =
  (* 最後の文を特別扱い *)
  let rec process_stmts env stmts acc_tstmts subst =
    match stmts with
    | [] ->
        (* 空リスト: Unit型 *)
        Ok (List.rev acc_tstmts, ty_unit, subst)
    | [last_stmt] ->
        (* 最後の文: ExprStmtなら式の型、それ以外はUnit *)
        (match last_stmt with
         | ExprStmt expr ->
             (* 最後の式文: 式の型がブロック全体の型 *)
             let env' = apply_subst_env subst env in
             let* (texpr, expr_ty, s) = infer_expr env' expr in
             let tstmt = TExprStmt texpr in
             Ok (List.rev (tstmt :: acc_tstmts), expr_ty, s)
         | _ ->
             (* 最後の文が宣言/代入/defer: Unit型 *)
             let* (tstmt, _new_env, s) = infer_stmt env last_stmt subst in
             Ok (List.rev (tstmt :: acc_tstmts), ty_unit, s))
    | stmt :: rest ->
        (* 中間の文: 処理して環境更新 *)
        let* (tstmt, new_env, s) = infer_stmt env stmt subst in
        process_stmts new_env rest (tstmt :: acc_tstmts) s
  in
  process_stmts env stmts [] subst

(** 文の型推論: infer_stmt(env, stmt, subst)
 *
 * Phase 2 Week 5: 文の型推論
 *
 * @param env 現在の型環境
 * @param stmt 文（AST）
 * @param subst 現在の代入
 * @return (型付き文, 新しい型環境, 新しい代入)
 *)
and infer_stmt (env: env) (stmt: stmt) (subst: substitution)
    : (typed_stmt * env * substitution, type_error) result =
  match stmt with
  | DeclStmt decl ->
      (* 宣言文: 型推論して型環境を更新 *)
      let env' = apply_subst_env subst env in
      let* (tdecl, new_env) = infer_decl env' decl in
      Ok (TDeclStmt tdecl, new_env, subst)

  | ExprStmt expr ->
      (* 式文: 式を推論（型環境は変更なし）*)
      let env' = apply_subst_env subst env in
      let* (texpr, _ty, s) = infer_expr env' expr in
      Ok (TExprStmt texpr, env, s)

  | AssignStmt (lhs, rhs) ->
      (* 代入文: 左辺と右辺を推論して型を統一
       *
       * 仕様書 1-1 §C.6: var 束縛の再代入 `:=` は Unit型を返す
       *)
      let env' = apply_subst_env subst env in
      let* (tlhs, lhs_ty, s1) = infer_expr env' lhs in
      let env'' = apply_subst_env s1 env' in
      let* (trhs, rhs_ty, s2) = infer_expr env'' rhs in
      (* 左辺と右辺の型を単一化 *)
      let lhs_ty' = apply_subst s2 lhs_ty in
      let* s3 = unify s2 lhs_ty' rhs_ty rhs.expr_span in
      Ok (TAssignStmt (tlhs, trhs), env, s3)

  | DeferStmt expr ->
      (* defer文: 式を推論（Unit型、型環境は変更なし）*)
      let env' = apply_subst_env subst env in
      let* (texpr, _ty, s) = infer_expr env' expr in
      Ok (TDeferStmt texpr, env, s)

(** 宣言の型推論: infer_decl(env, decl)
 *
 * Phase 2 Week 3-4 で実装
 *)
and infer_decl (env: env) (decl: decl)
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

  | FnDecl fn ->
      (* 関数宣言の型推論
       *
       * Phase 2 Week 5: 関数宣言の型推論を実装
       *
       * 1. ジェネリック型パラメータを型変数に変換
       * 2. パラメータの型推論
       * 3. 再帰関数のための暫定型を構築
       * 4. 関数本体の型推論
       * 5. 返り値型の検証
       * 6. 関数型の一般化
       *)

      (* 1. ジェネリック型パラメータを型変数に変換 *)
      let generic_bindings = List.map (fun id ->
        (id, TypeVarGen.fresh (Some id.name))
      ) fn.fn_generic_params in

      (* 2. ジェネリック型を型環境に追加 *)
      let env_with_generics = List.fold_left (fun acc (id, tv) ->
        extend id.name (mono_scheme (Types.TVar tv)) acc
      ) env generic_bindings in

      (* 3. パラメータの型推論 *)
      let* (tparams, param_tys, param_env, _s1) =
        infer_params env_with_generics fn.fn_params empty_subst in

      (* 4. 再帰関数のための暫定型を構築 *)
      let temp_ret_var = TypeVarGen.fresh None in
      let temp_fn_ty = List.fold_right (fun param_ty acc ->
        TArrow (param_ty, acc)
      ) param_tys (Types.TVar temp_ret_var) in

      (* 5. 関数名を型環境に追加（再帰呼び出しに対応） *)
      let env_with_fn = extend fn.fn_name.name
        (mono_scheme temp_fn_ty) param_env in

      (* 6. 関数本体の型推論 *)
      let* (tbody, body_ty, s2) = infer_fn_body env_with_fn fn.fn_body in

      (* 7. 返り値型注釈があれば単一化 *)
      let* (final_ret_ty, s3) = match fn.fn_ret_type with
        | Some annot ->
            let expected_ret_ty = convert_type_annot annot in
            let* s = unify s2 body_ty expected_ret_ty decl.decl_span in
            Ok (apply_subst s body_ty, s)
        | None ->
            Ok (body_ty, s2)
      in

      (* 8. 最終的な関数型を構築 *)
      let fn_ty = List.fold_right (fun param_ty acc ->
        TArrow (apply_subst s3 param_ty, acc)
      ) param_tys final_ret_ty in

      (* 9. 一般化してスキームを生成 *)
      let env' = apply_subst_env s3 env in
      let scheme = generalize env' fn_ty in

      (* 10. 型付き関数宣言を構築 *)
      let tfn = {
        tfn_name = fn.fn_name;
        tfn_generic_params = generic_bindings;
        tfn_params = tparams;
        tfn_ret_type = final_ret_ty;
        tfn_where_clause = fn.fn_where_clause;
        tfn_effect_annot = fn.fn_effect_annot;
        tfn_body = tbody;
      } in

      let tdecl = make_typed_decl
        decl.decl_attrs
        decl.decl_vis
        (TFnDecl tfn)
        scheme
        decl.decl_span in

      (* 11. 型環境に追加 *)
      let new_env = extend fn.fn_name.name scheme env' in

      Ok (tdecl, new_env)

  | _ ->
      (* その他の宣言 *)
      failwith "Declaration not yet implemented"

(** コンパイル単位の型推論 *)
let infer_compilation_unit (cu: compilation_unit)
    : (typed_compilation_unit, type_error) result =
  (* 初期型環境を作成 *)
  let init_env = initial_env in

  (* 各宣言を順次推論し、型環境を更新 *)
  let rec infer_items env items acc_decls =
    match items with
    | [] -> Ok (List.rev acc_decls, env)
    | item :: rest ->
        match infer_decl env item with
        | Ok (typed_decl, new_env) ->
            infer_items new_env rest (typed_decl :: acc_decls)
        | Error err -> Error err
  in

  match infer_items init_env cu.decls [] with
  | Ok (typed_items, _final_env) ->
      Ok {
        tcu_module_header = cu.header;
        tcu_use_decls = cu.uses;
        tcu_items = typed_items;
      }
  | Error err -> Error err

(* ========== デバッグ用 ========== *)

(** 推論結果の文字列表現 *)
let string_of_infer_result (texpr, ty, subst) =
  Printf.sprintf "%s : %s [%s]"
    (string_of_typed_expr texpr)
    (string_of_ty ty)
    (string_of_subst subst)
