(* Version — コンパイラバージョン情報
 *
 * Phase 1-7 で追加されたバージョン管理モジュール。
 * コンパイラのバージョン番号とビルド情報を提供します。
 *)

(** コンパイラのバージョン番号 (Phase 1 完了時点) *)
let version = "0.1.0-phase1"

(** LLVM バージョン (実装で使用している LLVM のバージョン) *)
let llvm_version = "18"

(** ビルド情報を含む完全なバージョン文字列 *)
let full_version =
  Printf.sprintf "Reml OCaml コンパイラ v%s (LLVM %s)" version llvm_version

(** バージョン情報を標準出力に表示 *)
let print_version () =
  Printf.printf "%s\n" full_version;
  Printf.printf "Phase 1 Bootstrap コンパイラ (OCaml 実装)\n";
  Printf.printf "\n";
  Printf.printf "詳細:\n";
  Printf.printf "  - Phase 1-3: Parser, Typer, Core IR, LLVM IR 生成\n";
  Printf.printf "  - Phase 1-5: ランタイム連携 (最小 API)\n";
  Printf.printf "  - Phase 1-6: 開発者体験整備 (診断, トレース, CLI)\n";
  Printf.printf "  - Phase 1-7: Linux 検証インフラ (CI/CD)\n";
  Printf.printf "\n";
  Printf.printf "次の Phase: Phase 2 - 仕様安定化\n"
