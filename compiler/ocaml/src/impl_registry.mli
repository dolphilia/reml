(* Impl_registry — Trait Implementation Registry for Reml Type System
 *
 * このモジュールは impl 宣言から得られるトレイト実装情報を管理する。
 * 型推論時に impl 宣言を登録し、制約解決時に参照できるようにする。
 *
 * Phase 2 Week 23-24: 制約ソルバーへの impl 宣言登録
 *
 * 設計方針:
 * - impl 宣言を解析してトレイト実装情報として保存
 * - トレイト名・実装対象型による検索をサポート
 * - ジェネリック型パラメータと where 句制約を保持
 * - 辞書生成システムとの連携のため、メソッド実装情報を保持
 *)

open Types
open Ast

type impl_info = {
  trait_name : string;
      (** 実装するトレイト名
       * 例: "Eq", "Ord", "Collector"
       * inherent impl の場合は "(inherent)"
       *)
  impl_type : ty;
      (** 実装対象型
       * 例: i64, String, Vec<T>
       * ジェネリック型変数を含む場合がある
       *)
  generic_params : type_var list;
      (** ジェネリック型パラメータ
       * impl<T, U> の T, U など
       *)
  where_constraints : trait_constraint list;
      (** where 句の制約
       * 例: where T: Eq, U: Ord
       *)
  methods : (string * string) list;
      (** メソッド実装のリスト
       * (メソッド名, 実装関数名) のペア
       * 例: [("eq", "Vec_Eq_eq"), ("ne", "Vec_Eq_ne")]
       *
       * Phase 2 Week 23-24: 簡易実装では関数名の文字列のみ保持
       * Phase 2 後半: 型付き式 (typed_expr) への拡張を検討
       *)
  span : span;  (** impl 宣言の位置情報 *)
}
(** impl 実装情報
 *
 * 一つの impl 宣言から抽出される情報を表す。
 *
 * 例:
 *   impl<T> Eq for Vec<T> where T: Eq {
 *     fn eq(self: Vec<T>, other: Vec<T>) -> Bool = ...
 *   }
 *
 * この場合:
 *   trait_name = "Eq"
 *   impl_type = Vec<T>  (型変数 T を含む)
 *   generic_params = [T]
 *   where_constraints = [Eq<T>]
 *   methods = [("eq", <typed_expr>)]
 *)

type impl_registry
(** impl 宣言レジストリ
 *
 * すべての impl 宣言を管理するレジストリ。
 * 型推論の過程で impl 宣言を登録し、制約解決時に検索する。
 *)

val empty : unit -> impl_registry
(** 空のレジストリを作成 *)

val register : impl_info -> impl_registry -> impl_registry
(** impl 実装情報をレジストリに登録
 *
 * 同一のトレイト・型への重複した impl は許容するが、
 * 制約解決時に曖昧性エラー (AmbiguousImpl) として検出される。
 *)

val lookup : string -> ty -> impl_registry -> impl_info option
(** 指定されたトレイト名と型に一致する impl を検索
 *
 * lookup trait_name impl_type registry
 *
 * 型の照合では、ジェネリック型パラメータを考慮した部分的な一致を行う。
 * 例:
 *   - 検索: Eq, i64 → 登録: Eq, i64 → マッチ
 *   - 検索: Eq, Vec<i64> → 登録: Eq, Vec<T> → マッチ（T を i64 に束縛）
 *
 * 返り値:
 *   Some impl_info: 一致する impl が見つかった場合
 *   None: 一致する impl がない場合
 *
 * 注意: 複数の impl が一致する場合は最初に見つかったものを返す。
 *      曖昧性の検出は find_matching_impls を使用すること。
 *)

val find_matching_impls : trait_constraint -> impl_registry -> impl_info list
(** トレイト制約に一致するすべての impl を検索
 *
 * find_matching_impls constraint registry
 *
 * 制約解決に使用する。制約にマッチする impl をすべて列挙する。
 *
 * 例:
 *   constraint = Eq<Vec<i64>>
 *   registry には以下が登録されている:
 *     - impl Eq for Vec<T> where T: Eq  → マッチ（where 句も満たす必要あり）
 *     - impl Eq for i64                  → マッチしない
 *
 * 返り値:
 *   空リスト: 一致する impl がない → NoImpl エラー
 *   1要素: 一意に impl が決定 → 成功
 *   2要素以上: 曖昧な impl → AmbiguousImpl エラー
 *)

val all_impls : impl_registry -> impl_info list
(** レジストリに登録されているすべての impl を取得（デバッグ用） *)

val string_of_registry : impl_registry -> string
(** レジストリの文字列表現（デバッグ用）
 *
 * 出力例:
 *   Impl Registry:
 *     impl Eq for i64
 *     impl<T> Eq for Vec<T> where T: Eq
 *     impl Ord for String
 *)

val string_of_impl_info : impl_info -> string
(** impl_info の文字列表現（デバッグ用） *)
