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

(* ========== Diagnostic への変換 ========== *)

(** Ast.span から Diagnostic.span への変換 *)
let span_to_diagnostic_span (ast_span: span) : Diagnostic.span =
  (* ダミーの location を作成（Phase 2では簡易実装） *)
  let make_loc offset =
    {
      Diagnostic.filename = "<入力>";
      line = 1;  (* TODO: バイトオフセットから行番号を計算 *)
      column = offset + 1;
      offset;
    }
  in
  {
    Diagnostic.start_pos = make_loc ast_span.start;
    Diagnostic.end_pos = make_loc ast_span.end_;
  }

(** 編集距離（Levenshtein距離）の計算
 *
 * 類似変数名の提案に使用
 *)
let levenshtein_distance s1 s2 =
  let len1 = String.length s1 in
  let len2 = String.length s2 in

  if len1 = 0 then len2
  else if len2 = 0 then len1
  else
    let dp = Array.make_matrix (len1 + 1) (len2 + 1) 0 in

    for i = 0 to len1 do
      dp.(i).(0) <- i
    done;
    for j = 0 to len2 do
      dp.(0).(j) <- j
    done;

    for i = 1 to len1 do
      for j = 1 to len2 do
        let cost = if s1.[i-1] = s2.[j-1] then 0 else 1 in
        dp.(i).(j) <- min (min
          (dp.(i-1).(j) + 1)      (* 削除 *)
          (dp.(i).(j-1) + 1))     (* 挿入 *)
          (dp.(i-1).(j-1) + cost) (* 置換 *)
      done
    done;
    dp.(len1).(len2)

(** 類似変数名の提案
 *
 * 編集距離2以内の変数を候補として返す
 *)
let suggest_similar_names (target: string) (candidates: string list) : string list =
  candidates
  |> List.map (fun name -> (name, levenshtein_distance target name))
  |> List.filter (fun (_, dist) -> dist > 0 && dist <= 2)
  |> List.sort (fun (_, d1) (_, d2) -> compare d1 d2)
  |> List.map fst
  |> (fun names -> if List.length names > 5 then List.filteri (fun i _ -> i < 5) names else names)

(** 型の差分を構造的に比較して説明を生成
 *
 * 型不一致の詳細を分かりやすく説明する
 *)
let explain_type_mismatch (expected: ty) (actual: ty) : string option =
  match (expected, actual) with
  (* タプルの要素型不一致 *)
  | (TTuple exp_elems, TTuple act_elems) when List.length exp_elems = List.length act_elems ->
      let mismatches = List.mapi (fun i (e, a) ->
        if not (Types.type_equal e a) then
          Some (Printf.sprintf "要素 %d: 期待 %s、実際 %s" i (string_of_ty e) (string_of_ty a))
        else None
      ) (List.combine exp_elems act_elems)
      |> List.filter_map (fun x -> x) in
      if mismatches <> [] then
        Some ("タプルの要素型が異なります:\n  " ^ String.concat "\n  " mismatches)
      else None
  (* 関数型の引数・返り値不一致 *)
  | (TArrow (exp_arg, exp_ret), TArrow (act_arg, act_ret)) ->
      if not (Types.type_equal exp_arg act_arg) then
        Some (Printf.sprintf "関数の引数型が異なります: 期待 %s、実際 %s"
          (string_of_ty exp_arg) (string_of_ty act_arg))
      else if not (Types.type_equal exp_ret act_ret) then
        Some (Printf.sprintf "関数の返り値型が異なります: 期待 %s、実際 %s"
          (string_of_ty exp_ret) (string_of_ty act_ret))
      else None
  (* その他の型不一致は一般的なメッセージのみ *)
  | _ -> None

(** 型エラーから診断情報への変換
 *
 * 仕様書 2-5 §B-11 に従い、構造化診断を生成
 *)
let to_diagnostic (err: type_error) : Diagnostic.t =
  let open Diagnostic in
  match err with
  | UnificationFailure (expected, actual, span) ->
      let message = "型が一致しません" in
      let diag_span = span_to_diagnostic_span span in

      (* 型差分の詳細説明 *)
      let detailed_explanation = match explain_type_mismatch expected actual with
        | Some explanation -> [( None, explanation)]
        | None -> []
      in

      let notes = [
        (None, Printf.sprintf "期待される型: %s" (string_of_ty expected));
        (None, Printf.sprintf "実際の型:     %s" (string_of_ty actual));
      ] @ detailed_explanation in

      (* FixIt: 型注釈の追加を提案 *)
      let fixits =
        match expected with
        | Types.TVar _ ->
            (* 期待型が型変数の場合は型注釈を提案 *)
            [Insert { at = diag_span; text = Printf.sprintf ": %s" (string_of_ty actual) }]
        | _ -> []
      in

      make_type_error
        ~code:"E7001"
        ~message
        ~span:diag_span
        ~notes
        ~fixits
        ()

  | OccursCheck (tv, ty, span) ->
      let message = "無限型が検出されました" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "型変数 %s が型 %s の中に出現しています"
           (string_of_type_var tv) (string_of_ty ty));
        (None, "再帰型を定義する場合は、明示的な型注釈が必要です");
      ] in

      make_type_error
        ~code:"E7002"
        ~message
        ~span:diag_span
        ~notes
        ()

  | UnboundVariable (name, span) ->
      let message = Printf.sprintf "変数 '%s' が定義されていません" name in
      let diag_span = span_to_diagnostic_span span in

      (* TODO: スコープ内の変数リストから類似名を提案 *)
      (* 現時点では提案なし *)
      let notes = [
        (None, "この変数はスコープ内に存在しません");
      ] in

      make_type_error
        ~code:"E7003"
        ~message
        ~span:diag_span
        ~notes
        ()

  | ArityMismatch { expected; actual; span } ->
      let message = Printf.sprintf "引数の数が一致しません（期待: %d、実際: %d）" expected actual in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "この関数は %d 個の引数を期待していますが、%d 個の引数が渡されました"
           expected actual);
      ] in

      make_type_error
        ~code:"E7004"
        ~message
        ~span:diag_span
        ~notes
        ()

  | NotAFunction (ty, span) ->
      let message = "関数ではない値に引数を適用しようとしています" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "この式の型は %s ですが、関数型が必要です" (string_of_ty ty));
      ] in

      make_type_error
        ~code:"E7005"
        ~message
        ~span:diag_span
        ~notes
        ()

  | ConditionNotBool (ty, span) ->
      let message = "条件式は Bool 型である必要があります" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "この条件式の型は %s です" (string_of_ty ty));
      ] in

      make_type_error
        ~code:"E7006"
        ~message
        ~span:diag_span
        ~notes
        ()

  | BranchTypeMismatch { then_ty; else_ty; span } ->
      let message = "if 式の分岐の型が一致しません" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "then 分岐の型: %s" (string_of_ty then_ty));
        (None, Printf.sprintf "else 分岐の型: %s" (string_of_ty else_ty));
        (None, "if 式の両方の分岐は同じ型を返す必要があります");
      ] in

      make_type_error
        ~code:"E7007"
        ~message
        ~span:diag_span
        ~notes
        ()

  | PatternTypeMismatch { pattern_ty; expr_ty; span } ->
      let message = "パターンと式の型が一致しません" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "パターンの期待型: %s" (string_of_ty pattern_ty));
        (None, Printf.sprintf "式の実際の型:     %s" (string_of_ty expr_ty));
      ] in

      make_type_error
        ~code:"E7008"
        ~message
        ~span:diag_span
        ~notes
        ()

  | ConstructorArityMismatch { constructor; expected; actual; span } ->
      let message = Printf.sprintf "コンストラクタ '%s' の引数の数が一致しません" constructor in
      let diag_span = span_to_diagnostic_span span in

      (* 使用例を提供 *)
      let example =
        if expected = 0 then
          Printf.sprintf "%s" constructor
        else if expected = 1 then
          Printf.sprintf "%s(_)" constructor
        else
          let args = String.concat ", " (List.init expected (fun _ -> "_")) in
          Printf.sprintf "%s(%s)" constructor args
      in

      let notes = [
        (None, Printf.sprintf "期待される引数の数: %d" expected);
        (None, Printf.sprintf "実際の引数の数:     %d" actual);
        (None, Printf.sprintf "正しい使用例: %s" example);
      ] in

      make_type_error
        ~code:"E7009"
        ~message
        ~span:diag_span
        ~notes
        ()

  | TupleArityMismatch { expected; actual; span } ->
      let message = "タプルの要素数が一致しません" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "期待される要素数: %d" expected);
        (None, Printf.sprintf "パターンの要素数: %d" actual);
      ] in

      (* FixIt: 要素数が不足している場合はワイルドカードの追加を提案 *)
      let fixits =
        if actual < expected then
          let missing = expected - actual in
          let wildcards = String.concat ", " (List.init missing (fun _ -> "_")) in
          [Insert { at = diag_span; text = Printf.sprintf ", %s" wildcards }]
        else
          []
      in

      make_type_error
        ~code:"E7010"
        ~message
        ~span:diag_span
        ~notes
        ~fixits
        ()

  | RecordFieldMissing { missing_fields; span } ->
      let message = "レコードに必須フィールドが不足しています" in
      let diag_span = span_to_diagnostic_span span in
      let fields_str = String.concat ", " missing_fields in
      let notes = [
        (None, Printf.sprintf "不足しているフィールド: %s" fields_str);
      ] in

      (* FixIt: 不足フィールドの挿入を提案 *)
      let fixits =
        missing_fields |> List.map (fun field ->
          Insert { at = diag_span; text = Printf.sprintf "%s: /* 値を入力 */, " field }
        )
      in

      make_type_error
        ~code:"E7011"
        ~message
        ~span:diag_span
        ~notes
        ~fixits
        ()

  | RecordFieldUnknown { field; span } ->
      let message = Printf.sprintf "レコード型に存在しないフィールド '%s' が指定されています" field in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, "このフィールドはレコード型で定義されていません");
      ] in

      make_type_error
        ~code:"E7012"
        ~message
        ~span:diag_span
        ~notes
        ()

  | NotARecord (ty, span) ->
      let message = "レコード型ではない型に対してレコードパターンを使用しています" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "この式の型は %s ですが、レコード型が必要です" (string_of_ty ty));
      ] in

      make_type_error
        ~code:"E7013"
        ~message
        ~span:diag_span
        ~notes
        ()

  | NotATuple (ty, span) ->
      let message = "タプル型ではない型に対してタプルパターンを使用しています" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, Printf.sprintf "この式の型は %s ですが、タプル型が必要です" (string_of_ty ty));
      ] in

      make_type_error
        ~code:"E7014"
        ~message
        ~span:diag_span
        ~notes
        ()

  | EmptyMatch span ->
      let message = "match 式にはアームが少なくとも1つ必要です" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [
        (None, "パターンマッチのケースを追加してください");
      ] in

      make_type_error
        ~code:"E7015"
        ~message
        ~span:diag_span
        ~notes
        ()

(** 環境情報を使った診断情報の生成
 *
 * スコープ内の変数リストから類似名を提案できる
 *)
let to_diagnostic_with_env ?(available_names: string list = []) (err: type_error) : Diagnostic.t =
  let open Diagnostic in
  match err with
  | UnboundVariable (name, span) ->
      let message = Printf.sprintf "変数 '%s' が定義されていません" name in
      let diag_span = span_to_diagnostic_span span in

      (* 類似変数名の提案 *)
      let suggestions = suggest_similar_names name available_names in
      let suggestion_notes = match suggestions with
        | [] -> [(None, "この変数はスコープ内に存在しません")]
        | names ->
            let candidates = String.concat ", " names in
            [
              (None, "この変数はスコープ内に存在しません");
              (None, Printf.sprintf "もしかして: %s ?" candidates);
            ]
      in

      make_type_error
        ~code:"E7003"
        ~message
        ~span:diag_span
        ~notes:suggestion_notes
        ()

  | other_error ->
      (* その他のエラーは通常の変換を使用 *)
      to_diagnostic other_error
