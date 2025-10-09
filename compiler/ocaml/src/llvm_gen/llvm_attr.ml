(** LLVM 属性の補助ユーティリティ
 *
 * LLVM 18 以降で導入された型付き属性（typed attribute）を OCaml から扱うための
 * 薄いラッパーを提供する。llvm-ocaml 標準バインディングでは
 * [Llvm.create_string_attr] などの文字列属性しか公開されていないため、
 * `LLVMCreateTypeAttribute` を直接呼び出す C スタブを併用する。
 *)

open Llvm

external create_type_attr_by_kind
  : llcontext -> llattrkind -> lltype -> llattribute
  = "reml_llvm_create_type_attr_by_kind"

(** 任意の属性名に対して型付き属性を生成する。
 *
 * @param llctx 対象となる LLVM コンテキスト
 * @param attr_name 属性名（例: `"sret"`, `"byval"`）
 * @param llty 属性に紐付ける LLVM 型（byval/sret の場合は構造体型）
 *)
let create_typed_attr llctx attr_name llty =
  let kind = Llvm.enum_attr_kind attr_name in
  create_type_attr_by_kind llctx kind llty

(** 型付き属性を試行し、未対応の場合は文字列属性にフォールバックする。
 *
 * @param attr_name 属性名
 * @param llctx LLVM コンテキスト
 * @param llty 属性に紐付ける LLVM 型
 *)
let create_attr_with_fallback attr_name llctx llty =
  try create_typed_attr llctx attr_name llty
  with Llvm.UnknownAttribute _ -> Llvm.create_string_attr llctx attr_name ""

(** sret 用の型付き属性を生成する *)
let create_sret_attr llctx llty =
  create_attr_with_fallback "sret" llctx llty

(** byval 用の型付き属性を生成する *)
let create_byval_attr llctx llty =
  create_attr_with_fallback "byval" llctx llty
