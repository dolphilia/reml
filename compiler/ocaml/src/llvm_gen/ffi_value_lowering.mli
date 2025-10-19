(** FFI Value Lowering — ブリッジスタブ関連の LLVM メタデータ生成 *)

val attach_stub_plans :
  Llvm.llcontext -> Llvm.llmodule -> Ffi_stub_builder.stub_plan list -> unit
(** [attach_stub_plans ctx module stubs]
    既存モジュールに `BridgeStubPlan` の情報をメタデータとして埋め込む。
    スタブが存在する場合は `reml.bridge.version = 1` のモジュールフラグも追加する。 *)
