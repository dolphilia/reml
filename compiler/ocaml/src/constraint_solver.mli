(* Constraint_solver — Type Class Constraint Solver for Reml (Phase 2)
 *
 * このファイルは型クラス制約解決器のインターフェースを提供する。
 * 仕様書 1-2 §B（トレイト）および §G（実装規約）に基づき、
 * 制約収集から辞書参照への変換パイプラインを実装する。
 *
 * Phase 2 Week 18-19:
 * - Eq, Ord, Collector の制約規則実装
 * - 制約グラフの構築と依存関係追跡
 * - 循環依存・未解決制約の検出
 *
 * 設計原則:
 * - 制約解決と単一化の分離
 * - トレイトごとの解決ロジックの明確化
 * - エラー報告の充実
 *)

open Types
open Ast

(* ========== 基本データ構造 ========== *)

(** 辞書参照
 *
 * トレイト実装の実行時表現への参照
 * Phase 2 Week 19-20 で LLVM 生成時に具体的なポインタへ変換される
 *)
type dict_ref =
  | DictImplicit of string * ty list
      (** 暗黙の辞書（impl検索による自動解決）
       * 例: DictImplicit("Eq", [TInt I64]) → Eq<i64> の標準実装
       *     DictImplicit("Iterator", [Array<T>, T]) → Iterator<Array<T>, T> の辞書 *)
  | DictParam of int
      (** 関数引数の辞書パラメータ
       * 例: DictParam(0) → 第1引数として渡される辞書 *)
  | DictLocal of string
      (** ローカル定義の辞書
       * 例: DictLocal("user_eq") → let user_eq = ... で定義された辞書 *)

(** Iterator 辞書の種別 *)
type iterator_dict_kind =
  | IteratorArrayLike  (** 配列・スライスなど固定長コレクション *)
  | IteratorCoreIter  (** `Core.Iter.Iter<T>` など標準イテレータ状態 *)
  | IteratorOptionLike  (** Option 型由来の 0/1 要素イテレータ *)
  | IteratorResultLike  (** Result 型由来の 0/1 要素イテレータ（Ok のみ） *)
  | IteratorCustom of string
      (** ユーザー定義実装。型名や説明を保持して識別する *)

(** Iterator 辞書の Stage 要件 *)
type iterator_stage_requirement =
  | IteratorStageExact of string
  | IteratorStageAtLeast of string

(** Iterator 辞書に付随するメタデータ *)
type iterator_dict_info = {
  dict_ref : dict_ref;  (** 解決された辞書参照 *)
  source_ty : ty;  (** 反復対象となる型 *)
  element_ty : ty;  (** 要素型 *)
  kind : iterator_dict_kind;  (** 辞書の種別 *)
  stage_requirement : iterator_stage_requirement;  (** Stage 要件 *)
  capability : string option;
      (** 必要な Capability ID（不明な場合は None） *)
}

(** 制約エラー
 *
 * 制約解決失敗時のエラー情報
 * Type_error.ml の型エラーと統合される
 *)
type constraint_error = {
  trait_name : string;  (** 解決失敗したトレイト名 *)
  type_args : ty list;  (** トレイトの型引数 *)
  reason : constraint_error_reason;  (** 失敗理由 *)
  span : span;  (** エラー発生位置 *)
}

and constraint_error_reason =
  | NoImpl
      (** 該当する impl が存在しない
       * 例: Eq<CustomType> で impl が未定義 *)
  | AmbiguousImpl of dict_ref list
      (** 複数の impl が候補となり曖昧
       * 例: Eq<T> で複数の実装が一致 *)
  | CyclicConstraint of trait_constraint list
      (** 循環依存が検出された
       * 例: Eq<A> requires Eq<B>, Eq<B> requires Eq<A> *)
  | UnresolvedTypeVar of type_var
      (** 型変数が未解決のまま残っている
       * 例: 関数引数の型が推論できない *)

(** 制約グラフ
 *
 * トレイト制約間の依存関係を表現
 * トポロジカルソートと循環検出に使用
 *)
type constraint_graph = {
  nodes : trait_constraint list;
      (** グラフのノード（制約のリスト） *)
  edges : (trait_constraint * trait_constraint) list;
      (** グラフのエッジ（依存関係のリスト）
       * (c1, c2) は「c2 が c1 に依存する」を表す
       * 例: (Eq<T>, Ord<T>) → Ord<T> は Eq<T> を必要とする *)
}

(** 制約解決状態
 *
 * 制約解決の進行状態を管理
 * 逐次的な解決プロセスをトラッキング
 *)
type solver_state = {
  constraints : trait_constraint list;
      (** 解決対象の全制約 *)
  resolved : (trait_constraint * dict_ref) list;
      (** 解決済み制約と対応する辞書参照 *)
  pending : trait_constraint list;
      (** 解決待ちの制約（依存関係により順序待ち） *)
  errors : constraint_error list;
      (** 解決失敗した制約のエラー情報 *)
}

(* ========== 制約解決のメインAPI ========== *)

(** 制約解決のメインエントリポイント
 *
 * トレイト制約のリストを受け取り、辞書参照のリストまたはエラーを返す
 *
 * Phase 2 Week 18-19:
 * - Eq, Ord, Collector の解決をサポート
 * - 依存関係の自動解決
 * - 循環依存の検出とエラー報告
 *
 * Phase 2 Week 23-24:
 * - レジストリパラメータを追加
 * - ユーザー定義impl宣言の検索対応
 *
 * @param registry impl宣言レジストリ
 * @param constraints 解決対象の制約リスト
 * @return 成功時は辞書参照のリスト、失敗時はエラーのリスト
 *)
val solve_constraints :
  Impl_registry.impl_registry ->
  trait_constraint list ->
  (dict_ref list, constraint_error list) result

(** Iterator 制約専用の解決ヘルパー
 *
 * `Iterator` 制約を解決し、辞書本体に加えて Stage / Capability 情報を返す。
 *
 * @param registry impl レジストリ
 * @param constraint_ 対象の Iterator 制約
 * @return 成功時は辞書メタデータ、失敗時は制約エラー
 *)
val solve_iterator_dict :
  Impl_registry.impl_registry ->
  trait_constraint ->
  (iterator_dict_info, constraint_error) result

(** 初期状態の作成
 *
 * 制約リストから解決器の初期状態を構築
 *
 * @param constraints 解決対象の制約リスト
 * @return 初期化された solver_state
 *)
val init_solver_state : trait_constraint list -> solver_state

(** 解決を1ステップ進める
 *
 * pending から解決可能な制約を1つ取り出して解決
 * 解決できない場合は errors に追加
 *
 * Phase 2 Week 23-24: レジストリパラメータを追加
 *
 * @param registry impl宣言レジストリ
 * @param state 現在の解決状態
 * @return 更新された解決状態
 *)
val step_solver : Impl_registry.impl_registry -> solver_state -> solver_state

(** 解決が完了したか判定
 *
 * @param state 現在の解決状態
 * @return pending が空なら true
 *)
val is_solved : solver_state -> bool

(* ========== 個別トレイトの解決 ========== *)

(** Eq トレイトの解決
 *
 * 仕様書 1-2 §B.1: 等価性比較のトレイト
 *
 * 解決規則:
 * - プリミティブ型（i8..i64, u8..u64, f32, f64, Bool, Char, String）は自動実装
 * - タプル型 (A, B) は Eq<A>, Eq<B> を再帰的に解決
 * - レコード型 {x: A, y: B} は Eq<A>, Eq<B> を再帰的に解決
 * - 配列型 [T] は Eq<T> を再帰的に解決
 * - Option<T>, Result<T, E> は Eq<T>, Eq<E> を再帰的に解決
 *
 * @param ty 対象の型
 * @return 成功時は辞書参照、失敗時は None
 *)
val solve_eq : ty -> dict_ref option

(** Ord トレイトの解決
 *
 * 仕様書 1-2 §B.1: 順序付けのトレイト
 *
 * 解決規則:
 * - Eq<T> を前提とする（スーパートレイト制約）
 * - プリミティブ型（整数・浮動小数・Bool・Char・String）は自動実装
 * - タプル型は辞書順比較（左から順に比較）
 * - 浮動小数型は IEEE 754 の全順序比較（NaN は最大値として扱う）
 *
 * @param ty 対象の型
 * @return 成功時は辞書参照、失敗時は None
 *)
val solve_ord : ty -> dict_ref option

(** Collector トレイトの解決
 *
 * 仕様書 3-1 §2.2: コレクション型の反復処理サポート
 *
 * 解決規則:
 * - [T] (スライス)、[T; N] (固定長配列) は自動実装
 * - Option<T>, Result<T, E> は要素型を返すイテレータとして実装
 * - タプル型は各要素を順に返すイテレータとして実装
 *
 * @param ty 対象の型
 * @return 成功時は辞書参照、失敗時は None
 *)
val solve_collector : ty -> dict_ref option

(* ========== 制約グラフの構築と解析 ========== *)

(** 制約グラフの構築
 *
 * トレイト制約のリストから依存関係グラフを構築
 * スーパートレイト関係（例: Ord requires Eq）を自動で追加
 *
 * Phase 2 Week 18-19:
 * - Eq → Ord の依存関係を認識
 * - 再帰的な制約（例: Eq<(A, B)> requires Eq<A>, Eq<B>）を展開
 *
 * @param constraints 制約リスト
 * @return 構築された制約グラフ
 *)
val build_constraint_graph : trait_constraint list -> constraint_graph

(** 循環依存の検出
 *
 * 制約グラフ内の循環を検出し、循環を構成する制約リストを返す
 * 複数の循環が存在する場合はすべて検出
 *
 * @param graph 制約グラフ
 * @return 検出された循環のリスト（各循環は制約のリスト）
 *)
val find_cycles : constraint_graph -> trait_constraint list list

(** トポロジカルソート
 *
 * 制約グラフをトポロジカルソートし、解決順序を決定
 * 循環が存在する場合は None を返す
 *
 * @param graph 制約グラフ
 * @return 成功時はソート済み制約リスト、循環がある場合は None
 *)
val topological_sort : constraint_graph -> trait_constraint list option

(* ========== ヘルパー関数 ========== *)

(** トレイト制約の等価性判定
 *
 * トレイト名と型引数が一致するか判定
 * 型引数は構造的等価性で比較
 *
 * @param c1 制約1
 * @param c2 制約2
 * @return 等価なら true
 *)
val trait_constraint_equal : trait_constraint -> trait_constraint -> bool

(** constraint_equal is an alias for trait_constraint_equal *)
val constraint_equal : trait_constraint -> trait_constraint -> bool

(** 型がプリミティブ型か判定
 *
 * Eq/Ord が自動実装されるプリミティブ型を判定
 *
 * @param ty 対象の型
 * @return プリミティブ型なら true
 *)
val is_primitive : ty -> bool

(* ========== デバッグ用 ========== *)

(** 辞書参照の文字列表現 *)
val string_of_dict_ref : dict_ref -> string

(** 制約エラーの文字列表現 *)
val string_of_constraint_error : constraint_error -> string

(** 制約グラフの文字列表現 *)
val string_of_constraint_graph : constraint_graph -> string

(** 解決状態の文字列表現 *)
val string_of_solver_state : solver_state -> string
