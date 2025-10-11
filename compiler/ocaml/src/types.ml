(* Types — Reml Type System (Phase 2)
 *
 * このファイルは docs/spec/1-2-types-Inference.md に基づいた型システムの基盤を提供する。
 * Phase 2 (M2マイルストーン) では Hindley-Milner 型推論を実装する。
 *
 * 設計原則:
 * - サブタイピングなし（HM系の推論をシンプルに保つ）
 * - ランク1の多相を基本とする
 * - 型変数の一意性を保証
 *)

(* ========== 型表現 ========== *)

type type_var = {
  tv_id : int;  (** 一意ID（単調増加） *)
  tv_name : string option;  (** 名前（デバッグ用、α, β など） *)
}
(** 型変数 *)

(** 整数型の種類（仕様書 1-2 §A.1） *)
type int_type = I8 | I16 | I32 | I64 | Isize | U8 | U16 | U32 | U64 | Usize

(** 浮動小数型の種類（仕様書 1-2 §A.1） *)
type float_type = F32 | F64

(** 型定数 *)
type type_const =
  | TCBool  (** Bool *)
  | TCChar  (** Char (Unicode scalar) *)
  | TCString  (** String (UTF-8) *)
  | TCInt of int_type  (** 整数型 i8..i64, u8..u64, isize, usize *)
  | TCFloat of float_type  (** 浮動小数型 f32, f64 *)
  | TCUser of string  (** ユーザ定義型 (Option, Result など) *)

(** 型（仕様書 1-2 §A）
 *
 * - TVar: 型変数（推論中に単一化される柔軟な型）
 * - TCon: 型定数（Bool, i64, String など）
 * - TApp: 型適用（Option<T> など）
 * - TArrow: 関数型（A -> B、右結合）
 * - TTuple: タプル型（(T1, T2, ..., Tn)）
 * - TRecord: レコード型（{ x: T1, y: T2 }）
 * - TArray: 配列型（[T; N]、固定長）
 * - TSlice: スライス型（[T]、動的配列）
 * - TUnit: 単位型 ()
 * - TNever: Never型（空集合、到達不能を表現）
 *)
type ty =
  | TVar of type_var
  | TCon of type_const
  | TApp of ty * ty  (** Vec<T> = TApp(TCon(TCUser "Vec"), TVar ...) *)
  | TArrow of ty * ty  (** A -> B -> C = TArrow(A, TArrow(B, C)) *)
  | TTuple of ty list  (** (A, B, C) *)
  | TRecord of (string * ty) list  (** { x: A, y: B } *)
  | TArray of ty  (** [T] (スライス、動的配列) *)
  | TSlice of ty * int option  (** [T; N] (固定長配列、N=Noneは未確定) *)
  | TUnit  (** () *)
  | TNever  (** Never (仕様書 3-1 §2.1) *)

type type_scheme = {
  quantified : type_var list;  (** 量化変数のリスト *)
  body : ty;  (** 型本体 *)
}
(** 型スキーム（∀α₁...αₙ. τ）
 *
 * 仕様書 1-2 §A.3: let多相で一般化された型
 * 例: id : ∀a. a -> a
 *)

(* ========== 型変数生成器 ========== *)

(** 型変数生成器（単調増加カウンタ、スレッドセーフ不要） *)
module TypeVarGen = struct
  let counter = ref 0

  (** リセット（テスト用） *)
  let reset () = counter := 0

  (** 新鮮な型変数を生成 *)
  let fresh name =
    let id = !counter in
    counter := id + 1;
    { tv_id = id; tv_name = name }

  (** 複数の新鮮な型変数を生成 *)
  let fresh_many n =
    List.init n (fun _ ->
        let id = !counter in
        counter := id + 1;
        { tv_id = id; tv_name = Some (Printf.sprintf "t%d" id) })

  (** デバッグ用: 名前付き型変数を生成（a, b, c, ...） *)
  let fresh_named prefix =
    let id = !counter in
    counter := id + 1;
    { tv_id = id; tv_name = Some (prefix ^ string_of_int id) }

  (** ギリシャ文字風の名前を生成（α, β, γ, ...） *)
  let fresh_greek () =
    let id = !counter in
    counter := id + 1;
    let greek = [| "α"; "β"; "γ"; "δ"; "ε"; "ζ"; "η"; "θ" |] in
    let name =
      if id < Array.length greek then greek.(id) else "τ" ^ string_of_int id
    in
    { tv_id = id; tv_name = Some name }
end

(* ========== 組み込み型定数 ========== *)

(** 組み込み型の定数（仕様書 1-2 §A.1/A.2） *)
let ty_bool = TCon TCBool

let ty_char = TCon TCChar
let ty_string = TCon TCString
let ty_unit = TUnit
let ty_never = TNever

(** 整数型 *)
let ty_i8 = TCon (TCInt I8)

let ty_i16 = TCon (TCInt I16)
let ty_i32 = TCon (TCInt I32)
let ty_i64 = TCon (TCInt I64)
let ty_isize = TCon (TCInt Isize)
let ty_u8 = TCon (TCInt U8)
let ty_u16 = TCon (TCInt U16)
let ty_u32 = TCon (TCInt U32)
let ty_u64 = TCon (TCInt U64)
let ty_usize = TCon (TCInt Usize)

(** 浮動小数型 *)
let ty_f32 = TCon (TCFloat F32)

let ty_f64 = TCon (TCFloat F64)

(** ユーザ定義型（Core.Prelude）
 *
 * 仕様書 3-1 §2.1:
 * - Option<T> = | Some(T) | None
 * - Result<T, E> = | Ok(T) | Err(E)
 * - Never = Result<Never, Never> (空集合)
 *)
let ty_option t = TApp (TCon (TCUser "Option"), t)

let ty_result t e = TApp (TApp (TCon (TCUser "Result"), t), e)

(** 配列/スライス型 *)
let ty_array t = TArray t

let ty_slice t n = TSlice (t, n)

(** 関数型の構築 *)
let ty_arrow arg ret = TArrow (arg, ret)

(** タプル型の構築 *)
let ty_tuple tys = TTuple tys

(** レコード型の構築 *)
let ty_record fields = TRecord fields

(* ========== 型スキームの操作 ========== *)

(** 単相型（量化なし）を型スキームに変換 *)
let mono_scheme ty = { quantified = []; body = ty }

(** 型スキームから型本体を取得 *)
let scheme_body scheme = scheme.body

(** 型スキームが多相かどうか判定 *)
let is_polymorphic scheme = scheme.quantified <> []

(* ========== デバッグ用: 型の表示 ========== *)

(** 型定数の文字列表現 *)
let string_of_type_const = function
  | TCBool -> "Bool"
  | TCChar -> "Char"
  | TCString -> "String"
  | TCInt I8 -> "i8"
  | TCInt I16 -> "i16"
  | TCInt I32 -> "i32"
  | TCInt I64 -> "i64"
  | TCInt Isize -> "isize"
  | TCInt U8 -> "u8"
  | TCInt U16 -> "u16"
  | TCInt U32 -> "u32"
  | TCInt U64 -> "u64"
  | TCInt Usize -> "usize"
  | TCFloat F32 -> "f32"
  | TCFloat F64 -> "f64"
  | TCUser name -> name

(** 型変数の文字列表現 *)
let string_of_type_var tv =
  match tv.tv_name with
  | Some name -> name
  | None -> "t" ^ string_of_int tv.tv_id

(** 型の文字列表現（簡易版） *)
let rec string_of_ty = function
  | TVar tv -> string_of_type_var tv
  | TCon tc -> string_of_type_const tc
  | TApp (t1, t2) -> Printf.sprintf "%s<%s>" (string_of_ty t1) (string_of_ty t2)
  | TArrow (t1, t2) ->
      Printf.sprintf "(%s -> %s)" (string_of_ty t1) (string_of_ty t2)
  | TTuple tys ->
      Printf.sprintf "(%s)" (String.concat ", " (List.map string_of_ty tys))
  | TRecord fields ->
      let field_strs =
        List.map
          (fun (name, ty) -> Printf.sprintf "%s: %s" name (string_of_ty ty))
          fields
      in
      Printf.sprintf "{ %s }" (String.concat ", " field_strs)
  | TArray t -> Printf.sprintf "[%s]" (string_of_ty t)
  | TSlice (t, None) -> Printf.sprintf "[%s; _]" (string_of_ty t)
  | TSlice (t, Some n) -> Printf.sprintf "[%s; %d]" (string_of_ty t) n
  | TUnit -> "()"
  | TNever -> "Never"

(** 型スキームの文字列表現 *)
let string_of_scheme scheme =
  match scheme.quantified with
  | [] -> string_of_ty scheme.body
  | vars ->
      let var_names = String.concat " " (List.map string_of_type_var vars) in
      Printf.sprintf "∀%s. %s" var_names (string_of_ty scheme.body)

(* ========== 型の等価性判定 ========== *)

(** 型変数の等価性判定 *)
let type_var_equal tv1 tv2 = tv1.tv_id = tv2.tv_id

(** 型定数の等価性判定 *)
let type_const_equal tc1 tc2 =
  match (tc1, tc2) with
  | TCBool, TCBool -> true
  | TCChar, TCChar -> true
  | TCString, TCString -> true
  | TCInt it1, TCInt it2 -> it1 = it2
  | TCFloat ft1, TCFloat ft2 -> ft1 = ft2
  | TCUser n1, TCUser n2 -> n1 = n2
  | _ -> false

(** 型の等価性判定（構造的等価性） *)
let rec type_equal t1 t2 =
  match (t1, t2) with
  | TVar tv1, TVar tv2 -> type_var_equal tv1 tv2
  | TCon tc1, TCon tc2 -> type_const_equal tc1 tc2
  | TApp (t11, t12), TApp (t21, t22) -> type_equal t11 t21 && type_equal t12 t22
  | TArrow (t11, t12), TArrow (t21, t22) ->
      type_equal t11 t21 && type_equal t12 t22
  | TTuple tys1, TTuple tys2 ->
      List.length tys1 = List.length tys2 && List.for_all2 type_equal tys1 tys2
  | TRecord fields1, TRecord fields2 ->
      List.length fields1 = List.length fields2
      && List.for_all2
           (fun (n1, t1) (n2, t2) -> n1 = n2 && type_equal t1 t2)
           fields1 fields2
  | TArray t1, TArray t2 -> type_equal t1 t2
  | TSlice (t1, n1), TSlice (t2, n2) -> type_equal t1 t2 && n1 = n2
  | TUnit, TUnit -> true
  | TNever, TNever -> true
  | _ -> false
