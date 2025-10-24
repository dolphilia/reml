# PARSER-003 コアコンビネーター抽出提案

## 1. 背景と症状
- 仕様では 15 個のコアコンビネーター（`rule` / `label` / `cut` / `recover` など）を標準 API として提供し、DSL・プラグインが共有することを想定している（docs/spec/2-2-core-combinator.md:9-88）。  
- 現行 OCaml 実装は `parser.mly` に LR 規則を直書きしており、`Core.Parse` モジュールやコンビネーター層が存在しない（compiler/ocaml/src/parser.mly:1）。  
- Phase 3 の self-host 計画で Reml 実装へ移行する際、コンビネーター API を経由したサンプルや DSL の写像が不可能で、`docs/guides/core-parse-streaming.md` のストリーミング設計とも齟齬が生じている。

## 2. Before / After
### Before
- Menhir 生成コードに直接アクセスし、コンビネーター ID や `rule(name, p)` 相当のメタデータを保持しない。  
- Packrat/左再帰/`recover` の仕様上の契約を確認する手段がなく、`Core.Parse` を前提としたガイド類（2-6/2-7）と断絶している。

### After
- OCaml 実装に `Core_parse` モジュール（仮称）を追加し、仕様で定義されたコンビネーターの最小セットを提供する。  
- `parser.mly` から生成される低レベル規則をラップし、`rule`/`label`/`cut` といったメタ情報を保持。`ParserId` を割り当て、Packrat/ストリーミングとの連携が可能になる。  
- DSL やプラグインが OCaml 実装のコンビネーターを利用できるよう、`compiler/ocaml/src/core_parse_combinator.ml`（新設）に API を公開し、Phase 3 以降も互換性を維持する。

#### API スケッチ
```ocaml
module Core_parse : sig
  type 'a parser
  val rule : string -> 'a parser -> 'a parser
  val label : string -> 'a parser -> 'a parser
  val cut : 'a parser -> 'a parser
  val recover : 'a parser -> until:'b parser -> with_:'a -> 'a parser
  (* ... *)
end
```

## 3. 影響範囲と検証
- **回帰テスト**: 既存の `parser` 単体テストに加えて、コンビネーター経由で同等の構文木が生成されるかを検証するゴールデンを追加。  
- **Packrat/左再帰**: 2-6 の契約に基づき、`rule` と `ParserId` を利用したメモ化が機能するかを `compiler/ocaml/tests/packrat_tests.ml`（新設）で確認。  
- **ドキュメント**: `docs/spec/2-2-core-combinator.md` へ OCaml 実装の進捗脚注を追加し、フェーズ移行時に差分を追跡する。

## 4. フォローアップ
- `PARSER-001` シム実装と連動し、`Reply` / `ParseResult` がコンビネーター層を経由するよう統合。  
- Phase 2-7 `execution-config` タスクで `RunConfig.extensions["lex"]`・`["recover"]` をコンビネーターから参照できるよう、設定伝播の設計を加える。  
- `docs/guides/plugin-authoring.md` に、OCaml 実装から提供されるコンビネーター API の利用例を追記する。

## 確認事項
- Menhir 生成コードを全面置換するのか、移行期間中はシム層で段階導入するのかの方針決定が必要。  
- `rule` / `ParserId` 割り当てを静的に行うか、実行時にハッシュで生成するかについてパフォーマンス評価が求められる。
