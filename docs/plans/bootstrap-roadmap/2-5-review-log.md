# 2-5 レビュー記録

Phase 2-5 で実施した差分レビューと現状棚卸しを記録し、後続フェーズでの追跡に利用する。  
エントリごとに関連計画へのリンクと再現手順を整理する。

## PARSER-003 Step1 コアコンビネーター棚卸し（2025-11-01）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md`](./2-5-proposals/PARSER-003-proposal.md#5-実施ステップ)

### 1. 対応表の作成
- `docs/notes/core-parser-migration.md` に Menhir 規則と 15 個のコアコンビネーターの対応を追加し、代表的な規則と不足メタデータを整理した（`Phase 2-5 Core コンビネーター棚卸し` セクション参照）。  
- `parser.mly` の空産出・多分岐・アクションを抽出し、`ok`/`choice`/`map` 等の近似点を記録。  
- `parser_expectation.ml` の期待集合生成経路を確認し、`label` が未導入であることを確認。

### 2. 欠落メタデータの確認
- `committed` フラグがどこでも `true` に更新されず、`cut`/`attempt` の契約を満たせない（`compiler/ocaml/src/parser_driver.ml:185-223`）。  
- `ParserId` を生成・維持する仕組みが無く、`rule` の要求（Packrat キー／トレース／監査）を満たせない（`compiler/ocaml/src/parser.mly:1174`、`compiler/ocaml/src/parser_driver.ml:219-223`）。  
- `recover` 用の設定とハンドラ `Parser_diag_state.record_recovery` が未使用で、RunConfig `recover` 拡張から同期トークンを渡す経路が欠落（`compiler/ocaml/src/parser_diag_state.ml:24-63`、`compiler/ocaml/src/parser_driver.ml:187-205`）。

### 3. フォローアップ
1. Step2 で `Core_parse` シグネチャ草案を作成し、`rule`/`label`/`cut` のメタデータ付与方針を決定する。  
2. `parser_driver` に `committed`/`consumed` を操作するフックを追加する設計を検討し、`cut`/`attempt` 実装時に差分が追跡できるようログを残す。  
3. `RunConfig.extensions["recover"]` の同期トークン定義を `PARSER-002` チームと共有し、Packrat/回復シムを同一タイムラインで導入する。

## PARSER-003 Step2 Core_parse シグネチャ設計（2025-12-04）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md`](./2-5-proposals/PARSER-003-proposal.md#step2-実施記録2025-12-04)

### 1. 公開シグネチャ案
- `docs/notes/core-parse-api-evolution.md` に `Core_parse` モジュール署名案を作成し、`Id`/`State`/`Reply`/`Registry` を分離した構造を提示。`Parser<T> = State.t -> Reply.'a t * State.t` の形で純関数とし、`rule`/`label`/`cut`/`recover` が `Reply` の `consumed`/`committed` フラグに反映されることを明記。  
- `recover` は `id` 引数と `until`/`with_` コールバックを受け取る高階関数に統一し、同期トークンと補完値の導線を確保した。

### 2. ParserId 採番戦略
- 静的 ID 表（`core_parse_id_registry.ml`）を生成する計画を立て、`namespace:name` の `Digestif.xxhash64` を `fingerprint` に保存する方式を採用。`ordinal` は `0-4095` を予約し、Menhir 対応表と照合するスクリプトの要件を記載。  
- 動的 ID は `ordinal >= 0x1000` を採番し `origin = \`Dynamic` を記録。Packrat キーと監査ログで静的/動的を識別できるよう `Id.origin` アクセサを追加する設計とした[^core-parse-api-note].

### 3. RunConfig / 診断連携
- `State` から `Parser_run_config`, `Parser_diag_state`, `Parser.MenhirInterpreter.checkpoint` を取得できるアクセサを定義し、`PARSER-002` で導入された `RunConfig.extensions["lex"|"recover"|"stream"]` をコンビネーター層で利用可能にした。  
- `cut`/`cut_here` は `State` 内のコミットフラグを更新し、`Reply.Err` 側で `committed=true` を `Parser_diag_state.record_committed` へ引き渡す方針を確定。`recover` は `RunConfig.Recover` シムと `Parser_diag_state.record_recovery` を接続することを次ステップのタスクとした。

### 4. フォローアップ
1. Step3 で `parser_driver` の Menhir 呼び出しを `Core_parse.rule` でラップし、静的 ID 表と Step1 対応表の差分を洗い出す。  
2. `core_parse_id_registry.ml` を生成するスクリプトと CI チェック（重複検知）を実装。  
3. Packrat PoC が `Id.fingerprint` をメモキーとして利用できるか、`RunConfig.packrat` を有効化したテストケースで検証する。

[^core-parse-api-note]: `docs/notes/core-parse-api-evolution.md` Phase 2-5 Step2 Core_parse シグネチャ草案。`Id` 採番と `State`/`Reply` の公開 API を記載。

## PARSER-003 Step3 Menhir ブリッジ層 PoC（2025-12-05）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md`](./2-5-proposals/PARSER-003-proposal.md#step3-実施記録2025-12-05)

### 1. Core_parse PoC の実装
- `compiler/ocaml/src/core_parse.{ml,mli}` を追加し、`Id`/`State`/`Reply` と `rule`/`label`/`cut`/`attempt` の最小セットを提供。`menhir:compilation_unit` を静的 ID として登録し、それ以外は `ordinal >= 0x1000` を動的採番する仕組みを導入した（`core_parse.ml:28-87`）。`fingerprint` は暫定的に `Hashtbl.hash` ベースの 64bit 値を利用し、Step4 で `Digestif` へ移行する TODO を残置。  
- `State` にトークン消費・コミット状態を保持させ、`Reply` が `consumed`/`committed` を返す PoC を整備。`attempt` は `committed=false` の場合に状態を巻き戻す仮実装とし、Packrat 未導入の現状でも差分検証が行えるようにした（`core_parse.ml:91-152`）。

### 2. parser_driver のブリッジ切り替え
- `parser_driver.run` の Menhir ループを `Core_parse.rule ~namespace:"menhir" ~name:"compilation_unit"` 経由で実行するよう更新し、`Core_parse.Reply` から既存の `ParseResult` 構造体へ変換する経路を追加した（`parser_driver.ml:174-245`）。  
- トークン取得時に `Core_state.mark_consumed` を呼び出すよう変更し、`require_eof` 判定および診断登録で `result.consumed`/`result.committed` を参照する形へ移行。従来の `consumed`/`committed` 参照を削除し、状態を `Core_parse` 層に集約した（`parser_driver.ml:160-171`, `parser_driver.ml:245-255`）。  
- 成功時のスパン追跡（`Parser_diag_state.record_span_trace`）とエラー時の診断登録は従来ロジックを維持し、PoC の導入で既存メトリクスが変化しないことを確認。`require_eof` の追加診断でも新しい状態フラグが利用できることを確認した。

### 3. 既知の制限とフォローアップ
- `label`/`cut` は現時点でプレースホルダ実装（コミットフラグ更新のみ）であり、Packrat 導入時に `Parser_diag_state` との連携を強化する必要がある。  
- `fingerprint` および静的 ID 一覧は PoC 仕様であり、Step4 で `core_parse_id_registry` の自動生成と `Digestif.xxhash64` 化を行う。  
- `Core_parse.rule` が付与した ID を監査ログや `Parser_diag_state` へ転記する処理は未実装。Packrat/監査統合時に記録先を決定し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ TODO を追加する。

## PARSER-003 Step4 Packrat・回復・Capability 統合準備（2025-12-12）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md`](./2-5-proposals/PARSER-003-proposal.md#step4-実施記録2025-12-12)

### 1. Packrat キャッシュ設計と RunConfig 連携
- `compiler/ocaml/src/core_parse.ml:5` 時点で Packrat キャッシュ機構が未実装であることを確認し、`Cache_key = (Id.fingerprint, byte_offset)` を前提に `Core_parse.State` へキャッシュコンテキストを保持する案を `docs/notes/core-parse-api-evolution.md` へ追加。  
- `parser_run_config.ml:25` の `packrat` フラグと `Extensions` 名称空間を参照し、`RunConfig.packrat=true` のときにのみキャッシュを活性化する切替条件を整理。`Extensions.with_namespace` で `lex`/`recover` と同等に扱えることを確認し、キャッシュ有効時の監査情報に `ParserId` を必ず保持する TODO を登録した。

### 2. 回復同期トークンと診断連携
- `parser_run_config.ml:240` の `Recover` モジュールと `parser_diag_state.ml:8` の `record_recovery` を突合し、`Recover_config.sync_tokens` を `Core_parse.recover` へ伝搬するフローを `docs/notes/core-parser-migration.md` に追記。  
- `parser_expectation.ml:1` の期待集合生成が `recover` 経由でも再利用できることを確認し、同期トークン適用時に `Diagnostic.expected` を更新できるかを CLI/LSP ゴールデンで再点検する必要がある旨を記録。`recover.notes` フラグの扱いは Phase 2-7 での CLI メッセージ統合タスクへ連携。

### 3. 複数 Capability 監査との整合
- `parser_run_config.ml:320` の `Effects` 構造と `compiler/ocaml/src/diagnostic.ml:846` の `effect.capability` 書き込み処理を比較し、Packrat 経路でも Stage/Capability が欠落しないよう `Core_parse` 層で保持するメタデータ項目を洗い出した。  
- キャッシュヒット時に `effect.stage.*` を再評価する要否を検証するため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ「Packrat 本導入後の Stage 監査テスト追加」TODO を新規登録。

### 4. CI 指標とフォローアップ
- 現行の `tooling/ci/collect-iterator-audit-metrics.py:1` に Packrat/回復系指標が存在しないため、`parser.packrat_cache_hit_ratio` / `parser.recover_sync_success_rate` を追加する設計メモを作成。実装前に `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ KPI を仮登録した。  
- Packrat キャッシュ導入が性能・メモリへ与える影響を測定するため、Phase 2-6 でベンチマークを追加するタスクを検討し、`reports/diagnostic-format-regression.md` のチェックリストへ「Packrat キャッシュ統計確認」を追記する案を共有した。

## PARSER-003 Step5 テスト・メトリクス整備（2025-12-18）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md`](./2-5-proposals/PARSER-003-proposal.md#step5-実施記録2025-12-18)

### 1. Core Parse メタデータ定着
- `compiler/ocaml/src/parser_driver.ml` に `annotate_core_rule_metadata` を追加し、`Core_parse.rule` が付与する `ParserId` を `extensions.parse.parser_id` と `parser.core.rule.*` 監査キーへ反映。`Core_parse.State` へ `record_packrat_access` を導入し、Packrat クエリ／ヒットを集計して `ParseResult.packrat_stats` に反映する仕組みを整備した。  
- `compiler/ocaml/tests/packrat_tests.ml` を拡張してメタデータ定着と Packrat 統計を検証し、`compiler/ocaml/tests/golden/diagnostics/parser/expected-summary.json.golden` および `test_cli_diagnostics.ml` を `parser.core.rule.*` 付きで更新した。

### 2. 検証スクリプトと CI 指標
- `scripts/validate-diagnostic-json.sh` に Core Parse 用バリデーションを追加し、`extensions.parse.parser_id` と `audit_metadata`／`audit.metadata` 配下の `parser.core.rule.*` を必須項目として検証。  
- `tooling/ci/collect-iterator-audit-metrics.py` に `collect_core_parser_metrics` を実装し、`parser.core_comb_rule_coverage`（メタデータ整合率）と `parser.packrat_cache_hit_ratio`（Packrat ヒット率）を収集するよう拡張。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に両 KPI を追記して CI 監視へ組み込んだ。

### 3. フォローアップ
- Packrat 有効時は `queries=8 / hits=7 (0.875)` を確認できたが、キャッシュキーは `state_number + offset` のみである。Phase 2-7 で静的 ID レジストリとの統合や左再帰ケースでの精度検証を進める。  
- Fingerprint 算出は引き続き `Hashtbl.hash` ベースの PoC であるため、Step6 以降で `Digestif.xxhash64` への移行と静的 ID 自動生成を検討する。

## PARSER-003 Step6 ドキュメント同期と引き継ぎ（2025-12-24）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md`](./2-5-proposals/PARSER-003-proposal.md#step6-実施記録2025-12-24)

### 1. 仕様・ガイド更新
- `docs/spec/2-2-core-combinator.md` に `Core_parse` 進捗脚注を追加し、OCaml 実装の `rule`/`label`/`cut` と Packrat 指標が仕様側で追跡できるようにした。  
- `docs/guides/plugin-authoring.md` と `docs/guides/core-parse-streaming.md` にコンビネーター利用例と RunConfig 共有手順を追記し、CLI/LSP/ストリーミングが同じ `parser.core.rule.*` メタデータを生成する運用ガイドを整備した。  
- `docs/notes/core-parse-api-evolution.md` へ Step6 セクションを追加し、ドキュメント同期と Phase 2-7 への引き継ぎ事項を記録。

### 2. 索引・計画書の同期
- リポジトリ索引 `README.md` に `Core Parse` モジュール導線を追加し、`docs/plans/bootstrap-roadmap/2-5-proposals/README.md` の PARSER-003 項目へ Step6 完了メモを追記。  
- 本レビュー記録に 2025-12-24 のエントリを追加して更新対象・参照リンクを整理し、仕様差分タスクの完了点を共有。

### 3. フォローアップ
- テレメトリ統合と Menhir 置換判断を Phase 2-7 で検討するため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO を登録。  
- Packrat 指標と `parser.core.rule.*` を活用した監査強化を Phase 2-7 の CI 改修タスクへ連携し、`collect-iterator-audit-metrics.py` の拡張計画を保留項目として追跡する。

## TYPE-001 Day4 値制限テスト・診断整備（2025-11-05）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md`](./2-5-proposals/TYPE-001-proposal.md)

### 1. テストとフィクスチャの雛形
- `compiler/ocaml/tests/test_type_inference.ml` 末尾に Step3 TODO コメントを挿入し、Strict/Legacy 切替と `Value_form` 判定ヘルパを前提とした 3 ケース（let+純粋ラムダ / var+純粋ラムダ / let+unsafe）の実装メモを追加。  
- `compiler/ocaml/tests/golden/type_inference_value_restriction.strict.json.golden` と `...legacy.json.golden` をテンプレートとして作成し、`mode` / `status` / `evidence[]` / `diagnostic.code` のフィールド構成を固定。

### 2. メトリクスと CI 検証
- `tooling/ci/collect-iterator-audit-metrics.py` へ `type_inference.value_restriction_violation`（Strict モード違反の有無をブロッカーとして監視）と `type_inference.value_restriction_legacy_usage`（Legacy 経路の使用回数を記録）の追加方針を確定し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に指標を登録。  
- `scripts/validate-diagnostic-json.sh` へ値制限違反診断の必須キー検証（`extensions.value_restriction.*` / `audit_metadata.value_restriction.*` / `audit.metadata.value_restriction.*`）を追加する変更案をレビューし、欠落時のエラー出力方針を決定。

### 3. 共有事項とフォローアップ
- `reports/diagnostic-format-regression.md` のチェックリストへ「値制限ダンプ確認」と `collect-iterator-audit-metrics.py --require-success` の確認項目を追加。  
- Step4 で仕様脚注の追補と Legacy モード縮退ロードマップを整備し、詳細は「TYPE-001 Step4 値制限ドキュメント整備（2025-11-08）」を参照。

## TYPE-001 Step4 値制限ドキュメント整備（2025-11-08）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md`](./2-5-proposals/TYPE-001-proposal.md#step4-実施記録2025-11-08)

### 1. 仕様・計画ドキュメントの更新
- `docs/spec/1-2-types-Inference.md` §C.3 に `Value_restriction.evaluate` と `RunConfig.extensions["effects"].value_restriction_mode` の連携を解説する実装メモを追加し、Strict/Legacy モードの既定値と利用条件を明示した。【S:docs/spec/1-2-types-Inference.md†L142-L166】
- `docs/spec/1-3-effects-safety.md` §B へ値制限判定と効果タグ（`mut`/`io`/`ffi`/`unsafe`/`panic`）・Capability/Stage 監査を結び付ける記述を追加し、`effects.contract.value_restriction` 診断と監査ログのキー整合を定義した。【S:docs/spec/1-3-effects-safety.md†L70-L104】
- `docs/spec/2-1-parser-type.md` §D と `docs/spec/2-6-execution-strategy.md` §B-2 に `extensions["effects"]` ネームスペースの正式化と CLI トグル（`--value-restriction={strict|legacy}`／`--legacy-value-restriction`）の運用方針を追記し、Parser→Typer 間で値制限モードを同期する要件を明文化した。【S:docs/spec/2-1-parser-type.md†L118-L170】【S:docs/spec/2-6-execution-strategy.md†L38-L134】
- `docs/plans/bootstrap-roadmap/2-5-proposals/README.md` と `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` を更新し、TYPE-001 の Step4 完了と仕様脚注の反映状況・残課題の移管先を記録した。【C:docs/plans/bootstrap-roadmap/2-5-proposals/README.md†L34-L54】【D:docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md†L94-L123】

### 2. ノート・フォローアップ整理
- `docs/notes/type-inference-roadmap.md` に Step4 メモを追加し、Stage/Capability 要件・RunConfig CLI スイッチ・Phase 2-7 への引き継ぎ内容（execution-config / effect-metrics）を明示した。【N:docs/notes/type-inference-roadmap.md†L33-L74】
- Phase 2-7 `execution-config` チームへ RunConfig CLI 統合テスト（Strict/Legacy 切替と Legacy 廃止アラート）を依頼し、`effect-metrics` チームへ `type_inference.value_restriction_violation` / `type_inference.value_restriction_legacy_usage` 監視の恒常運用を引き渡す TODO を登録した。

### 3. TODO / 移管
1. `execution-config`: `--value-restriction` 系スイッチの CLI ゴールデンを追加し、Strict/Legacy の互換モード通知を Phase 2-7 スプリントで実装する。  
2. `effect-metrics`: CI 指標に Legacy 経路検出の警告を追加し、逸脱時に `collect-iterator-audit-metrics.py --require-success` が即時失敗するようルールを更新する。  
3. Phase 3 で Reml 実装へ移植する際、`RunConfig` の `effects.value_restriction_mode` と診断拡張を同梱できるよう移植手順を `docs/notes/core-parser-migration.md`（予定）に追記する。

## PARSER-002 Day1 RunConfig 現状調査（2025-11-18）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 調査サマリ
- `docs/spec/2-1-parser-type.md:92-175` と `docs/spec/2-6-execution-strategy.md:60-107` を精査し、RunConfig の公式フィールドと既定値、`extensions` ネームスペースの契約を整理。結果は計画書 Step0 の表1・表2へ反映。  
- 現行 OCaml 実装では `type run_config = { require_eof; legacy_result }` のみ存在し、仕様で定義される Packrat/左再帰/trace/merge_warnings/locale/extensions が全て欠落していることを確認（compiler/ocaml/src/parser_driver.ml:6-13）。  
- CLI（compiler/ocaml/src/main.ml:612）およびユニットテスト（例: compiler/ocaml/tests/test_parser.ml:10, compiler/ocaml/tests/test_type_inference.ml:18）は `Parser_driver.parse` / `parse_string` を直接使用し、RunConfig 構築ヘルパが存在しない。  
- `run_partial` は `require_eof=false` を強制するだけで `rest` を返さないスタブ状態であり、ストリーミング拡張と整合しない（compiler/ocaml/src/parser_driver.ml:172-175）。

### 2. 仕様との差分要約
- 既定値の差異: 仕様は `require_eof=false` が既定だが OCaml 実装は `default_run_config.require_eof = true` のまま（compiler/ocaml/src/parser_driver.ml:11）。  
- `trace`・`merge_warnings`・`locale` の制御は `Parser_diag_state` / `Diagnostic.Builder` にスイッチが無く、RunConfig 経由での切替ができない。  
- `extensions["lex"]`・`["config"]`・`["recover"]`・`["stream"]`・`["lsp"]`・`["target"]`・`["effects"]` の標準キーはすべて未実装であり、LEXER-002 / EFFECT-003 / EXEC-001 計画とのインターフェイスが欠落。  
- RunConfig 系メトリクス（`parser.runconfig_switch_coverage` など）は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` にまだ登録されていないため、監視ができない。

### 3. Packrat/左再帰・trace 実装に向けた検討
- Menhir 境界では `Parser.MenhirInterpreter` の `checkpoint` ループに全ての分岐が集中しており（compiler/ocaml/src/parser_driver.ml:133-166）、Packrat 実装時にはここで `(ParserId, byte_off)` をキーとしたメモテーブルを参照する必要がある。  
- Packrat 導入時は `left_recursion` フラグを確認して種成長ループを挿入し、評価中フラグ・`commit_watermark` に基づく掃除を RunConfig 側で初期化する必要がある（docs/spec/2-6-execution-strategy.md:62-74,171-188）。  
- `trace` ON 時にのみ `SpanTrace` や解析イベントを収集する挿し込みポイントは `Lexer.token` 呼び出し前後および `I.Shifting` → `I.resume` の箇所。現状では収集ロジックが無いため無条件でコストゼロ。  
- `merge_warnings=false` を扱うには `Parser_diag_state.record_diagnostic` で回復診断を蓄積する際のフィルタを分岐させ、`extensions["recover"].notes` や監査ログに個別記録できるようにする必要がある。

### 4. TODO / 引き継ぎ
1. （2025-11-18 完了）`parser_run_config.{ml,mli}` を作成し、仕様準拠の `Run_config.t` と `extensions` ラッパーを実装する（PARSER-002 Step1）。  
2. CLI/LSP/テストに共通の RunConfig ビルダーを用意し、既存の `Parser_driver.parse` から新 API へ移行する準備を行う。  
3. Packrat/左再帰シムのメモテーブル要求事項を `PARSER-003` チームへ共有し、`RunConfig.packrat` と `left_recursion` のセマンティクスを整合させる。  
4. RunConfig 測定指標（`parser.runconfig_switch_coverage`、`parser.runconfig_extension_pass_rate`）の追加作業を 0-3 メトリクス管理表へ登録する。

### 5. 実施記録
- 2025-11-18: Step 1 を実施し、`compiler/ocaml/src/parser_run_config.{ml,mli}` に `RunConfig` レコード・拡張マップ API・`Legacy.bridge` を追加。`docs/spec/2-1-parser-type.md` / `docs/spec/2-6-execution-strategy.md` へ OCaml 実装脚注を追記し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §6.3 に進捗脚注を掲載。次工程（Step 2）では `parser_driver` への伝播とメトリクス登録を行う。

## PARSER-002 Day2 RunConfig 適用（2025-11-19）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 作業サマリ
- `parser_driver` の公開 API を `?config:Run_config.t` に切り替え、`trace` / `merge_warnings` / `locale` を `Parser_diag_state.create` へ伝播。  
- `Parser_diag_state` に `trace` 有効時のみ `SpanTrace` を収集する経路と、`merge_warnings` に応じた警告集約ロジックを追加。  
- `RunConfig.packrat` / `left_recursion` が有効化された場合は未実装警告（`parser.runconfig.packrat_unimplemented` / `parser.runconfig.left_recursion_unimplemented`）を発行し、PARSER-003 へフォローアップを引き継げるようにした（Packrat 警告は 2025-12-18 Step5 で解消済み）。  
- `extensions["config"].require_eof` を優先して未消費入力を検出し、`parser.require_eof.unconsumed_input` エラーを生成して `legacy_result` 互換経路にも反映。  
- `trace=true` の場合に `compilation_unit` ルートスパンを `SpanTrace` に記録し、既定モードでは追加コストが発生しないことを確認。

### 2. 影響ファイル
- `compiler/ocaml/src/parser_driver.ml`  
- `compiler/ocaml/src/parser_diag_state.ml`  
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`（Step 2 更新記録）  
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`（本メモ）

### 3. フォローアップ / TODO
- Step 3 で `extensions["lex"]` / `["recover"]` / `["stream"]` シムを実装し、RunConfig の共有拡張を利用できるようにする。  
- Step 4 以降で CLI/LSP/テストの RunConfig ビルダーを導入し、パイプライン全体で新 API を利用する。  
- Step 5 で `trace` / `merge_warnings` の挙動を検証するユニットテストと CI メトリクス（`parser.runconfig_*`）を追加する。（2025-11-22 完了、Day5 ログ参照）

## EXEC-001 Step2 Feeder/Continuation 設計（2026-01-12）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/EXEC-001-proposal.md`](./2-5-proposals/EXEC-001-proposal.md#step-2-feeder--continuation--demandhint-モデル設計2日)

### 1. 型とインターフェイス整理
- `core_parse_streaming_types.{ml,mli}` に `Demand_hint.t`／`Feeder.yield`／`Continuation_meta.t` を定義。`Demand_hint` は `min_bytes`／`preferred_bytes`／`frame_boundary`／`reason` を保持し、`frame_boundary` は `Parser_token.classification` と `Diagnostic.span` の両方を扱えるようバリアント構造にした。
- Feeder は同期 pull 関数を基点としつつ、`Await` を返した場合の後続通知（Step4 で CLI/LSP へ接続）に備えて `yield` バリアントを用意。PoC では非同期ランタイム依存を避ける方針。
- `Continuation.t` には Step1 で導入した `Session.t` を埋め込み、Packrat 状態と診断メタデータを `Continuation_meta` に集約する設計へ改めた。

### 2. RunConfig 拡張と継続メタデータ
- `RunConfig.extensions["stream"]` を `stream` / `stream.demand` / `stream.flow` の 3 ネームスペースへ整理。`Demand_hint` を `stream.demand` に、そのほか（`checkpoint` や `flow_mode` 等）を `stream` 側に収容する。
- `Demand_hint.of_namespace`／`to_namespace` に検証処理を追加し、0 以下の `preferred_bytes` を拒否・`min_bytes <= preferred_bytes` を強制。未知キーはログへ集約し、PoC では無視する。
- `Continuation_meta.expected` に `parser_diag_state.farthest_snapshot` 由来の `ExpectationSummary` を格納し、`span_trace_pairs` から `last_checkpoint` と `trace_label` を抽出。`resume_hint` と `Stream_outcome.Pending.demand` を同一レコードで共有する方針を `core_parse_streaming.ml` へ TODO 化した。

### 3. 検証結果とフォローアップ
- `Parser_token.Class.to_symbol`／`of_symbol` が `frame_boundary` の往復に利用できることを確認。未知シンボルは `Frame_boundary.Unsupported` として `Stream_error_kind.FeederBug` を返す設計を採用。
- `ExpectationSummary.alternatives` を 16 件に切り詰める保護処理を追加し、継続メタデータ全体で約 512B に収まる見込みを確認。`Pending` 監査ログも 1.5KB 前後を維持できる。
- フォローアップ: Step3 で `Stream_outcome.Pending` 生成時に `Demand_hint` を共有して GC 圧力を抑制すること、Step4 で `Await` 発生率メトリクス（`parser.stream.await_ratio`）を `0-3-audit-and-metrics.md` へ登録すること。

## PARSER-002 Day3 extensions シム構築（2025-11-20）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 作業サマリ
- `parser_run_config` に `Config` / `Lex` / `Recover` / `Stream` サブモジュールを追加し、`extensions` ネームスペースを型安全に読み出すシムを導入。`lex.profile` は未指定時に `ConfigTriviaProfile::strict_json` を返し、`ParserId` は `int option` として保持するよう整理した。  
- `Parser_diag_state.create` が `recover` 設定を受け取れるようになり、`recover.sync_tokens` / `recover.notes` を `recover_sync_tokens` / `recover_notes_enabled` で参照可能。`parser_driver` から `Run_config.Recover.of_run_config` を呼び出し、RunConfig の回復設定が診断経路へ伝播する。  
- `Run_config.Config.require_eof_override` を切り出して `parser_driver` の未消費入力判定を共通化し、`Run_config.Stream.of_run_config` がストリーミング用プレースホルダを返すよう準備した。`compiler/ocaml/src/dune` を更新後に `dune build` を実行し、ワーニングのみでビルド完了を確認。  

### 2. 影響ファイル
- `compiler/ocaml/src/parser_run_config.{ml,mli}`  
- `compiler/ocaml/src/parser_diag_state.ml`  
- `compiler/ocaml/src/parser_driver.ml`  
- `compiler/ocaml/src/dune`  
- `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`（Step 3 更新記録）  
- `docs/plans/bootstrap-roadmap/2-5-review-log.md`（本メモ）

### 3. フォローアップ / TODO
- `LEXER-002` で `Run_config.Lex.effective_trivia` を利用し、共有トリビア API に接続する。  
- `EXEC-001` の `run_stream` 実装時に `Run_config.Stream` プレースホルダを利用して checkpoint/resume 情報を連携する。  
- Step 5 で `Recover` 設定を切り替えた際の診断出力とメトリクス（`parser.runconfig_extension_pass_rate`）を追加検証する。  

## ERR-001 Day1 Menhir 期待集合 API 棚卸し（2025-11-13）

### 1. Menhir 出力サマリ
- `menhir --list-errors compiler/ocaml/src/parser.mly` を実行し、`compiler/ocaml/src/parser.automaton` を再確認したところ状態数は 467 件、shift/reduce 27・reduce/reduce 10 の既存コンフリクト構成に変化なし。
- 期待集合に現れた終端は 74 種類（予約語 33・記号 32・リテラル 5・EOF 1）で、`IDENT`/`STRING`/`INT` が 200 件超の頻出項目、`#` は Menhir の入力終端番兵として扱われる。
- `BREAK`/`CHANNELS`/`CHANNEL_PIPE`/`CONDUCTOR`/`CONTINUE`/`DARROW`/`DO`/`EXECUTION`/`HANDLE`/`MONITORING`/`PERFORM`/`UPPER_IDENT` は期待集合に登場せず、`compiler/ocaml/src/token.ml:49` 以降の予約語定義でも未使用警告の対象となっている。
- 期待集合候補は `compiler/ocaml/src/parser.automaton` から抽出でき、縮約時は記号優先 → 文字クラス → 規則の順で整序する仕様（`docs/spec/2-5-error.md:129`）に従うことで CLI/LSP 表示との整合を保てる。

### 2. API 仕様確認
- `compiler/ocaml/_build/default/src/parser.mli:14` で `Parser.MenhirInterpreter` が `MenhirLib.IncrementalEngine.INCREMENTAL_ENGINE` を公開していることを確認。
- `MenhirLib.IncrementalEngine` は `acceptable` と `MenhirLib.EngineTypes.TABLE.foreach_terminal` を備えており、全終端を走査して checkpoint ごとの期待集合を導出できる。
- トークン定義は `compiler/ocaml/src/token.ml:7` 以降で 85 種類が列挙されており、期待集合生成時はキーワード → 記号 → リテラル → `EOF` のカテゴリごとにサンプル値を用意すれば `acceptable` の判定に利用できる。

### 3. Expectation 写像ルール草案
| Menhir 終端カテゴリ | 対応案 | 備考 |
| --- | --- | --- |
| 予約語 (`FN`/`MATCH` 等) | `Expectation.Keyword (Token.to_string tok)` | `compiler/ocaml/src/token.ml:100` 以降の `to_string` で小文字化 |
| 記号・区切り (`LPAREN`/`PLUS` 等) | `Expectation.Token (Token.to_string tok)` | `PIPE` や `DOTDOT` など複合演算子も記号扱い |
| リテラル (`INT`/`STRING`/`CHAR`/`FLOAT`) | `Expectation.Class "<literal-kind>"` | サンプル値は空文字列・既定基数で構築し `Class` へ収容 |
| 識別子 (`IDENT`/`UPPER_IDENT`) | `Expectation.Class "identifier"` / `"upper-identifier"` | 後者は現状未登場だが仕様整合のため先行定義 |
| 終端番兵 (`EOF`/`#`) | `Expectation.Eof` | Menhir の `#` は `EOF` 相当として扱う |
| 補助 (`Rule`/`Not`/`Custom`) | 上位規則や否定条件を後段で合成 | `docs/spec/2-5-error.md:129` の優先順位へ合わせる |

### 4. Parser_diag_state 制約メモ
- `compiler/ocaml/src/parser_diag_state.ml:24` の `normalize_expectations` は `Stdlib.compare` で並べ替えるため、期待集合の優先順位を保持するにはカテゴリ単位の整列器を別途用意する必要がある。
- `record_diagnostic`（`compiler/ocaml/src/parser_diag_state.ml:27`）は `Diagnostic.expected` が `None` の場合に空リストを採用するため、`ERR-001/S2` 以降で必ず `ExpectationSummary` を生成しないと最遠スナップショットが空集合のままになる。
- `farthest_snapshot`（`compiler/ocaml/src/parser_diag_state.ml:7`）は同一オフセット時に集合和を取る実装なので、Menhir から得た候補をカテゴリ別に縮約してから保存すればノイズを抑制できる。

## ERR-001 Day2 期待集合マッピング実装（2025-11-14）

- `compiler/ocaml/src/parser_expectation.{ml,mli}` を追加し、終端トークン → `Diagnostic.expectation` の写像、`dedup_and_sort` による優先順位整列、`summarize_with_defaults` のフォールバック（`parse.expected` / `parse.expected.empty`）を実装。`humanize` は `Keyword`/`Token` をバッククォートで包む日本語メッセージを生成する。
- `expectation_of_nonterminal` / `expectation_not` / `expectation_custom` を公開し、S3 以降で `Rule`・否定条件・任意候補を `ExpectationSummary` へ集約できるようにした。
- 単体テスト `compiler/ocaml/tests/test_parser_expectation.ml` でキーワード・演算子・リテラル・識別子・EOF・Rule・Not・Custom の 8 ケースとサマリ生成を検証済み。`dune exec tests/test_parser_expectation.exe` の結果を添付し、humanize の自然文と空集合フォールバックを確認。

## ERR-001 Day3 パーサドライバ組込み（2025-11-15）

- `compiler/ocaml/src/parser_expectation.ml` に `collect` を実装し、Menhir チェックポイントから受理可能トークンを走査して `ExpectationSummary` を生成。期待集合が空の際は `Parser_diag_state.farthest_snapshot` 経由でサマリを補完するフォールバックを整理。
- `compiler/ocaml/src/parser_driver.ml` で `HandlingError` / `Rejected` 分岐が `collect` を呼び出し、`Diagnostic.Builder` で期待集合サマリを直接設定するように変更。legacy 互換用 `parse_result.legacy_error.expected` へも同じ候補が伝播することを確認した。
- `compiler/ocaml/src/parser_diag_state.ml` の `farthest_snapshot` に `expected_summary` フィールドを追加し、同一オフセットで診断が蓄積された場合も候補を集合和で縮約するよう更新。
- テスト: `compiler/ocaml/tests/test_parser_driver.ml` / `compiler/ocaml/tests/test_parse_result_state.ml` に期待集合の非空検証を追加し、`run_string` / legacy API の両方で `Diagnostic.expected` と `legacy_error.expected` が一致することをケース化した。
## ERR-001 Day4 ゴールデンと CI 監視整備（2025-11-16）

- `compiler/ocaml/tests/golden/diagnostics/parser/expected-summary.json.golden` を追加し、`dune exec tests/test_cli_diagnostics.exe` で CLI JSON スナップショットを再生成。`tooling/lsp/tests/client_compat/fixtures/diagnostic-v2-sample.json` も `expected.message_key = "parse.expected"` と `locale_args` を付与して LSP 互換テストへ期待集合を反映した。
- `scripts/validate-diagnostic-json.sh` に Parser 診断専用の検証を追加し、`expected` セクションが欠落または `alternatives` が空の場合は即時にエラーを報告するよう強化。
- `tooling/ci/collect-iterator-audit-metrics.py` へ `parser.expected_summary_presence` / `parser.expected_tokens_per_error` を導入し、`summarize_diagnostics` でも Parser 期待集合の統計を集計。`--require-success` 時には期待集合が 0 件の構文エラーが検出された段階で CI を失敗させる。
- 指標リスト `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に両指標を追加し、収集タイミングを `scripts/validate-diagnostic-json.sh` と `collect-iterator-audit-metrics.py` に合わせて明記。`reports/diagnostic-format-regression.md` のチェックリストにも期待集合検証の手順を追記した。

## ERR-001 Day5 ドキュメントと共有タスク（2025-11-17）

- `docs/spec/2-5-error.md` から暫定脚注（実装未導入の注記）を整理し、Phase 2-5 完了後の状態を示す脚注 `[^err001-phase25]` を追加。`ExpectationSummary` の説明に Menhir 期待集合が CLI/LSP/監査で共有される旨を明記した。
- `docs/spec/3-6-core-diagnostics-audit.md` の `ExpectedSummary` 解説へ同様の反映を行い、診断モデル側でも Phase 2-5 ERR-001 の実装完了を参照できるよう脚注 `[^err001-phase25-core]` を追加。
- `docs/guides/core-parse-streaming.md` と `docs/guides/plugin-authoring.md` を更新し、ストリーミング経路およびプラグイン API が `ExpectationSummary` をそのまま活用できる運用ガイドを追記。S4 時点の CLI/LSP ゴールデンは再利用し、ドキュメント差分のみで完結しているため追加のスナップショット生成は不要と判断。
- フォローアップ共有として `docs/notes/spec-integrity-audit-checklist.md` の草案を作成し、Phase 2-8 で利用する監査チェックリストに期待集合モニタリング項目（`parser.expected_summary_presence` / `parser.expected_tokens_per_error`）を登録できるよう TODO セクションを整備。
- `docs/plans/bootstrap-roadmap/2-5-proposals/ERR-001-proposal.md` の S5 セクションを更新し、仕様・ガイド・ノート更新とレビュー共有が完了したことを記録。差分は Git 差分レビューで確認済み、追加のコマンド実行は無し。

## DIAG-002 Day1 調査

DIAG-002 の初期洗い出し結果を記録し、後続フェーズでの追跡に利用する。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-002-proposal.md`](./2-5-proposals/DIAG-002-proposal.md)

## 1. Diagnostic を直接構築している経路
| 種別 | ファイル:行 | 状態 | 想定対応 |
|------|-------------|------|----------|
| Legacy 変換 | `compiler/ocaml/src/diagnostic.ml:181` | `Diagnostic.Legacy.t` から `Diagnostic.t` をレコード直接構築。`audit = None` のまま返却され、`Legacy.audit_metadata` が空の場合は監査キーが欠落する。 | Week31 Day2 以降で `Diagnostic.Builder` 経由の移行パスを追加し、最低限 `Audit_envelope.empty_envelope` と `iso8601_timestamp` を強制する。既存のテストは Builder 経路へ切り替える。 |

## 2. 監査メタデータが不足する経路（`Diagnostic.Builder.create` → `Builder.build`）
| 優先度 | ファイル:行 | 出力チャネル | 現状 | 対応メモ |
|--------|-------------|--------------|--------|----------|
| 高 | `compiler/ocaml/src/llvm_gen/verify.ml:131` | `--verify-ir` 失敗時 (CLI) | `Builder.build` 直後の診断をそのまま `main.ml:597` から出力。`attach_audit` が呼ばれないため `cli.audit_id` / `cli.change_set` など `tooling/ci/collect-iterator-audit-metrics.py` が必須とするキーが欠落し、`ffi_bridge.audit_pass_rate` 集計で非準拠扱い。 | Day2 で `Verify.error_to_diagnostic` に `Diagnostic.set_audit_id` / `set_change_set` を注入するか、`main.ml` 側で再利用している `attach_audit` を適用する。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:945` | `Parser_driver.process_lexer_error` | Builder 直後は監査メタデータが空だが、`main.ml:803` で `attach_audit` を通すため CLI/LSP 出力時点では `cli.audit_id` / `cli.change_set` が補完される。 | 現状維持でも仕様違反にはならないが、計測ログ用の `parser.*` 系キーを Builder 側で自動付与する改善案を検討。 |
| 中 | `compiler/ocaml/src/diagnostic.ml:950` | `Parser_driver.process_parser_error` | Lexer エラーと同じ挙動。`attach_audit` により最終的な監査キーは揃う。 | Parser 向けメタデータ自動化を Lexer と合わせて検討。 |
| 低 | `compiler/ocaml/tests/test_cli_diagnostics.ml:27` | CLI フォーマッタのゴールデン | テスト専用のダミー診断。監査キーが空のままのため、必須化後は `Diagnostic.set_audit_id` 等でフィクスチャを更新する必要がある。 | Day3 以降でゴールデン再生成。レビュー時に `REMLC_FIXED_TIMESTAMP` を考慮。 |

## 3. 補足メモ
- `main.ml:665-694` の Core IR / Codegen 例外、`main.ml:744-748` の型推論エラー、`main.ml:803-804` のパース失敗は `attach_audit` を経由しており、`cli.audit_id`・`cli.change_set` が付与される。
- `tooling/ci/collect-iterator-audit-metrics.py` は 14 件の audit メタデータキーを必須としている。High 優先度の経路から出力される診断は pass rate を 0.0 に固定する要因となるため、Phase 2-5 内での修正を優先する。

## 4. Legacy / シリアライズ整備 進捗（2025-11-02 更新）
- **監査キー補完**: Builder/Legacy 双方で `ensure_audit_id` / `ensure_change_set` を導入し、空値の場合は `phase2.5.audit.v1` テンプレート（CLI: `audit_id = "cli/" ^ build_id ^ "#" ^ sequence`、Legacy: `audit_id = "legacy-import/" ^ build_id`）を生成してから `Audit_envelope.has_required_keys` を通過させる。`missing` フィールドは必須キーが揃った段階で自動的に除去される（compiler/ocaml/src/diagnostic.ml:304-370）。
- **Audit_envelope 拡張**: `Audit_envelope.has_required_keys` を CLI 監査キー込みで再定義し、`missing_required_keys` を公開して検証・エラーメッセージ両方に利用できるようにした（compiler/ocaml/src/audit_envelope.ml:120-189）。
- **シリアライズ検証**: `Diagnostic_serialization.of_diagnostic` で必須キーと `timestamp` をチェックし、欠落時は `[diagnostic_serialization] …` を stderr に出力して `Invalid_argument` を送出する運用へ移行した（compiler/ocaml/src/diagnostic_serialization.ml:75-88）。
- **テスト/ログ**: `dune runtest`（compiler/ocaml）を再実行し、更新された診断ゴールデン（Typeclass/FFI/Effects）を整合させた。`tooling/ci/collect-iterator-audit-metrics.py` は不足フィールドを stderr に出力するようになり、`--require-success` 実行時のトラブルシューティングが容易になった。

## 5. `phase2.5.audit.v1` テンプレート実装後の検証（2025-11-06 更新）
- **CLI/テスト経路の統一**: `compiler/ocaml/src/main.ml` と `test_cli_diagnostics.ml` / `test_ffi_contract.ml` / `test_effect_residual.ml` を更新し、CLI 実行・ユニットテストいずれの経路でも `audit_id = "cli/<build_id>#<sequence>"` とテンプレート化された change-set を出力するようになった。  
- **ゴールデン更新**: Typeclass / FFI / Effects 系ゴールデン（診断 JSON・監査 JSONL）を再生成し、`bridge.audit_pass_rate`・`effect.handler_stack`・`typeclass.*` など必須メタデータが埋まっていることを確認。  
- **CI メトリクス**: `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` をローカルで実行し、`iterator.stage.audit_pass_rate`・`typeclass.dictionary_pass_rate`・`ffi_bridge.audit_pass_rate` がすべて 1.0 となることを確認（従来の `auto-*` / `legacy-*` プレースホルダによる欠落は解消済み）。  
- **残タスク**: LSP／Legacy 経路へのテンプレート適用手順と、`timestamp` 生成の最終的な責務分担（`Ptime` への移行可否）を別途整理し、監査チームとの合意を待つ。

## 6. Week31 Day4-5 テスト／ドキュメント反映ログ（2025-10-27）
- `scripts/validate-diagnostic-json.sh` を既定ディレクトリ（`compiler/ocaml/tests/golden/diagnostics`, `compiler/ocaml/tests/golden/audit`）で実行し、スキーマ違反がないことを確認。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute.json.golden --source compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute-unknown-tag.json.golden --source compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden --source compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden --source compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-linux.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-macos.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-windows.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/effects-stage.json.golden --audit-source compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` を完走。`diagnostic.audit_presence_rate` / `typeclass.metadata_pass_rate` / `ffi_bridge.audit_pass_rate` がいずれも `1.0` に到達した。
- 上記に伴い、以下のゴールデンを `phase2.5.audit.v1` テンプレートへ整備:
  `compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute.json.golden`,
  `compiler/ocaml/tests/golden/diagnostics/effects/invalid-attribute-unknown-tag.json.golden`,
  `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden`,
  `compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden`,
  `compiler/ocaml/tests/golden/typeclass_dictionary_resolved.json.golden`（監査キー重複出力の調整を含む）。
- Spec 3.6 に DIAG-002 完了脚注を追加し、`phase2.5.audit.v1` 必須化の合意を記録。`reports/diagnostic-format-regression.md` チェックリストにも `audit` / `timestamp` の確認項目を追記済み。

# 2-5 レビュー記録 — DIAG-001 Week31 Day1-2 現状棚卸し（2025-11-07 更新）

DIAG-001 ステップ 1「現状棚卸しと仕様突合」の調査メモ。Severity 列挙の定義差異と周辺実装の挙動を整理し、後続ステップの改修範囲を明確化する。

## 1. 列挙定義と仕様参照の比較
| 区分 | 参照先 | 列挙内容 / 状態 | 観測メモ |
| ---- | ------ | ---------------- | -------- |
| 仕様 (Chapter 3) | `docs/spec/3-6-core-diagnostics-audit.md:24-43` | `Severity = Error | Warning | Info | Hint` を正式仕様として定義。 | CLI/LSP で情報診断とヒントを区別することを前提にしている。 |
| 仕様 (Chapter 2) | `docs/spec/2-5-error.md:12-55` | `Severity = Error | Warning | Note` のまま据え置き。 | Chapter 3 と不一致。Phase 2-5 でいずれかを統一する必要あり。 |
| 実装 — モデル層 | `compiler/ocaml/src/diagnostic.ml:39-46` | `type severity = Error | Warning | Note`。`severity_label` も 3 値前提。 | `Hint` 相当のバリアントなし。 |
| 実装 — V2 変換 | `compiler/ocaml/src/diagnostic.ml:803-821` | `module V2` で `Severity = Error | Warning | Info | Hint` を定義し、`Note -> Info` へ丸め込み。 | 新バリアントはここでのみ登場。`Hint` 未使用。 |
| JSON スキーマ | `tooling/json-schema/diagnostic-v2.schema.json:14-37` | LSP 準拠で `severity enum = [1,2,3,4]` を要求。 | スキーマ上は `Hint` 値（4）を許容するが、実装側に対応経路がない。 |

## 2. シリアライズと出力経路の挙動
- `compiler/ocaml/src/diagnostic_serialization.ml:249-269` では `severity_to_string` が `note` を出力し、`severity_level_of_severity` が 1/2/3 のみを返却。CLI JSON（`compiler/ocaml/src/cli/json_formatter.ml:90-145`）および LSP トランスポート（`tooling/lsp/lsp_transport.ml:48-116`）はいずれもこの 3 値を前提にしている。
- `compiler/ocaml/src/cli/color.ml:86-102` は `Note` 用の配色を定義しており、`Info`/`Hint` を考慮していない。
- `tooling/ci/collect-iterator-audit-metrics.py:1004-1025` は診断 JSON の集計時に `note -> info` へ正規化し、`hint` も集計カテゴリとして確保しているが現在は未使用。
- `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden` は `severity: "info"` を保持するが、日本語ラベルや古いフィールド構成が混在しており、`Diagnostic_serialization` 由来の最新形式とは乖離している（改修後に再生成予定）。

## 3. ギャップとフォローアップ
- `Hint` バリアントが仕様に存在する一方で実装経路が未実装のため、Phase 2-5 ステップ 2 での列挙拡張時に CLI/LSP/メトリクスすべてを 4 値対応へ更新する必要がある。
- Chapter 2（`docs/spec/2-5-error.md`）が旧 3 値のままのため、仕様の改訂または脚注での移行方針整理が必要。Chapter 3 の脚注と整合する説明を追加する。
- `reports/diagnostic-format-regression.md` チェックリストには Severity 4 値化のレビューポイントが未記載。DIAG-001 完了時に更新し、情報診断／ヒント診断のゴールデン差分を追跡できるようにする。
- `tooling/json-schema/diagnostic-v2.schema.json` と `scripts/validate-diagnostic-json.sh` は `severity=4` を許容しているが、既存フィクスチャに Hint ケースが存在しない。改修後に AJV フィクスチャを追加する。
- メトリクス集計（`diagnostic.info_hint_ratio` 予定値）を Phase 2-5 で追加する際は、`collect-iterator-audit-metrics.py` の出力拡張と連動させ、旧 `note` データの移行を計画する。

## 4. CLI/LSP/監査パイプライン整合確認（2025-11-09 更新）
- LSP: `tooling/lsp/tests/client_compat/tests/client_compat.test.ts:95` に Info/Hint 専用ケースを追加し、`diagnostic-v2-info-hint.json` で `severity = [3, 4]` を確認。`npm run ci --prefix tooling/lsp/tests/client_compat` を実行し、新フィクスチャが AJV 検証を通過することを確認した。  
- CLI: `compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` を `scripts/validate-diagnostic-json.sh` で検証し、文字列 Severity が維持されていることと `audit` / `timestamp` が欠落しないことを再確認。  
- 監査メトリクス: `tooling/ci/collect-iterator-audit-metrics.py:993-1036` に `info_fraction` / `hint_fraction` / `info_hint_ratio` を導入し、`python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/severity/info-hint.json.golden` で Info/Hint の出現比率が `diagnostics.info_hint_ratio` として JSON 出力へ含まれることを確認。  
- ドキュメント: `reports/diagnostic-format-regression.md` へ Info/Hint 用チェックを追加し、Severity 拡張の確認手順をレビュー運用に組み込んだ。

## 5. ドキュメントとメトリクス更新（Week32 Day3, 2025-11-10 更新）
- 仕様反映: `docs/spec/3-6-core-diagnostics-audit.md` に DIAG-001 脚注を追加し、`severity` フィールドが 4 値へ統一された経緯と `Note` 廃止方針を明文化。`Severity` 説明に CLI/LSP/監査での区別運用を追記した。  
- 指標定義: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標表へ `diagnostic.info_hint_ratio` を追加し、CI 集計で情報診断とヒント診断の比率を監視できるようにした。`diagnostic.hint_surface_area` は Phase 2-7 で集計実装予定として暫定登録。  
- 集計スクリプト連携: `collect-iterator-audit-metrics.py` のサマリ出力に追従した説明を同文書へ追記し、`info_fraction` / `hint_fraction` / `info_hint_ratio` が `diagnostics.summary` へ記録されることを明示。  
- 残課題: `diagnostic.hint_surface_area` の算出はスパン計測ロジックを追加した後に `tooling/ci/collect-iterator-audit-metrics.py` へ組み込む。Phase 2-7 で CLI テキスト出力刷新と合わせて優先度を再評価する。

# 2-5 レビュー記録 — EFFECT-001 Day1 タグ棚卸し

Phase 2-5 Week31 Day1。`EFFECT-001` のステップ 1（タグ語彙と既存実装の棚卸し）を実施し、仕様と実装のギャップを整理した。

## 1. Phase 2-5 で扱うタグ語彙
| タグ | 区分 | 主な仕様出典 | 想定 API / Capability 例 |
| ---- | ---- | ------------ | ------------------------ |
| `mut` | Σ_core | docs/spec/1-3-effects-safety.md §A | `var` 再代入、`Vec.push`, `Cell.set` |
| `io` | Σ_core | docs/spec/1-3-effects-safety.md §A | `Core.IO.print`, `Core.File.read` |
| `ffi` | Σ_core | docs/spec/1-3-effects-safety.md §A, docs/spec/3-8-core-runtime-capability.md §10 | `extern "C"` 呼び出し、Capability Bridge |
| `panic` | Σ_core | docs/spec/1-3-effects-safety.md §A | `panic`, `assert`, `Result.expect` |
| `unsafe` | Σ_core | docs/spec/1-3-effects-safety.md §A, docs/spec/3-6-core-diagnostics-audit.md §4.2 | `unsafe { … }`, `addr_of`, 生ポインタ操作 |
| `syscall` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-8-core-runtime-capability.md §8 | `Core.System.raw_syscall`, ランタイム Capability `system.call` |
| `process` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Process.spawn_process`, `Capability.process` |
| `thread` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Process.create_thread`, `Capability.thread` |
| `memory` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-4-core-collection.md §5 | `Core.Memory.mmap`, `Core.Memory.mprotect` |
| `signal` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Signal.register_signal_handler` |
| `hardware` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.Hardware.rdtsc`, `Capability.hardware` |
| `realtime` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Core.RealTime.set_scheduler_priority` |
| `audit` | Σ_system | docs/spec/1-3-effects-safety.md §A, docs/spec/3-6-core-diagnostics-audit.md §3 | `Diagnostics.audit_ctx.log`, 監査 Capability |
| `security` | Σ_system | docs/spec/1-3-effects-safety.md §A | `Capability.enforce_security_policy` |
| `mem` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-0-core-library-overview.md §2 | `Core.Collection.Vec.reserve`, `@no_alloc` 連携 |
| `debug` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1 | `Core.Debug.inspect`, `expect_eq` |
| `trace` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-6-core-diagnostics-audit.md §5 | `Core.Diagnostics.emit_trace`, 監査ログ拡張 |
| `unicode` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1, docs/spec/3-3-core-text-unicode.md §4 | `Core.Text.normalize`, Unicode テーブル参照 |
| `time` | Σ_stdlib | docs/spec/1-3-effects-safety.md §A.1 | `Core.Time.now`, 高精度タイマ |

> 備考: Phase 2-5 では `Σ_core` と `Σ_system` の主要タグを Typer で検出し、`Σ_stdlib` のタグは監査メタデータ補完と脚注整備を優先する。Capability Registry 側の命名はすべて小文字化して突合する必要がある。

## 2. Effect_analysis 実装観察（compiler/ocaml/src/type_inference.ml:37-190）
| 対象 | 現状実装 | 検出漏れ・論点 | 備考 |
| ---- | -------- | -------------- | ---- |
| `TCall` (関数呼出) | `callee_name = "panic"` の場合のみ `add_tag "panic"`。引数は再帰解析。 | `ffi` / `io` / `syscall` / Capability 付き API を識別する経路が存在しない。`Ffi_contract`・`Effect_profile.normalize_effect_name` 未連携。 | `expr.texpr_span` をタグに付与できるため、判別ロジック追加でスパンは再利用可。 |
| `TAssign` / `TAssignStmt` | 左右を再帰的に解析するのみ。 | `mut` タグが付与されない。`docs/spec/1-3-effects-safety.md §E` の再代入制約と乖離。 | `lhs.texpr_span` が利用できるが範囲が Dummy の場合は fallback 必要。 |
| `TVarDecl` / `TLetDecl` | 初期化式を解析するがタグ付与なし。 | `var` 宣言自体が `mut`（再代入許容）であることをタグに反映していない。 | `collect_decl` では宣言種別を判定できるため、`mut` 追加を検討。 |
| `TUnsafe` / `TUnsafe` ブロック | 内部式のみ解析し、自身でタグ付与しない。 | `unsafe` タグおよびブロック内の残余効果へのマーキングが欠落。 | ブロック span が取得可能。`unsafe` ブロック内で検出した他タグに対する扱いも要設計。 |
| `TCall` (外部呼出検出) | `callee_name` を文字列一致でしか評価しない。 | `extern` / Capability Bridge 呼出を `ffi` / `syscall` 等へ分類できない。 | `Ffi_bridge` スナップショット (`record_ffi_bridge_snapshot`) からタグ推論する案を検討。 |
| `Effect_analysis.add_tag` | 小文字化して重複排除。 | Dummy span (`start=0/end=0`) の扱いは `merge_usage_into_profile` 側で補うのみ。 | 追加タグの span を確保できれば `residual_leaks` へ直接反映可能。 |
| `collect_block` / `collect_stmt` | 逐次的に再帰解析。 | 宣言外の `unsafe` / `io` などを検出する入口は `collect_expr` のまま。 | AST から Statement 種別を判定でき、タグ付けの挿入ポイントは明確。 |

## 3. Stage 判定・Capability 連携メモ
- `Type_inference_effect.resolve_function_profile`（compiler/ocaml/src/type_inference_effect.ml:35-115）は `effect_node.effect_capabilities` の先頭要素しか解決せず、残りの Capability 名を破棄している。Phase 2-5 では配列全体を保持し、`resolved_capabilities` 的な構造を導入する余地がある。
- `stage_for_capability` は Capability 名を小文字化して照合するが、複数 Capability の Stage を合成する仕組みがなく、デフォルト Stage (`Stable`) を返すケースが多い。CI で取り込んだ Stage Trace (`runtime_stage.stage_trace`) との突合タイミングも Typer 側で一回のみ。
- `stage_trace_with_typer` は `cli_option` / `env_var` 由来のステップを先頭に保持しつつ `typer` ステップを挿入するが、Capability が複数ある場合でも `capability` フィールドには先頭名しか格納されない。
- `Effect_analysis.merge_usage_into_profile` の `residual_leaks` は `fallback_span` に関数宣言 span を渡しており、タグ追加時にスパンを確保できれば診断へ反映可能。`normalize_effect_name` で小文字化されるため、タグ一覧も小文字で統一する方針が必要。

## 4. 後続タスクへのインパクト
- タグ検出のギャップを埋めるため、`collect_expr`・`collect_decl` への分岐追加と、Capability 判別のための `Ffi_contract` / 標準ライブラリ API テーブルが必要。ホワイトリスト案は次ステップで `docs/plans/bootstrap-roadmap/2-5-review-log.md` に追記する。
- Stage 判定については `resolved_capability` を単一値で保持しているため、EFFECT-003 で予定している複数 Capability 出力に備えて型拡張が必要。`AuditEnvelope.metadata["effects.required"]` への反映計画とも連動させる。
- スパン情報は `expr.texpr_span` と `decl.tdecl_span` で取得できるため、タグ追加時に Diagnostic へ確実に渡す実装方針を後続工程でまとめる。

## SYNTAX-002 Day1 調査（2025-10-27）

SYNTAX-002 `use` 多段ネスト対応計画のステップ S1（現状棚卸し）結果。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. 仕様と実装の突き合わせ
- `docs/spec/1-1-syntax.md:68-86` で `use Core.Parse.{Lex, Op.{Infix, Prefix}}` のような多段ネストを明示。Formal BNF でも `UseItem ::= Ident ["as" Ident] [ "." UseBrace ]`（`docs/spec/1-5-formal-grammar-bnf.md:24-33`）と再帰展開を認めている。
- AST 定義は `compiler/ocaml/src/ast.ml:372-389` で `item_nested : use_item list option` を保持し、構文木レベルではネストを受け入れる前提になっている。
- Menhir 実装は（修正前の）`compiler/ocaml/src/parser.mly:758-792` で `UseBrace` を構築するが、`use_item` 生成時に常に `{ item_nested = None }` を設定しており、`item_nested` に子要素を格納する経路が存在しない。
- 結果として `. {` 以降で構文エラーが発生し、Chapter 1 のサンプルおよび Formal BNF と実装の間にギャップが残っている。

### 2. 再現手順
1. `cd compiler/ocaml`
2. テスト用ファイル `tmp/use_nested.reml` を作成:
   ```reml
   module sample

   use Core.Parse.{Lex, Op.{Infix, Prefix}}
   ```
3. `dune exec remlc -- --emit-ast tmp/use_nested.reml` を実行すると、`tmp/use_nested.reml:3:24: エラー (構文解析)` が出力され、`Op.{` の直前で解析が停止する。
4. 実行後は `rm tmp/use_nested.reml` でクリーンアップする。

### 3. 修正対象メモ
- `parser.mly` に `use_item` 再帰分岐を追加し、子リストを `item_nested` に格納する必要がある。`items @ [item]` の線形結合は既存のため、保持構造の変更は最小で済む想定。
- `parser_diag_state` / `parser_driver` の期待集合および FixIt は `ERR-001` と連携して更新する。ネスト展開を受理した際の診断メッセージ差分を共有する準備が必要。
- Formal BNF と Chapter 1 の記述に変更不要であることを確認済み。実装側の修正と AST プリンタのテスト追加でギャップ解消が可能。

## SYNTAX-002 Day1-2 AST/型付きAST整合確認（2025-10-27）

S2（AST/型付き AST 整合確認）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. AST と設計メモの確認
- `compiler/ocaml/src/ast.ml:372-389` の `use_tree`/`use_item` は `item_nested : use_item list option` を保持しており、構造上の拡張は既に定義済み。`compiler/ocaml/docs/parser_design.md` へ同内容を再確認する脚注を追加。
- AST プリンタ (`compiler/ocaml/src/ast_printer.ml:452-490`) は `item_nested` を再帰的に出力できる実装になっており、多段構造を持つ `use` が構築されてもシリアライズに追加対応は不要。

### 2. 型付き AST と Typer の追跡
- `compiler/ocaml/src/typed_ast.ml:150-163` では `typed_compilation_unit.tcu_use_decls` を `use_decl list` のまま保持し、`use_item` の構造を変換しない設計であることを確認。
- `compiler/ocaml/src/type_inference.ml:2796-2833` でコンパイル単位を生成する際に `tcu_use_decls = cu.uses` としており、Menhir が `item_nested` を埋めれば Typer 側への伝播がそのまま成立する。

### 3. ギャップ評価と次ステップ
- 型付き AST と Typer に追加改修は現時点で不要。S3 以降は Menhir で `item_nested` を組み立てる実装に集中できる。
- S5 で予定しているメトリクス追加（`parser.use_nested_support`）は、AST/Typer 側がネスト情報を保持できる前提の上に計測を構築する方針で問題なし。

## SYNTAX-002 Day2-3 Menhir ルール実装（2025-10-28）

S3（Menhir ルール実装）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. 実装内容
- `compiler/ocaml/src/parser.mly:780-804` の `use_item` を `ident` + `as` + `.{...}` の再帰構築へ変更し、`item_nested` に `Some nested` を設定できるよう `use_item_nested_opt` 非終端記号を追加。これにより `use Core.Parse.{Lex, Op.{Infix, Prefix}}` などの構文を Menhir レベルで受理可能になった。

### 2. 検証手順
1. `cd compiler/ocaml/src`
2. `menhir --list-errors parser.mly` を実行し、`parser.conflicts`／`parser.automaton` を再生成。既存の shift/reduce / reduce/reduce 件数に変化が無いこと、およびネスト分岐追加による新規コンフリクトが発生しないことを確認した（差分なし）。
3. 生成結果は `ERR-001` チームへ共有し、期待集合リストに変化が無いことのフィードバックを取得。

### 3. フォローアップ
- S4 で予定している Typer／診断連携へ向けて、`tcu_use_decls` の利用箇所（`type_inference.ml`）にネスト構造を踏まえた再帰探索が必要か評価する。
- S5 でのテスト追加（`test_parser.ml`）および CLI ゴールデン更新を行う際は、今回の Menhir 修正に基づいた AST 期待値をベースラインとする。

## EFFECT-003 Week32 Day1 効果プロファイル棚卸し（2025-11-21）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-003-proposal.md`](./2-5-proposals/EFFECT-003-proposal.md)

### 1. 調査サマリ
- Parser 段階の `Effect_profile.profile` は依然として `resolved_capability` に先頭要素のみを格納し、配列本体は `resolved_capabilities` に保持する構造で Stage 情報が未設定（`compiler/ocaml/src/effect_profile.ml:474`, `compiler/ocaml/src/effect_profile.ml:484`）。
- Typer の `resolve_function_profile` は全 Capability に Stage を割り当てる一方、`resolved_stage` と `stage_trace` を代表値（先頭 Capability）で決定しており、残りの Capability が Stage 判定に影響できない（`compiler/ocaml/src/type_inference_effect.ml:73`, `compiler/ocaml/src/type_inference_effect.ml:86`）。
- Core IR への転写も配列が空の場合に単一フィールドへフォールバックするため、複数 Capability を要求する関数が IR メタデータ上で区別されない（`compiler/ocaml/src/core_ir/desugar.ml:1729`）。
- 診断／監査は `effect.stage.capability` を必須スカラーとして扱い、配列は補助情報扱いに留まっている。`metadata_for_effect` と `with_effect_stage_extension` の双方で一次 Capability を前提にメタデータを組み立てている（`compiler/ocaml/src/main.ml:21`, `compiler/ocaml/src/diagnostic.ml:680`）。
- CI 集計スクリプトは `extensions.effects.capability` が空文字でないことを必須条件にしており、配列の検証経路が存在しない（`tooling/ci/collect-iterator-audit-metrics.py:419`）。
- 仕様側は複数 Capability を契約として列挙し、Stage 検証と監査出力に同じ集合を要求しているため、現状の単一値処理が仕様の想定に一致していない（`docs/spec/1-3-effects-safety.md:236`, `docs/spec/3-8-core-runtime-capability.md:115`）。

### 2. 観測されたギャップ
- `resolved_stage` と `stage_trace` が常に先頭 Capability を参照するため、複数 Capability の Stage 判定結果を保存・報告できない。監査側でも `effect.stage.capability` が単一値のまま出力され、規約上必須の配列整合が欠落している（`compiler/ocaml/src/type_inference_effect.ml:82`）。
- 診断生成は `profile.resolved_capability` をメッセージと監査メタデータの基点にしており、複数 Capability を含む関数で部分的な Stage 逸脱が発生してもエラー表示が先頭 Capability に固定される（`compiler/ocaml/src/type_error.ml:1001`）。
- `collect-iterator-audit-metrics.py` が単一 Capability を前提とした必須項目と pass rate を定義しているため、配列化すると現行 CI 指標が未定義になる。指標とゴールデンの改訂を同時に計画する必要がある（`tooling/ci/collect-iterator-audit-metrics.py:401`）。

### 3. TODO / 引き継ぎ
1. Step 1 で `resolved_capability` を段階的に廃止し、全利用箇所を `resolved_capabilities` ベースへ置換する（Typer → ConstraintSolver → Core IR → Diagnostics の順で洗い替え）。
2. 診断／監査出力を配列主体に改修し、`effect.stage.capability` を互換目的の補助フィールドへ後退させながら `collect-iterator-audit-metrics.py` と `reports/diagnostic-format-regression.md` の検証項目を更新する。
3. `stage_trace` を Capability ごとのステップを保持できる構造へ拡張し、`Type_inference_effect.stage_for_capability` の結果を全件転写できるようにする。

## EFFECT-003 Week32 Day2-3 Typer 多重 Capability 適用（2025-11-23）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-003-proposal.md`](./2-5-proposals/EFFECT-003-proposal.md)

### 1. 作業サマリ
- `compiler/ocaml/src/effect_profile.ml:421` と `compiler/ocaml/src/effect_profile.ml:466` に `primary_capability_*` ヘルパと派生処理を追加し、`resolved_capabilities` を一次データとして扱う `make_profile` へ更新。単一フィールド `resolved_capability` は配列から導出する互換用フィールドへ切り替えた。  
- `compiler/ocaml/src/type_inference_effect.ml:50` 以降で Capability 配列全件を Stage 解決し、`stage_trace` に Capability ごとの `typer` ステップを注入。`capability_stage_pairs` も配列起点で生成し、`Type_error.effect_stage_mismatch_error` へ渡すデータを拡充。  
- `compiler/ocaml/src/constraint_solver.ml:48`／`compiler/ocaml/src/core_ir/desugar.ml:1734`／`compiler/ocaml/src/main.ml:21`／`compiler/ocaml/src/type_error.ml:1002` で主 Capability を配列から導出するよう統一し、監査メタデータ・IR メタデータ・診断の各経路で複数 Capability を保持できるようにした。`metadata_for_effect` は `Effect_profile.capability_names` と `capability_resolutions_to_json` を利用して配列ベースの出力に備える。  
- `compiler/ocaml/src/type_inference.ml:2765` の残余効果検出経路は新しい `make_profile` API で再構築し直し、既存の `EffectConstraintTable` へ配列情報をそのまま記録。既存テスト（`compiler/ocaml/tests/test_type_inference.ml`、`compiler/ocaml/tests/test_cli_diagnostics.ml`）の前提条件を再確認し、後方互換性が保たれることを手動確認した（自動実行は未実施）。

### 2. 仕様整合の確認
- Stage 判定は `docs/spec/1-3-effects-safety.md` §I の要件に従い、各 Capability へ個別に `StageRequirement` を適用する動きが Typer 内で再現できた。`stage_trace` にも各 Capability を明示するステップが挿入され、`docs/spec/3-8-core-runtime-capability.md` §8 の監査要求に沿う形で実装トレースが取得できる。  
- Core IR メタデータに複数 Capability が保持されることで、Phase 3 のランタイム検証で必要な `capabilities.required` 配列（`docs/spec/3-8-core-runtime-capability.md` 表 3.8-2）と突き合わせ可能になった。

### 3. 残課題 / 次ステップ
1. 診断・監査側の配列化は未着手のため、Step 2 で `Diagnostic.extensions["effects"]`／`AuditEnvelope.metadata` を更新し、ゴールデンと CI 指標を再整備する。  
2. `stage_trace` の多重化に伴う表示フォーマット（CLI/LSP）の最終調整とカラーリング見直しは Step 2 で扱う。  
3. 自動テストとメトリクス追加（Step 4）での再検証を待ちつつ、Capability 名正規化ポリシーは Runtime チームと協議して `残課題` 行に追記する。

## EFFECT-003 Week32 Day4-5 診断／監査出力多重化（2025-11-29）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-003-proposal.md`](./2-5-proposals/EFFECT-003-proposal.md)

### 1. 作業サマリ
- `compiler/ocaml/src/diagnostic.ml`・`compiler/ocaml/src/main.ml`・`compiler/ocaml/tests/test_effect_residual.ml` を改修し、`effect.required_capabilities` / `effect.actual_capabilities` および `effect.stage.*` 系配列を拡充。単一 Capability 互換のため `effect.stage.capabilities` は残しつつ配列キーを正規化した。  
- 監査・CI 系ツールを更新し、`tooling/ci/collect-iterator-audit-metrics.py` に新フィールド検証を追加。`scripts/validate-diagnostic-json.sh` では効果診断・監査メタデータの配列存在チェックを追加してゴールデン再生成後の逸脱を検知可能にした。  
- `compiler/ocaml/tests/golden/diagnostics/effects/*.json.golden`、`typeclass_iterator_stage_mismatch.json.golden`、監査 JSONL／LSP 実測 `_actual` を再生成し、複数 Capability を含むケースで `capabilities_detail`・`actual_capabilities` が同期することを確認。

### 2. 検証
- `docs/spec/3-6-core-diagnostics-audit.md` §3.2 と `docs/spec/3-8-core-runtime-capability.md` §8 を参照し、命名規則と Stage 契約整合をクロスチェック。  
- `scripts/validate-diagnostic-json.sh` の新検証で効果診断ゴールデンを手動検証（自動実行は未実施）。CI 集計は `tooling/ci/collect-iterator-audit-metrics.py --require-success` で想定キーが揃っていることをローカル確認（メトリクス導出のみ、CI 実行は次フェーズ）。

### 3. 残課題 / 次ステップ
1. Capability 名の正規化ポリシー（小文字化・ハイフン統一）は Runtime 連携タスクに引き継ぎ、`Effect_profile.normalize_capability_name` と RunConfig 側の小文字化処理を Phase 2-7 で最終確定する。  
2. LSP まわりのカラーリング・整形は Step 4 のテスト整備と併せて実施予定。`tooling/lsp/tests/client_compat` のフィクスチャ更新は Step 4 での自動テスト追加と同時に進める。  
3. 監査ダッシュボードへ複数 Capability 指標を追加するタスクを Phase 2-7 `diagnostics.dashboard-update` に連携し、可視化要件（配列比較／Stage 集計）を整理する。

## EFFECT-003 Week33 Day1 RunConfig／lex シム統合（2025-12-03）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-003-proposal.md`](./2-5-proposals/EFFECT-003-proposal.md)

### 1. 作業サマリ
- `compiler/ocaml/src/parser_run_config.{ml,mli}` に `Effects` サブモジュールを追加し、`stage` / `registry_path` / `required_capabilities` を設定・除去できるユーティリティを実装。`Cli.Options.to_run_config` で CLI オプションを同ネームスペースへ反映するよう更新した。  
- `compiler/ocaml/src/runtime_capability_resolver.ml` を更新し、RunConfig 由来の Stage override と Capability ヒントを `resolve` が取り込むよう変更。RunConfig で指定した Capability は default stage で補完され、`stage_trace` に `source="run_config"` のステップを追加するようにした。  
- `compiler/ocaml/src/main.ml` の RunConfig 構築を解析前に行い、Runtime resolver の結果を `Effects.set_required_capabilities` で RunConfig へ書き戻す経路を追加。`Core_parse_lex.Bridge.derive` と併用しても `extensions["effects"]` が保持されることを手動確認した。

### 2. 検証
- CLI 実行時に `Runtime_capability_resolver.resolve` が返す `stage_trace` へ `run_config` ステップが追加され、`effect.stage.required_capabilities` が CLI 指定の Capability を維持することをログで確認。  
- RunConfig を経由しない既存経路（LSP・テスト）の動作に変化が無いことを差分実行で確認。Stage override 未指定時には `extensions["effects"].stage` が生成されず互換性が保たれる。  
- `tooling/ci/collect-iterator-audit-metrics.py --require-success` を再実行し、`extensions.effects.required_capabilities` が欠落しないことを確認。

### 3. 残課題 / 次ステップ
1. LSP 側の RunConfig ビルダーにも `Effects` ネームスペース設定を導入し、CLI と同じ Capability 配列を共有できるよう Phase 2-7 へ連携する。  
2. Runtime resolver が返す Capability 名の正規化ポリシーを Runtime チームと調整し、RunConfig 側の正規化処理と統一する。  
3. `docs/spec/2-6-execution-strategy.md` で予定されている `max_handler_depth` など追加ポリシーを `RunConfig.extensions["effects"]` へ拡張するタイムラインを TYPE-001 / EFFECT-001 フォローアップに記録する。

## EFFECT-003 Week33 Day2 テスト・メトリクス整備（2025-12-06）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/EFFECT-003-proposal.md`](./2-5-proposals/EFFECT-003-proposal.md)

### 1. 作業サマリ
- `compiler/ocaml/tests/capability_profile_tests.ml` を新設し、`StageRequirement::{AtLeast, Exact}` と複数 Capability の組み合わせで `resolve_function_profile` が配列を保持し、`stage_trace` に全 Capability が記録されることを確認。  
- `compiler/ocaml/tests/test_cli_diagnostics.ml` に配列検証ロジックを追加し、`typeclass_iterator_stage_mismatch.json.golden` を複数 Capability 例で更新。CLI/LSP/Audit の各出力が同一配列を返すかを JSON レベルで比較。  
- `tooling/ci/collect-iterator-audit-metrics.py` に `effect.capability_array_pass_rate` 指標を追加し、`--require-success` の強制判定へ組み込み。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` を更新し、`diagnostics.effect_stage_consistency` が Stage ミスマッチ検知、`effect.capability_array_pass_rate` が配列欠落検証を担うことを明示。  
- 仕様書（`docs/spec/1-3-effects-safety.md`、`docs/spec/3-8-core-runtime-capability.md`）へ脚注を追加し、Phase 2-5 完了条件として配列出力が必須になったことを記録。関連差分は本ログおよび `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` に追記。

### 2. 検証
- `dune runtest compiler/ocaml/tests/capability_profile_tests.exe` と `dune runtest compiler/ocaml/tests/test_cli_diagnostics.exe` をローカル実行し、配列検証がグリーンであることを確認。  
- `tooling/ci/collect-iterator-audit-metrics.py --source compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden --require-success` を実行し、新指標が `pass_rate = 1.0` となることを確認。

### 3. TODO / 引き継ぎ
- Phase 2-7 では監査ダッシュボードに `effect.capability_array_pass_rate` を表示し、複数 Capability 可視化の UI 要件を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に反映する。  
- Capability 名の正規化（大文字・ハイフン変換）と Stage 表示スタイルは残課題として Step6 で追跡。  
- Self-host 実装移行時に Reml 側の診断出力でも同配列フォーマットを維持するか、EFFECT-003 フォローアップで確認。

## SYNTAX-002 Day3-4 束縛診断連携（2025-10-29）

S4（束縛・診断連携）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. 実装内容
- `compiler/ocaml/src/module_env.ml` を新設し、`flatten_use_decls` で `use` ネストを `binding_local`／`binding_path`／`binding_is_pub` に展開する `use_binding` レコードを導入。
- 型付き AST (`typed_ast.ml:156-164`) に `tcu_use_bindings` を追加し、`type_inference.ml:2796-2833` で Typer 完了時に束縛リストを生成。今後のモジュール解決や診断で再利用できる共有データを確保。
- `compiler/ocaml/tests/test_module_env.ml` を追加し、単純な `use`／`alias`／多段ネスト／`pub use` の 4 ケースを検証。展開結果（ローカル名・解決パス・pub フラグ）が仕様と一致することを確認した。

### 2. 診断影響の確認
- `parser_diag_state.ml` の最遠エラー集約と期待集合のソートは `use` 展開に依存していないため追加変更は不要。`menhir --list-errors parser.mly` 実行結果にも S3 からの差分がないことを再確認。
- `ERR-001` 計画へ「S4 完了時点で期待集合の変化が無い」旨を共有し、FixIt 拡張の追従は不要であることを合意済み。

### 3. フォローアップ
- `Module_env.use_binding` を Phase 2-7 再エクスポート解決タスクへ引き渡し、`binding_local` 名で型環境へ取り込む処理を設計する。
- S5 で予定している `parser.use_nested_support` メトリクス算出は `flatten_use_decls` の結果を基に成功率を評価する。

## SYNTAX-002 Day4-5 検証・ドキュメント更新（2025-11-12）

S5（検証とドキュメント更新）の結果共有。  
関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-002-proposal.md`](./2-5-proposals/SYNTAX-002-proposal.md)

### 1. テストと検証
- `compiler/ocaml/tests/test_parser.ml` に多段ネスト `use` を検証するユニットテストを追加。`UseBrace` 配下で `item_nested` が `Some [...]` となり、`Op.{Infix, Prefix}` が再帰的に構築されることを直接確認するヘルパー（`expect_use_nested`）を実装。
- `compiler/ocaml/tests/test_module_env.ml` と併せて `dune runtest compiler/ocaml/tests/test_parser.exe` および `dune runtest compiler/ocaml/tests/test_module_env.exe` を実行し、`flatten_use_decls` まで含めた再エクスポート展開が成功することを確認。（CI 連携時は `dune runtest` 全体で取得したログを `reports/diagnostic-format-regression.md` に添付予定。）

### 2. メトリクスとドキュメント
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `parser.use_nested_support` を追加し、`dune runtest` 完了後に `tooling/ci/collect-iterator-audit-metrics.py --summary` で収集する運用を明記。成功率が 1.0 未満の場合は Phase 2-7 Parser チームへ即時エスカレーションする。
- 仕様側では `docs/spec/1-5-formal-grammar-bnf.md` に脚注を追加し、`UseItem` の再帰規則と実装・監視体制を記録。`docs/spec/3-0-core-library-overview.md` には Core.* の再エクスポートが同機能に依存する旨を追記し、標準ライブラリ観点からのギャップが解消されたことを明示した。

### 3. フォローアップ
- `parser.use_nested_support` を CI ダッシュボードへ表示する際の閾値設定と、失敗時に収集する追加ログ（Menhir `--list-errors` 出力など）のテンプレートを Phase 2-7 で整備する。
- `pub use` の可視性ルール検証は Phase 2-7 `SYNTAX-002` 後続タスクへ引き継ぐ。`binding_is_pub` を利用した公開面積の測定は `Module_env` で準備済み。

## PARSER-002 Day4 RunConfig クライアント統合（2025-11-21）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 実装サマリ
- `compiler/ocaml/src/cli/options.ml` に `Cli.Options.to_run_config` を追加し、`--require-eof` / `--packrat` / `--left-recursion` / `--no-merge-warnings` を経由して RunConfig を構築。従来 CLI が暗黙に設定していた `require_eof=true` は仕様の既定値（false）へ合わせ、互換モードはフラグ指定で明示する方針に変更した。
- `compiler/ocaml/src/main.ml` で `Parser_driver.run ~config` を採用し、RunConfig を経由したパース結果を既存パイプラインへ接続。`Test_support` を新設してユニットテストから同一の RunConfig を再利用できるようにし、`test_parser.ml` / `test_type_inference.ml` をヘルパ経由へ移行した。
- LSP 側に `tooling/lsp/run_config_loader.ml` を追加し、`tooling/lsp/config/default.json` に定義した設定から `extensions["lex"|"recover"|"stream"]` を復元するロードパスを定義。CLI と同様に `extensions["config"].source = "lsp"` を記録してトレースの出所を区別できるようにした。

### 2. 測定・ドキュメント更新
- `reports/diagnostic-format-regression.md` のローカル検証手順に RunConfig 切替シナリオを追加し、`extensions.config.*` の差分を比較できるチェックリストを整備。
- RunConfig 移行時に残っている Packrat/左再帰シムの未実装事項を `docs/notes/core-parser-migration.md` に TODO として記録し、今後の追跡先（`PARSER-003`・`LEXER-002`）を明文化。

### 3. フォローアップ
- `--packrat` / `--left-recursion=on|auto` は現状警告のみを発する暫定実装のため、`PARSER-003` でメモ化シムが揃い次第 CLI/LSP ランタイムを再検証する。
- LSP 設定を利用した自動テストは未整備。`tooling/lsp/tests/client_compat` に RunConfig フィクスチャを追加し、`run_config_loader` 経由で CLI と同じ JSON 出力になることを確認するタスクを Phase 2-7 へ引き継ぐ。

## PARSER-002 Day5 RunConfig テスト・メトリクス整備（2025-11-22）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 実装サマリ
- `compiler/ocaml/tests/run_config_tests.ml` を新設し、`require_eof` 上書き・`merge_warnings` の重複許容・`trace` による SpanTrace 収集・`extensions["lex"]` デコード・Legacy ブリッジ互換を個別に検証するユニットテストを追加。`dune` テストリストへ `run_config_tests` を組み込み、parser スイートから常時実行されるようにした。
- `compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden` を更新し、Packrat 実装後は左再帰警告のみが出力されることを記録。`run_config.switches` と `audit_metadata.parser.runconfig.*` キーを通じて Packrat / 左再帰 / trace / merge_warnings / lex / recover / stream の値が JSON・監査ログに保存されることを示した。
- `tooling/ci/collect-iterator-audit-metrics.py` に `collect_runconfig_metrics` を追加し、`parser.runconfig_switch_coverage` と `parser.runconfig_extension_pass_rate` を集計。`--require-success` 時には両指標が 1.0 未満の場合に失敗させるゲートを組み込んだ。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標表に上記 2 指標を追記し、Phase 2-5 以降の監視対象へ正式追加した。

### 2. 検証・運用手順
- `scripts/validate-diagnostic-json.sh` 既定ターゲットに新ゴールデンを配置したため、既存フロー（parser ゴールデン→AJV 検証）で RunConfig サンプルも同時検査される。`collect-iterator-audit-metrics.py --require-success --source compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden` をローカルで実行し、新指標が `pass_rate=1.0` になることを確認（CI 統合待ち）。
- `dune runtest parser` に `run_config_tests` が追加されたため、Packrat/左再帰の警告と Legacy ブリッジ互換性が常時テストされる。CI ログで新テストの標準出力（`✓ ...`）が確認できるように `Printf` レポート形式を既存テストと合わせた。

### 3. フォローアップ
- RunConfig メトリクスは CLI ゴールデンの存在に依存するため、LSP 側から生成した JSON を追加し `extensions["stream"]` などのバリエーションを拡張する。Phase 2-7 `EXEC-001` と連携し、ストリーミング PoC の JSON も `parser.runconfig_extension_pass_rate` で評価できるようにする。
- Packrat / 左再帰シム実装後には `parser.runconfig_switch_coverage` のサンプルを更新し、警告コードの代わりにメモ化が有効化された証跡（監査ログの `parser.runconfig.packrat.enabled` 等）を測定できるよう指標定義を再検討する。
- 既存 TODO（`PARSER-002 Day1` の項目 3,4）と合わせ、RunConfig 指標の推移を `reports/audit/index.json` に export するスクリプト整備を Phase 2-7 で計画する。

## PARSER-002 Day6 RunConfig 共有・レビュー記録（2025-11-24）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md`](./2-5-proposals/PARSER-002-proposal.md)

### 1. 共有サマリ
- `docs/spec/2-1-parser-type.md` §D `RunConfig` に CLI/LSP 共通設定の利用例を追記し、`with_extension` を用いて `extensions["lex"|"recover"|"stream"]` を同一値で供給する手順を明文化。実装脚注[^runconfig-ocaml-phase25-log] を更新し、Phase 2-5 Step6 で CLI/LSP が `parser_run_config` を共有する構成へ移行したことを記録した。
- `docs/spec/2-6-execution-strategy.md` §B-2 に RunConfig スイッチの運用メモ（CLI/LSP/ストリーミングでの共有ポリシー）を追加し、`parser_driver`・`run_stream` が同一 RunConfig を参照することを脚注で明記した。
- `docs/guides/core-parse-streaming.md` §9 を更新し、RunConfig 共有時に CLI 側の JSON 設定をストリーミング経路へ引き渡すワークフローと `parser-runconfig-packrat.json.golden` を用いた検証手順を紹介した。

### 2. レビュー記録とリンク
- `docs/notes/core-parser-migration.md` に Phase 2-5 RunConfig 移行ステップを追記し、完了タスク（Step1〜6）と残課題（Packrat 実装、LSP 自動テスト、監査指標拡張）を一覧化した。今後の検証先として `PARSER-003`・`LEXER-002`・`EXEC-001` を明示。
- 仕様変更箇所を `docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-002-proposal.md` Step6 更新メモへリンクし、2-5 ステアリングレビューで確認できるよう脚注番号を同期した。
- 共有結果を 2-5 レビュー会議へ提出し、`docs/plans/bootstrap-roadmap/2-5-review-log.md` の該当エントリから各資料へ遷移できるよう相互リンクを確認した。

### 3. 残課題
- LSP 側で RunConfig フィクスチャを用いた自動テストが未整備のため、Phase 2-7 `EXEC-001` タスクで `tooling/lsp/tests/client_compat` にストリーミング設定を追加する。
- Packrat/左再帰シムが完成した際には、仕様脚注を更新して暫定警告コードの撤廃タイムラインを追記し、`parser.runconfig_switch_coverage` 指標を再評価する。
- `RunConfig.locale` と `Diagnostic` のロケール同期は `DIAG-003` の判断待ち。仕様脚注に暫定運用（CLI/LSP は未指定時に英語へフォールバック）を記載しているため、決定次第ガイドと脚注を更新する。

[^runconfig-ocaml-phase25-log]: `docs/spec/2-1-parser-type.md` と `docs/spec/2-6-execution-strategy.md` の脚注参照。`compiler/ocaml/src/main.ml` および `tooling/lsp/run_config_loader.ml` で `parser_run_config` の共有初期化を実施した記録を反映している。

## LEXER-002 Day1 Core.Parse.Lex ギャップ調査（2025-11-25）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md)

### 1. 調査サマリ
- `docs/spec/2-3-lexer.md` が要求する `Core.Parse.Lex` 公開 API（`lexeme` / `symbol` / `config_trivia` / `config_symbol` ほか）を提供するモジュールが現行実装に存在せず、`parser_driver` は `Lexer.token` を直接呼び出しているため `RunConfig.extensions["lex"]` や `ParserId` による共有が機能していない[^lex-spec-api][^parser-driver-token]。
- `parser_run_config.Lex.Trivia_profile` は仕様と同形のフィールドを持つが、`lexer.mll` では `space_id` / `profile` / `doc_comment` 等を参照せず、`shebang` や `hash_inline` の挙動も未実装である[^parser-run-config][^lexer-comment]。
- 現在 `RunConfig.extensions["lex"]` を読むコードはユニットテストのみであり、CLI/LSP/ランナーは `lex` ネームスペースを設定・検証していない。`config_trivia` 相当のユーティリティも欠如しているため設定値が死蔵している[^run-config-tests][^config-trivia-spec]。
- Streaming 系タスクとの依存関係（`ParserId` の安定化、`RunConfig` の伝播、Streaming PoC での lex 再利用）を整理し、`docs/notes/core-parse-streaming-todo.md` に共有メモを追加した。

### 2. 仕様との差分要約

#### 表1: `ConfigTriviaProfile` と `Run_config.Lex.Trivia_profile` の比較
| フィールド / 契約 | 仕様 `ConfigTriviaProfile` | 現行実装 `Run_config.Lex.Trivia_profile` | 差分・課題 |
| --- | --- | --- | --- |
| `line: List<Str>` | コメント接頭辞を列挙し、`config_trivia` で空白スキップに合成する。既定は `["//"]`。 | `line` フィールドあり。`strict_json`/`json_relaxed`/`toml_relaxed` を定義。 | 値は保持するものの、`lexer.mll` で `#` など追加接頭辞を処理せず未使用。 |
| `block: List<CommentPair>` | `start`/`end`/`nested` を保持し、ネスト可否既定は `true`。 | `comment_pair` 型は `start`/`stop`/`nested`。既定プロファイルは `nested=false` を手動設定。 | フィールド名が `stop` となっており `ConfigTriviaProfile` とのマッピング関数が未実装。ネスト再帰処理は `lexer.mll` に固定値。 |
| `shebang: Bool` | 先頭行のみ `#!` を読み飛ばす。 | `shebang` フィールド保持。 | `lexer.mll` は shebang を認識せず、値が常に未使用。 |
| `hash_inline: Bool` | `#` 以降を行コメント扱いにする。 | フィールド保持。 | `lexer.mll` は `//` と `/* */` のみ対応。`#` コメントはエラー扱い。 |
| `doc_comment: Option<Str>` | ドキュメントコメントを診断ノートへ反映。 | フィールド保持。 | `lexer.mll` で `doc_comment` を判別・通知する経路が存在しない。 |
| `config_trivia` / `config_lexeme` / `config_symbol` | `ConfigTriviaProfile` を受け取り、空白・コメント・トークン処理を共通化。 | 未実装。`Run_config.Lex` は値のデコードのみ。 | Lex API 抽出時に新規モジュールを設けてユーティリティを再構築する必要。 |

#### 表2: `RunConfig.extensions["lex"]` 利用状況
| 利用箇所 | 種別 | 現状 | 課題 |
| --- | --- | --- | --- |
| `compiler/ocaml/src/parser_driver.ml` | ランナー | 未使用。`Run_config.Lex.of_run_config` も呼ばれない。 | Lex 設定をランナー初期化へ渡し、`space_id` や `profile` を共有する配線が必要。 |
| `compiler/ocaml/tests/run_config_tests.ml` | ユニットテスト | `Run_config.Lex.of_run_config` で `profile`/`space_id` を復元するテストのみ。 | 実運用経路（CLI/LSP/テストランナー）での検証が欠如。 |
| `parser_run_config` 以外のモジュール | 共通処理 | 存在せず。値の読込・検証は未実装。 | `Core.Parse.Lex` 抽出後に共有ヘルパを追加し、`RunConfig` からの読み出しを一本化する必要。 |

### 3. TODO / 引き継ぎ
1. Step1 で `core_parse_lex.{mli,ml}` を新設し、`ConfigTriviaProfile` マッピングと `lexeme`/`symbol` 等のユーティリティを導入する。
2. `lexer.mll` に `shebang`・`hash_inline`・`doc_comment` の分岐を追加し、`Run_config.Lex.Trivia_profile` からの設定を反映できるよう改修する（Unicode XID 対応も同時に検討）。
3. `parser_driver` と CLI/LSP 初期化コードを更新し、`RunConfig.extensions["lex"]` の値を `Core.Parse.Lex` モジュールへ伝播しつつ、監査メトリクス（例: `lexer.shared_profile_pass_rate`）を 0-3 メトリクス表へ登録する。

### 4. 実施記録
- 2025-11-25: Step0 調査を完了し、本ログと `LEXER-002` 計画書にサマリを反映。Streaming TODO ノートへ依存関係を追記し、次工程（Step1 設計）で参照できる状態にした。

[^lex-spec-api]: `docs/spec/2-3-lexer.md` §C〜§L。
[^parser-driver-token]: `compiler/ocaml/src/parser_driver.ml` の `run` 実装で `Lexer.token` を直接呼び出している。
[^parser-run-config]: `compiler/ocaml/src/parser_run_config.ml` モジュール `Lex`。
[^lexer-comment]: `compiler/ocaml/src/lexer.mll` の `token` ルールは `//`/`/* */` のみ対応し、`shebang`/`#` コメントを扱わない。
[^run-config-tests]: `compiler/ocaml/tests/run_config_tests.ml` `test_lex_extension_profile`。
[^config-trivia-spec]: `docs/spec/2-3-lexer.md` §G-1（`ConfigTriviaProfile` と `config_trivia` 系ユーティリティ）。

## LEXER-002 Day2 Core.Parse.Lex ベースモジュール設計（2025-11-26）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md#step-1-coreparselex-ベースモジュール設計2025-11-26-完了)

### 1. 作業サマリ
- `core_parse_lex` の公開境界を 3 層（`Trivia_profile` alias、`Pack` record、`Api`/`Bridge` ヘルパ）で構成する設計メモを作成し、`ConfigTriviaProfile` と `lexeme`/`symbol`/`config_trivia` を仕様どおり提供する草案を固めた。
- `RunConfig` との round-trip に必要な `effective_profile` / `attach_space` を定義し、`extensions["lex"].space_id` を `Parser_run_config.Extensions.Parser_id` で保持する方針を確認。未設定時のフォールバックは `strict_json` とする。
- `Parser_diag_state` の ID 生成器を流用して `space_id` を払い出す設計と、`doc_comment` を `Diagnostic.notes["comment.doc"]` へ伝播させる拡張ポイント（`Pack.doc_channel` 追加余地）を明示した。試験計画（`core_parse_lex_tests.ml` と `lexer.shared_profile_pass_rate` 指標）も整理。
- 設計結果を計画書 Step1 へ反映し、Streaming TODO ノートに依存関係の更新を書き込んだ。

### 2. 成果物
- `LEXER-002` 計画書 Step1 を完了状態へ更新。
- `docs/notes/core-parse-streaming-todo.md` へ `space_id` round-trip の決定事項を追記。

### 3. 残課題
- Step2 で `lexer.mll` にプロフィールを適用し、Unicode コメント・`doc_comment` 吸い上げを実装する。
- CLI/LSP から `space_id` を必須化する警告コードと監査メトリクスは未決定のため、Step3 以降で案を提示する。

## LEXER-002 Day2 ConfigTriviaProfile ↔ RunConfig 橋渡し（2025-11-27）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md)

### 1. 作業サマリ
- `compiler/ocaml/src/core_parse_lex.ml:59-175` に `Trivia_profile.of_profile` と `Bridge.{derive,with_space_id}` を実装し、`RunConfig.extensions["config"].trivia` と `extensions["lex"]` を読み取って `ConfigTriviaProfile` を再構成する経路を整備。`core_parse_lex.mli:1-40` で公開シグネチャを追加した。
- `parser_run_config` に `Lex.set_profile` / `set_space_id` と `Config.trivia_profile` / `with_trivia_profile` を追加し、プロファイルシンボルと `space_id` を `Extensions.Parser_id` で往復できるようにした（`compiler/ocaml/src/parser_run_config.ml:224-260`、`compiler/ocaml/src/parser_run_config.mli:74-87`）。
- `compiler/ocaml/src/dune:18` へ `core_parse_lex` を登録し、`dune build` で新モジュールをビルド。Phase 1-2 の既存構成に影響がないことを確認した。

### 2. RunConfig 同期フローの確認
- `Bridge.derive` は `extensions["config"].trivia` を優先し、未設定時に `extensions["lex"]`、双方とも未設定なら `Lex.default` を利用する。同期時に `Lex.set_profile` と `Config.with_trivia_profile` を介して RunConfig 内のプロファイルをそろえる（`compiler/ocaml/src/core_parse_lex.ml:134-166`）。
- `Bridge.with_space_id` が `space_id` を `Extensions.Parser_id` で再格納するため、後続ステップで生成した `ParserId` を CLI/LSP/Streaming に共有できる（`compiler/ocaml/src/core_parse_lex.ml:168-174`）。
- `Trivia_profile.of_profile` は namespace の `line` / `block` / `shebang` / `hash_inline` / `doc_comment` を読み取り、仕様どおり `ConfigTriviaProfile` を上書きできる。互換設定を namespace で配列・ブール値として受ける想定をコード化した（`compiler/ocaml/src/core_parse_lex.ml:59-116`）。

### 3. 残課題
- `Custom` プロファイルで namespace が空の場合は `strict_json` を基底に復元している。CLI/LSP からカスタム設定を投入する際の表現形式（`line`/`block` のシリアライズ）を決め、Step3 で namespace への転写ロジックを拡張する。
- `config_trivia` / `config_lexeme` / `config_symbol` の Parser 実装は未着手。`Pack.t` を用いて `lexer.mll` の空白・コメント処理を委譲するステップを次工程で実装する。
- `ParserId` の払い出しは `parser_diag_state`（`compiler/ocaml/src/parser_diag_state.ml`）との統合が必要。`Bridge.with_space_id` を呼ぶタイミングを Step3 で設計し、Packrat との整合を確認する。

## LEXER-002 Day3 lexeme/symbol ユーティリティ実装（2025-11-28）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md#step-3-lexemesymbol-系ユーティリティ実装week33-day3)

### 1. 作業サマリ
- `Core.Parse.Lex.Api` に `config_trivia`/`leading`/`lexeme`/`trim`/`symbol`/`token` を追加し、`Lexer.read_token` を利用して期待記号の検証と `Ast.span` 付与を実装（compiler/ocaml/src/core_parse_lex.ml:177）。
- `lexer.mll` に `set_trivia_profile`／`current_trivia_profile`／`read_token` を導入し、`hash_inline`・`shebang`・`block.nested` をプロファイルで切り替え可能にした（compiler/ocaml/src/lexer.mll:10,98,110,266）。

### 2. 検証と確認事項
- `Lexer.token` が後続トリビアを既に消費するため、`lexeme` 後段の処理は RunConfig 同期のフックに留めた。挙動は従来どおりで、`Lexer_error` の文言も変更なし（compiler/ocaml/src/lexer.mll:196）。
- `Parser_expectation`／`parser_diag_state` の既存フローに変更なし。`span` 付与は単一トークン単位で行い、複合トークンの扱いは Step4 の統合時に再確認する。

### 3. 残課題
- `ParserId` を `Bridge.with_space_id` と結線し、`RunConfig.extensions["lex"]` へ戻すタイミングを Step4 で決定する。
- `doc_comment` 抽出および診断拡張の配線が未着手。プロファイルでコメントを収集できるよう `lexer.mll` のフックを整理する必要がある。

## LEXER-002 Day4 テスト・メトリクス整備（2025-11-30）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md#step-5-%E3%83%86%E3%82%B9%E3%83%88%E3%83%BB%E3%83%A1%E3%83%88%E3%83%AA%E3%82%AF%E3%82%B9%E3%83%BB%E6%80%A7%E8%83%BD%E7%A2%BA%E8%AA%8Dweek33-day4-5-%E2%86%92-2025-11-30-%E5%AE%8C%E4%BA%86)

### 1. 作業サマリ
- `core_parse_lex` のプロフィール切替を検証するユニットテストを追加し、`strict_json` が shebang を拒否する挙動、`json_relaxed` が shebang を許容する挙動、`toml_relaxed` が `#` コメントを読み飛ばす挙動、`Api.symbol` がトリビアを吸収しミスマッチ時に例外を送出する挙動を確認した（compiler/ocaml/tests/core_parse_lex_tests.ml:54-140）。
- `tooling/ci/collect-iterator-audit-metrics.py` に `lexer.shared_profile_pass_rate` を実装し、`run_config` と診断 (`audit_metadata` / `extensions.runconfig.extensions.lex`) で同一プロファイルが共有されているかを判定できるようにした。メトリクス一覧（docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md:23-35）を更新し、CI 監視指標へ追加。
- 字句性能計測ノート（docs/notes/lexer-performance-study.md:1-32）を作成し、`scripts/benchmark-parse-throughput.sh` による測定手順と `remlc` 未構築により計測を後続へ持ち越した事情、再計測 TODO を整理した。

### 2. 成果物
- `compiler/ocaml/tests/core_parse_lex_tests.ml:1-140`
- `tooling/ci/collect-iterator-audit-metrics.py:1218-1516`
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md:20-35`
- `docs/notes/lexer-performance-study.md:1-32`

### 3. 残課題
- `scripts/benchmark-parse-throughput.sh` を実行できる `remlc` 環境を整備し、3 プロファイルで解析時間を取得してノートへ追記する。
- `Core_parse_lex.Record.consume` の集計結果を `lexer.shared_profile_pass_rate` の補助統計としてエクスポートする処理（Step6 以降へ繰り越し）。
- CLI/LSP 経路で `RunConfig.extensions["lex"].space_id` が欠落した場合の警告出力と、計測結果における逸脱検知の自動化。

## LEXER-002 Day5 ドキュメント反映とレビュー記録（2025-12-02）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-002-proposal.md`](./2-5-proposals/LEXER-002-proposal.md#step-6-%E3%83%89%E3%82%AD%E3%83%A5%E3%83%A1%E3%83%B3%E3%83%88%E5%8F%8D%E6%98%A0%E3%81%A8%E3%83%AC%E3%83%93%E3%83%A5%E3%83%BC%E8%A8%98%E9%8C%B2week33-day5)

### 1. 作業サマリ
- 仕様章 `docs/spec/2-3-lexer.md` に `Core.Parse.Lex.Api` と `RunConfig.extensions["lex"]` の連動状況を記した脚注 `[^lex-ocaml-phase25-step6]` を追加し、Step6 で構築した Lex シムが CLI/LSP と共有されることを明文化。
- `docs/spec/2-6-execution-strategy.md` の実装メモを更新し、`RunConfig` と Lex プロファイルの橋渡しを説明する脚注 `[^runconfig-lex-phase25-step6]` を追記。`parser_driver.run`・`Core.Parse.Streaming.run_stream` が同じ設定を受け取る流れを整理した。
- ガイド `docs/guides/core-parse-streaming.md` に `9.2 Core.Parse.Lex プロファイル共有サンプル` を追加し、ストリーミング経路で `Core.Parse.Lex.Bridge.derive` と `Core.Parse.Lex.Api.lexeme` を利用する手順を記載。RunConfig ビルダーと `lexer.shared_profile_pass_rate` の活用方法を共有した。
- レビュー記録本体（本エントリ）へ Step6 の成果を記録し、残課題を 2-7 以降へ引き継ぐ準備を整えた。

### 2. 検証と確認事項
- `docs/spec/2-3-lexer.md` / `docs/spec/2-6-execution-strategy.md` の脚注リンクを手動確認し、`compiler/ocaml/src/core_parse_lex.ml`・`parser_driver.ml`・`parser_run_config.ml`・`tooling/lsp/run_config_loader.ml` の該当行へ遷移できることを確認。
- `docs/guides/core-parse-streaming.md` の新セクションが `RunConfig` 共有手順（9.1）と矛盾しないことをレビュー。`Core.Parse.Lex.Api` 利用例が既存の `StreamDriver` 設計と整合することを確認した。
- メトリクス記録 (`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`) とのリンクを再確認し、`lexer.shared_profile_pass_rate` が引用元として一貫していることをチェック。

### 3. 残課題
- `Core_parse_lex.Record.consume` の集計と `space_id` 警告は引き続き未実装。`Core_parse_lex` チームと連携し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ転記予定。
- `docs/spec/2-3-lexer.md` へ doc_comment 収集の制限事項を追記する判断が残っている。`lexer.mll` の TODO 解消時に脚注更新が必要。

## DIAG-003 Step1 診断ドメイン棚卸し（2025-12-03）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md`](./2-5-proposals/DIAG-003-proposal.md)

### 1. 調査サマリ
- OCaml 側の `error_domain` 列挙は 9 値（`Parser` / `Type` / `Config` / `Runtime` / `Network` / `Data` / `Audit` / `Security` / `CLI`）のみで、仕様が要求する `Effect` / `Target` / `Plugin` / `Lsp` / `Manifest` / `Syntax` / `Regex` / `Template` / `Text` / `Other(Str)` が未定義（`compiler/ocaml/src/diagnostic.ml:54-63`）。  
- `domain_to_string` および CLI/LSP 変換は上記 9 値を前提にしており、未知ドメインはフィールド欠落として出力される。`Effect` 系診断に必要なメタデータを転送できない状態を確認（`compiler/ocaml/src/diagnostic_serialization.ml:125-139`、`compiler/ocaml/src/cli/json_formatter.ml:108-198`、`tooling/lsp/lsp_transport.ml:68-126`）。  
- 生成側では `parser_driver` が `Diagnostic.Parser` / `Diagnostic.Config` を強制し、他領域は未分類のまま。CI の集計ロジックも `domain == "parser"` など限定的な比較に留まっている（`compiler/ocaml/src/parser_driver.ml:58-145`、`tooling/ci/collect-iterator-audit-metrics.py:334-352`）。

### 2. 仕様差分と語彙整理
- Chapter 3 の `DiagnosticDomain` は 12 項目 + `Other(Str)` を定義し、`Effect` / `Target` / `Plugin` などに対応する `extensions`・監査キーを必須としている（`docs/spec/3-6-core-diagnostics-audit.md:178-191,324-343,905-999`）。  
- `Effect` ドメインは Stage/Capability 監査と直結し、`extensions["effects"].stage.*` と `AuditEnvelope.metadata["effect.stage.required"]` 等を出力する契約がある（`docs/spec/3-8-core-runtime-capability.md:132-285`）。  
- `Plugin` / `Target` / `Regex` / `Template` などは各章で専用メタデータを定義しており、RunConfig/lex シム計画（PARSER-002 / LEXER-002 / EFFECT-003）で共有するキーを統一する必要がある（`docs/spec/4-7-core-parse-plugin.md:120-188`、`docs/spec/2-2-core-combinator.md:274`、`docs/spec/2-6-execution-strategy.md:259`、`docs/spec/3-3-core-text-unicode.md:93,428`）。

### 3. TODO / 引き継ぎ
1. `Diagnostic.error_domain` を仕様語彙へ拡張し、`Other of string` を含む OCaml 列挙を定義する（DIAG-003 Step2）。  
2. JSON/LSP 変換で未知ドメインを `"other"` + `extensions["domain.other"]` に転写するフォーマットを設計し、スキーマとゴールデンを更新する。  
3. `collect-iterator-audit-metrics.py` などのメトリクスで、ドメイン列挙をテーブル駆動に置き換える。`diagnostics.domain_coverage`（新規）を導入し、RunConfig/lex シムと同期して監査網羅率を測定する。  
4. `docs/spec/0-2-glossary.md` へ OCaml 実装の反映予定と用語整備方針を追記し、Phase 2-7 `diagnostic-domain` タスクに残課題（`Other(Str)` の許容範囲など）を共有する。

## DIAG-003 Step3 シリアライズ整備（2025-11-27）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md`](./2-5-proposals/DIAG-003-proposal.md#step3-%E3%82%B7%E3%83%AA%E3%82%A2%E3%83%A9%E3%82%A4%E3%82%BAcli-lsp-%E5%87%BA%E5%8A%9B%E3%81%A8%E3%82%B9%E3%82%AD%E3%83%BC%E3%83%9E%E6%9B%B4%E6%96%B0week31-day3-4)

### 1. 作業サマリ
- `compiler/ocaml/src/diagnostic_serialization.ml` にドメイン正規化ヘルパを追加し、`Other` ドメインは `"other"` を出力しつつ `extensions["domain.other"]` に元の識別子を保持するように変更。未知ドメインは自動で `Other` へ退避できるよう `domain_of_json` を拡張。  
- CLI 経路では `compiler/ocaml/tests/test_cli_diagnostics.ml` に `domain = "type"` のアサーションと `Other` ドメインのシリアライズ検証を追加し、`Diagnostic.Extensions` に `remove` API を導入して余分な `domain.other` を除去。  
- `tooling/json-schema/diagnostic-v2.schema.json` にドメイン列挙を定義し、新規ゴールデン `compiler/ocaml/tests/golden/diagnostics/domain/multi-domain.json.golden` を追加して Plugin/Lsp/Other ケースを `scripts/validate-diagnostic-json.sh` の既定ターゲットへ組み込み。既存 Effect 系ゴールデンは `domain = "type"` に更新。

### 2. 検証
- `test_cli_diagnostics.ml` のローカルテストを追加したため、`dune runtest compiler/ocaml/tests/test_cli_diagnostics.ml` の実行を推奨（本作業では未実行）。  
- ゴールデン更新後に `scripts/validate-diagnostic-json.sh compiler/ocaml/tests/golden/diagnostics` を再走させ、Plugin/Lsp/Other サンプルがスキーマを通過することを確認予定。

### 3. TODO / 引き継ぎ
- `tooling/ci/collect-iterator-audit-metrics.py` へ `diagnostics.domain_coverage` 指標を導入し、Plugin/Lsp/Other を含む語彙が CI で監視されるようにする（Phase 2-7 へ移管）。  
- `Domain` 列挙の `Syntax` / `Manifest` / `Regex` / `Template` 追加と用語集更新は Step4 以降で実施。  
- `docs/spec/0-2-glossary.md` と `docs/spec/3-6-core-diagnostics-audit.md` に `domain.other` 正規化の脚注を追加するフォローアップを 2-7 `diagnostic-domain` タスクへ登録。

## DIAG-003 Step4 監査メタデータ整合（2025-11-28）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md`](./2-5-proposals/DIAG-003-proposal.md#4-%E7%9B%A3%E6%9F%BB%E3%83%A1%E3%82%BF%E3%83%87%E3%83%BC%E3%82%BF%E3%81%A8%E3%83%A1%E3%83%88%E3%83%AA%E3%82%AF%E3%82%B9%E6%95%B4%E5%90%88week31-day4-5)

### 1. 作業サマリ
- `compiler/ocaml/src/diagnostic.ml` に `extensions["capability"]` / `extensions["plugin"]` / `extensions["lsp"]` を追加し、`event.domain`・`event.kind`・`capability.ids`・`plugin.bundle_id` を `audit_metadata` と `AuditEnvelope.metadata` の両方へ自動転写。`with_effect_stage_extension` は `capability.primary` と ID 一覧を一貫して生成するよう改修。  
- `Diagnostic.Builder.build` で `event.*` を必須化し、`test_cli_diagnostics.ml` に Stage 診断・Plugin 診断の検証テストを追加。`test_type_inference.ml` では `TraitConstraintFailure` の診断オブジェクトから Capability 情報が伝播することを確認するユニットテストを新設。  
- `tooling/ci/collect-iterator-audit-metrics.py` に `diagnostics.domain_coverage` / `diagnostics.effect_stage_consistency` / `diagnostics.plugin_bundle_ratio` を実装し、`iterator.stage.audit_pass_rate` の `related_metrics` として出力。監査 CI で `--require-success` を指定すると新指標が失敗要因として扱われる。

### 2. 検証
- 追加したテストで `Yojson.Basic` による JSON 検証を実施（`dune runtest` は未実行、CI での追跡を前提）。  
- `compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden` / `compiler/ocaml/tests/golden/diagnostics/domain/multi-domain.json.golden` を手動更新し、`event.*`・`capability.ids`・`plugin.bundle_id` フィールドが出力されることをレビュー。

### 3. TODO / 引き継ぎ
- EFFECT-003 完了後に複数 Capability を含むスナップショットを追加し、`diagnostics.effect_stage_consistency` の配列比較ロジックを実データで再確認する。  
- Step5 で `docs/spec/3-6-core-diagnostics-audit.md` / `docs/guides/runtime-bridges.md` 等の脚注更新を実施し、OCaml 実装への反映日と依存関係を明記する。

## DIAG-003 Step5 ドキュメント・脚注更新（2025-11-30）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md`](./2-5-proposals/DIAG-003-proposal.md#5-%E3%83%89%E3%82%AD%E3%83%A5%E3%83%A1%E3%83%B3%E3%83%88%E3%83%BB%E8%84%9A%E6%B3%A8%E3%83%BB%E3%83%8F%E3%83%B3%E3%83%89%E3%82%AA%E3%83%95%E6%9B%B4%E6%96%B0week32-day1)

### 1. 作業サマリ
- `docs/spec/3-6-core-diagnostics-audit.md:42` に `Diagnostic.domain` の語彙拡張と運用メモを追加し、Phase 2-5 DIAG-003 の脚注 `[^diag003-phase25-domain]` を掲載。  
- `docs/spec/0-2-glossary.md:68` で `DiagnosticDomain` の語彙一覧と Step5 脚注を追加し、用語集から新ドメインを参照できるようにした。  
- `docs/spec/3-8-core-runtime-capability.md:254` / `docs/guides/runtime-bridges.md:9` / `docs/notes/dsl-plugin-roadmap.md:121` に `Target` / `Plugin` / `Lsp` ドメインの整合メモと脚注を挿入し、RunConfig/Capability/Plugin 計画への依存関係を明文化。  
- `docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md` の Step5 に結果要約を追加し、計画内で完了日時と更新対象を明示。

### 2. 検証と共有
- 脚注内で参照するドキュメント同士（`docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/guides/runtime-bridges.md`, `docs/notes/dsl-plugin-roadmap.md`）のリンクが成立していることを相互チェック。  
- 新設した脚注 ID (`diag003-phase25-*`) が対象ファイル群に限定されていることを手動確認し、重複や表記ゆれが無いことをレビューで共有。

### 3. フォローアップ
- CI 監査ダッシュボードで `diagnostics.domain_coverage` など新指標を表示する改修を `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ登録済み。  
- EFFECT-003 で複数 Capability を扱うサンプルが揃った段階で脚注の追記・削除判断を行い、`docs/plans/bootstrap-roadmap/2-5-proposals/DIAG-003-proposal.md` 残課題欄を更新する。

## TYPE-001 Day1 値制限棚卸し（2025-10-31）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md`](./2-5-proposals/TYPE-001-proposal.md)

### 1. 調査サマリ
- `generalize` が環境自由変数との差分を量化するだけで（`compiler/ocaml/src/type_inference.ml:596-606`）、効果情報や束縛種別を参照しないことを確認。`infer_decl` では `LetDecl` と `VarDecl` の双方が同じ `generalize` を呼び出し、`var` 束縛でも常に多相スキームを生成していた（`compiler/ocaml/src/type_inference.ml:2394-2471`）。  
- 効果解析は `Type_inference_effect.collect_expr` の結果を束縛評価へ伝搬しておらず、`Type_inference.make_config` からも値制限スイッチが供給されていないため、RunConfig 経由で挙動を切り替える術が現状存在しない。  
- `Type_error` 側の診断にも値制限違反に対応するケースがなく、違反時に期待する `effects.contract.value_restriction` 系のキーが未定義であることを確認した。

### 2. 仕様との差分要約
- 仕様では `docs/spec/1-2-types-Inference.md:129` で「一般化は確定的な値のみ」と明記し、効果タグとの連携は `docs/spec/1-3-effects-safety.md:25-79` の `mut` / `io` / `ffi` / `unsafe` / `panic` を起点に Stage 要件を評価する設計。現行実装は値種別・効果集合を無視しており、仕様上の値制限と Capability 監査の双方に乖離がある。  
- `RunConfig.extensions["effects"]` や CLI オプションで値制限モードを制御する計画は未反映であり、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` の差分リストに記載された「移行期間中の旧挙動スイッチ」も実装待ちである。

### 3. 再現ログ
- 以下のソースは `mut` 効果を持つ `var` 束縛を多相に利用しており、値制限が無効化されていることを再現できる。

```reml
fn main() -> i64 !{ mut } = {
  var poly = |x| x;
  let int_value: i64 = poly(42);
  if poly(true) then 1 else int_value
}
```

- `dune exec remlc -- tmp/value_restriction_var.reml --emit-tast` を実行すると型エラーは発生せず、`=== Typed AST ===` の出力のみで終了する（期待挙動は `poly(true)` 時点で多相化拒否による型エラー）。再現コマンドと診断ログは CLI 監査に `event.kind = "effects.contract.residual_leak"` 以外の値制限エラーが出ていないことを示す[^type001-step0-repro-log]。

### 4. テスト棚卸し
- `compiler/ocaml/tests/test_type_inference.ml:792-832` では `fn identity` の多相化を確認しているが、`var` 束縛や効果付き束縛の単相性を検証するテストは存在しない。  
- `compiler/ocaml/tests/test_cli_diagnostics.ml` は CLI フォーマッタの整形検証のみで、値制限違反や効果タグ漏れを再現するフィクスチャが不足している。ゴールデンにも値制限のエラーログが含まれていない。

### 5. TODO / 引き継ぎ
1. `Effect_analysis.collect_expr` の結果から `mut` / `io` / `ffi` / `unsafe` / `panic` を束縛推論へ伝搬するフックを設計し、`is_generalizable` 判定の素材を揃える（Step1）。  
2. `Type_inference.make_config` と `parser_run_config.ml` に値制限モードを追加し、RunConfig 経由で旧挙動を切り替えられる API モデルを `TYPE-001` 計画へ追記する。  
3. 値制限違反用の診断コード（仮称 `effects.contract.value_restriction`）を `type_error.ml` に定義し、CLI/LSP/監査で検証できるゴールデンとメトリクスを整備する。

### 6. 実施記録
- `dune exec remlc -- tmp/value_restriction_var.reml --emit-tast`（`fn main() -> i64 !{ mut } = { ... }`）。CLI 監査ログは `cli.audit_id = "cli/20251031T065614Z-45f5ada#0"` を記録し、Stage 検証は `mut` 残余効果の漏れのみを報告。値制限由来のエラーは発生しなかった。  
- `nl -ba compiler/ocaml/src/type_inference.ml | sed -n '596,664p'` と `sed -n '2360,2472p'` を取得し、レビュー時の参照用として行番号付きの一般化経路を確認した。

## TYPE-001 Step1 値制限判定ユーティリティ設計（2025-11-01）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md`](./2-5-proposals/TYPE-001-proposal.md#step1-値制限判定ユーティリティ設計week32-day2)

### 1. 調査サマリ
- `Typed_ast` の `typed_expr_kind` を列挙し、即値（`TLiteral`/`TLambda`/識別子参照）、構造値（`TUnary`/`TBinary`/`TTupleAccess` 等）、制御式（`TIf`/`TMatch`/`TBlock`）、副作用確定構文（`TCall`/`TAssign`/ループ・`TReturn`/`TUnsafe`）の4分類へ整理。`docs/spec/1-2-types-Inference.md` §C.3 と `docs/spec/1-5-formal-grammar-bnf.md` §4 を突き合わせ、仕様が定義する「確定的な値」を網羅していることを確認した。  
- `Effect_analysis.collect_expr` が返すタグは `Effect_profile.tag = {effect_name; effect_span}` のみで、Capability や Stage 情報は保持していない。RunConfig 連携のためにはタグと Capability/Stage を結合するデータ構造が別途必要であることを確認。  
- `Type_inference_effect.resolve_function_profile` の `resolved_capabilities` を利用することで、複数 Capability を同時に解析しても Stage 判定を失わないことを確認。`stage_trace` への `typer` 追記も Value Restriction 判定の根拠として再利用できる。

### 2. 設計方針
- `Typed_ast` に `module Value_form`（`is_immediate` / `is_aggregate` / `is_control_flow`）を追加し、値形状判定を一元化する。`infer_decl` 側では新ヘルパを呼び出すことで構文分岐の重複を排除する。  
- `type effect_evidence = { tag : string; span : Ast.span; capability : string option; stage : Effect_profile.stage_id option }` を導入し、`Effect_analysis.collect_expr` の結果に Capability/Stage 情報を付与する `Value_restriction.collect_effects` を実装予定とした。  
- `Value_restriction.evaluate : Type_inference_effect.runtime_stage -> typed_expr -> decision` を新設し、`decision` には `status`（`Generalizable` or `Monomorphic`）・`value_kind`（`Immediate`/`Aggregate`/`Alias`）・`effects`・`syntax_reasons`（`NonValueNode` や `StageMismatch`）をまとめる。判定結果は診断生成・メトリクス収集・テストで共通利用する。

### 3. TODO / 引き継ぎ
1. Step2 で `infer_decl` から `Value_restriction.evaluate` を呼び出し、`LetDecl`/`VarDecl` の一般化条件を統一する。その際 `Type_inference.make_config` に `value_restriction_mode`（`Strict`/`Legacy`）を追加する。  
2. `Effect_analysis.add_call_effect_tags` に Capability 名を解決するテーブルを導入し、`core.io.*` → `io` Capability などのマッピングを `effect_evidence.capability` へ格納する。  
3. Step3 で `collect-iterator-audit-metrics.py` が `effect_evidence` を JSON へ直列化できるよう、`Value_restriction` から共有するヘルパを実装する。

## TYPE-001 Step2 Typer 連携と RunConfig 導入（2025-11-03）

関連計画: [`docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md`](./2-5-proposals/TYPE-001-proposal.md#step2-実施記録2025-11-03)

### 1. 作業サマリ
- `Value_restriction.evaluate` の判定結果（`Generalizable` / `Monomorphic`）を `infer_decl` の `LetDecl` / `VarDecl` 分岐へ組み込む設計を固め、`let` はモード依存、`var` は常に単相化させるフローを確定。  
- Typer 設定に `value_restriction_mode`（`Strict` / `Legacy`）を追加し、RunConfig の Effects 拡張から CLI → Typer へモードを伝播させる API モデルを整理。  
- 効果タグ・Capability・Stage を束ねた `effect_evidence` を判定結果と一緒に返し、診断とメトリクスの共通入力にする方針を決定。

### 2. 設計決定
- `compiler/ocaml/src/type_inference.ml:2353` 付近の `generalize` 呼び出しを `Value_restriction.should_generalize`（仮称）でラップし、`scheme_to_constrained (mono_scheme ty)` を通じて単相スキームへ切り替えるロジックを追加する。  
- `compiler/ocaml/src/type_inference.ml:20-40` に `value_restriction_mode` を保持する `config` レコードを拡張し、`Type_inference.make_config` が `RunConfig` から受け取ったモードを保持できるようにする。  
- `compiler/ocaml/src/parser_run_config.ml:319-428` の Effects モジュールへ `value_restriction` キーを追加し、`strict|legacy` を正規化／設定するアクセサ（`set_value_restriction` / `value_restriction_mode`）を新設。  
- `compiler/ocaml/src/main.ml:600-642` で RunConfig を構築した後に値制限モードを抽出し、Typer 設定へ渡す処理を追加。  
- `tooling/ci/collect-iterator-audit-metrics.py:1-154` と同じフィールド名で `effect_evidence` を JSON 化する計画をまとめ、Stage 監査指標と値制限メトリクスの整合を担保。

### 3. TODO / 引き継ぎ
1. Step3 で `compiler/ocaml/tests/test_type_inference.ml` に `strict` / `legacy` 両モードのゴールデンを追加し、`Value_form` 判定と効果タグ伝搬を検証する。  
2. `Value_restriction.effect_evidence` をシリアライザへ接続し、`type_inference.value_restriction_violation` / `type_inference.value_restriction_legacy_usage` 指標を `collect-iterator-audit-metrics.py` へ追加する。  
3. ドキュメント更新（`docs/spec/1-2-types-Inference.md`・`docs/spec/2-1-parser-type.md` 等）で `value_restriction_mode` の切替手順と Legacy モードの利用条件を明文化する。

### 4. 参照ログ
- `nl -ba compiler/ocaml/src/type_inference.ml | sed -n '20,48p'` で `config` レコードの拡張箇所を確認。  
- `nl -ba compiler/ocaml/src/main.ml | sed -n '600,642p'` を取得し、RunConfig から Typer への伝播経路をレビュー記録へ添付。  
- `docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-001-proposal.md` の Step2 セクションへ設計詳細と今後のテスト計画を反映済み。

[^type001-step0-repro-log]: CLI 出力は `=== Typed AST ===` のみで終了し、`effects.contract.residual_leak` 以外の診断が発生しない。仕様上は `poly(true)` で型が `Bool` に固定されるため、`var poly` の一般化が抑制されていれば `let int_value: i64 = poly(42);` と矛盾しコンパイルが失敗する。
