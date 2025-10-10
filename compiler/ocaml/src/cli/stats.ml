(* Stats — コンパイル統計情報収集
 *
 * Phase 1-6 Week 15 の開発者体験整備タスクにおいて、
 * コンパイル過程で生成されたデータ量を記録し、パフォーマンス分析に活用する。
 *
 * 使用方法:
 *   (* Lexer *)
 *   Stats.incr_token_count ();
 *
 *   (* 最後に統計を出力 *)
 *   if opts.stats then Stats.print_stats ();
 *
 * 出力例:
 *   [STATS] ===== Compilation Statistics =====
 *   [STATS] Tokens parsed: 245
 *   [STATS] AST nodes: 87
 *   [STATS] Unify calls: 152
 *   [STATS] Optimization passes: 3
 *   [STATS] LLVM instructions: 421
 *   [STATS] ====================================
 *)

(** 統計情報カウンタ *)
type stats = {
  mutable token_count: int;          (** パースしたトークン数 *)
  mutable ast_node_count: int;       (** 生成したASTノード数 *)
  mutable unify_calls: int;          (** 型推論のunify呼び出し回数 *)
  mutable optimization_passes: int;  (** 最適化パスの適用回数 *)
  mutable llvm_instructions: int;    (** 生成したLLVM IR命令数 *)
}

(** グローバル統計カウンタ *)
let global_stats : stats = {
  token_count = 0;
  ast_node_count = 0;
  unify_calls = 0;
  optimization_passes = 0;
  llvm_instructions = 0;
}

(** 統計カウンタをリセット（テスト用）
 *
 * すべてのカウンタを0にリセットする。
 *)
let reset () =
  global_stats.token_count <- 0;
  global_stats.ast_node_count <- 0;
  global_stats.unify_calls <- 0;
  global_stats.optimization_passes <- 0;
  global_stats.llvm_instructions <- 0

(** トークン数をインクリメント
 *
 * Lexer がトークンを生成するたびに呼び出す。
 *)
let incr_token_count () =
  global_stats.token_count <- global_stats.token_count + 1

(** ASTノード数をインクリメント
 *
 * Parser がASTノードを生成するたびに呼び出す。
 *)
let incr_ast_node_count () =
  global_stats.ast_node_count <- global_stats.ast_node_count + 1

(** unify呼び出し回数をインクリメント
 *
 * Type_inference.unify が呼び出されるたびに呼び出す。
 *)
let incr_unify_calls () =
  global_stats.unify_calls <- global_stats.unify_calls + 1

(** 最適化パス適用回数をインクリメント
 *
 * Core_ir.Pipeline が最適化パスを適用するたびに呼び出す。
 *)
let incr_optimization_passes () =
  global_stats.optimization_passes <- global_stats.optimization_passes + 1

(** LLVM IR命令数をインクリメント
 *
 * Llvm_gen.Codegen がLLVM IR命令を生成するたびに呼び出す。
 *)
let incr_llvm_instructions () =
  global_stats.llvm_instructions <- global_stats.llvm_instructions + 1

(** 統計情報を取得（テスト用）
 *
 * @return 現在の統計情報
 *)
let get_stats () = global_stats

(** 統計情報を出力
 *
 * 収集した統計情報を標準エラー出力に表示する。
 *)
let print_stats () =
  Printf.eprintf "[STATS] ===== Compilation Statistics =====\n%!";
  Printf.eprintf "[STATS] Tokens parsed: %d\n%!" global_stats.token_count;
  Printf.eprintf "[STATS] AST nodes: %d\n%!" global_stats.ast_node_count;
  Printf.eprintf "[STATS] Unify calls: %d\n%!" global_stats.unify_calls;
  Printf.eprintf "[STATS] Optimization passes: %d\n%!" global_stats.optimization_passes;
  Printf.eprintf "[STATS] LLVM instructions: %d\n%!" global_stats.llvm_instructions;
  Printf.eprintf "[STATS] ====================================\n%!"

(** 統計情報をJSON形式で出力
 *
 * @return JSON文字列
 *)
let to_json () =
  Printf.sprintf {|{
  "tokens_parsed": %d,
  "ast_nodes": %d,
  "unify_calls": %d,
  "optimization_passes": %d,
  "llvm_instructions": %d
}|}
    global_stats.token_count
    global_stats.ast_node_count
    global_stats.unify_calls
    global_stats.optimization_passes
    global_stats.llvm_instructions
