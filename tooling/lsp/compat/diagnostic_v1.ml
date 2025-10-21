(* diagnostic_v1.ml — 診断 V1 互換変換レイヤ草案
 *
 * Phase 2-4 の移行期間中は V1/V2 の同時サポートが必要なため、
 * ここに V1 表現へのダウングレード実装を集約する予定。
 *
 * TODO:
 * - `Diagnostic_serialization.normalized_diagnostic` から V1 JSON への変換実装
 * - 欠落フィールドのフォールバック戦略検討
 *)

open Diagnostic_serialization

let lsp_position line column =
  `Assoc [ ("line", `Int line); ("character", `Int column) ]

let lsp_range span =
  `Assoc
    [
      ("start", lsp_position span.start_line span.start_col);
      ("end", lsp_position span.end_line span.end_col);
    ]

let to_v1_json (diag : normalized_diagnostic) =
  let base =
    [
      ("range", lsp_range diag.primary);
      ("severity", `Int (severity_level_of_severity diag.severity));
      ("message", `String diag.message);
      ("source", `String "reml");
    ]
  in
  let base =
    match diag.codes with
    | code :: _ -> ("code", `String code) :: base
    | [] -> base
  in
  `Assoc (List.rev base)
