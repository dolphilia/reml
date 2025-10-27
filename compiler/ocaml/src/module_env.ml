(* module_env.ml — モジュール束縛ユーティリティ
 *
 * Phase 2-5 SYNTAX-002 S4: `use` 多段ネストの束縛・診断連携を整備する。
 * 仕様（docs/spec/1-1-syntax.md §B.1）に基づき、`use` 宣言を再帰的に解釈し
 * バインディング情報へ展開するヘルパーを提供する。
 *)

open Ast

type use_binding = {
  binding_path : module_path;  (** 実際に解決するモジュール/シンボルの完全パス *)
  binding_source : ident option;  (** 元の識別子（ネストを展開した最終セグメント） *)
  binding_local : ident;  (** ローカルに束縛される識別子（alias を含む） *)
  binding_is_pub : bool;  (** `pub use` → true *)
  binding_decl_span : span;  (** 元の use 宣言全体の Span *)
}

let rec last_of_list = function
  | [] -> None
  | [ x ] -> Some x
  | _ :: xs -> last_of_list xs

let module_path_last_ident = function
  | Root ids -> last_of_list ids
  | Relative (PlainIdent id, tail) -> (
      match last_of_list tail with Some last -> Some last | None -> Some id)
  | Relative (Self, tail) -> last_of_list tail
  | Relative (Super _, tail) -> last_of_list tail

let append_ident_to_module_path path ident =
  match path with
  | Root ids -> Root (ids @ [ ident ])
  | Relative (head, tail) -> Relative (head, tail @ [ ident ])

let should_emit_binding alias nested =
  match alias with Some _ -> true | None -> Option.is_none nested

let build_binding ~decl ~path ~source ~local =
  {
    binding_path = path;
    binding_source = source;
    binding_local = local;
    binding_is_pub = decl.use_pub;
    binding_decl_span = decl.use_span;
  }

let rec flatten_use_item decl base item =
  let path = append_ident_to_module_path base item.item_name in
  let current_binding =
    if should_emit_binding item.item_alias item.item_nested then
      let local =
        match item.item_alias with Some alias -> alias | None -> item.item_name
      in
      Some
        (build_binding ~decl ~path ~source:(Some item.item_name) ~local)
    else
      None
  in
  let nested_bindings =
    match item.item_nested with
    | None -> []
    | Some nested ->
        List.concat_map (flatten_use_item decl path) nested
  in
  match current_binding with
  | Some binding -> binding :: nested_bindings
  | None -> nested_bindings

let flatten_use_tree decl = function
  | UsePath (path, alias) -> (
      let binding_source = module_path_last_ident path in
      let local =
        match (alias, binding_source) with
        | Some alias, _ -> Some alias
        | None, Some ident -> Some ident
        | None, None -> None
      in
      match (binding_source, local) with
      | _, Some local ->
          [ build_binding ~decl ~path ~source:binding_source ~local ]
      | _ -> [])
  | UseBrace (prefix, items) ->
      List.concat_map (flatten_use_item decl prefix) items

let flatten_use_decl decl = flatten_use_tree decl decl.use_tree

let flatten_use_decls decls =
  List.concat_map flatten_use_decl decls

let ident_names idents = List.map (fun id -> id.name) idents

let string_of_module_path = function
  | Root [] -> "::"
  | Root ids -> "::" ^ String.concat "." (ident_names ids)
  | Relative (Self, tail) ->
      String.concat "." ("self" :: ident_names tail)
  | Relative (Super n, tail) ->
      let prefix =
        if n <= 0 then []
        else List.init n (fun _ -> "super")
      in
      String.concat "."
        (prefix @
         (match ident_names tail with [] -> [] | names -> names))
  | Relative (PlainIdent id, tail) -> (
      match ident_names tail with
      | [] -> id.name
      | names -> String.concat "." (id.name :: names))
