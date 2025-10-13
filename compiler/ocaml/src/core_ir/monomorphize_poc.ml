(* Core_ir.Monomorphize_poc — 型クラスモノモルフィゼーション PoC パス
 *
 * Phase 2 M1 で検証するモノモルフィゼーション比較用の骨格実装。
 * 現時点では辞書渡し実装と同等の Core IR を維持しつつ、
 * 型推論フェーズで収集したトレイトインスタンス情報を集約して
 * 後続のベンチマーク/診断で利用できるようにする。
 *)

open Type_env
open Ir

(** 実行モード *)
type mode =
  | UseDictionary  (** 辞書渡しのみを使用 *)
  | UseMonomorph  (** モノモルフィゼーション PoC のみを使用 *)
  | UseBoth  (** 辞書渡しとモノモルフィゼーションを比較出力 *)

(** PoC パスが収集したサマリー情報 *)
module Summary = struct
  type entry = Monomorph_registry.trait_instance

  let last_mode : mode ref = ref UseDictionary
  let recorded_entries : entry list ref = ref []

  let reset () = recorded_entries := []
  let set_mode m = last_mode := m

  let record entries =
    recorded_entries := entries

  let mode () = !last_mode
  let entries () = !recorded_entries
end

(** PoC パスの適用
 *
 * 現段階では Core IR への変換は行わず、収集済みインスタンスを記録する。
 * 将来的に具象関数生成を行う際の足場として、ここで情報を共有する。
 *)
let apply ~(mode : mode) (m : module_def) : module_def =
  Summary.set_mode mode;
  (match mode with
  | UseDictionary ->
      (* 辞書渡しのみ: PoC 情報はリセットして終了 *)
      Summary.reset ()
  | UseMonomorph ->
      (* 収集済みインスタンスを保存して後続の比較に備える *)
      Summary.record (Monomorph_registry.all ())
  | UseBoth ->
      (* 比較モード: 両方式の情報を保持するために PoC 収集結果を保存 *)
      Summary.record (Monomorph_registry.all ()));
  m
