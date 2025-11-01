(* core_parse.ml — Core Parse ブリッジ層 PoC
 *
 * PARSER-003 Step3: Menhir 実装へコアコンビネーター層を挿入する足場。
 * 仕様に合わせた完全実装ではなく、Phase 2-5 の PoC として
 * `rule`/`label`/`cut` 等のメタデータ付与と状態管理を最小限で提供する。
 *)

module Id = struct
  type origin = [ `Static | `Dynamic ]

  type t = {
    namespace : string;
    name : string;
    ordinal : int;
    fingerprint : int64;
    origin : origin;
  }

  let namespace t = t.namespace
  let name t = t.name
  let ordinal t = t.ordinal
  let fingerprint t = t.fingerprint
  let origin t = t.origin

  let make ~namespace ~name ~ordinal ~origin =
    let fingerprint =
      Int64.of_int (Stdlib.abs (Hashtbl.hash (namespace, name)))
    in
    { namespace; name; ordinal; fingerprint; origin }
end

module Registry = struct
  type entry = {
    key : string * string;
    id : Id.t;
  }

  let static_entries =
    [
      {
        key = ("menhir", "compilation_unit");
        id =
          Id.make ~namespace:"menhir" ~name:"compilation_unit" ~ordinal:0
            ~origin:`Static;
      };
    ]

  let table : (string * string, Id.t) Hashtbl.t =
    let tbl = Hashtbl.create 32 in
    List.iter
      (fun { key; id } -> Hashtbl.replace tbl key id)
      static_entries;
    tbl

  let next_dynamic = ref 0x1000

  let ensure ~namespace ~name =
    let key = (namespace, name) in
    match Hashtbl.find_opt table key with
    | Some id -> id
    | None ->
        let ordinal = !next_dynamic in
        incr next_dynamic;
        let id =
          Id.make ~namespace ~name ~ordinal ~origin:`Dynamic
        in
        Hashtbl.replace table key id;
        id
end

module Reply = struct
  type 'a t =
    | Ok of {
        id : Id.t option;
        value : 'a;
        span : Diagnostic.span option;
        consumed : bool;
        committed : bool;
      }
    | Err of {
        id : Id.t option;
        diagnostic : Diagnostic.t;
        consumed : bool;
        committed : bool;
      }

  let ok ?id ~value ~span ~consumed ~committed =
    Ok { id; value; span; consumed; committed }

  let err ?id ~diagnostic ~consumed ~committed =
    Err { id; diagnostic; consumed; committed }
end

module State = struct
  type t = {
    config : Parser_run_config.t;
    diag : Parser_diag_state.t;
    mutable consumed : bool;
    mutable committed : bool;
    mutable packrat_queries : int;
    mutable packrat_hits : int;
  }

  let create ~config ~diag =
    {
      config;
      diag;
      consumed = false;
      committed = false;
      packrat_queries = 0;
      packrat_hits = 0;
    }

  let config t = t.config
  let diag t = t.diag
  let consumed t = t.consumed
  let committed t = t.committed

  let mark_consumed t = t.consumed <- true
  let mark_committed t = t.committed <- true

  let with_consumed t value = t.consumed <- value
  let with_committed t value = t.committed <- value

  let record_packrat_access t ~hit =
    t.packrat_queries <- t.packrat_queries + 1;
    if hit then t.packrat_hits <- t.packrat_hits + 1

  let packrat_queries t = t.packrat_queries
  let packrat_hits t = t.packrat_hits
end

type 'a parser = State.t -> 'a Reply.t * State.t

let attach_id id reply =
  match reply with
  | Reply.Ok ok ->
      let id =
        match ok.id with Some existing -> Some existing | None -> Some id
      in
      Reply.Ok { ok with id }
  | Reply.Err err ->
      let id =
        match err.id with Some existing -> Some existing | None -> Some id
      in
      Reply.Err { err with id }

let rule ~namespace ~name parser state =
  let id = Registry.ensure ~namespace ~name in
  let reply, state' = parser state in
  (attach_id id reply, state')

let label ~printable:_ parser state = parser state

let cut parser state =
  State.mark_committed state;
  parser state

let cut_here state =
  State.mark_committed state;
  (Reply.ok ~value:() ~span:None ~consumed:(State.consumed state)
     ~committed:true, state)

let attempt parser state =
  let consumed_before = State.consumed state in
  let committed_before = State.committed state in
  match parser state with
  | Reply.Err err, state' when not err.committed ->
      State.with_consumed state' consumed_before;
      State.with_committed state' committed_before;
      (Reply.Err err, state')
  | other -> other
