(* Core_ir.Ir — Core Intermediate Representation for Reml (Phase 3)
 *
 * このファイルは docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md に基づいた
 * Core IR の型定義を提供する。
 *
 * 設計原則:
 * - Typed AST からの型情報を完全に保持
 * - SSA形式への変換を前提とした構造（Phi ノード含む）
 * - 診断・効果・Capability のメタデータを保持
 * - LLVM IR 生成への橋渡しとなる中間表現
 *
 * 参考資料:
 * - docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md §1
 * - docs/guides/llvm-integration-notes.md §3-5
 * - docs/spec/3-6-core-diagnostics-audit.md
 *)

open Types
open Ast

(* ========== 変数とラベル ========== *)

type var_id = {
  vname : string;  (** 変数名 *)
  vid : int;  (** 一意ID *)
  vty : ty;  (** 型 *)
  vspan : span;  (** 定義位置 *)
  vmutable : bool;  (** 可変変数かどうか *)
}
(** 変数ID (SSA形式準備)
 *
 * 各変数は一意なIDを持ち、型情報を保持する。
 * SSA変換時に同名変数を区別するために使用。
 *)

type label = string
(** ラベル (基本ブロック識別子) *)

(** 変数ID生成器 *)
module VarIdGen = struct
  let counter = ref 0
  let reset () = counter := 0

  let fresh ?(mutable_ = false) name ty span =
    let id = !counter in
    counter := id + 1;
    { vname = name; vid = id; vty = ty; vspan = span; vmutable = mutable_ }
end

(** ラベル生成器 *)
module LabelGen = struct
  let counter = ref 0
  let reset () = counter := 0

  let fresh prefix =
    let id = !counter in
    counter := id + 1;
    Printf.sprintf "%s_%d" prefix id
end

(* ========== プリミティブ演算 ========== *)

(** プリミティブ演算子
 *
 * LLVM IR の命令に直接対応する基本演算。
 * 型チェック済みのため、型エラーは発生しない前提。
 *)
type prim_op =
  (* 算術演算 *)
  | PrimAdd  (** 加算 (整数/浮動小数) *)
  | PrimSub  (** 減算 *)
  | PrimMul  (** 乗算 *)
  | PrimDiv  (** 除算 *)
  | PrimMod  (** 剰余 (整数のみ) *)
  | PrimPow  (** 累乗 *)
  (* 比較演算 *)
  | PrimEq  (** 等価 *)
  | PrimNe  (** 非等価 *)
  | PrimLt  (** 未満 *)
  | PrimLe  (** 以下 *)
  | PrimGt  (** 超過 *)
  | PrimGe  (** 以上 *)
  (* 論理演算 *)
  | PrimAnd  (** 論理積 (Bool) *)
  | PrimOr  (** 論理和 (Bool) *)
  | PrimNot  (** 論理否定 (Bool) *)
  (* ビット演算 (将来拡張) *)
  | PrimBitAnd  (** ビット積 *)
  | PrimBitOr  (** ビット和 *)
  | PrimBitXor  (** ビット排他的論理和 *)
  | PrimBitNot  (** ビット否定 *)
  | PrimShl  (** 左シフト *)
  | PrimShr  (** 右シフト *)
  (* 配列操作 *)
  | PrimArrayLength  (** 配列またはスライスの長さ取得 (i64) *)

(* ========== 効果とCapability ========== *)

type effect_tag = {
  effect_name : string;  (** 効果名 (例: "diagnostic", "io") *)
  effect_span : span;  (** 宣言位置 *)
}
(** 効果タグ
 *
 * 仕様書 3-6 §1: 診断・監査用の効果情報
 *)

type effect_set = { declared : effect_tag list; residual : effect_tag list }
(** 効果集合
 *
 * 関数が持つ効果の集合。
 * declared: 明示的に宣言された効果
 * residual: 推論された残余効果
 *)

type capability_id = {
  cap_name : string;  (** Capability名 *)
  cap_span : span;  (** 参照位置 *)
}
(** Capability ID
 *
 * ランタイム機能へのアクセス権限を表す。
 * Phase 3 では基本構造のみ定義、詳細は Phase 2 後半で実装。
 *)

(** Stage 要件
 *
 * Capability の成熟度要件。
 *)
type stage_requirement =
  | StageExact of string  (** 正確なステージ一致 *)
  | StageAtLeast of string  (** 最低限のステージ *)

type capability_set = {
  required : capability_id list;  (** 必要な Capability *)
  stage : stage_requirement option;  (** ステージ要件 *)
}
(** Capability 集合 *)

type dict_ref = {
  trait_name : string;  (** トレイト名 *)
  type_args : ty list;  (** 型引数 *)
  dict_span : span;  (** 参照位置 *)
}
(** 辞書参照 (型クラス/トレイト)
 *
 * 型クラス制約を満たす辞書への参照。
 * Phase 2 で辞書渡しの基盤として使用。
 *)

type dict_instance = {
  trait : string;  (** トレイト名 *)
  impl_ty : ty;  (** 実装型 *)
  methods : (string * var_id) list;  (** メソッド名 → 関数ID *)
}
(** 辞書インスタンス *)

(** 辞書型（Phase 2: 辞書渡し型システム）
 *
 * トレイト実装の実行時表現。vtable構造として扱われ、
 * LLVM IRでは関数ポインタ配列を含む構造体に変換される。
 *)
type dict_type = {
  dict_trait : string;  (** トレイト名 *)
  dict_impl_ty : ty;  (** 実装対象の型 *)
  dict_methods : (string * ty) list;  (** メソッド名と型 (vtable順) *)
  dict_layout_info : dict_layout_info option;  (** レイアウト情報（Phase 2後半で確定） *)
}

(** 辞書レイアウト情報
 *
 * LLVM IR生成時に必要な具体的なメモリレイアウト情報
 *)
and dict_layout_info = {
  vtable_size : int;  (** vtableサイズ（バイト） *)
  method_offsets : (string * int) list;  (** メソッド名 → オフセット(バイト) *)
  alignment : int;  (** アラインメント要件（バイト） *)
}

(** 辞書パラメータ（暗黙的引数）
 *
 * 関数が型クラス制約を持つ場合に自動挿入される辞書引数
 *)
type dict_param = {
  param_constraint : Types.trait_constraint;  (** 対応するトレイト制約 *)
  param_name : string;  (** パラメータ名（デバッグ用、例: "__dict_Add_T"） *)
  param_ty : ty;  (** 辞書型 *)
}

(* ========== クロージャ ========== *)

type closure_info = {
  env_vars : var_id list;  (** キャプチャされた変数 *)
  fn_ref : string;  (** 関数名への参照 *)
  closure_span : span;  (** 生成位置 *)
}
(** クロージャ情報
 *
 * クロージャの環境キャプチャと関数ポインタ情報。
 * LLVM IR では { env_ptr*, code_ptr } として表現される。
 *)

(* ========== Core IR 式 ========== *)

type expr = {
  expr_kind : expr_kind;
  expr_ty : ty;  (** 式の型 *)
  expr_span : span;  (** 位置情報 *)
}
(** Core IR 式
 *
 * すべての式は型情報 (ty) と位置情報 (span) を保持する。
 * Typed AST から糖衣を剥がした正規化された形式。
 *)

and expr_kind =
  | Literal of literal  (** リテラル値 *)
  | Var of var_id  (** 変数参照 *)
  | App of expr * expr list  (** 関数適用 *)
  | Let of var_id * expr * expr  (** let 束縛 *)
  | If of expr * expr * expr  (** if 式 (else 必須) *)
  | Match of expr * case list  (** match 式 (糖衣削除後) *)
  | Primitive of prim_op * expr list  (** プリミティブ演算 *)
  | Closure of closure_info  (** クロージャ生成 *)
  | DictLookup of dict_ref  (** 辞書参照 (型クラス) *)
  | DictConstruct of dict_type  (** 辞書構築（Phase 2: vtable初期化） *)
  | DictMethodCall of expr * string * expr list
      (** 辞書メソッド呼び出し
       * expr: 辞書値、string: メソッド名、expr list: 引数
       * vtableインデックスへ変換される
       *)
  | CapabilityCheck of capability_id  (** Capability チェック *)
  | TupleAccess of expr * int  (** タプル要素アクセス *)
  | RecordAccess of expr * string  (** レコードフィールドアクセス *)
  | ArrayAccess of expr * expr  (** 配列インデックスアクセス *)
  | ADTConstruct of string * expr list  (** ADT コンストラクタ *)
  | ADTProject of expr * int  (** ADT フィールド射影 *)
  | AssignMutable of var_id * expr  (** ミュータブル変数への代入（Unitを返す） *)
  | Loop of loop_info  (** ループ構造（CFG 展開用メタデータ） *)
  | Continue  (** continue 制御フローマーカー（Loop 内で使用） *)

and case = {
  case_pattern : simple_pattern;  (** 簡略化されたパターン *)
  case_guard : expr option;  (** ガード条件 *)
  case_body : expr;  (** 本体式 *)
  case_span : span;
}
(** match ケース
 *
 * パターンマッチの各ケース。
 * 糖衣削除後は決定木に変換される。
 *)

(** 簡略化パターン
 *
 * 複雑なパターンは糖衣削除時に決定木へ変換される。
 * ここでは基本的なパターンのみ保持。
 *)
and simple_pattern =
  | PLiteral of literal  (** リテラルパターン *)
  | PVar of var_id  (** 変数束縛 *)
  | PWildcard  (** ワイルドカード *)
  | PConstructor of string * simple_pattern list  (** コンストラクタパターン *)

and loop_kind =
  | WhileLoop of expr  (** while 条件式 *)
  | ForLoop of for_lowering  (** for イテレーション情報 *)
  | InfiniteLoop  (** 無限ループ *)

and for_lowering = {
  for_pattern : simple_pattern option;  (** ループ変数への束縛情報（未整備時は None） *)
  for_source : expr;  (** イテレータソース（配列など） *)
  for_init : (var_id * expr) list;  (** 事前初期化式（インデックスなど） *)
  for_step : (var_id * expr) list;  (** 各周回後の更新式 *)
}

and loop_source_kind =
  | LoopSourcePreheader  (** ループ突入前（preheader）からの入力 *)
  | LoopSourceLatch  (** latch ブロックからの入力（通常の更新） *)
  | LoopSourceContinue  (** continue 経路からの入力（将来拡張） *)

and loop_source = {
  ls_kind : loop_source_kind;  (** 入力元の種別 *)
  ls_span : span;  (** 入力が定義されたソース位置 *)
  ls_expr : expr;  (** 入力値を表す式 *)
}

and loop_carried_var = {
  lc_var : var_id;  (** ループ本体で再代入される変数 *)
  lc_sources : loop_source list;  (** ループヘッダ φ に取り込む候補値群 *)
}

and loop_info = {
  loop_kind : loop_kind;  (** ループの種別と付随情報 *)
  loop_body : expr;  (** ループ本体（脱糖済み） *)
  loop_span : span;  (** ループ全体のソース位置 *)
  loop_carried : loop_carried_var list;  (** PHI 候補となる変数群（暫定） *)
  loop_contains_continue : bool;  (** continue がボディ内に存在するか *)
}

(* ========== Core IR 文 ========== *)

(** Core IR 文
 *
 * 基本ブロック内の命令列を構成する。
 * SSA形式への変換を容易にするための設計。
 *)
type stmt =
  | Assign of var_id * expr  (** 代入 *)
  | Store of var_id * expr  (** ミュータブル変数への格納 *)
  | Alloca of var_id  (** ミュータブル変数用の領域確保 *)
  | Return of expr  (** 関数からの復帰 *)
  | Jump of label  (** 無条件ジャンプ *)
  | Branch of expr * label * label  (** 条件分岐 *)
  | Phi of var_id * (label * var_id) list  (** φノード (SSA) *)
  | EffectMarker of effect_info  (** 効果マーカー *)
  | ExprStmt of expr  (** 式文 *)

and effect_info = {
  effect_tag : effect_tag;
  effect_expr : expr option;  (** 効果を引き起こす式 *)
}
(** 効果情報
 *
 * 診断・監査用の効果追跡情報。
 *)

(** 終端命令
 *
 * 基本ブロックの末尾に配置される制御フロー命令。
 *)
type terminator =
  | TermReturn of expr  (** 関数復帰 *)
  | TermJump of label  (** 無条件ジャンプ *)
  | TermBranch of expr * label * label  (** 条件分岐 *)
  | TermSwitch of expr * (literal * label) list * label  (** switch (match用) *)
  | TermUnreachable  (** 到達不能 *)

(* ========== 基本ブロック ========== *)

type block = {
  label : label;  (** ブロックラベル *)
  params : var_id list;  (** ブロックパラメータ (SSA) *)
  stmts : stmt list;  (** 命令列 *)
  terminator : terminator;  (** 終端命令 *)
  block_span : span;  (** ブロック全体の位置 *)
}
(** 基本ブロック
 *
 * ラベル + 命令列 + 終端命令からなる。
 * CFG (Control Flow Graph) の基本単位。
 *)

(* ========== 関数定義 ========== *)

type param = {
  param_var : var_id;  (** パラメータ変数 *)
  param_default : expr option;  (** デフォルト値 *)
}
(** 関数パラメータ *)

type opt_flags = {
  allow_dce : bool;  (** 死コード削除を許可 *)
  allow_inline : bool;  (** インライン展開を許可 *)
  preserve_for_diagnostics : bool;  (** 診断用に保持 *)
}
(** 最適化フラグ *)

type fn_metadata = {
  fn_span : span;  (** 関数全体の位置 *)
  effects : effect_set;  (** 効果集合 *)
  capabilities : capability_set;  (** Capability 集合 *)
  dict_instances : dict_instance list;  (** 辞書インスタンス *)
  opt_flags : opt_flags;  (** 最適化フラグ *)
}
(** 関数メタデータ *)

type function_def = {
  fn_name : string;  (** 関数名 *)
  fn_params : param list;  (** パラメータリスト *)
  fn_return_ty : ty;  (** 返り値型 *)
  fn_blocks : block list;  (** 基本ブロックリスト *)
  fn_metadata : fn_metadata;  (** メタデータ *)
}
(** 関数定義 *)

(* ========== モジュール定義 ========== *)

type global_def = {
  global_var : var_id;  (** グローバル変数 *)
  global_init : expr;  (** 初期化式 *)
  global_mutable : bool;  (** 可変性 *)
}
(** グローバル変数定義 *)

type type_def = {
  type_name : string;  (** 型名 *)
  type_params : string list;  (** 型パラメータ *)
  type_variants : variant list;  (** バリアント *)
}
(** 型定義 (ADT) *)

and variant = {
  variant_name : string;  (** バリアント名 *)
  variant_fields : ty list;  (** フィールド型 *)
}

type module_def = {
  module_name : string;  (** モジュール名 *)
  type_defs : type_def list;  (** 型定義 *)
  global_defs : global_def list;  (** グローバル変数 *)
  function_defs : function_def list;  (** 関数定義 *)
}
(** モジュール定義 *)

(* ========== ヘルパー関数 ========== *)

(** 式の構築 *)
let make_expr kind ty span =
  { expr_kind = kind; expr_ty = ty; expr_span = span }

(** 基本ブロックの構築 *)
let make_block label params stmts terminator span =
  { label; params; stmts; terminator; block_span = span }

(** 関数の構築 *)
let make_function name params return_ty blocks metadata =
  {
    fn_name = name;
    fn_params = params;
    fn_return_ty = return_ty;
    fn_blocks = blocks;
    fn_metadata = metadata;
  }

(** 最適化フラグのデフォルト *)
let default_opt_flags =
  { allow_dce = true; allow_inline = true; preserve_for_diagnostics = false }

(** 空の効果集合 *)
let empty_effect_set = { declared = []; residual = [] }

(** 空のCapability集合 *)
let empty_capability_set = { required = []; stage = None }

(** デフォルトのメタデータ *)
let default_metadata span =
  {
    fn_span = span;
    effects = empty_effect_set;
    capabilities = empty_capability_set;
    dict_instances = [];
    opt_flags = default_opt_flags;
  }

(* ========== 辞書レイアウト計算 ========== *)

(** 辞書レイアウト情報を計算する
 *
 * vtableはメソッドごとに関数ポインタ（8バイト）を保持し、
 * 型情報メタデータ用に追加の8バイトを確保する。
 *
 * Phase 2 Week 19-20:
 * - ポインタサイズは x86_64 を想定（8バイト）
 * - アラインメントはポインタサイズに合わせる
 * - メソッドの順序は辞書順でソート
 *
 * @param methods メソッド名と型のリスト
 * @return 計算されたレイアウト情報
 *)
let calculate_dict_layout (methods : (string * ty) list) : dict_layout_info =
  let ptr_size = 8 in  (* x86_64 ポインタサイズ *)
  let metadata_size = 8 in  (* 型情報メタデータ *)

  (* メソッドを名前順にソート（vtable順序の決定） *)
  let sorted_methods = List.sort (fun (n1, _) (n2, _) -> String.compare n1 n2) methods in

  (* 各メソッドのオフセットを計算 *)
  let method_offsets =
    List.mapi (fun i (name, _ty) ->
      (name, i * ptr_size)
    ) sorted_methods
  in

  (* vtableサイズ = メソッド数 × ポインタサイズ + メタデータサイズ *)
  let vtable_size = (List.length sorted_methods * ptr_size) + metadata_size in

  {
    vtable_size;
    method_offsets;
    alignment = ptr_size;  (* ポインタサイズに合わせる *)
  }

(** デフォルトの辞書型を構築する
 *
 * Phase 2 Week 19-20:
 * - トレイト名と実装型から辞書型を構築
 * - メソッドリストは空の場合もある（後で埋める）
 *
 * @param trait_name トレイト名
 * @param impl_ty 実装対象の型
 * @param methods メソッド名と型のリスト
 * @return 構築された辞書型
 *)
let make_dict_type (trait_name : string) (impl_ty : ty) (methods : (string * ty) list) : dict_type =
  let layout = calculate_dict_layout methods in
  {
    dict_trait = trait_name;
    dict_impl_ty = impl_ty;
    dict_methods = methods;
    dict_layout_info = Some layout;
  }
