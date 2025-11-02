# Phase 2-5 修正計画カタログ

このディレクトリは Phase 2-5「仕様差分是正」（`../2-5-spec-drift-remediation.md`）で扱う修正計画の置き場です。計画を参照・更新する際は以下の方針を守ってください。

- **前提資料の確認**: `../../spec/0-1-project-purpose.md` と `../2-0-phase2-stabilization.md` を参照し、優先度と成果物の期待値を再確認する。
- **差分管理**: 各計画の実装状況や脚注追加・更新時には関連仕様（`docs/spec/`）と `README.md`（リポジトリ索引）を同時に更新する。
- **記録保持**: 重要な判断・保留事項は計画内の「残課題」または `docs/notes/` 配下の関連ノートへ記録し、追跡可能な状態を維持する。

## 目次とハイライト

### 診断ドメイン（DIAG）
- [DIAG-001 修正計画](./DIAG-001-proposal.md): `Severity = {Error, Warning, Info, Hint}` を導入して Chapter 3（`docs/spec/3-6-core-diagnostics-audit.md`）との整合を回復。（2025-10-27 更新: OCaml 実装の列挙型・シリアライズ・CLI カラーを Info/Hint 対応に改修済み。2025-11-08 追記: JSON スキーマと CLI ゴールデン/テストに Info/Hint ケースを追加し、`validate-diagnostic-json.sh` で新フィクスチャの検証まで完了。2025-11-09 追記: LSP 互換テストと `diagnostic.info_hint_ratio` 指標を追加し、CLI/LSP/監査パイプラインの整合チェックを完了。2025-11-10 追記: 仕様書と測定ガイドへ Severity 4 値化の脚注・指標（`diagnostic.info_hint_ratio`／`diagnostic.hint_surface_area`）を反映し、レビュー記録へ完了メモを追加。）
- [DIAG-002 修正計画](./DIAG-002-proposal.md): `Diagnostic.audit` と `timestamp` を必須化し、Builder/Legacy で `cli.audit_id` / `cli.change_set` を `phase2.5.audit.v1` テンプレート（`audit_id = "cli/" ^ build_id ^ "#" ^ sequence` など）として自動補完、シリアライズで欠落検知を行いつつ `collect-iterator-audit-metrics.py --require-success` が全指標 1.0 で完走する状態まで復旧した（詳細は [`../2-5-review-log.md`](../2-5-review-log.md) を参照）。
- [DIAG-003 修正計画](./DIAG-003-proposal.md): `DiagnosticDomain` を効果・プラグイン・LSP など仕様準拠の語彙へ拡張し、監査ログ分析を改善。2025-11-27 更新: シリアライズ/スキーマ/ゴールデン整備（`domain.other` 拡張、Plugin/Lsp/Other サンプル）が完了し、残課題は CI 指標の拡充へ引き継ぎ。2025-11-28 追記: `event.domain` / `event.kind` / `capability.ids` / `plugin.bundle_id` の監査整合と CI 指標（`diagnostics.domain_coverage`, `diagnostics.effect_stage_consistency`, `diagnostics.plugin_bundle_ratio`）を実装。2025-11-30 更新: Step5 で仕様・ガイド・ノートへ脚注を追加し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` にダッシュボード改修 TODO を登録済み。

### 効果システム（EFFECT）
- [EFFECT-001 修正計画](./EFFECT-001-proposal.md): `mut`/`io`/`ffi`/`unsafe` などのタグ検出を強化し、残余効果診断を Chapter 1-3 と一致させる。
- [EFFECT-002 修正計画](./EFFECT-002-proposal.md): `perform`/`handle` を含む効果操作 PoC の方針を明確化し、`Σ_before`/`Σ_after` の検証を可能にする。2026-04-18 追記: Step4 で `extensions.effects.sigma.*` / `audit.metadata["effect.sigma.*"]` の出力設計と `syntax.effect_construct_acceptance`・`effects.syntax_poison_rate` の算出手順を確定し、CI／ゴールデン／監査ログへ共通 KPI を引き継ぐ準備を整えた。2026-04-20 追記: Step5 で Chapter 1/索引へ PoC 脚注と KPI 参照を反映し、`docs/notes/effect-system-tracking.md`・`0-4-risk-handling.md`・`2-5-spec-drift-remediation.md` を更新して Phase 2-7 への移行条件を明示した。
- [EFFECT-003 修正計画](./EFFECT-003-proposal.md): 複数 Capability を解析・監査へ出力する仕組みを整備し、Stage 契約（`docs/spec/3-8-core-runtime-capability.md`）との齟齬を是正。

### 型システム（TYPE）
- [TYPE-002 効果行統合ポリシー計画](./TYPE-002-proposal.md): Step2 で `effect_row` 統合ドラフトを策定し、`TArrow of ty * effect_row * ty` を前提とした型・診断・IR の影響調査を設計ノートに集約[^type-002-row-design]（2026-04-18 更新: 暫定採用案と Phase 2-7 への宿題を登録。2026-04-22 追記: Step3 で仕様脚注と `type_row_mode` ガードを整備し、`effects.type_row.integration_blocked` 診断と監査キー `effect.type_row.*` を定義）。

### エラー回復（ERR）
- [ERR-001 修正計画](./ERR-001-proposal.md): Menhir の期待集合を `ExpectationSummary` に反映させ、`docs/spec/2-5-error.md` で定義された期待値提示を実現。（2025-11-15 追記: `collect` の導入と `parser_driver`/`parser_diag_state` の組込みにより、期待集合が `Diagnostic.expected` と legacy API 双方へ伝播することを確認。）
- 2025-11-16 追記: CLI ゴールデンと LSP フィクスチャを期待集合付きで更新し、`scripts/validate-diagnostic-json.sh`／`tooling/ci/collect-iterator-audit-metrics.py` に `parser.expected_*` 指標を追加して CI 監視を有効化。
- 2025-11-17 追記: `docs/spec/2-5-error.md`・`docs/spec/3-6-core-diagnostics-audit.md`・ガイド類へ Phase 2-5 完了脚注と運用ガイドを反映し、`docs/notes/spec-integrity-audit-checklist.md` で監査 TODO を共有する S5（ドキュメントと共有タスク）を完了。
- [ERR-002 修正計画](./ERR-002-proposal.md): `Parse.recover` の同期トークンと FixIt を導入し、CLI/LSP での自動修正と診断補助を整備。（2025-12-06 追記: Step1 で `pending_recovery` スナップショットと `extensions["recover"]` の設計を確定し、同期トークン抽出フックを `parser_driver` に配置。2025-12-09 追記: Step2 で FixIt 生成・notes 拡張のテンプレートを定義し、`has_fixits` を含む `recover` 拡張 JSON 形を確定。テストケースとメトリクスの更新計画を Step3 へ引き継ぎ。2025-12-12 追記: Step3 で CLI/LSP ゴールデン・`parser_recover_tests.ml`・`streaming_runner_tests.ml` を更新し、`parser.recover_fixit_coverage` 指標を `scripts/validate-diagnostic-json.sh`／`collect-iterator-audit-metrics.py` に統合。`reports/diagnostic-format-regression.md` へサンプル JSON を追加し、Phase 2-7 へ Packrat 経路の残課題を引き継いだ。2025-12-15 追記: Step4 で仕様脚注とレビュー共有を更新し、`docs/notes/core-parse-streaming-todo.md` および Phase 2-7 計画 §3.4 へ残課題を正式移管した。）

### 実行戦略（EXEC）
- [EXEC-001 修正計画](./EXEC-001-proposal.md): `run_stream`/`resume` を備えたストリーミング実行 PoC を構築し、`docs/spec/2-6-execution-strategy.md` の契約を検証。
  - 2026-01-12 追記: Step2 で Feeder／Continuation／DemandHint の型体系と RunConfig 変換を整理し、`core_parse_streaming_types` の骨格を確定。
  - 2026-01-16 追記: Step3 で ストリーミング制御ループ PoC（`StreamDriver` ステートマシン、`DemandHint` 再計算、`StreamMeta` 指標集計）を設計し、`Pending`/`Completed` の遷移表と計測方針を文書化。
  - 2026-01-24 追記: Step4 で CLI/LSP/CI を接続。`--streaming` 系フラグと `Parser_driver.Streaming` を実装し、`streaming_runner_tests.ml`・`streaming-outcome.json.golden` を追加。CI メトリクス（`parser.stream_extension_field_coverage`）と `stream_meta` 検証を有効化。
  - 2026-01-26 追記: Step5 で仕様・ガイド・計画書へ PoC 状態と既知制限を脚注化し、`parser.stream.outcome_consistency` / `parser.stream.demandhint_coverage` を `0-3-audit-and-metrics.md` に登録。Phase 2-7 へ引き継ぐ Packrat 共有・バックプレッシャ自動化・監査ログ拡張の TODO を `2-7-deferred-remediation.md` / `runtime-bridges.md` に反映。

### 字句解析（LEXER）
- [LEXER-001 修正計画](./LEXER-001-proposal.md): Unicode 識別子プロファイル導入までの暫定対応を明文化し、DSL/プラグイン計画と共有。（2025-12-12 追記: Step1 ASCII 実装棚卸しで再現フィクスチャとテストを追加し、現状のエラーメッセージをレビュー記録へ保存。Phase 2-7 での脚注整備に向けた先行調査を完了。2026-02-18 追記: Step2 で仕様脚注・索引・用語集・メトリクスを更新し、`lexer.identifier_profile_unicode` 指標とレビュー記録を整備。2026-03-04 追記: Step3 で DSL/プラグインチーム向け ASCII 限定運用と Phase 2-7 フォローアップ TODO を `docs/notes/dsl-plugin-roadmap.md` §7 に集約し、ガイド類へ周知フローを追加。2026-03-21 追記: Step4 で CI 指標と `--lex-profile=ascii|unicode` スイッチ設計を確定し、`collect-iterator-audit-metrics.py` のプレースホルダ集計とテスト切替手順を定義。）
- [LEXER-002 修正計画](./LEXER-002-proposal.md): `Core.Parse.Lex` ユーティリティを抽出し、字句設定 (`RunConfig.extensions["lex"]`) を仕様準拠に整備。

### 構文解析（PARSER）
- [PARSER-001 修正計画](./PARSER-001-proposal.md): `ParseResult` シムを導入し、`Reply{consumed, committed}` と診断集約を再現。Week31 Day1-5 で `parser_driver` を段階的に差し替え、`parser.parse_result_consistency` / `parser.farthest_error_offset` を `0-3-audit-and-metrics.md` に登録して CI 監視する（実装済: `parser_driver.ml` シム化・`parser_diag_state.ml` 追加・`dune runtest tests` 成功・メトリクス/脚注/`scripts/validate-diagnostic-json.sh` の自動検証まで反映完了）。
- [PARSER-002 修正計画](./PARSER-002-proposal.md): `RunConfig` をランナーへ統合し、Packrat／recover／stream 設定を反映できるようにする。（2025-11-18 追記: Step 1 で `parser_run_config` モジュールを実装し、仕様書と修正計画への移行脚注を整備。2025-11-19 追記: Step 2 で `parser_driver` が `Run_config.t` を受け取り、`trace`/`merge_warnings`/`locale` を診断状態へ伝播させる更新を完了。2025-11-20 追記: Step 3 で `RunConfig` 拡張シム（`lex`/`recover`/`stream`）と `Parser_diag_state` 連携を実装し、`dune build` で検証済み。2025-11-21 追記: Step 4 で CLI/LSP/テスト支援を RunConfig 経由へ統合し、`Cli.Options.to_run_config`・`tooling/lsp/run_config_loader.ml`・`Test_support` を追加してクライアントと測定基盤の導線を揃えた。2025-11-22 追記: Step 5 で RunConfig ユニットテスト・監査メトリクス（`parser.runconfig_switch_coverage` / `parser.runconfig_extension_pass_rate`）・RunConfig ゴールデン JSON を整備し、`collect-iterator-audit-metrics.py` が CI で新指標を強制するよう更新済み。）
- [PARSER-003 修正計画](./PARSER-003-proposal.md): 15 個のコアコンビネーターを OCaml 実装へ抽出し、`Core.Parse` API と DSL の互換性を確保。（2025-11-01 Step1 コアコンビネーター棚卸し完了: Menhir 対応表を `docs/notes/core-parser-migration.md` に追加し、`committed` 未更新・`ParserId` 未割当・`recover` フック未使用をレビュー記録へ登録。2025-12-04 Step2 Core_parse シグネチャ案を確定し、`docs/notes/core-parse-api-evolution.md` に `Id`/`State`/`Reply`/`Registry` 構成と静的/動的 `ParserId` 割当方針を記録。2025-12-05 Step3 で `core_parse.{ml,mli}` を追加し、`parser_driver` をコアコンビネーター層経由に切り替える PoC を実装。2025-12-12 Step4 では Packrat キャッシュ・`recover` 同期トークン・複数 Capability 監査の統合設計を整理し、キャッシュキー・RunConfig 連携・CI 指標追加まで準備済み（実装は Step5 へ引き継ぎ）。2025-12-18 Step5 で Packrat 指標と `parser.core.rule.*` メタデータを CI へ導入し、2025-12-24 Step6 では仕様・ガイド・索引を更新して `Core_parse` PoC の進捗と引き継ぎ先 TODO（テレメトリ統合・Menhir 置換判断）を共有。）

### 構文仕様（SYNTAX）
- [SYNTAX-001 修正計画](./SYNTAX-001-proposal.md): Unicode 識別子制約の暫定状態を仕様脚注で明示し、Phase 2-7 の対応計画を共有。（2025-11-02 追記: Step1 で Chapter 1/1-5 の差分棚卸・レビュー記録・ASCII 拒否テスト固定化を完了。2026-02-24 追記: Step2 で BNF/用語集/索引へ ASCII 暫定脚注を波及し、`lexer.identifier_profile_unicode` 指標と Phase 2-7 `lexer-unicode` タスクへの橋渡しを実施。同日 Step3 で Unicode 受理テスト雛形とフィクスチャを追加し、`REML_ENABLE_UNICODE_TESTS` で実行切替できるよう準備。2026-03-31 追記: Step4 でメトリクス（`lexer.identifier_profile_unicode = 0.0`）、リスク登録、Phase 2-7 ロードマップ連携を完了）
- [SYNTAX-002 修正計画](./SYNTAX-002-proposal.md): `use` 文の多段ネストを AST に反映し、Chapter 1 のサンプル通過を保証。
  - 2025-10-27 追記: S2（AST/型付き AST 整合確認）まで完了。Typer 側は `tcu_use_decls` をそのまま保持できることを検証済み。
  - 2025-10-28 追記: S3（Menhir ルール実装）を完了し、`use_item` の再帰構築と `menhir --list-errors` の検証まで実施。
  - 2025-10-29 追記: S4（束縛・診断連携）で `Module_env.flatten_use_decls` を実装し、`tcu_use_bindings` とユニットテストを追加。診断期待集合への影響が無いことを確認済み。
  - 2025-11-12 追記: S5（検証とドキュメント更新）でユニットテスト・メトリクス・仕様脚注を更新し、`parser.use_nested_support` の監視体制を整備。
- [SYNTAX-003 修正計画](./SYNTAX-003-proposal.md): 効果構文（`perform`/`handle`）の実装ステージを明確化し、Formal BNF との乖離を是正。（2026-03-27 追記: S3 で診断・CI 計測の計画を策定し、`syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` 指標の準備とゴールデン更新手順を整備。`0-3-audit-and-metrics.md`・`reports/diagnostic-format-regression.md`・`docs/notes/effect-system-tracking.md` に同期済み。2026-04-03 追記: S4 で Phase 2-7 向け引き継ぎチェックリストとフラグ運用メモを整備し、`2-7-deferred-remediation.md`・`effect-system-tracking.md` に移行タスクを同期。）

### 型システム（TYPE）
- [TYPE-001 修正計画](./TYPE-001-proposal.md): 値制限と効果タグ連携を復元し、副作用を持つ束縛の多相化を防止。（2025-10-31 Step0 棚卸し完了: 再現ログ記録とチェックリスト共有済み。2025-11-01 Step1 判定ユーティリティ設計完了: `Typed_ast` 値形状分類と `Value_restriction.evaluate` API 案を確定。2025-11-03 Step2 Typer/RunConfig 連携方針確定: `value_restriction_mode` と効果証跡共有モデルを整理。2025-11-05 Step3 テスト雛形・診断テンプレート・CI メトリクス設計を追加し、Strict/Legacy の監視ケースを定義。2025-11-08 Step4 仕様・RunConfig ドキュメント整備完了: 1-2/1-3/2-1/2-6 へ脚注を追加し、Phase 2-7 へのフォローアップを登録。）
- [TYPE-002 修正計画](./TYPE-002-proposal.md): 効果行を型表現へ統合するロードマップを策定し、型と効果の一体管理を再構築。（2026-04-18 更新: Step2 で `effect_row` 統合ドラフトとデータ構造比較を確定。2026-04-22 追記: Step3 で脚注 `[^type-row-metadata-phase25]` と `RunConfig.extensions["effects"].type_row_mode` ガードを整備。2026-04-24 追記: Step4 で Phase 2-7 の 3 スプリント実装計画・`metadata-only → dual-write → ty-integrated` 移行手順・新規 KPI (`diagnostics.effect_row_stage_consistency` / `type_effect_row_equivalence` / `effect_row_guard_regressions`) とテスト観点を確定し、`2-7-deferred-remediation.md`・`0-3-audit-and-metrics.md`・`effect-system-tracking.md`・`2-5-review-log.md` を同期。)
- [TYPE-003 修正計画](./TYPE-003-proposal.md): 型クラス辞書渡しを Core IR へ復元し、監査ログへの Capability 情報出力を再開。（2025-10-30 更新: Typer／Core IR／CI メトリクス整備まで完了。2025-10-31 追記: Stage 逆引き・辞書付き診断ゴールデン・ドキュメント整備まで完了。）

## 着手順序ガイド
| 時期と順序 | 対象計画 | 目的と前提関係 |
|------------|----------|----------------|
| Phase 2-5 開始直後（Week31 前半） | PARSER-001, TYPE-003, DIAG-002 | パーサ基盤・型クラス監査・監査ログ必須化を最初に整備し、以降の差分検証を可能にする |
| Phase 2-5 前半（Week31 後半〜Week32） | EFFECT-001, DIAG-001, SYNTAX-002, ERR-001 | 効果タグと Severity を拡張し、`use` ネスト・期待集合のギャップを早期に解消する |
| Phase 2-5 中盤（Week32〜Week33） | PARSER-002, LEXER-002, DIAG-003, EFFECT-003, TYPE-001 | RunConfig/lex シムと複数 Capability を整備し、値制限復元を可能にする |
| Phase 2-5 後半（Week33〜Week34） | PARSER-003, EXEC-001, ERR-002 | コアコンビネーター抽出後にストリーミング PoC と FixIt 拡張を実装し、ランナー整合を仕上げる |
| Phase 2-5 クロージング〜Phase 2-7 準備 | LEXER-001, SYNTAX-001, SYNTAX-003, EFFECT-002, TYPE-002 | Unicode・効果構文・効果行は脚注整備とロードマップ策定を Phase 2-5 で行い、Phase 2-7 以降で本実装する |

## 運用メモ
- 新しい計画を追加する際は、ドメイン別セクションに箇条書きを追加し、関連仕様とメトリクスを併記する。
- 計画のステータス更新（完了・棚上げ等）は本文と併せてここにも反映し、Phase 2-5 全体の進捗を一目で把握できるようにする。
- 大幅な構造更新やファイル移動を行った場合は `docs-migrations.log` と `README.md`（リポジトリ索引）を忘れずに追記する。

[^type-002-row-design]: `compiler/ocaml/docs/effect-system-design-note.md`「## 3. 型表現統合ドラフト（TYPE-002 Step2, 2026-04-18）」、`docs/spec/1-2-types-Inference.md` / `1-3-effects-safety.md` / `3-6-core-diagnostics-audit.md` に追加した脚注 `[^type-row-metadata-phase25]`（TYPE-002 Step3, 2026-04-22）、および `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#type-002-effect-row-integration`・`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`（TYPE-002 Step4, 2026-04-24）を参照。
