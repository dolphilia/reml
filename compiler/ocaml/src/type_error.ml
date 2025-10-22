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
open Effect_profile
module Json = Yojson.Basic
module Ffi = Ffi_contract
module Typeclass_metadata = Typeclass_metadata

type trait_constraint_stage_extension = {
  required_stage : string;
  iterator_required : string;
  actual_stage : string option;
  capability : string option;
  provider : string option;
  manifest_path : string option;
  iterator_kind : string option;
  iterator_source : string option;
  capability_metadata : Json.t option;
  residual : Json.t option;
  stage_trace : stage_trace;
}

(* ========== 型エラーの定義 ========== *)

(** 型エラー
 *
 * 仕様書 1-2 §C.7: 期待/実際、候補トレイト、不足制約を列挙
 *)
type type_error =
  | UnificationFailure of ty * ty * span  (** 型不一致: 期待型、実際の型、位置 *)
  | OccursCheck of type_var * ty * span  (** 無限型検出: 型変数、型、位置 *)
  | UnboundVariable of string * span  (** 未定義変数: 変数名、位置 *)
  | ArityMismatch of { (* 引数数不一致 *)
                       expected : int; actual : int; span : span }
  | NotAFunction of ty * span  (** 関数型でない型に対する適用 *)
  | ConditionNotBool of ty * span  (** 条件式がBool型でない *)
  | BranchTypeMismatch of {
      (* if式の分岐型不一致 *)
      then_ty : ty;
      else_ty : ty;
      span : span;
    }
  | PatternTypeMismatch of {
      (* パターンと式の型不一致 *)
      pattern_ty : ty;
      expr_ty : ty;
      span : span;
    }
  | ConstructorArityMismatch of {
      (* コンストラクタ引数数不一致 *)
      constructor : string;
      expected : int;
      actual : int;
      span : span;
    }
  | TupleArityMismatch of {
      (* タプル要素数不一致 *)
      expected : int;
      actual : int;
      span : span;
    }
  | RecordFieldMissing of {
      (* レコードフィールド不足 *)
      missing_fields : string list;
      span : span;
    }
  | RecordFieldUnknown of { (* レコードフィールド不明 *)
                            field : string; span : span }
  | NotARecord of ty * span  (** レコード型でない型に対するレコードパターン *)
  | NotATuple of ty * span  (** タプル型でない型に対するタプルパターン *)
  | EmptyMatch of span  (** 空のmatch式 *)
  (* Phase 2 Week 18-19: 型クラス関連エラー *)
  | TraitConstraintFailure of {
      (* トレイト制約の解決失敗 *)
      trait_name : string;
      type_args : ty list;
      reason : string;
      span : span;
      effect_stage : trait_constraint_stage_extension option;
      typeclass_state : Typeclass_metadata.resolution_state;
      typeclass_pending : string list;
      typeclass_generalized : string list;
    }
  | EffectStageMismatch of {
      required_stage : string;
      actual_stage : string;
      span : span;
      function_name : string option;
      capability : string option;
      stage_trace : stage_trace;
    }
  | EffectInvalidAttribute of {
      function_name : string option;
      profile : Effect_profile.profile;
      invalid_attribute : Effect_profile.invalid_attribute;
    }
  | EffectResidualLeak of {
      function_name : string option;
      profile : Effect_profile.profile;
      leaks : Effect_profile.residual_effect_leak list;
    }
  | FfiContractSymbolMissing of Ffi.normalized_contract
  | FfiContractOwnershipMismatch of Ffi.normalized_contract
  | FfiContractUnsupportedAbi of Ffi.normalized_contract
  | AmbiguousTraitImpl of {
      (* トレイト実装の曖昧性 *)
      trait_name : string;
      type_args : ty list;
      candidates : Constraint_solver.dict_ref list;
      span : span;
    }
  | CyclicTraitConstraint of {
      (* トレイト制約の循環依存 *)
      cycle : string list; (* トレイト名のリスト *)
      span : span;
    }
  | NotAssignable of { span : span }  (** 代入不可能な左辺 *)
  | ImmutableBinding of { name : string;  (** ミュータブルでない変数名 *) span : span }
  | ContinueOutsideLoop of span  (** ループ外での continue 使用 *)

(* ========== エラーメッセージ生成 ========== *)

(** 型エラーの人間可読な文字列表現
 *
 * 仕様書 2-5 §D に従い、期待型と実際の型を明示
 *)
let string_of_error = function
  | UnificationFailure (expected, actual, span) ->
      Printf.sprintf "Type mismatch at %d:%d\n  Expected: %s\n  Found:    %s"
        span.start span.end_ (string_of_ty expected) (string_of_ty actual)
  | OccursCheck (tv, ty, span) ->
      Printf.sprintf
        "Occurs check failed at %d:%d\n\
        \  Cannot construct infinite type: %s occurs in %s" span.start span.end_
        (string_of_type_var tv) (string_of_ty ty)
  | UnboundVariable (name, span) ->
      Printf.sprintf
        "Unbound variable at %d:%d\n  Variable '%s' is not defined in scope"
        span.start span.end_ name
  | ArityMismatch { expected; actual; span } ->
      Printf.sprintf
        "Arity mismatch at %d:%d\n  Expected %d argument%s, but found %d"
        span.start span.end_ expected
        (if expected = 1 then "" else "s")
        actual
  | NotAFunction (ty, span) ->
      Printf.sprintf
        "Not a function at %d:%d\n  Cannot apply arguments to type: %s"
        span.start span.end_ (string_of_ty ty)
  | ConditionNotBool (ty, span) ->
      Printf.sprintf "Condition must be Bool at %d:%d\n  Found: %s" span.start
        span.end_ (string_of_ty ty)
  | BranchTypeMismatch { then_ty; else_ty; span } ->
      Printf.sprintf
        "Branch type mismatch at %d:%d\n\
        \  'then' branch has type: %s\n\
        \  'else' branch has type: %s" span.start span.end_
        (string_of_ty then_ty) (string_of_ty else_ty)
  | PatternTypeMismatch { pattern_ty; expr_ty; span } ->
      Printf.sprintf
        "Pattern type mismatch at %d:%d\n\
        \  Pattern expects: %s\n\
        \  Expression has:  %s" span.start span.end_ (string_of_ty pattern_ty)
        (string_of_ty expr_ty)
  | ConstructorArityMismatch { constructor; expected; actual; span } ->
      Printf.sprintf
        "Constructor arity mismatch at %d:%d\n\
        \  Constructor '%s' expects %d argument%s, but got %d" span.start
        span.end_ constructor expected
        (if expected = 1 then "" else "s")
        actual
  | TupleArityMismatch { expected; actual; span } ->
      Printf.sprintf
        "Tuple arity mismatch at %d:%d\n\
        \  Expected %d element%s, but pattern has %d" span.start span.end_
        expected
        (if expected = 1 then "" else "s")
        actual
  | RecordFieldMissing { missing_fields; span } ->
      Printf.sprintf "Missing record fields at %d:%d\n  Missing fields: %s"
        span.start span.end_
        (String.concat ", " missing_fields)
  | RecordFieldUnknown { field; span } ->
      Printf.sprintf
        "Unknown record field at %d:%d\n  Field '%s' not found in record type"
        span.start span.end_ field
  | NotARecord (ty, span) ->
      Printf.sprintf
        "Not a record type at %d:%d\n  Cannot use record pattern on type: %s"
        span.start span.end_ (string_of_ty ty)
  | NotATuple (ty, span) ->
      Printf.sprintf
        "Not a tuple type at %d:%d\n  Cannot use tuple pattern on type: %s"
        span.start span.end_ (string_of_ty ty)
  | EmptyMatch span ->
      Printf.sprintf
        "Empty match expression at %d:%d\n\
        \  Match expression must have at least one arm" span.start span.end_
  | TraitConstraintFailure
      {
        trait_name;
        type_args;
        reason;
        span;
        effect_stage = _;
        typeclass_state = _;
        typeclass_pending = _;
        typeclass_generalized = _;
      } ->
      let type_args_str =
        String.concat ", " (List.map string_of_ty type_args)
      in
      Printf.sprintf
        "Trait constraint '%s<%s>' cannot be satisfied at %d:%d\n  Reason: %s"
        trait_name type_args_str span.start span.end_ reason
  | EffectStageMismatch { required_stage; actual_stage; span; function_name; _ }
    ->
      let subject =
        match function_name with
        | Some name -> Printf.sprintf " for '%s'" name
        | None -> ""
      in
      Printf.sprintf
        "Effect stage mismatch%s at %d:%d\n  Required: %s\n  Actual:   %s"
        subject span.start span.end_ required_stage actual_stage
  | EffectInvalidAttribute { function_name; invalid_attribute; _ } ->
      let subject =
        match function_name with
        | Some name -> Printf.sprintf " in '%s'" name
        | None -> ""
      in
      Printf.sprintf "Invalid effect attribute%s: %s" subject
        invalid_attribute.attribute_display
  | EffectResidualLeak { profile; leaks; _ } ->
      let subject =
        match profile.Effect_profile.source_name with
        | Some name -> Printf.sprintf " in '%s'" name
        | None -> ""
      in
      let missing =
        match leaks with
        | [] -> "<none>"
        | _ ->
            leaks
            |> List.map (fun leak -> leak.Effect_profile.leaked_tag.effect_name)
            |> String.concat ", "
      in
      Printf.sprintf "Residual effects%s are not declared: %s" subject missing
  | FfiContractSymbolMissing normalized ->
      Printf.sprintf "FFI contract missing link symbol for extern '%s'"
        normalized.contract.extern_name
  | FfiContractOwnershipMismatch normalized ->
      let actual =
        match normalized.ownership_raw with
        | Some raw when String.trim raw <> "" -> raw
        | _ -> "(unspecified)"
      in
      Printf.sprintf "FFI ownership mismatch for extern '%s' (actual: %s)"
        normalized.contract.extern_name actual
  | FfiContractUnsupportedAbi normalized ->
      let actual =
        match normalized.abi_raw with
        | Some raw when String.trim raw <> "" -> raw
        | _ -> "(unspecified)"
      in
      let expected =
        match normalized.expected_abi with
        | Some abi -> Ffi_contract.string_of_abi_kind abi
        | None -> "<unknown>"
      in
      Printf.sprintf
        "FFI ABI unsupported for extern '%s' (actual: %s, expected: %s)"
        normalized.contract.extern_name actual expected
  | AmbiguousTraitImpl { trait_name; type_args; candidates; span } ->
      let type_args_str =
        String.concat ", " (List.map string_of_ty type_args)
      in
      let candidates_str =
        candidates
        |> List.map Constraint_solver.string_of_dict_ref
        |> String.concat "\n  - "
      in
      Printf.sprintf
        "Ambiguous trait implementation for '%s<%s>' at %d:%d\n\
        \  Multiple candidates found:\n\
        \  - %s" trait_name type_args_str span.start span.end_ candidates_str
  | CyclicTraitConstraint { cycle; span } ->
      let cycle_str = String.concat " -> " cycle in
      Printf.sprintf
        "Cyclic trait constraint detected at %d:%d\n  Cycle: %s -> ..."
        span.start span.end_ cycle_str
  | NotAssignable { span } ->
      Printf.sprintf
        "Left-hand side is not assignable at %d:%d\n\
        \  Expected a mutable variable or lvalue expression" span.start
        span.end_
  | ImmutableBinding { name; span } ->
      Printf.sprintf
        "Cannot assign to immutable binding at %d:%d\n\
        \  Variable '%s' was declared with 'let'" span.start span.end_ name
  | ContinueOutsideLoop span ->
      Printf.sprintf "continue はループ内でのみ使用できます（位置: %d:%d）" span.start span.end_

(* ========== エラー生成ヘルパー ========== *)

(** 型不一致エラーを生成 *)
let unification_error expected actual span =
  UnificationFailure (expected, actual, span)

(** 無限型エラーを生成 *)
let occurs_check_error tv ty span = OccursCheck (tv, ty, span)

(** 未定義変数エラーを生成 *)
let unbound_variable_error name span = UnboundVariable (name, span)

(** 引数数不一致エラーを生成 *)
let arity_mismatch_error ~expected ~actual span =
  ArityMismatch { expected; actual; span }

(** 代入不可能エラーを生成 *)
let not_assignable_error span = NotAssignable { span }

(** 効果 Stage ミスマッチのエラーを生成 *)
let effect_stage_mismatch_error ~function_name ~required_stage ~actual_stage
    ~span ~capability ~stage_trace =
  EffectStageMismatch
    {
      required_stage;
      actual_stage;
      span;
      function_name = Some function_name;
      capability;
      stage_trace;
    }

let effect_invalid_attribute_error ~function_name ~profile ~invalid =
  EffectInvalidAttribute
    { function_name = Some function_name; profile; invalid_attribute = invalid }

let effect_residual_leak_error ~function_name ~profile ~leaks =
  EffectResidualLeak { function_name; profile; leaks }

let ffi_contract_symbol_missing_error normalized =
  FfiContractSymbolMissing normalized

let ffi_contract_ownership_mismatch_error normalized =
  FfiContractOwnershipMismatch normalized

let ffi_contract_unsupported_abi_error normalized =
  FfiContractUnsupportedAbi normalized

let append_runtime_stage_trace ?capability stage_trace ~actual_stage =
  let has_runtime =
    List.exists
      (fun (step : stage_trace_step) -> String.equal step.source "runtime")
      stage_trace
  in
  if has_runtime then stage_trace
  else
    let runtime_step =
      match capability with
      | Some cap ->
          make_stage_trace_step ~stage:actual_stage ~capability:cap "runtime"
      | None -> make_stage_trace_step ~stage:actual_stage "runtime"
    in
    let rec aux acc inserted = function
      | [] ->
          let acc = if inserted then acc else runtime_step :: acc in
          List.rev acc
      | ({ source; _ } as step) :: rest ->
          if (not inserted) && String.equal source "typer" then
            aux (runtime_step :: step :: acc) true rest
          else aux (step :: acc) inserted rest
    in
    aux [] false stage_trace

(** ミュータブルでない束縛への代入エラーを生成 *)
let immutable_binding_error name span = ImmutableBinding { name; span }

(** continue をループ外で使用した際のエラーを生成 *)
let continue_outside_loop_error span = ContinueOutsideLoop span

(** 関数型でないエラーを生成 *)
let not_a_function_error ty span = NotAFunction (ty, span)

(** 条件式がBool型でないエラーを生成 *)
let condition_not_bool_error ty span = ConditionNotBool (ty, span)

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

(** バイトオフセットから行列番号を計算
 *
 * ソース文字列を線形走査して、指定されたオフセットの行列番号を計算する
 *)
let compute_line_column (source : string) (offset : int) : int * int =
  let len = String.length source in
  let safe_offset = min offset len in

  let rec loop line col idx =
    if idx >= safe_offset then (line, col)
    else
      let c = source.[idx] in
      if c = '\n' then loop (line + 1) 1 (idx + 1)
      else loop line (col + 1) (idx + 1)
  in
  loop 1 1 0

(** Ast.span から Diagnostic.span への変換（ソース文字列付き） *)
let span_to_diagnostic_span_with_source (source : string) (filename : string)
    (ast_span : span) : Diagnostic.span =
  let start_line, start_col = compute_line_column source ast_span.start in
  let end_line, end_col = compute_line_column source ast_span.end_ in
  {
    Diagnostic.start_pos =
      {
        Diagnostic.filename;
        line = start_line;
        column = start_col;
        offset = ast_span.start;
      };
    Diagnostic.end_pos =
      {
        Diagnostic.filename;
        line = end_line;
        column = end_col;
        offset = ast_span.end_;
      };
  }

(** Ast.span から Diagnostic.span への変換（簡易版・後方互換）
 *
 * ソース文字列が利用できない場合の簡易実装
 *)
let span_to_diagnostic_span (ast_span : span) : Diagnostic.span =
  (* ダミーの location を作成（後方互換のため維持） *)
  let make_loc offset =
    {
      Diagnostic.filename = "<入力>";
      line = 1;
      (* 簡易版では行番号を計算しない *)
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
        let cost = if s1.[i - 1] = s2.[j - 1] then 0 else 1 in
        dp.(i).(j) <-
          min
            (min (dp.(i - 1).(j) + 1) (* 削除 *) (dp.(i).(j - 1) + 1)) (* 挿入 *)
            (dp.(i - 1).(j - 1) + cost)
        (* 置換 *)
      done
    done;
    dp.(len1).(len2)

(** 類似変数名の提案
 *
 * 編集距離2以内の変数を候補として返す
 *)
let suggest_similar_names (target : string) (candidates : string list) :
    string list =
  candidates
  |> List.map (fun name -> (name, levenshtein_distance target name))
  |> List.filter (fun (_, dist) -> dist > 0 && dist <= 2)
  |> List.sort (fun (_, d1) (_, d2) -> compare d1 d2)
  |> List.map fst
  |> fun names ->
  if List.length names > 5 then List.filteri (fun i _ -> i < 5) names else names

(** 型の差分を構造的に比較して説明を生成
 *
 * 型不一致の詳細を分かりやすく説明する
 *)
let explain_type_mismatch (expected : ty) (actual : ty) : string option =
  match (expected, actual) with
  (* タプルの要素型不一致 *)
  | TTuple exp_elems, TTuple act_elems
    when List.length exp_elems = List.length act_elems ->
      let mismatches =
        List.mapi
          (fun i (e, a) ->
            if not (Types.type_equal e a) then
              Some
                (Printf.sprintf "要素 %d: 期待 %s、実際 %s" i (string_of_ty e)
                   (string_of_ty a))
            else None)
          (List.combine exp_elems act_elems)
        |> List.filter_map (fun x -> x)
      in
      if mismatches <> [] then
        Some ("タプルの要素型が異なります:\n  " ^ String.concat "\n  " mismatches)
      else None
  (* 関数型の引数・返り値不一致 *)
  | TArrow (exp_arg, exp_ret), TArrow (act_arg, act_ret) ->
      if not (Types.type_equal exp_arg act_arg) then
        Some
          (Printf.sprintf "関数の引数型が異なります: 期待 %s、実際 %s" (string_of_ty exp_arg)
             (string_of_ty act_arg))
      else if not (Types.type_equal exp_ret act_ret) then
        Some
          (Printf.sprintf "関数の返り値型が異なります: 期待 %s、実際 %s" (string_of_ty exp_ret)
             (string_of_ty act_ret))
      else None
  (* その他の型不一致は一般的なメッセージのみ *)
  | _ -> None

(** 型エラーから診断情報への変換
 *
 * 仕様書 2-5 §B-11 に従い、構造化診断を生成
 *)
let to_diagnostic (err : type_error) : Diagnostic.t =
  let open Diagnostic in
  match err with
  | UnificationFailure (expected, actual, span) ->
      let message = "型が一致しません" in
      let diag_span = span_to_diagnostic_span span in

      (* 型差分の詳細説明 *)
      let detailed_explanation =
        match explain_type_mismatch expected actual with
        | Some explanation -> [ (None, explanation) ]
        | None -> []
      in

      let notes =
        [
          (None, Printf.sprintf "期待される型: %s" (string_of_ty expected));
          (None, Printf.sprintf "実際の型:     %s" (string_of_ty actual));
        ]
        @ detailed_explanation
      in

      (* FixIt: 型注釈の追加を提案 *)
      let fixits =
        match expected with
        | Types.TVar _ ->
            (* 期待型が型変数の場合は型注釈を提案 *)
            [
              Insert
                {
                  at = diag_span;
                  text = Printf.sprintf ": %s" (string_of_ty actual);
                };
            ]
        | _ -> []
      in

      make_type_error ~code:"E7001" ~message ~span:diag_span ~notes ~fixits ()
  | OccursCheck (tv, ty, span) ->
      let message = "無限型が検出されました" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          ( None,
            Printf.sprintf "型変数 %s が型 %s の中に出現しています" (string_of_type_var tv)
              (string_of_ty ty) );
          (None, "再帰型を定義する場合は、明示的な型注釈が必要です");
        ]
      in

      make_type_error ~code:"E7002" ~message ~span:diag_span ~notes ()
  | UnboundVariable (name, span) ->
      let message = Printf.sprintf "変数 '%s' が定義されていません" name in
      let diag_span = span_to_diagnostic_span span in

      (* TODO: スコープ内の変数リストから類似名を提案 *)
      (* 現時点では提案なし *)
      let notes = [ (None, "この変数はスコープ内に存在しません") ] in

      make_type_error ~code:"E7003" ~message ~span:diag_span ~notes ()
  | ArityMismatch { expected; actual; span } ->
      let message =
        Printf.sprintf "引数の数が一致しません（期待: %d、実際: %d）" expected actual
      in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          ( None,
            Printf.sprintf "この関数は %d 個の引数を期待していますが、%d 個の引数が渡されました" expected
              actual );
        ]
      in

      make_type_error ~code:"E7004" ~message ~span:diag_span ~notes ()
  | NotAFunction (ty, span) ->
      let message = "関数ではない値に引数を適用しようとしています" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [ (None, Printf.sprintf "この式の型は %s ですが、関数型が必要です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7005" ~message ~span:diag_span ~notes ()
  | ConditionNotBool (ty, span) ->
      let message = "条件式は Bool 型である必要があります" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [ (None, Printf.sprintf "この条件式の型は %s です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7006" ~message ~span:diag_span ~notes ()
  | BranchTypeMismatch { then_ty; else_ty; span } ->
      let message = "if 式の分岐の型が一致しません" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, Printf.sprintf "then 分岐の型: %s" (string_of_ty then_ty));
          (None, Printf.sprintf "else 分岐の型: %s" (string_of_ty else_ty));
          (None, "if 式の両方の分岐は同じ型を返す必要があります");
        ]
      in

      make_type_error ~code:"E7007" ~message ~span:diag_span ~notes ()
  | PatternTypeMismatch { pattern_ty; expr_ty; span } ->
      let message = "パターンと式の型が一致しません" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, Printf.sprintf "パターンの期待型: %s" (string_of_ty pattern_ty));
          (None, Printf.sprintf "式の実際の型:     %s" (string_of_ty expr_ty));
        ]
      in

      make_type_error ~code:"E7008" ~message ~span:diag_span ~notes ()
  | ConstructorArityMismatch { constructor; expected; actual; span } ->
      let message = Printf.sprintf "コンストラクタ '%s' の引数の数が一致しません" constructor in
      let diag_span = span_to_diagnostic_span span in

      (* 使用例を提供 *)
      let example =
        if expected = 0 then Printf.sprintf "%s" constructor
        else if expected = 1 then Printf.sprintf "%s(_)" constructor
        else
          let args = String.concat ", " (List.init expected (fun _ -> "_")) in
          Printf.sprintf "%s(%s)" constructor args
      in

      let notes =
        [
          (None, Printf.sprintf "期待される引数の数: %d" expected);
          (None, Printf.sprintf "実際の引数の数:     %d" actual);
          (None, Printf.sprintf "正しい使用例: %s" example);
        ]
      in

      make_type_error ~code:"E7009" ~message ~span:diag_span ~notes ()
  | TupleArityMismatch { expected; actual; span } ->
      let message = "タプルの要素数が一致しません" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, Printf.sprintf "期待される要素数: %d" expected);
          (None, Printf.sprintf "パターンの要素数: %d" actual);
        ]
      in

      (* FixIt: 要素数が不足している場合はワイルドカードの追加を提案 *)
      let fixits =
        if actual < expected then
          let missing = expected - actual in
          let wildcards =
            String.concat ", " (List.init missing (fun _ -> "_"))
          in
          [ Insert { at = diag_span; text = Printf.sprintf ", %s" wildcards } ]
        else []
      in

      make_type_error ~code:"E7010" ~message ~span:diag_span ~notes ~fixits ()
  | RecordFieldMissing { missing_fields; span } ->
      let message = "レコードに必須フィールドが不足しています" in
      let diag_span = span_to_diagnostic_span span in
      let fields_str = String.concat ", " missing_fields in
      let notes = [ (None, Printf.sprintf "不足しているフィールド: %s" fields_str) ] in

      (* FixIt: 不足フィールドの挿入を提案 *)
      let fixits =
        missing_fields
        |> List.map (fun field ->
               Insert
                 {
                   at = diag_span;
                   text = Printf.sprintf "%s: /* 値を入力 */, " field;
                 })
      in

      make_type_error ~code:"E7011" ~message ~span:diag_span ~notes ~fixits ()
  | RecordFieldUnknown { field; span } ->
      let message = Printf.sprintf "レコード型に存在しないフィールド '%s' が指定されています" field in
      let diag_span = span_to_diagnostic_span span in
      let notes = [ (None, "このフィールドはレコード型で定義されていません") ] in

      make_type_error ~code:"E7012" ~message ~span:diag_span ~notes ()
  | NotARecord (ty, span) ->
      let message = "レコード型ではない型に対してレコードパターンを使用しています" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [ (None, Printf.sprintf "この式の型は %s ですが、レコード型が必要です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7013" ~message ~span:diag_span ~notes ()
  | NotATuple (ty, span) ->
      let message = "タプル型ではない型に対してタプルパターンを使用しています" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [ (None, Printf.sprintf "この式の型は %s ですが、タプル型が必要です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7014" ~message ~span:diag_span ~notes ()
  | EmptyMatch span ->
      let message = "match 式にはアームが少なくとも1つ必要です" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [ (None, "パターンマッチのケースを追加してください") ] in

      make_type_error ~code:"E7015" ~message ~span:diag_span ~notes ()
  | TraitConstraintFailure
      {
        trait_name;
        type_args;
        reason;
        span;
        effect_stage;
        typeclass_state;
        typeclass_pending;
        typeclass_generalized;
      }
    ->
      let type_args_str =
        String.concat ", " (List.map string_of_ty type_args)
      in
      let message =
        Printf.sprintf "トレイト制約 '%s<%s>' を満たすことができません" trait_name type_args_str
      in
      let diag_span = span_to_diagnostic_span span in
      let base_notes =
        [
          (None, Printf.sprintf "理由: %s" reason); (None, "この型に対するトレイト実装が見つかりません");
        ]
      in
      let stage_notes =
        match effect_stage with
        | Some info ->
            let capability_display =
              Option.value info.capability ~default:"<未指定>"
            in
            let actual_display =
              match info.actual_stage with Some s -> s | None -> "<未検証>"
            in
            let kind_display =
              match info.iterator_kind with Some k -> k | None -> "<未指定>"
            in
            let source_display =
              match info.iterator_source with Some s -> s | None -> "<未指定>"
            in
            [
              ( None,
                Printf.sprintf "Iterator 要件: %s / 実際の Stage: %s"
                  info.iterator_required actual_display );
              ( None,
                Printf.sprintf "Capability: %s / 種別: %s / ソース: %s"
                  capability_display kind_display source_display );
            ]
        | None -> []
      in
      let notes = base_notes @ stage_notes in
      let diag =
        make_type_error ~code:"E7016" ~message ~span:diag_span ~notes ()
      in
      let diag =
        match effect_stage with
        | Some info ->
            let iterator_fields =
              [
                ("required", `String info.iterator_required);
                ( "actual",
                  match info.actual_stage with
                  | Some s -> `String s
                  | None -> `Null );
                ( "kind",
                  match info.iterator_kind with
                  | Some k -> `String k
                  | None -> `Null );
                ( "capability",
                  match info.capability with
                  | Some c -> `String c
                  | None -> `Null );
                ( "source",
                  match info.iterator_source with
                  | Some s -> `String s
                  | None -> `Null );
              ]
            in
            with_effect_stage_extension ?actual_stage:info.actual_stage
              ?residual:info.residual ?provider:info.provider
              ?manifest_path:info.manifest_path
              ?capability_meta:info.capability_metadata ~iterator_fields
              ~stage_trace:info.stage_trace ~required_stage:info.required_stage
              ~capability:(Option.value info.capability ~default:"<unknown>")
              diag
        | None -> diag
      in
      let constraint_record =
        { trait_name; type_args; constraint_span = span }
      in
      let summary =
        Typeclass_metadata.make_summary ~constraint_:constraint_record
          ~resolution_state:typeclass_state ~pending:typeclass_pending
          ~generalized_typevars:typeclass_generalized ()
      in
      let diag =
        Diagnostic.set_extension "typeclass"
          (Typeclass_metadata.extension_json summary) diag
      in
      let diag =
        List.fold_left
          (fun acc (key, value) -> Diagnostic.set_extension key value acc)
          diag (Typeclass_metadata.extension_pairs summary)
      in
      let diag =
        Diagnostic.merge_audit_metadata
          (Typeclass_metadata.metadata_pairs summary) diag
      in
      diag
  | EffectStageMismatch
      {
        required_stage;
        actual_stage;
        span;
        function_name;
        capability;
        stage_trace;
      } ->
      let message =
        match function_name with
        | Some name -> Printf.sprintf "関数 '%s' の効果 Stage が実行環境の要件を満たしていません" name
        | None -> "効果 Stage が実行環境の要件を満たしていません"
      in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, Printf.sprintf "要求される Stage: %s" required_stage);
          (None, Printf.sprintf "実際の Stage: %s" actual_stage);
        ]
      in
      let diag =
        make_type_error ~code:"E7801" ~message ~span:diag_span ~notes ()
      in
      let capability_name = Option.value capability ~default:"runtime" in
      let enriched_trace =
        append_runtime_stage_trace stage_trace ~actual_stage ?capability
      in
      let diag =
        with_effect_stage_extension ~required_stage ~capability:capability_name
          ~actual_stage ~stage_trace:enriched_trace diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.required"
          (`String required_stage) diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.actual" (`String actual_stage)
          diag
      in
      let diag =
        match function_name with
        | Some name ->
            Diagnostic.set_extension "effect.stage.subject" (`String name) diag
        | None -> diag
      in
      let audit_trace =
        enriched_trace
        |> List.filter (fun step ->
               String.equal step.source "typer"
               || String.equal step.source "runtime")
        |> List.map (fun step ->
               let fields =
                 [
                   ("source", `String step.source);
                   ( "stage",
                     match step.stage with Some s -> `String s | None -> `Null
                   );
                 ]
               in
               `Assoc fields)
      in
      let audit_entries =
        let base =
          [
            ("effect.stage.required", `String required_stage);
            ("effect.stage.actual", `String actual_stage);
            ("effect.capability", `String capability_name);
          ]
        in
        let base =
          if audit_trace <> [] then ("stage_trace", `List audit_trace) :: base
          else base
        in
        List.rev base
      in
      Diagnostic.merge_audit_metadata audit_entries diag
  | EffectInvalidAttribute
      { function_name; profile; invalid_attribute = invalid } ->
      let attribute_display = invalid.attribute_display in
      let message = Printf.sprintf "効果属性 %s は無効です" attribute_display in
      let span = invalid.invalid_span in
      let diag_span = span_to_diagnostic_span span in
      let required_stage =
        stage_requirement_to_string profile.stage_requirement
      in
      let actual_stage_opt =
        match profile.resolved_stage with
        | Some stage -> Some (stage_id_to_string stage)
        | None -> None
      in
      let capability_name =
        match profile.resolved_capability with
        | Some cap -> cap
        | None -> "runtime"
      in
      let expected_msg =
        match invalid.reason with
        | UnknownAttributeKey _ ->
            "allows_effects / handles / effect / effects のいずれかのキーを指定してください"
        | UnsupportedStageValue -> "stage は文字列または StageId"
        | UnsupportedCapabilityValue -> "capability は文字列または識別子で指定してください"
        | UnknownEffectTag -> "効果タグは識別子または文字列で指定してください"
        | MissingStageValue -> "stage を指定してください"
      in
      let provided_display =
        match invalid.provided_display with
        | Some text -> text
        | None -> (
            match invalid.provided_json with
            | Some json -> Json.to_string json
            | None -> "<未指定>")
      in
      let notes =
        let base = [ (None, expected_msg) ] in
        match invalid.reason with
        | UnknownAttributeKey key ->
            base @ [ (None, Printf.sprintf "未宣言キー '%s' は使用できません" key) ]
        | UnsupportedStageValue ->
            base @ [ (None, Printf.sprintf "指定された値: %s" provided_display) ]
        | UnsupportedCapabilityValue ->
            base
            @ [ (None, Printf.sprintf "指定された capability: %s" provided_display) ]
        | UnknownEffectTag ->
            base @ [ (None, Printf.sprintf "指定された効果タグ: %s" provided_display) ]
        | MissingStageValue -> base
      in
      let diag =
        make_type_error ~code:"effects.syntax.invalid_attribute" ~message
          ~span:diag_span ~notes ()
      in
      let stage_json =
        let base =
          [
            ("required", `String required_stage);
            ( "actual",
              match actual_stage_opt with
              | Some stage -> `String stage
              | None -> `Null );
          ]
        in
        `Assoc base
      in
      let provided_field =
        match (invalid.provided_json, invalid.provided_display) with
        | Some json, _ -> json
        | None, Some display -> `String display
        | None, None -> `Null
      in
      let payload_json =
        effect_diagnostic_payload_to_json profile.diagnostic_payload
      in
      let invalids_list_json =
        `List
          (List.map invalid_attribute_to_json
             profile.diagnostic_payload.invalid_attributes)
      in
      let effects_fields =
        [
          ("attribute", `String attribute_display);
          ("expected", `String expected_msg);
          ("provided", provided_field);
          ("stage", stage_json);
          ("invalid_attributes", invalids_list_json);
          ("diagnostic_payload", payload_json);
        ]
      in
      let effects_fields =
        match profile.stage_trace with
        | [] -> effects_fields
        | trace -> ("stage_trace", stage_trace_to_json trace) :: effects_fields
      in
      let effects_fields =
        match invalid.key with
        | Some key -> ("key", `String key) :: effects_fields
        | None -> effects_fields
      in
      let effects_fields =
        match invalid.provided_display with
        | Some display ->
            ("provided_display", `String display) :: effects_fields
        | None -> effects_fields
      in
      let diag =
        Diagnostic.set_extension "effects"
          (`Assoc (List.rev effects_fields))
          diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.required"
          (`String required_stage) diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.actual"
          (match actual_stage_opt with
          | Some stage -> `String stage
          | None -> `Null)
          diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.capability"
          (`String capability_name) diag
      in
      let diag =
        match function_name with
        | Some name ->
            Diagnostic.set_extension "effect.stage.subject" (`String name) diag
        | None -> diag
      in
      let diag =
        match profile.stage_trace with
        | [] -> diag
        | trace ->
            Diagnostic.set_extension "effect.stage_trace"
              (stage_trace_to_json trace)
              diag
      in
      let diag =
        Diagnostic.set_extension "effect.invalid_attributes" invalids_list_json
          diag
      in
      let location_label =
        match (function_name, profile.source_name) with
        | Some name, _ -> name
        | None, Some source -> source
        | None, None -> "<anonymous>"
      in
      let stage_trace_json =
        match profile.stage_trace with
        | [] -> None
        | trace -> Some (stage_trace_to_json trace)
      in
      let audit_entries =
        let base =
          [
            ("effect.stage.required", `String required_stage);
            ( "effect.stage.actual",
              match actual_stage_opt with
              | Some stage -> `String stage
              | None -> `Null );
            ("effect.capability", `String capability_name);
            ( "effect.attribute.invalid",
              match invalid.key with
              | Some key -> `String key
              | None -> `String "attribute" );
            ("effect.attribute.location", `String location_label);
            ( "effect.attribute.reason",
              `String (string_of_invalid_reason invalid.reason) );
          ]
        in
        let base =
          match stage_trace_json with
          | Some json -> ("stage_trace", json) :: base
          | None -> base
        in
        List.rev base
      in
      Diagnostic.merge_audit_metadata audit_entries diag
  | EffectResidualLeak { function_name; profile; leaks } ->
      let message = "残余効果が閉じていません" in
      let diag_span = span_to_diagnostic_span profile.source_span in
      let leak_names =
        List.map (fun leak -> leak.Effect_profile.leaked_tag.effect_name) leaks
      in
      let notes =
        if leak_names = [] then [ (None, "宣言された効果集合が残余集合を包含していません") ]
        else
          List.map
            (fun name ->
              (None, Printf.sprintf "`%s` のハンドラが宣言されていないためステージ検証に失敗しました" name))
            leak_names
      in
      let diag =
        make_type_error ~code:"effects.contract.residual_leak" ~message
          ~span:diag_span ~notes ()
      in
      let declared_json =
        `List
          (List.map
             (fun tag -> `String tag.Effect_profile.effect_name)
             profile.effect_set.Effect_profile.declared)
      in
      let missing_json =
        `List (List.map (fun name -> `String name) leak_names)
      in
      let leaked_from =
        match function_name with
        | Some name -> Printf.sprintf "fn %s" name
        | None -> "fn <unknown>"
      in
      let residual_json =
        `Assoc
          [ ("missing", missing_json); ("leaked_from", `String leaked_from) ]
      in
      let required_stage =
        stage_requirement_to_string profile.stage_requirement
      in
      let actual_stage_opt =
        profile.resolved_stage |> Option.map stage_id_to_string
      in
      let capability_str =
        match profile.resolved_capability with Some cap -> cap | None -> ""
      in
      let stage_json =
        `Assoc
          [
            ("required", `String required_stage);
            ( "actual",
              match actual_stage_opt with
              | Some stage -> `String stage
              | None -> `Null );
          ]
      in
      let enriched_trace =
        match actual_stage_opt with
        | Some actual ->
            let cap_opt =
              if String.equal capability_str "" then None
              else Some capability_str
            in
            append_runtime_stage_trace profile.stage_trace ~actual_stage:actual
              ?capability:cap_opt
        | None -> profile.stage_trace
      in
      let effects_fields =
        [
          ("declared", declared_json);
          ("residual", residual_json);
          ("stage", stage_json);
        ]
      in
      let effects_fields =
        if enriched_trace <> [] then
          ("stage_trace", stage_trace_to_json enriched_trace) :: effects_fields
        else effects_fields
      in
      let diag =
        Diagnostic.set_extension "effects"
          (`Assoc (List.rev effects_fields))
          diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.required"
          (`String required_stage) diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.actual"
          (match actual_stage_opt with
          | Some stage -> `String stage
          | None -> `Null)
          diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.capability"
          (`String capability_str) diag
      in
      let diag =
        if enriched_trace <> [] then
          Diagnostic.set_extension "effect.stage_trace"
            (stage_trace_to_json enriched_trace)
            diag
        else diag
      in
      let audit_trace =
        enriched_trace
        |> List.filter (fun step ->
               String.equal step.source "typer"
               || String.equal step.source "runtime")
        |> List.map (fun step ->
               let fields =
                 [
                   ("source", `String step.source);
                   ( "stage",
                     match step.stage with Some s -> `String s | None -> `Null
                   );
                 ]
               in
               `Assoc fields)
      in
      let audit_entries =
        let base =
          [
            ("effect.stage.required", `String required_stage);
            ( "effect.stage.actual",
              match actual_stage_opt with
              | Some stage -> `String stage
              | None -> `Null );
            ("effect.capability", `String capability_str);
            ( "effect.residual.tags",
              `List (List.map (fun name -> `String name) leak_names) );
          ]
        in
        let base =
          if audit_trace <> [] then ("stage_trace", `List audit_trace) :: base
          else base
        in
        List.rev base
      in
      Diagnostic.merge_audit_metadata audit_entries diag
  | FfiContractSymbolMissing normalized ->
      let span =
        span_to_diagnostic_span normalized.Ffi.contract.Ffi.source_span
      in
      let message =
        Printf.sprintf "外部関数 `%s` にリンクシンボルが指定されていません"
          normalized.contract.extern_name
      in
      let notes =
        [ (None, "extern 宣言に `link_name` 属性を追加し、ブリッジが参照する シンボル名を明示してください") ]
      in
      let diag =
        make_type_error ~code:"ffi.contract.symbol_missing" ~message ~span
          ~notes ()
      in
      let diag =
        Diagnostic.set_extension "bridge"
          (Ffi.bridge_json_of_normalized normalized)
          diag
      in
      let audit_entries =
        [
          ( "bridge",
            Ffi.bridge_json_of_normalized ~status:"error" normalized );
          ( "bridge.source_span",
            Ffi.span_to_json normalized.contract.source_span );
        ]
      in
      Diagnostic.merge_audit_metadata audit_entries diag
  | FfiContractOwnershipMismatch normalized ->
      let span =
        span_to_diagnostic_span normalized.Ffi.contract.Ffi.source_span
      in
      let message =
        Printf.sprintf "外部関数 `%s` の所有権契約が無効です" normalized.contract.extern_name
      in
      let specified =
        match normalized.ownership_raw with
        | Some raw when String.trim raw <> "" -> Printf.sprintf "`%s`" raw
        | _ -> "`(未指定)`"
      in
      let notes =
        [
          (None, Printf.sprintf "指定された値: %s" specified);
          ( None,
            Printf.sprintf "サポートされる値: %s"
              (String.concat ", " Ffi.supported_ownership_labels) );
        ]
      in
      let diag =
        make_type_error ~code:"ffi.contract.ownership_mismatch" ~message ~span
          ~notes ()
      in
      let diag =
        Diagnostic.set_extension "bridge"
          (Ffi.bridge_json_of_normalized normalized)
          diag
      in
      let audit_entries =
        [
          ( "bridge",
            Ffi.bridge_json_of_normalized ~status:"error" normalized );
          ( "bridge.source_span",
            Ffi.span_to_json normalized.contract.source_span );
        ]
      in
      Diagnostic.merge_audit_metadata audit_entries diag
  | FfiContractUnsupportedAbi normalized ->
      let span =
        span_to_diagnostic_span normalized.Ffi.contract.Ffi.source_span
      in
      let message =
        Printf.sprintf "外部関数 `%s` の ABI 契約がターゲットと整合していません"
          normalized.contract.extern_name
      in
      let actual =
        match normalized.abi_raw with
        | Some raw when String.trim raw <> "" -> Printf.sprintf "`%s`" raw
        | _ -> "`(未指定)`"
      in
      let expected_note =
        match normalized.expected_abi with
        | Some expected ->
            Printf.sprintf "要求される ABI: %s" (Ffi.string_of_abi_kind expected)
        | None -> "ターゲットが未指定のため適切な ABI を決定できません"
      in
      let supplementary =
        [
          (None, Printf.sprintf "指定された値: %s" actual);
          (None, expected_note);
          ( None,
            Printf.sprintf "サポートされる値: %s"
              (String.concat ", " Ffi.supported_abi_labels) );
        ]
      in
      let diag =
        make_type_error ~code:"ffi.contract.unsupported_abi" ~message ~span
          ~notes:supplementary ()
      in
      let diag =
        Diagnostic.set_extension "bridge"
          (Ffi.bridge_json_of_normalized normalized)
          diag
      in
      let audit_entries =
        [
          ( "bridge",
            Ffi.bridge_json_of_normalized ~status:"error" normalized );
          ( "bridge.source_span",
            Ffi.span_to_json normalized.contract.source_span );
        ]
      in
      Diagnostic.merge_audit_metadata audit_entries diag
  | AmbiguousTraitImpl { trait_name; type_args; candidates; span } ->
      let type_args_str =
        String.concat ", " (List.map string_of_ty type_args)
      in
      let message =
        Printf.sprintf "トレイト '%s<%s>' の実装が曖昧です" trait_name type_args_str
      in
      let diag_span = span_to_diagnostic_span span in
      let candidates_str =
        candidates
        |> List.map Constraint_solver.string_of_dict_ref
        |> String.concat "\n  - "
      in
      let notes =
        [
          (None, "複数の候補実装が見つかりました:");
          (None, Printf.sprintf "  - %s" candidates_str);
          (None, "型注釈を追加して曖昧性を解消してください");
        ]
      in

      let diag =
        make_type_error ~code:"E7017" ~message ~span:diag_span ~notes ()
      in
      let constraint_record =
        { trait_name; type_args; constraint_span = span }
      in
      let summary =
        Typeclass_metadata.make_summary ~constraint_:constraint_record
          ~resolution_state:Typeclass_metadata.Ambiguous ~candidates ()
      in
      let diag =
        Diagnostic.set_extension "typeclass"
          (Typeclass_metadata.extension_json summary) diag
      in
      let diag =
        List.fold_left
          (fun acc (key, value) -> Diagnostic.set_extension key value acc)
          diag (Typeclass_metadata.extension_pairs summary)
      in
      let diag =
        Diagnostic.merge_audit_metadata
          (Typeclass_metadata.metadata_pairs summary) diag
      in
      diag
  | CyclicTraitConstraint { cycle; span } ->
      let cycle_str = String.concat " -> " cycle in
      let message = "トレイト制約に循環依存が検出されました" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, Printf.sprintf "循環: %s -> ..." cycle_str);
          (None, "トレイト制約の依存関係に循環があると解決できません");
        ]
      in

      let diag =
        make_type_error ~code:"E7018" ~message ~span:diag_span ~notes ()
      in
      let constraint_record =
        { trait_name = "<cyclic>"; type_args = []; constraint_span = span }
      in
      let summary =
        Typeclass_metadata.make_summary ~constraint_:constraint_record
          ~resolution_state:Typeclass_metadata.Cyclic ~pending:cycle ()
      in
      let diag =
        Diagnostic.set_extension "typeclass"
          (Typeclass_metadata.extension_json summary) diag
      in
      let diag =
        List.fold_left
          (fun acc (key, value) -> Diagnostic.set_extension key value acc)
          diag (Typeclass_metadata.extension_pairs summary)
      in
      let diag =
        Diagnostic.merge_audit_metadata
          (Typeclass_metadata.metadata_pairs summary) diag
      in
      diag
  | NotAssignable { span } ->
      let message = "この式は代入可能な左辺値ではありません" in
      let diag_span = span_to_diagnostic_span span in
      let notes = [ (None, "再代入できるのは var で宣言した変数などの左辺値のみです") ] in
      make_type_error ~code:"E7019" ~message ~span:diag_span ~notes ()
  | ImmutableBinding { name; span } ->
      let message = "不変な束縛に対して再代入はできません" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, Printf.sprintf "変数 '%s' は let で宣言されています" name);
          (None, "値を変更する必要がある場合は `var` を使用してください");
        ]
      in
      make_type_error ~code:"E7020" ~message ~span:diag_span ~notes ()
  | ContinueOutsideLoop span ->
      let message = "continue はループ内でのみ使用できます" in
      let diag_span = span_to_diagnostic_span span in
      let notes =
        [
          (None, "while / for / loop のボディ以外では使用できません");
          (None, "制御フローを継続したい場合はループ内に配置してください");
        ]
      in
      make_type_error ~code:"E7021" ~message ~span:diag_span ~notes ()

(** 環境情報を使った診断情報の生成
 *
 * スコープ内の変数リストから類似名を提案できる
 *)
let to_diagnostic_with_env ?(available_names : string list = [])
    (err : type_error) : Diagnostic.t =
  let open Diagnostic in
  match err with
  | UnboundVariable (name, span) ->
      let message = Printf.sprintf "変数 '%s' が定義されていません" name in
      let diag_span = span_to_diagnostic_span span in

      (* 類似変数名の提案 *)
      let suggestions = suggest_similar_names name available_names in
      let suggestion_notes =
        match suggestions with
        | [] -> [ (None, "この変数はスコープ内に存在しません") ]
        | names ->
            let candidates = String.concat ", " names in
            [
              (None, "この変数はスコープ内に存在しません");
              (None, Printf.sprintf "もしかして: %s ?" candidates);
            ]
      in

      make_type_error ~code:"E7003" ~message ~span:diag_span
        ~notes:suggestion_notes ()
  | other_error ->
      (* その他のエラーは通常の変換を使用 *)
      to_diagnostic other_error

(** ソースコード情報を使った診断情報の生成
 *
 * 正確な行列番号を含む診断情報を生成する
 *)
let to_diagnostic_with_source ?(available_names : string list = [])
    (source : string) (filename : string) (err : type_error) : Diagnostic.t =
  let open Diagnostic in
  (* Span変換用のヘルパー関数 *)
  let make_span = span_to_diagnostic_span_with_source source filename in

  match err with
  | UnificationFailure (expected, actual, span) ->
      let message = "型が一致しません" in
      let diag_span = make_span span in

      (* 型差分の詳細説明 *)
      let detailed_explanation =
        match explain_type_mismatch expected actual with
        | Some explanation -> [ (None, explanation) ]
        | None -> []
      in

      let notes =
        [
          (None, Printf.sprintf "期待される型: %s" (string_of_ty expected));
          (None, Printf.sprintf "実際の型:     %s" (string_of_ty actual));
        ]
        @ detailed_explanation
      in

      (* FixIt: 型注釈の追加を提案 *)
      let fixits =
        match expected with
        | Types.TVar _ ->
            [
              Insert
                {
                  at = diag_span;
                  text = Printf.sprintf ": %s" (string_of_ty actual);
                };
            ]
        | _ -> []
      in

      make_type_error ~code:"E7001" ~message ~span:diag_span ~notes ~fixits ()
  | OccursCheck (tv, ty, span) ->
      let message = "無限型が検出されました" in
      let diag_span = make_span span in
      let notes =
        [
          ( None,
            Printf.sprintf "型変数 %s が型 %s の中に出現しています" (string_of_type_var tv)
              (string_of_ty ty) );
          (None, "再帰型を定義する場合は、明示的な型注釈が必要です");
        ]
      in

      make_type_error ~code:"E7002" ~message ~span:diag_span ~notes ()
  | UnboundVariable (name, span) ->
      let message = Printf.sprintf "変数 '%s' が定義されていません" name in
      let diag_span = make_span span in

      (* 類似変数名の提案 *)
      let suggestions = suggest_similar_names name available_names in
      let suggestion_notes =
        match suggestions with
        | [] -> [ (None, "この変数はスコープ内に存在しません") ]
        | names ->
            let candidates = String.concat ", " names in
            [
              (None, "この変数はスコープ内に存在しません");
              (None, Printf.sprintf "もしかして: %s ?" candidates);
            ]
      in

      make_type_error ~code:"E7003" ~message ~span:diag_span
        ~notes:suggestion_notes ()
  | ArityMismatch { expected; actual; span } ->
      let message =
        Printf.sprintf "引数の数が一致しません（期待: %d、実際: %d）" expected actual
      in
      let diag_span = make_span span in
      let notes =
        [
          ( None,
            Printf.sprintf "この関数は %d 個の引数を期待していますが、%d 個の引数が渡されました" expected
              actual );
        ]
      in

      make_type_error ~code:"E7004" ~message ~span:diag_span ~notes ()
  | NotAFunction (ty, span) ->
      let message = "関数ではない値に引数を適用しようとしています" in
      let diag_span = make_span span in
      let notes =
        [ (None, Printf.sprintf "この式の型は %s ですが、関数型が必要です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7005" ~message ~span:diag_span ~notes ()
  | ConditionNotBool (ty, span) ->
      let message = "条件式は Bool 型である必要があります" in
      let diag_span = make_span span in
      let notes =
        [ (None, Printf.sprintf "この条件式の型は %s です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7006" ~message ~span:diag_span ~notes ()
  | BranchTypeMismatch { then_ty; else_ty; span } ->
      let message = "if 式の分岐の型が一致しません" in
      let diag_span = make_span span in
      let notes =
        [
          (None, Printf.sprintf "then 分岐の型: %s" (string_of_ty then_ty));
          (None, Printf.sprintf "else 分岐の型: %s" (string_of_ty else_ty));
          (None, "if 式の両方の分岐は同じ型を返す必要があります");
        ]
      in

      make_type_error ~code:"E7007" ~message ~span:diag_span ~notes ()
  | PatternTypeMismatch { pattern_ty; expr_ty; span } ->
      let message = "パターンと式の型が一致しません" in
      let diag_span = make_span span in
      let notes =
        [
          (None, Printf.sprintf "パターンの期待型: %s" (string_of_ty pattern_ty));
          (None, Printf.sprintf "式の実際の型:     %s" (string_of_ty expr_ty));
        ]
      in

      make_type_error ~code:"E7008" ~message ~span:diag_span ~notes ()
  | ConstructorArityMismatch { constructor; expected; actual; span } ->
      let message = Printf.sprintf "コンストラクタ '%s' の引数の数が一致しません" constructor in
      let diag_span = make_span span in

      (* 使用例を提供 *)
      let example =
        if expected = 0 then Printf.sprintf "%s" constructor
        else if expected = 1 then Printf.sprintf "%s(_)" constructor
        else
          let args = String.concat ", " (List.init expected (fun _ -> "_")) in
          Printf.sprintf "%s(%s)" constructor args
      in

      let notes =
        [
          (None, Printf.sprintf "期待される引数の数: %d" expected);
          (None, Printf.sprintf "実際の引数の数:     %d" actual);
          (None, Printf.sprintf "正しい使用例: %s" example);
        ]
      in

      make_type_error ~code:"E7009" ~message ~span:diag_span ~notes ()
  | TupleArityMismatch { expected; actual; span } ->
      let message = "タプルの要素数が一致しません" in
      let diag_span = make_span span in
      let notes =
        [
          (None, Printf.sprintf "期待される要素数: %d" expected);
          (None, Printf.sprintf "パターンの要素数: %d" actual);
        ]
      in

      (* FixIt: 要素数が不足している場合はワイルドカードの追加を提案 *)
      let fixits =
        if actual < expected then
          let missing = expected - actual in
          let wildcards =
            String.concat ", " (List.init missing (fun _ -> "_"))
          in
          [ Insert { at = diag_span; text = Printf.sprintf ", %s" wildcards } ]
        else []
      in

      make_type_error ~code:"E7010" ~message ~span:diag_span ~notes ~fixits ()
  | RecordFieldMissing { missing_fields; span } ->
      let message = "レコードに必須フィールドが不足しています" in
      let diag_span = make_span span in
      let fields_str = String.concat ", " missing_fields in
      let notes = [ (None, Printf.sprintf "不足しているフィールド: %s" fields_str) ] in

      (* FixIt: 不足フィールドの挿入を提案 *)
      let fixits =
        missing_fields
        |> List.map (fun field ->
               Insert
                 {
                   at = diag_span;
                   text = Printf.sprintf "%s: /* 値を入力 */, " field;
                 })
      in

      make_type_error ~code:"E7011" ~message ~span:diag_span ~notes ~fixits ()
  | RecordFieldUnknown { field; span } ->
      let message = Printf.sprintf "レコード型に存在しないフィールド '%s' が指定されています" field in
      let diag_span = make_span span in
      let notes = [ (None, "このフィールドはレコード型で定義されていません") ] in

      make_type_error ~code:"E7012" ~message ~span:diag_span ~notes ()
  | NotARecord (ty, span) ->
      let message = "レコード型ではない型に対してレコードパターンを使用しています" in
      let diag_span = make_span span in
      let notes =
        [ (None, Printf.sprintf "この式の型は %s ですが、レコード型が必要です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7013" ~message ~span:diag_span ~notes ()
  | NotATuple (ty, span) ->
      let message = "タプル型ではない型に対してタプルパターンを使用しています" in
      let diag_span = make_span span in
      let notes =
        [ (None, Printf.sprintf "この式の型は %s ですが、タプル型が必要です" (string_of_ty ty)) ]
      in

      make_type_error ~code:"E7014" ~message ~span:diag_span ~notes ()
  | EmptyMatch span ->
      let message = "match 式にはアームが少なくとも1つ必要です" in
      let diag_span = make_span span in
      let notes = [ (None, "パターンマッチのケースを追加してください") ] in

      make_type_error ~code:"E7015" ~message ~span:diag_span ~notes ()
  | TraitConstraintFailure
      {
        trait_name;
        type_args;
        reason;
        span;
        effect_stage;
        typeclass_state = _;
        typeclass_pending = _;
        typeclass_generalized = _;
      }
    ->
      let type_args_str =
        String.concat ", " (List.map string_of_ty type_args)
      in
      let message =
        Printf.sprintf "トレイト制約 '%s<%s>' を満たすことができません" trait_name type_args_str
      in
      let diag_span = make_span span in
      let base_notes =
        [
          (None, Printf.sprintf "理由: %s" reason); (None, "この型に対するトレイト実装が見つかりません");
        ]
      in
      let stage_notes =
        match effect_stage with
        | Some info ->
            let capability_display =
              Option.value info.capability ~default:"<未指定>"
            in
            let actual_display =
              match info.actual_stage with Some s -> s | None -> "<未検証>"
            in
            let kind_display =
              match info.iterator_kind with Some k -> k | None -> "<未指定>"
            in
            let source_display =
              match info.iterator_source with Some s -> s | None -> "<未指定>"
            in
            [
              ( None,
                Printf.sprintf "Iterator 要件: %s / 実際の Stage: %s"
                  info.iterator_required actual_display );
              ( None,
                Printf.sprintf "Capability: %s / 種別: %s / ソース: %s"
                  capability_display kind_display source_display );
            ]
        | None -> []
      in
      let notes = base_notes @ stage_notes in
      let diag =
        make_type_error ~code:"E7016" ~message ~span:diag_span ~notes ()
      in
      let diag =
        match effect_stage with
        | Some info ->
            let iterator_fields =
              [
                ("required", `String info.iterator_required);
                ( "actual",
                  match info.actual_stage with
                  | Some s -> `String s
                  | None -> `Null );
                ( "kind",
                  match info.iterator_kind with
                  | Some k -> `String k
                  | None -> `Null );
                ( "capability",
                  match info.capability with
                  | Some c -> `String c
                  | None -> `Null );
                ( "source",
                  match info.iterator_source with
                  | Some s -> `String s
                  | None -> `Null );
              ]
            in
            with_effect_stage_extension ?actual_stage:info.actual_stage
              ?residual:info.residual ?provider:info.provider
              ?manifest_path:info.manifest_path
              ?capability_meta:info.capability_metadata ~iterator_fields
              ~stage_trace:info.stage_trace ~required_stage:info.required_stage
              ~capability:(Option.value info.capability ~default:"<unknown>")
              diag
        | None -> diag
      in
      diag
  | EffectStageMismatch
      {
        required_stage;
        actual_stage;
        span;
        function_name;
        capability;
        stage_trace;
      } ->
      let message =
        match function_name with
        | Some name -> Printf.sprintf "関数 '%s' の効果 Stage が実行環境の要件を満たしていません" name
        | None -> "効果 Stage が実行環境の要件を満たしていません"
      in
      let diag_span = make_span span in
      let notes =
        [
          (None, Printf.sprintf "要求される Stage: %s" required_stage);
          (None, Printf.sprintf "実際の Stage: %s" actual_stage);
        ]
      in
      let diag =
        make_type_error ~code:"E7801" ~message ~span:diag_span ~notes ()
      in
      let capability_name = Option.value capability ~default:"runtime" in
      let enriched_trace =
        append_runtime_stage_trace stage_trace ~actual_stage ?capability
      in
      let diag =
        with_effect_stage_extension ~required_stage ~capability:capability_name
          ~actual_stage ~stage_trace:enriched_trace diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.required"
          (`String required_stage) diag
      in
      let diag =
        Diagnostic.set_extension "effect.stage.actual" (`String actual_stage)
          diag
      in
      let diag =
        match function_name with
        | Some name ->
            Diagnostic.set_extension "effect.stage.subject" (`String name) diag
        | None -> diag
      in
      diag
  | EffectInvalidAttribute _ as invalid -> to_diagnostic invalid
  | EffectResidualLeak _ as leak -> to_diagnostic leak
  | FfiContractSymbolMissing _ | FfiContractOwnershipMismatch _
  | FfiContractUnsupportedAbi _ ->
      to_diagnostic err
  | AmbiguousTraitImpl { trait_name; type_args; candidates; span } ->
      let type_args_str =
        String.concat ", " (List.map string_of_ty type_args)
      in
      let message =
        Printf.sprintf "トレイト '%s<%s>' の実装が曖昧です" trait_name type_args_str
      in
      let diag_span = make_span span in
      let candidates_str =
        candidates
        |> List.map Constraint_solver.string_of_dict_ref
        |> String.concat "\n  - "
      in
      let notes =
        [
          (None, "複数の候補実装が見つかりました:");
          (None, Printf.sprintf "  - %s" candidates_str);
          (None, "型注釈を追加して曖昧性を解消してください");
        ]
      in

      let diag =
        make_type_error ~code:"E7017" ~message ~span:diag_span ~notes ()
      in
      let constraint_record =
        { trait_name; type_args; constraint_span = span }
      in
      let summary =
        Typeclass_metadata.make_summary ~constraint_:constraint_record
          ~resolution_state:Typeclass_metadata.Ambiguous ~candidates ()
      in
      let diag =
        Diagnostic.set_extension "typeclass"
          (Typeclass_metadata.extension_json summary) diag
      in
      let diag =
        List.fold_left
          (fun acc (key, value) -> Diagnostic.set_extension key value acc)
          diag (Typeclass_metadata.extension_pairs summary)
      in
      let diag =
        Diagnostic.merge_audit_metadata
          (Typeclass_metadata.metadata_pairs summary) diag
      in
      diag
  | CyclicTraitConstraint { cycle; span } ->
      let cycle_str = String.concat " -> " cycle in
      let message = "トレイト制約に循環依存が検出されました" in
      let diag_span = make_span span in
      let notes =
        [
          (None, Printf.sprintf "循環: %s -> ..." cycle_str);
          (None, "トレイト制約の依存関係に循環があると解決できません");
        ]
      in

      let diag =
        make_type_error ~code:"E7018" ~message ~span:diag_span ~notes ()
      in
      let constraint_record =
        { trait_name = "<cyclic>"; type_args = []; constraint_span = span }
      in
      let summary =
        Typeclass_metadata.make_summary ~constraint_:constraint_record
          ~resolution_state:Typeclass_metadata.Cyclic ~pending:cycle ()
      in
      let diag =
        Diagnostic.set_extension "typeclass"
          (Typeclass_metadata.extension_json summary) diag
      in
      let diag =
        List.fold_left
          (fun acc (key, value) -> Diagnostic.set_extension key value acc)
          diag (Typeclass_metadata.extension_pairs summary)
      in
      let diag =
        Diagnostic.merge_audit_metadata
          (Typeclass_metadata.metadata_pairs summary) diag
      in
      diag
  | NotAssignable { span } ->
      let message = "この式は代入可能な左辺値ではありません" in
      let diag_span = make_span span in
      let notes = [ (None, "再代入できるのは var で宣言した変数などの左辺値のみです") ] in
      make_type_error ~code:"E7019" ~message ~span:diag_span ~notes ()
  | ImmutableBinding { name; span } ->
      let message = "不変な束縛に対して再代入はできません" in
      let diag_span = make_span span in
      let notes =
        [
          (None, Printf.sprintf "変数 '%s' は let で宣言されています" name);
          (None, "値を変更するには `var` を使用してください");
        ]
      in
      make_type_error ~code:"E7020" ~message ~span:diag_span ~notes ()
  | ContinueOutsideLoop span ->
      let message = "continue はループ内でのみ使用できます" in
      let diag_span = make_span span in
      let notes =
        [
          (None, "while / for / loop ブロックの内部でのみ使用可能です");
          (None, "制御フローを継続したい場合は対象のループへ移動してください");
        ]
      in
      make_type_error ~code:"E7021" ~message ~span:diag_span ~notes ()
