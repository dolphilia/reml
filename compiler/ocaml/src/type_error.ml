(* Type_error — Type Error Definitions for Reml (Phase 2)
 *
 * このファイルは型推論・型検査で発生するエラーの定義を提供する。
 * 仕様書 2-5 §D（エラー設計）に従い、詳細な診断情報を保持する。
 *
 * 設計原則:
 * - エラー種別ごとに期待型・実際の型を保持
 * - Span情報を保持してエラー位置を正確に報告
 * - ユーザーフレンドリーなエラーメッセージ生成
 *)

open Types
open Ast

(* ========== 型エラーの定義 ========== *)

(** 型エラー
 *
 * 仕様書 1-2 §C.7: 期待/実際、候補トレイト、不足制約を列挙
 *)
type type_error =
  | UnificationFailure of ty * ty * span      (** 型不一致: 期待型、実際の型、位置 *)
  | OccursCheck of type_var * ty * span       (** 無限型検出: 型変数、型、位置 *)
  | UnboundVariable of string * span          (** 未定義変数: 変数名、位置 *)
  | ArityMismatch of {                        (* 引数数不一致 *)
      expected: int;
      actual: int;
      span: span;
    }
  | NotAFunction of ty * span                 (** 関数型でない型に対する適用 *)
  | ConditionNotBool of ty * span             (** 条件式がBool型でない *)
  | BranchTypeMismatch of {                   (* if式の分岐型不一致 *)
      then_ty: ty;
      else_ty: ty;
      span: span;
    }
  | PatternTypeMismatch of {                  (* パターンと式の型不一致 *)
      pattern_ty: ty;
      expr_ty: ty;
      span: span;
    }
  | ConstructorArityMismatch of {             (* コンストラクタ引数数不一致 *)
      constructor: string;
      expected: int;
      actual: int;
      span: span;
    }
  | TupleArityMismatch of {                   (* タプル要素数不一致 *)
      expected: int;
      actual: int;
      span: span;
    }
  | RecordFieldMissing of {                   (* レコードフィールド不足 *)
      missing_fields: string list;
      span: span;
    }
  | RecordFieldUnknown of {                   (* レコードフィールド不明 *)
      field: string;
      span: span;
    }
  | NotARecord of ty * span                   (** レコード型でない型に対するレコードパターン *)
  | NotATuple of ty * span                    (** タプル型でない型に対するタプルパターン *)
  | EmptyMatch of span                        (** 空のmatch式 *)

(* ========== エラーメッセージ生成 ========== *)

(** 型エラーの人間可読な文字列表現
 *
 * 仕様書 2-5 §D に従い、期待型と実際の型を明示
 *)
let string_of_error = function
  | UnificationFailure (expected, actual, span) ->
      Printf.sprintf
        "Type mismatch at %d:%d\n  Expected: %s\n  Found:    %s"
        span.start span.end_
        (string_of_ty expected)
        (string_of_ty actual)

  | OccursCheck (tv, ty, span) ->
      Printf.sprintf
        "Occurs check failed at %d:%d\n  Cannot construct infinite type: %s occurs in %s"
        span.start span.end_
        (string_of_type_var tv)
        (string_of_ty ty)

  | UnboundVariable (name, span) ->
      Printf.sprintf
        "Unbound variable at %d:%d\n  Variable '%s' is not defined in scope"
        span.start span.end_
        name

  | ArityMismatch { expected; actual; span } ->
      Printf.sprintf
        "Arity mismatch at %d:%d\n  Expected %d argument%s, but found %d"
        span.start span.end_
        expected
        (if expected = 1 then "" else "s")
        actual

  | NotAFunction (ty, span) ->
      Printf.sprintf
        "Not a function at %d:%d\n  Cannot apply arguments to type: %s"
        span.start span.end_
        (string_of_ty ty)

  | ConditionNotBool (ty, span) ->
      Printf.sprintf
        "Condition must be Bool at %d:%d\n  Found: %s"
        span.start span.end_
        (string_of_ty ty)

  | BranchTypeMismatch { then_ty; else_ty; span } ->
      Printf.sprintf
        "Branch type mismatch at %d:%d\n  'then' branch has type: %s\n  'else' branch has type: %s"
        span.start span.end_
        (string_of_ty then_ty)
        (string_of_ty else_ty)

  | PatternTypeMismatch { pattern_ty; expr_ty; span } ->
      Printf.sprintf
        "Pattern type mismatch at %d:%d\n  Pattern expects: %s\n  Expression has:  %s"
        span.start span.end_
        (string_of_ty pattern_ty)
        (string_of_ty expr_ty)

  | ConstructorArityMismatch { constructor; expected; actual; span } ->
      Printf.sprintf
        "Constructor arity mismatch at %d:%d\n  Constructor '%s' expects %d argument%s, but got %d"
        span.start span.end_
        constructor
        expected
        (if expected = 1 then "" else "s")
        actual

  | TupleArityMismatch { expected; actual; span } ->
      Printf.sprintf
        "Tuple arity mismatch at %d:%d\n  Expected %d element%s, but pattern has %d"
        span.start span.end_
        expected
        (if expected = 1 then "" else "s")
        actual

  | RecordFieldMissing { missing_fields; span } ->
      Printf.sprintf
        "Missing record fields at %d:%d\n  Missing fields: %s"
        span.start span.end_
        (String.concat ", " missing_fields)

  | RecordFieldUnknown { field; span } ->
      Printf.sprintf
        "Unknown record field at %d:%d\n  Field '%s' not found in record type"
        span.start span.end_
        field

  | NotARecord (ty, span) ->
      Printf.sprintf
        "Not a record type at %d:%d\n  Cannot use record pattern on type: %s"
        span.start span.end_
        (string_of_ty ty)

  | NotATuple (ty, span) ->
      Printf.sprintf
        "Not a tuple type at %d:%d\n  Cannot use tuple pattern on type: %s"
        span.start span.end_
        (string_of_ty ty)

  | EmptyMatch span ->
      Printf.sprintf
        "Empty match expression at %d:%d\n  Match expression must have at least one arm"
        span.start span.end_

(* ========== エラー生成ヘルパー ========== *)

(** 型不一致エラーを生成 *)
let unification_error expected actual span =
  UnificationFailure (expected, actual, span)

(** 無限型エラーを生成 *)
let occurs_check_error tv ty span =
  OccursCheck (tv, ty, span)

(** 未定義変数エラーを生成 *)
let unbound_variable_error name span =
  UnboundVariable (name, span)

(** 引数数不一致エラーを生成 *)
let arity_mismatch_error ~expected ~actual span =
  ArityMismatch { expected; actual; span }

(** 関数型でないエラーを生成 *)
let not_a_function_error ty span =
  NotAFunction (ty, span)

(** 条件式がBool型でないエラーを生成 *)
let condition_not_bool_error ty span =
  ConditionNotBool (ty, span)

(** if式の分岐型不一致エラーを生成 *)
let branch_type_mismatch_error then_ty else_ty span =
  BranchTypeMismatch { then_ty; else_ty; span }

(** パターンと式の型不一致エラーを生成 *)
let pattern_type_mismatch_error pattern_ty expr_ty span =
  PatternTypeMismatch { pattern_ty; expr_ty; span }

(** カスタムメッセージで型エラーを生成（一時的なヘルパー）
 *
 * TODO: より具体的なエラー型を追加してこの関数を削除
 *)
let type_error_with_message message span =
  (* 仮実装: UnboundVariable として表現 *)
  UnboundVariable (message, span)
