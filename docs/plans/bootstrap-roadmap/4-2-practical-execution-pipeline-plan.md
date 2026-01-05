# 4.2 Phase 4 実行パイプライン構築計画

## 目的
- Phase 4 M2 「実行パイプライン稼働」を達成し、`.reml` シナリオを `compile → run → diagnose → verify` まで自動連鎖させる。
- Rust 実装 CLI（`reml_frontend`, `remlc`）と `tooling/examples/run_examples.sh`、`cargo test -p reml_e2e` を統合し、ローカルと GitHub Actions の両方で再現可能な手順を確立する。
- `examples/spec_core`, `examples/practical`, `examples/pipeline_*` を Phase 4 専用スイート（`spec-core`, `practical`, `integration`, `selfhost-smoke`）へ再編し、Phase 5 Self-host や Phase 6 リリースパイプラインで再利用できるアーティファクトを残す。
- `docs/spec/1-x`〜`3-x` のサンプルが `.reml` 実行で期待どおりの挙動を示すことを、`reports/spec-audit/ch5/*.md` のログで証明する。
- 1-x 章の全構文規則に対して `.reml` の正例/境界例/ギリギリエラー/負例をすべて実行し、仕様が許容する複数表記や意地悪ケースを Pipeline 上で常に検証できるようにする。

## スコープ
- **含む**: `tooling/examples/run_examples.sh` のスイート拡張、`compiler/tests/practical/` の e2e テスト整備、`.github/workflows/phase4-practical.yml`（仮）の作成、`reports/spec-audit/ch5/` への実行ログ出力、Golden ファイル更新フローの策定。
- **含まない**: ランタイムや CLI の内部最適化（必要な修正は別チームへエスカレーション）、セルフホスト Stage 切り替え、Phase 3 の仕様追加。
- **前提条件**: `4-1-scenario-matrix-plan.md` で `.reml` ケースの分類と参照先が確定している、Rust 実装が Chapter 3 API まで揃っている、`docs/guides/tooling/audit-metrics.md` の KPI に Phase 4 指標を追加できる状態。

## 成果物と出口条件
- `tooling/examples/run_examples.sh` に `--suite spec-core|practical|integration|selfhost-smoke` を追加し、`./tooling/examples/run_examples.sh --suite practical --update-golden` が成功する。
- `compiler/tests/practical/` 配下に `spec_core.rs`, `practical.rs`, `integration.rs` を追加し、`cargo test -p reml_e2e -- --scenario practical` が Chapter 1〜3 の代表 `.reml` を実行する。
- `.github/workflows/phase4-practical.yml`（または既存ワークフローへのジョブ追加）で `run_examples.sh` と `cargo test -p reml_e2e` を並行実行し、監査ログ/成果物を `reports/spec-audit/ch5/` にアップロードする。
- `reports/spec-audit/ch5/practical-suite-YYYYMMDD.md` に実行ログ、性能、診断サマリが揃い、M2 exit レビューで承認される。
- `spec-core` スイートでは `.reml` の各ファイルに `variant=canonical|alternate|boundary|invalid` を埋め込み、すべてのバリエーションが CI で自動実行されることを exit 条件とする。

## 作業ブレークダウン

### 1. ランナー構成と CLI 整備（72〜73週目）
- `tooling/examples/run_examples.sh` を再設計し、`phase4` プレフィックスの設定ファイル（`tooling/examples/config/phase4-practical.toml` 仮）を読み取れるようにする。
- `.reml` の分類ごとにデフォルト Capability / Runtime オプションを切り替える仕組みを導入し、`spec-core` は最小 Capability、`practical` は `core.io`・`core.runtime` を有効化する。
- `docs/spec/0-3-code-style-guide.md` に従ったエラーレポートを行うため、実行結果を JSON (`*.diagnostic.json`, `*.audit.jsonl`) と Markdown (`*.md`) に自動変換する。

### 2. Rust テストスイートと Golden 資産の統合（73〜74週目）
- `compiler/tests/practical/` に `mod spec_core`, `mod practical`, `mod integration`, `mod selfhost_smoke` を追加し、各モジュールで `Phase4TestCase`（新設 struct）を利用する。
- `tests/data/phase4/` に `.reml` 入力と期待出力 (`.stdout`, `.stderr`, `.diagnostic.json`, `.audit.jsonl`) を配置し、`cargo test -p reml_e2e -- --bless` で正規化できるようにする。
- Golden ファイルは `reports/spec-audit/ch5/golden/` に集約し、`tooling/examples/run_examples.sh --update-golden` 実行時に `git diff` で差分が明示されるよう README を整備。
- 章 1 の BNF 規則ごとに `Phase4TestCase` へ `spec_anchor` と `variant` を必須フィールドとして保持し、正例/境界/ギリギリエラー/負例の 4 ケースが揃っていない場合はテストを失敗させる。

### 3. CI / GitHub Actions への接続（74〜75週目）
- `.github/workflows` に Phase 4 専用ジョブを追加し、`run_examples.sh` と `cargo test -p reml_e2e` を Linux/macOS 並列で走らせる。Windows サポートは Phase 5 以降の準備として `allow-failure` で監視。
- `tooling/ci/` に `phase4-practical-upload.sh` を追加し、`reports/spec-audit/ch5/practical-suite-{date}.md` や診断 JSON をアーティファクト化。
- CI ジョブに `collect-iterator-audit-metrics.py --section practical --require-success` を仕込む準備を行い（詳細は `4-3` へ委譲）、失敗時のログ収集を統一。

### 4. レビューと運用移管（75週目）
- `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` のレビュー会と連携し、`.reml` 実行結果を週次で確認、`impl_fix` / `spec_fix` の切り分けプロセスを確定。
- `docs/plans/bootstrap-roadmap/README.md` / `SUMMARY.md` へ Phase 4 スイートの説明を追記し、利用者がナビゲーションできるようにする。
- 成果を `docs/guides/tooling/audit-metrics.md` に記録し、`practical_suite.pass_rate`, `spec_core_suite.pass_rate` を KPI として登録。
- `.reml` 実行ログには `resolution_hint`（`impl_fix`/`spec_fix`/`ok`）を強制出力し、`phase4-scenario-matrix.csv` と同期できるようにする。

## リスクとフォローアップ
- **CI 実行時間の増大**: `--scenario smoke` / `--scenario full` の 2 モードを `run_examples.sh` と `cargo test` に導入し、`0-4-risk-handling.md` の「実行コスト肥大化」対策に従って段階的に実行する。
- **Golden 破損**: `scripts/validate-phase4-golden.sh`（仮）で `.diagnostic.json` の schema を検証し、差分が検出されたら `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ記録。
- **Capability 依存の再現性不足**: `4-1` で定義した `capability` 列を `run_examples.sh` の引数へ渡し、`docs/spec/3-8-core-runtime-capability.md` に沿って Stage を切り替える。外部依存が必要なケースは `examples/practical/capability_stub/` を介してスタブ化する。
