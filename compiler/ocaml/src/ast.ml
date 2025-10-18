(* AST — Reml Abstract Syntax Tree (Phase 1)
 *
 * このファイルは docs/spec/1-1-syntax.md に基づいた AST 定義を提供する。
 * Phase 1 (M1マイルストーン) では構文解析とSpan情報付与に焦点を当て、
 * 型検査や効果解析は Phase 2 以降で実装する。
 *
 * 設計原則:
 * - すべてのノードに Span 情報を付与
 * - パターンマッチしやすいバリアント型
 * - 将来の拡張を見据えた柔軟な構造
 *)

(* ========== 位置情報 ========== *)

type span = {
  start : int;  (** 開始位置 (バイトオフセット) *)
  end_ : int;  (** 終了位置 (バイトオフセット) *)
}
(** バイトオフセットによる位置範囲 *)

(** 空のSpan (ダミー用) *)
let dummy_span = { start = 0; end_ = 0 }

(* ========== 識別子とパス ========== *)

type ident = { name : string; span : span }
(** 識別子 *)

(** Stage 要件（AST段階・parser版） *)
type stage_requirement_annot = StageExact of ident | StageAtLeast of ident

(** モジュールパス *)
type module_path =
  | Root of ident list  (** ::Core.Parse *)
  | Relative of relative_head * ident list  (** self.module, super.module *)

and relative_head =
  | Self
  | Super of int  (** super の連続数 (super.super → 2) *)
  | PlainIdent of ident

(* ========== リテラル ========== *)

(** 整数リテラルの基数 *)
type int_base =
  | Base2  (** 2進数 0b... *)
  | Base8  (** 8進数 0o... *)
  | Base10  (** 10進数 *)
  | Base16  (** 16進数 0x... *)

(** 文字列リテラルの種類 *)
type string_kind =
  | Normal  (** "..." C系エスケープ *)
  | Raw  (** r"..." バックスラッシュ非解釈 *)
  | Multiline  (** """...""" 複数行 *)

(** リテラル値 *)
type literal =
  | Int of string * int_base
  | Float of string
  | Char of string
  | String of string * string_kind
  | Bool of bool
  | Unit
  | Tuple of expr list
  | Array of expr list
  | Record of (ident * expr) list
(* ========== 演算子 ========== *)

(** 二項演算子 *)
and binary_op =
  | Add
  | Sub
  | Mul
  | Div
  | Mod
  | Pow
  | Eq
  | Ne
  | Lt
  | Le
  | Gt
  | Ge
  | And
  | Or
  | PipeOp

(** 単項演算子 *)
and unary_op = Not  (** ! *) | Neg  (** - *)
(* ========== 式 ========== *)

and expr = { expr_kind : expr_kind; expr_span : span }
(** 式ノード *)

and expr_kind =
  | Literal of literal
  | Var of ident
  | ModulePath of module_path * ident  (** Core.Parse.rule *)
  | Call of expr * arg list
  | Lambda of param list * type_annot option * expr
  | Pipe of expr * expr  (** x |> f *)
  | Binary of binary_op * expr * expr
  | Unary of unary_op * expr
  | FieldAccess of expr * ident  (** obj.field *)
  | TupleAccess of expr * int  (** tuple.0 *)
  | Index of expr * expr  (** arr[i] *)
  | Propagate of expr  (** expr? *)
  | If of expr * expr * expr option
  | Match of expr * match_arm list
  | While of expr * expr
  | For of pattern * expr * expr
  | Loop of expr
  | Continue
  | Block of stmt list
  | Unsafe of expr
  | Return of expr option
  | Defer of expr
  | Assign of expr * expr  (** lvalue := rvalue (lvalue は postfix_expr) *)

(** 関数引数 *)
and arg = PosArg of expr | NamedArg of ident * expr

and match_arm = {
  arm_pattern : pattern;
  arm_guard : expr option;
  arm_body : expr;
  arm_span : span;
}
(** match アーム *)

(* ========== パターン ========== *)
and pattern = { pat_kind : pattern_kind; pat_span : span }

and pattern_kind =
  | PatLiteral of literal
  | PatVar of ident
  | PatWildcard
  | PatTuple of pattern list
  | PatRecord of (ident * pattern option) list * bool
      (** bool = has_rest (..) *)
  | PatConstructor of ident * pattern list
  | PatGuard of pattern * expr

(* ========== 型注釈 ========== *)
and type_annot = { ty_kind : type_kind; ty_span : span }

and type_kind =
  | TyIdent of ident
  | TyApp of ident * type_annot list  (** Vec<T> *)
  | TyTuple of type_annot list
  | TyRecord of (ident * type_annot) list
  | TyFn of type_annot list * type_annot  (** A -> B *)

(* ========== 文 ========== *)
and stmt =
  | DeclStmt of decl
  | ExprStmt of expr
  | AssignStmt of expr * expr  (** lvalue := rvalue (lvalue は postfix_expr) *)
  | DeferStmt of expr

(* ========== 宣言 ========== *)
and decl = {
  decl_attrs : attribute list;
  decl_vis : visibility;
  decl_kind : decl_kind;
  decl_span : span;
}

and visibility = Public | Private
and attribute = { attr_name : ident; attr_args : expr list; attr_span : span }

and decl_kind =
  | LetDecl of pattern * type_annot option * expr
  | VarDecl of pattern * type_annot option * expr
  | FnDecl of fn_decl
  | TypeDecl of type_decl
  | TraitDecl of trait_decl
  | ImplDecl of impl_decl
  | ExternDecl of extern_decl
  | EffectDecl of effect_decl
  | HandlerDecl of handler_decl
  | ConductorDecl of conductor_decl

and effect_invalid_attribute_reason =
  | EffectAttrUnknownKey of ident
  | EffectAttrUnsupportedStageValue of expr option
  | EffectAttrUnsupportedCapabilityValue of expr option
  | EffectAttrUnknownEffectTag of expr
  | EffectAttrMissingStageValue

and effect_invalid_attribute = {
  invalid_attribute : attribute;
  invalid_reason : effect_invalid_attribute_reason;
  invalid_span : span;
}

and effect_profile_node = {
  effect_declared : ident list;  (** 明示された効果タグ集合（順序保持） *)
  effect_residual : ident list;  (** AST段階では declared と同一で初期化 *)
  effect_stage : stage_requirement_annot option;  (** StageRequirement (未確定可) *)
  effect_capabilities : ident list;  (** Capability 要求（規約化済み文字列） *)
  effect_span : span;  (** 宣言位置 *)
  effect_invalid_attributes : effect_invalid_attribute list;
}
(** EffectProfileNode — docs/effect-system-design-note.md §2 を参照 *)

(* 関数宣言 *)
and fn_decl = {
  fn_name : ident;
  fn_generic_params : ident list;
  fn_params : param list;
  fn_ret_type : type_annot option;
  fn_where_clause : constraint_ list;
  fn_effect_profile : effect_profile_node option;
  fn_body : fn_body;
}

and fn_body = FnExpr of expr | FnBlock of stmt list

and param = {
  pat : pattern;
  ty : type_annot option;
  default : expr option;
  param_span : span;
}

(* 型宣言 *)
and type_decl =
  | AliasDecl of ident * ident list * type_annot
  | SumDecl of ident * ident list * variant list
  | NewtypeDecl of ident * ident list * type_annot

and variant = {
  variant_name : ident;
  variant_types : type_annot list;
  variant_span : span;
}

(* トレイト宣言 *)
and trait_decl = {
  trait_name : ident;
  trait_params : ident list;
  trait_where : constraint_ list;
  trait_items : trait_item list;
}

and trait_item = {
  item_attrs : attribute list;
  item_sig : fn_signature;
  item_default : fn_body option;
}

(* impl 宣言 *)
and impl_decl = {
  impl_params : ident list;
  impl_trait : (ident * type_annot list) option;
      (** Some (trait, args) for "impl Trait for Type" *)
  impl_type : type_annot;
  impl_where : constraint_ list;
  impl_items : impl_item list;
}

and impl_item =
  | ImplFn of fn_decl
  | ImplLet of pattern * type_annot option * expr

and extern_invalid_attribute_reason =
  | ExternAttrUnknownKey of string
  | ExternAttrMissingStringValue of string
  | ExternAttrDuplicateKey of string

and extern_invalid_attribute = {
  extern_attr : attribute;
  extern_reason : extern_invalid_attribute_reason;
  extern_attr_span : span;
}

and extern_metadata = {
  extern_target : string option;
  extern_calling_convention : string option;
  extern_link_name : string option;
  extern_ownership : string option;
  extern_invalid_attributes : extern_invalid_attribute list;
}

(* extern 宣言 *)
and extern_decl = {
  extern_abi : string;
  extern_block_target : string option;
  extern_items : extern_item list;
}

and extern_item = {
  extern_attrs : attribute list;
  extern_sig : fn_signature;
  extern_metadata : extern_metadata;
}

(* 効果宣言 (実験段階) *)
and effect_decl = {
  effect_name : ident;
  effect_tag : ident;
  operations : operation_decl list;
}

and operation_decl = { op_name : ident; op_type : type_annot; op_span : span }

(* ハンドラ宣言 (実験段階) *)
and handler_decl = {
  handler_name : ident;
  handler_entries : handler_entry list;
}

and handler_entry =
  | HandlerOperation of handler_operation
  | HandlerReturn of handler_return

and handler_operation = {
  handler_op_name : ident;
  handler_op_params : param list;
  handler_op_body : stmt list;
  handler_op_span : span;
}

and handler_return = {
  handler_return_name : ident;
  handler_return_body : stmt list;
  handler_return_span : span;
}

(* Conductor宣言 (実験段階) *)
and conductor_decl = {
  conductor_name : ident;
  conductor_body : conductor_section list;
}

and conductor_section =
  | DslDef of ident * ident * expr option * ident list
      (** name : type = init |> pipes *)
  | Channels of channel_route list
  | Execution of stmt list
  | Monitoring of ident * stmt list

and channel_route = {
  from_endpoint : ident;
  to_endpoint : ident;
  channel_type : type_annot;
  route_span : span;
}

(* 制約 *)
and constraint_ = {
  constraint_trait : ident;
  constraint_types : type_annot list;
  constraint_span : span;
}

(* 関数シグネチャ (trait/extern で共通) *)
and fn_signature = {
  sig_name : ident;
  sig_params : ident list;
  sig_args : param list;
  sig_ret : type_annot option;
  sig_where : constraint_ list;
  sig_effect_profile : effect_profile_node option;
}

(* ========== use 宣言 ========== *)

type use_tree =
  | UsePath of module_path * ident option  (** use ::Core.Parse [as P] *)
  | UseBrace of module_path * use_item list
      (** use Core.{Lex, Op as Operator} *)

and use_item = {
  item_name : ident;
  item_alias : ident option;
  item_nested : use_item list option;  (** ネスト展開対応 Core.{Lex.{...}} *)
}

type use_decl = { use_pub : bool; use_tree : use_tree; use_span : span }

(* ========== モジュールヘッダ ========== *)

type module_header = { module_path : module_path; header_span : span }

(* ========== コンパイル単位 ========== *)

type compilation_unit = {
  header : module_header option;
  uses : use_decl list;
  decls : decl list;
}

(* ========== ヘルパー関数 ========== *)

(** Span の結合 *)
let merge_span s1 s2 =
  { start = min s1.start s2.start; end_ = max s1.end_ s2.end_ }

(** 識別子の作成 *)
let make_ident name span = { name; span }

(** 式の作成 *)
let make_expr expr_kind expr_span = { expr_kind; expr_span }

(** パターンの作成 *)
let make_pattern pat_kind pat_span = { pat_kind; pat_span }

(** 型注釈の作成 *)
let make_type ty_kind ty_span = { ty_kind; ty_span }
