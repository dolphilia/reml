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
let infer_expr (env: env) (expr: expr) : (infer_result, type_error) result =
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
           let texpr = make_typed_expr (TVar (id, scheme)) ty expr.expr_span in
           Ok (texpr, ty, empty_subst)
       | None ->
           Error (UnboundVariable (id.name, expr.expr_span)))

  | Call (_fn_expr, _args) ->
      (* 関数適用: Phase 2 Week 3 で実装 *)
      failwith "Function application not yet implemented"

  | Lambda (_params, _ret_ty_annot, _body) ->
      (* ラムダ式: Phase 2 Week 3 で実装 *)
      failwith "Lambda not yet implemented"

  | Binary (_op, _e1, _e2) ->
      (* 二項演算: Phase 2 Week 3 で実装 *)
      failwith "Binary operators not yet implemented"

  | If (_cond, _then_e, _else_e) ->
      (* if式: Phase 2 Week 3 で実装 *)
      failwith "If expression not yet implemented"

  | Match (_scrutinee, _arms) ->
      (* match式: Phase 2 Week 4 で実装 *)
      failwith "Match expression not yet implemented"

  | Block _stmts ->
      (* ブロック式: Phase 2 Week 4 で実装 *)
      failwith "Block expression not yet implemented"

  | _ ->
      (* その他の式は Phase 2 で順次実装 *)
      failwith "Expression not yet implemented"

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
let infer_decl (_env: env) (decl: decl)
    : (typed_decl * env, type_error) result =
  match decl.decl_kind with
  | LetDecl (_pat, _ty_annot, _expr) ->
      (* let束縛: 式を推論 → 一般化 → 型環境に追加 *)
      failwith "Let declaration not yet implemented"

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
