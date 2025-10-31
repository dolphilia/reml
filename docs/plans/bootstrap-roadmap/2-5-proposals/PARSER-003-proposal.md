# PARSER-003 コアコンビネーター抽出計画

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
- **API レビュー**: `docs/notes/core-parse-api-evolution.md`（新設予定）にコンビネーター抽出時の公開 API をモジュール署名付きで記録し、Phase 3 の self-host 設計レビューに備える。

## 4. フォローアップ
- `PARSER-001` シム実装と連動し、`Reply` / `ParseResult` がコンビネーター層を経由するよう統合。  
- Phase 2-7 `execution-config` タスクで `RunConfig.extensions["lex"]`・`["recover"]` をコンビネーターから参照できるよう、設定伝播の設計を加える。  
- `docs/guides/plugin-authoring.md` に、OCaml 実装から提供されるコンビネーター API の利用例を追記する。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にコンビネーター抽出後の残課題（テレメトリ、エラーメッセージ統合）を記録し、実装移行を段階化する。
- **タイミング**: PARSER-001/002 と Lex API 抽出が揃った Phase 2-5 後半に着手し、Phase 2-6 へ入る前までにコアコンビネーター層の PoC を完成させる。

## 5. 実施ステップ
1. **Menhir 資産の棚卸しと仕様マッピング（Week31 Day1-2）**  
   - **調査**: `compiler/ocaml/src/parser.mly` と `parser_expectation.{ml,mli}` の規則・診断経路を洗い出し、`docs/spec/2-2-core-combinator.md` で定義された 15 個のコアコンビネーターと対応付けるマトリクスを作成する[^spec-core-comb]。  
   - **記録**: マッピング結果と既存の LR 規則で欠落しているメタデータ（`rule` 名称、`ParserId`、`recover` 同期点）を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に Day エントリとして追記し、後続ステップの前提情報を共有する[^review-log].  
   - **成果物**: `docs/notes/core-parser-migration.md` に「Menhir → Core_parse 対応表」を追加し、Phase 3 の self-host 作業でも参照できる状態にする。

2. **`Core_parse` シグネチャと ID 付与戦略の設計（Week31 Day3-4）**  
   - **調査**: `docs/spec/2-2-core-combinator.md` §A〜§C と `docs/spec/2-6-execution-strategy.md` の Packrat/ストリーミング契約、`docs/guides/core-parse-streaming.md` の API 期待値を確認し、`ParserId`・`Reply`・`recover` のメタデータ要件を整理する[^spec-exec][^guide-stream].  
   - **設計**: `compiler/ocaml/src/core_parse_combinator.mli`（新設）に公開する最小シグネチャ案を作成し、`rule`・`label`・`cut` など committed/consumed フラグの扱いを `Reply` 型に写像する。`PARSER-002` で導入した `Run_config` と `extensions` のフック点を洗い直し、コンビネーター側から `RunConfig.extensions["lex"]`/`["recover"]` を参照するフックを定義する。  
   - **承認**: モジュール署名案を `docs/notes/core-parse-api-evolution.md`（新設予定）に掲載し、Phase 2-5 レビューで承認を得る。

3. **Menhir ブリッジ層 PoC の実装（Week32 Day1-4）**  
    - **調査**: `parser_driver.ml`・`parser_diag_state.ml`・`parser_expectation.ml` における AST 生成と診断の流れを確認し、`Core_parse` の各コンビネーターへ委譲する際の移行ポイントを特定する。  
    - **実装**: `Core_parse` モジュールを作成し、Menhir 生成関数を `rule`/`label`/`cut` などでラップするシムを追加。`Run_config`（`parser_run_config.ml`）から Packrat/左再帰/trace フラグを受け取り、ブリッジ層で `Parser_context` に注入する。  
    - **検証**: 既存の `dune runtest parser` と CLI/LSP 経路を実行し、コンビネーター層経由でも AST と診断が一致することを確認。差分は `docs/plans/bootstrap-roadmap/2-5-review-log.md` に PoC 結果として記録する。

4. **Packrat・回復・マルチ Capability の統合（Week32 Day5-Week33 Day2）**  
    - **調査**: `docs/spec/2-6-execution-strategy.md` と `docs/spec/2-5-error.md` に記載された Packrat メモ化と回復戦略、`PARSER-002` の RunConfig 拡張手順を参照し、`rule` による `ParserId` 固定化と `recover` の同期トークン設計を固める。  
    - **実装**: `Core_parse` 内に Packrat キャッシュ管理フックを追加し、`parser.capability.packrat`（RunConfig extensions）有効時に `ParserId` ごとのメモ化を行う。`recover` は `parser_expectation` の期待集合と診断拡張を統合し、`RunConfig.extensions["recover"]` を通じて同期トークンの設定を受け取る。複数 Capability 監査のため、`effect-stage` 情報を `Parser_context` へ引き渡す。  
    - **検証**: Packrat/回復を有効化したテストケースを追加し、`tooling/ci/collect-iterator-audit-metrics.py` の `parser.packrat_cache_hit_ratio`（追加予定）や `parser.recover_sync_success_rate` が想定値になるか確認する。

5. **テスト・メトリクス・ゴールデン整備（Week33 Day3-5）**  
    - **実装**: `compiler/ocaml/tests/packrat_tests.ml`（新設）と既存の CLI/LSP ゴールデンを更新し、コンビネーター経由のパース結果・診断が Menhir 直呼びと一致することを検証する。`scripts/validate-diagnostic-json.sh` に `Core_parse` 由来の `rule`/`ParserId` 付与チェックを追加。  
    - **計測**: `0-3-audit-and-metrics.md` に `parser.core_comb_rule_coverage` や `parser.packrat_cache_hit_ratio` などの指標を登録し、CI で追跡する。必要に応じて `tooling/ci/collect-iterator-audit-metrics.py --require-success` の閾値を更新する。  
    - **記録**: テスト結果とメトリクス導入状況を `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記し、残課題を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ移送する。

6. **ドキュメントとロードマップの同期（Week33 Day5）**  
    - **更新**: `docs/spec/2-2-core-combinator.md` に OCaml 実装の進捗脚注を追加し、`docs/guides/plugin-authoring.md` / `docs/guides/core-parse-streaming.md` にコンビネーター利用例と RunConfig 連携の手順を追記する。  
    - **索引整備**: `README.md`・`docs/plans/bootstrap-roadmap/2-5-proposals/README.md` を更新して新モジュール導線を掲載し、`docs/notes/core-parse-api-evolution.md` に API 変更履歴を記録する。  
    - **引き継ぎ**: Phase 2-7 以降へ渡す TODO（テレメトリ統合、Menhir 置換判断）を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に整理し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` の最終日にまとめる。

## 残課題
- Menhir 生成コードを全面置換するのか、移行期間中はシム層で段階導入するのかの方針決定が必要。  
- `rule` / `ParserId` 割り当てを静的に行うか、実行時にハッシュで生成するかについてパフォーマンス評価が求められる。

[^spec-core-comb]: `docs/spec/2-2-core-combinator.md` §A〜§H。コアコンビネーター 15 種と Capability 連携要件を定義。
[^review-log]: `docs/plans/bootstrap-roadmap/2-5-review-log.md`。Day エントリに作業ログ・検証結果を追記する運用。
[^spec-exec]: `docs/spec/2-6-execution-strategy.md`。Packrat メモ化、ストリーミング実行時の契約、`reply.committed` の規則を規定。
[^guide-stream]: `docs/guides/core-parse-streaming.md`。RunConfig 連携とストリーミング利用時の `Core.Parse` API 期待値を整理。
