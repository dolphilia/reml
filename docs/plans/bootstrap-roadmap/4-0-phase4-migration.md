# 4.0 Phase 4 — 実用テスト統合計画

Phase 4 は Rust 実装コンパイラを "実務レベル" で検証することに特化した工程であり、`.reml` ファイルを実際に読み込み、コンパイルし、実行する統合テストを整備する。Phase 3 で整った標準ライブラリと Capability を活かし、Phase 5（セルフホスト）および Phase 6（正式リリース）以前に「現場投入できる挙動か」を確認する。

## 4.0.1 目的
- `docs/spec/0-1-project-purpose.md` が求める性能と安全性の指標を、実際の `.reml` プロジェクトで測定可能な形に落とし込む。
- Rust 実装の CLI (`reml_frontend`, `remlc`) とランタイムを用い、**コンパイル→実行→結果検証** のエンドツーエンドパイプラインを GitHub Actions / ローカル検証の両方で再現できるようにする。
- `docs/spec/1-x`（言語コア）、`docs/spec/2-x`（Parser/Runtime API）、`docs/spec/3-x`（標準ライブラリ）に掲載されているサンプル・仕様要件を `.reml` 実行テストへ落とし込み、仕様と実装の乖離を迅速に検出できる状態にする。特に Chapter 1（構文・型・効果）については `.reml` 入力ごとの Pass/Fail を細粒度に蓄積する。
- Phase 5 でセルフホストビルドへ進むための「現実的なシナリオカバレッジ」「診断/監査メトリクス」「失敗時の切り分けログ」を整備する。

## 4.0.2 スコープ境界
- **含む**: `.reml` シナリオの再整理、入力/出力/診断のゴールデン化、Rust 実装 CLI を使った自動テスト実行、CI と `collect-iterator-audit-metrics.py` の統合、`examples/`・`tests/` の再分類、`docs/spec/1-x`〜`3-x` のサンプルを `.reml` テストへ変換する作業。
- **含まない**: セルフホスト（Phase 5）に必要な自己コンパイルや Stage 昇格判定、正式リリース手続（Phase 6）。ただし、それらの前提となるシナリオ・メトリクス・レポートは Phase 4 で準備する。
- **前提条件**: Phase 3 の Chapter 3 実装完了、`docs/plans/rust-migration/overview.md` の P1〜P3 成果、`docs/guides/tooling/audit-metrics.md` で定義された KPI。

## 4.0.3 作業ディレクトリ / 主な対象
- `examples/`（`core_*` / `dsl_*` / `pipeline_*` 系、および `docs/spec/1-x`〜`3-x` のサンプルを反映した `spec_core_*` スイート）
- `tooling/examples/run_examples.sh`, `tooling/ci/collect-iterator-audit-metrics.py`
- `compiler/frontend`, `compiler/runtime`, `compiler/tests`
- `docs/spec/3-x`, `docs/guides/compiler/core-parse-streaming.md`, `docs/guides/runtime/runtime-bridges.md`
- 新設予定: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`, `reports/spec-audit/ch5/*.md`

## 4.0.4 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: シナリオマトリクス確定 | `.reml` 入力の分類（Prelude/IO/Capability/Runtime/Plugin/CLI）と評価観点 | `phase4-scenario-matrix.csv`、レビューサインオフ | Phase 4 開始後 3 週 |
| M2: 実行パイプライン稼働 | `run_examples.sh --suite practical` と `cargo test --package reml_e2e` で compile→run→inspect を自動化 | GitHub Actions サンドボックス実行、`reports/spec-audit/ch5/practical-suite-*.md` | 開始後 6 週 |
| M3: 観測メトリクス接続 | `collect-iterator-audit-metrics.py` の実用シナリオ対応、`docs/guides/tooling/audit-metrics.md` KPI 更新 | メトリクス JSON / Markdown レポート、`--require-success` 完走 | 開始後 8 週 |
| M4: Phase 5 ハンドオーバー判定 | シナリオ網羅率 ≥ 85%、回帰テスト自動化、未解決リスク整理 | `phase4-readiness.md`、レビュー記録 | 開始後 10 週 |

## 4.0.5 ワークストリーム
1. **シナリオ設計と資産棚卸し**
   - `examples/`、`tests/`、`docs/spec`（特に `1-0-language-core-overview.md`〜`1-5-formal-grammar-bnf.md`）のコード片を洗い出し、`.reml` の入力形式・依存 Capability・期待出力を `phase4-scenario-matrix.csv` に整理。各行には「対応する仕様書（例: `docs/spec/1-2-types-inference.md §3.1`）」「検証に使う `.reml` ファイル」「想定される診断/実行結果」を必ず記録する。
   - `docs/spec/1-x` の構文・型・効果サンプル、および `docs/spec/3-5-core-io-path.md` や `3-8-core-runtime-capability.md` に掲載されたサンプルを実行可能な `.reml` へ拡張し、`reports/spec-audit/ch1`〜`ch3` の資産とリンク。
   - `docs/plans/rust-migration/p1-front-end-checklists.csv`、`p1-test-migration-*.txt` で未使用のケースを Phase 4 シナリオへ転用。
   - 言語コア仕様の「許容範囲」を見極めるため、各構文規則に対して「正例（受理されるべき `.reml`）」「境界例（ギリギリ許容/ギリギリエラーの `.reml`）」「負例（明確にエラーになる `.reml`）」を 1 セットとして記録する。複数表記を許容する規則は、そのバリエーションが網羅されるまで `.reml` を追加する。

2. **実行パイプライン構築**
   - `tooling/examples/run_examples.sh` に Phase 4 専用スイート（`practical`, `integration`, `selfhost-smoke`, `spec-core`）を追加し、Rust 実装 CLI `remlc` を使った compile→run を共通 API に統一。`spec-core` スイートは `docs/spec/1-x` の節ごとに `.reml` ファイルを整備し、最低 1 ファイルをコンパイル・実行して仕様通りの動作を確認する。
   - `.github/workflows/` に「Phase 4 practical tests」ジョブを追加し、`cargo test -p reml_e2e -- --scenario practical`（仮）と `run_examples.sh --suite practical --suite spec-core --update-golden` を監査ログ付きで実行。
   - `compiler/tests/practical/` に新しい統合テスト（ファイルごと compile run）を追加し、`Result`/`Option`/`Capability` の挙動を JSON で保存。Chapter 1 の仕様を網羅するため、`tests/spec_core/` へ `.reml` テスト資産をまとめ、`language-core` サブコマンドで個別に再実行できるようにする。

3. **観測・診断メトリクス統合**
   - `collect-iterator-audit-metrics.py` に Phase 4 シナリオ用のセクション（`--section practical`）を追加し、`tooling/ci` レポートと `docs/guides/tooling/audit-metrics.md` KPI を同期。
   - `reports/spec-audit/ch5/` を新設し、`.reml` ごとの compile→run ログ、診断 JSON、監査 JSONL、性能カウンタをまとめる。
   - `.reml` 実行時の AuditEnvelope に `scenario.id`, `input.hash`, `runtime.bridge`, `spec.chapter`（例: `chapter1.syntax`）などのタグを追加し、Phase 5 以降の自己ホスト計測へ引き継ぐ。

4. **フィールドデータ/レグレッション管理**
   - `docs/notes/dsl/dsl-plugin-roadmap.md`、`docs/guides/dsl/plugin-authoring.md` で紹介している DSL / Plugin 例を Phase 4 スイートへ取り込み、`RuntimeBridge`/`Capability` チェックを伴う実行例を追加。
   - 実運用想定のワークスペース（複数 `.reml` ファイル、`@dsl_export`、`core.io` 連携）のミニプロジェクトを `examples/practical/` に作成し、`reports/spec-audit/ch5/practical-bundle-*.md` に結果を残す。Chapter 1〜3 の仕様確認用に `examples/spec_core/` を設け、`language-core-ops.reml` などの代表ケースを維持する。
   - レグレッションが発生した場合、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` と連携し、Phase 4 専用の `PRACTICAL-###` 問題として登録する手順を定義。
   - `.reml` テストの実行結果を整理するレビュー会を週次で実施し、「コンパイラ修正が必要か」「仕様を追記/明確化すべきか」を切り分ける。判定フローは `phase4-scenario-matrix.csv` の各行に `resolution` 列（`impl_fix` / `spec_fix` / `ok`）として残し、対応内容を `docs/spec/1-x` または `compiler/*` のタスクへ紐付ける。

### 4.0.5a 言語コア徹底テスト指針
- **仕様理解の徹底**: Phase 4 着手前に `docs/spec/1-0-language-core-overview.md`, `1-1-syntax.md`, `1-2-types-inference.md`, `1-3-effects-safety.md`, `1-5-formal-grammar-bnf.md` の該当章を読み込み、各規則の許容範囲・例外条件・未定義事項をリスト化する。`phase4-scenario-matrix.csv` には「仕様で許容される理由」「未定義の扱い（仕様の TODO）」を備考として記録する。
- **多表記の網羅**: 1 つの構文に複数の書き方が存在する場合（例えば `effect handler` の `match` / `with` パターン、`let` のパターン束縛など）、それぞれを `.reml` で表現したテストを追加し、仕様どおり受理されるか確認する。許容されない書き方も `.reml` 化し、診断が期待どおりか検証する。
- **境界/意地悪ケース**: 「重箱の隅をつつく」入力を体系的に用意する。例: 最大深度付近のネスト、`Unicode` のサロゲートペア境界、`effect` と `type inference` の境界値、`parser` の回復限界など。これらの `.reml` は `spec_core/boundary/*.reml` にまとめ、Pass/Fail の判断根拠を `reports/spec-audit/ch5/spec-core-dashboard.md` に記載する。
- **実行結果の分類と対処**: `.reml` 実行の結果、「実装修正が必要」と判断したものは `impl_fix` ラベルで Issue/タスク化し、「仕様の記述が不足」と判断したものは `spec_fix` ラベルで `docs/spec/1-x` の該当章へ脚注または本文追記を行う。判断理由は `phase4-scenario-matrix.csv` と `reports/spec-audit/ch5/*.md` に残し、Phase 5, 6 で参照できるようにする。

### 4.0.5b `.reml` 実行による仕様検証ガイドライン
- **.reml ベースの必須検証**: 1-x（言語コア）から 3-x（標準ライブラリ）までの各仕様は必ず `.reml` ファイルで再現し、`compile → run → diagnose` の結果をゴールデン化する。設計レビューのみでの承認は不可とし、すべて `.reml` 実行ログを `reports/spec-audit/ch5/` に添付する。
- **1-x 章の徹底テスト**: Chapter 1 の各構文規則ごとに「正例」「ギリギリ許容例」「ギリギリエラー」「明確なエラー」の 4 タイプを `.reml` で用意する。`phase4-scenario-matrix.csv` の `spec_anchor` 列には BNF 規則 ID を必ず記載し、許容範囲の根拠をコメント/備考に残す。
- **多表記・多経路の網羅**: 代替記法（`if`/`match`、`effect handler` の `with`/`match`、`let` の分配書式、Unicode リテラルのエスケープ形式など）はそれぞれ単独の `.reml` として登録し、仕様が許容すると明記している場合はすべてのパターンをテストする。1 種類でも落ちた場合は `impl_fix` または `spec_fix` を即時トリアージする。
- **意地悪/境界テスト**: 想定外コードを能動的に作成し、例: 最大ネスト、極端な Unicode、Capability Stage のギリギリ判定、`Result`/`Option` の型推論限界などを `.reml` で再現する。これらは `phase4-scenario-matrix.csv` の `priority=boundary` として明示し、監査ログにも同じタグを付与する。
- **結果の切り分け**: すべてのテスト結果に「Compiler 修正」「Specification 修正」「挙動問題なし」を割り当て、`phase4-scenario-matrix.csv` の `resolution` と `4-4-field-regression-and-readiness-plan.md` のプロセスに従って Issue/TODO を生成する。仕様追記が必要な場合は `docs/spec/1-x`〜`3-x` の該当節番号を備考に残す。

## 4.0.6 測定と検証
- **シナリオ網羅率**: `phase4-scenario-matrix.csv` に登録したカテゴリのうち、最低 85% を週次で実行（`core`, `io`, `diagnostics`, `capability`, `plugin`）。特に `spec.chapter1.*` 行は 100% 実行を必須とし、Chapter 2/3 も 90% 以上を維持する。
- **仕様準拠スコア**: `collect-iterator-audit-metrics.py --section practical` に `spec_compliance` を追加し、`docs/spec/1-x`〜`3-x` の節ごとに Pass/Fail 件数を記録。`reports/spec-audit/ch5/spec-core-dashboard.md` に集計表を掲載し、`docs/guides/tooling/audit-metrics.md` に `spec.chapter1.pass_rate`, `spec.chapter2.pass_rate`, `spec.chapter3.pass_rate` を KPI として追記する。
- **性能指標**: `.reml` 単位で `parse_throughput` / `memory_peak_ratio` を測定し、`reports/spec-audit/ch5/perf-*.md` に保存。
- **診断ギャップ**: 実行パイプラインで得た診断 JSON を `scripts/validate-diagnostic-json.sh` で検証し、差異ゼロを Phase 4 の進捗条件とする。
- **監査メトリクス**: `collect-iterator-audit-metrics.py --section practical --require-success` を CI で必須化し、観測指標（`practical.pass_rate`, `practical.stage_mismatch`）を `docs/guides/tooling/audit-metrics.md` に追記。

## 4.0.7 リスクとフォローアップ
- **シナリオ不足**: Phase 3 の章別作業で生まれたケースが不足している場合、`docs/notes/stdlib/core-library-outline.md` を再確認し、欠落分は Phase 4 で追加する。特に Chapter 1 の構文/型/効果サンプルが足りない場合は優先度を上げ、必要に応じて `docs/notes/phase4-practical-test-backlog.md`（新設）へ TODO 記録。
- **実行コストの肥大化**: `.reml` 実行に時間がかかる場合、`--scenario smoke` と `--scenario full` の 2 モードを定義して CI を段階化する。詳細は `6-2-multitarget-release-pipeline.md` へ連携。
- **診断差分の発生**: 実行系で検出された差分は `docs/plans/rust-migration/1-3-dual-write-runbook.md` の手順に従って報告し、Phase 3 の該当章や Phase 2-7 の残課題票に紐付ける。
- **再現性欠如**: 実運用 `.reml` が依存する外部ファイル/Capability を再現できない場合、`compiler/runtime/native` にテスト用 Capability Stub を実装し、`docs/spec/3-8-core-runtime-capability.md` の Stage ルールに基づき明示的に opt-in させる。

## 4.0.8 連携とハンドオーバー
- Phase 5 のセルフホスト計画に向け、`phase4-readiness.md` に実用テストの観測値と既知の制約をまとめる（Self-Host MVP が参照）。
- Phase 6（旧 Phase 4）で利用する互換性検証シナリオは、Phase 4 の成果（`.reml` + メトリクス）をそのまま入力にする。`6-0-phase6-migration.md` 冒頭にリンクを貼り、再実行手順を共通化する。
- `docs/plans/bootstrap-roadmap/README.md` と `SUMMARY.md` に Phase 4 の役割を追記し、`README` のクリティカルパスを「Phase 4 practical → Phase 5 self-host → Phase 6 release」へ更新する。
