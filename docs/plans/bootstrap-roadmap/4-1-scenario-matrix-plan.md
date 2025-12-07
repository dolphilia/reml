# 4.1 Phase 4 シナリオマトリクス整備計画

## 目的
- Phase 4 M1（シナリオマトリクス確定）の出口条件を満たすため、`.reml` シナリオの分類・仕様根拠・期待結果を一元管理する。
- `docs/spec/0-1-project-purpose.md` が定める性能と安全性の指標を、Chapter 1〜3 のコード例に沿って測定可能なテストケースへ落とし込む。
- `docs/spec/1-0-language-core-overview.md` から `docs/spec/3-10-core-env.md` までの既存サンプルを、`.reml` 実行資産として `examples/` および `reports/spec-audit/ch4/` に再配置する。
- Phase 3 で整備したリスト（`docs/plans/rust-migration/p1-test-migration-*.txt` 等）を再利用し、Phase 5 Self-host の前提となる「正例/境界例/負例」のトリオを Chapter ごとに揃える。
- `.reml` 実行を通じて、Chapter 1（構文・型・効果）〜Chapter 3（標準ライブラリ）の仕様どおりの許容範囲を明文化し、複数の表記揺れ・境界・意地悪ケースを網羅する。

## スコープ
- **含む**: `docs/spec/1-x`〜`3-x`・`docs/guides/core-parse-streaming.md` のサンプル抽出、`.reml` テストケース作成、`phase4-scenario-matrix.csv` の定義と更新フロー、`examples/spec_core`/`examples/practical` ディレクトリ構成案、`reports/spec-audit/ch4/` へのリンク整備。
- **含まない**: Rust 実装や CLI の挙動修正、セルフホスト工程そのもの、Phase 4 M2 以降で扱う CI ワークフロー設定（`4-2` 以降で管理）。
- **前提条件**: Phase 3 の章別資産が `compiler/rust/`・`examples/` に揃っている、`docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` に沿って新規ファイルの命名・参照が決まっている。

## 成果物と出口条件
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` を新設し、各行に `scenario_id`, `category`, `spec_anchor`, `input_path`, `expected`, `diagnostic_keys`, `resolution` を必須フィールドとして登録する。
- `examples/spec_core/`・`examples/practical/` にサブディレクトリ（`chapter1/boundary` 等）を定義し、マトリクスの `input_path` と 1:1 で対応させる命名規約を決める。
- `reports/spec-audit/ch4/spec-core-dashboard.md` と `reports/spec-audit/ch4/practical-suite-index.md` に、マトリクスと一致するハンドブックリンクを追加できる状態にする。
- `phase4-scenario-matrix.csv` に登録したカテゴリのうち 85% 以上が `.reml` 資産を伴い、`resolution` 列が `pending` 以外になっていることを確認する（M1 exit）。
- Chapter 1 のすべての構文規則について「正例/境界例/ギリギリエラー/明確なエラー」の 4 パターンを `.reml` で登録し、複数表記がある規則は各記法を個別の行として掲載する。

## 作業ブレークダウン

### 1. 資産棚卸しと分類軸の確定（69週目）
- `docs/spec/1-0-language-core-overview.md`, `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/1-3-effects-safety.md`, `docs/spec/3-0-core-library-overview.md` を横断し、サンプルコードを `Prelude/IO/Capability/Runtime/Plugin/CLI` のカテゴリへ分類。
- `docs/plans/rust-migration/p1-test-migration-*.txt` のケースを機械的に読み込み、既存 ID のまま `phase4-scenario-matrix.csv` にインポートするスクリプト（`scripts/migrate_phase4_matrix.py` 仮）を準備。
- `category` と `spec.chapter`（例: `chapter1.syntax`）の表を `docs/plans/bootstrap-roadmap/assets/README.md` に追記し、Phase 4 以降の参照に備える。

### 2. `.reml` ケース作成とリンク付け（70〜71週目）
- `docs/spec/1-x` 各節に対して「正例/境界例/負例」の `.reml` を最低 1 セット作成し、`examples/spec_core/chapter1/` に配置。`docs/spec/1-5-formal-grammar-bnf.md` の各規則 ID をファイル名に含め、双方向参照を可能にする。
- `docs/spec/3-5-core-io-path.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-10-core-env.md` の実用例を `examples/practical/` に移植し、入出力および監査ログ例を `expected/` ディレクトリに保存。
- `docs/guides/runtime-bridges.md` / `docs/guides/plugin-authoring.md` と連携し、Capability を要求する `.reml` には `runtime_bridge`/`capability` の列を追加。Stage 要件を `phase4-scenario-matrix.csv` へ反映する。
- Chapter 1 の各構文に対し、`.reml` で表現可能な全バリエーションを列挙（例: `let` のパターン束縛書式、`match` の分岐、`effect handler` の `with`/`match` 等）。規則ごとに `variant` 列を設け、表記揺れの漏れが可視化されるようにする。

### 3. マトリクス検証とレビューサインオフ（72週目）
- `phase4-scenario-matrix.csv` に `resolution` 列を設け、`ok` / `impl_fix` / `spec_fix` を入力。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` とリンクするケースは `impl_fix` として登録。
- `reports/spec-audit/ch4/spec-core-dashboard.md` にシナリオ一覧と Pass/Fail 状態を出力する `scripts/gen_phase4_dashboard.py` を用意し、レビューで差分を確認できるようにする。
- Phase 4 レビュー会（週次）でマトリクスを共有し、未定義ケースを `docs/notes/phase4-practical-test-backlog.md` に追記。承認後に `phase4-scenario-matrix.csv` を `main` ブランチへ反映し、M1 完了を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録。
- `.reml` 実行結果から「コンパイラ修正」「仕様追記」「許容」の別を判定し、`resolution` + `notes` に根拠を記載。判断に迷うケースは `docs/spec/1-x` の該当節を引用し、レビュー時に仕様の解釈を再確認する。

### 4. 更新運用とハンドオーバー（73週目）
- `phase4-scenario-matrix.csv` 更新ガイド（列定義、PR テンプレート、レビュー観点）を `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix-guideline.md` として作成。
- `docs/plans/bootstrap-roadmap/4-4-field-regression-and-readiness-plan.md` と連携し、`resolution` が `impl_fix` / `spec_fix` のケースを自動で Issue/タスクに連携するワークフロー案を記述。
- `docs-migrations.log` に Phase 4 資産追加の履歴を残し、Phase 5 `phase5-readiness.md` で参照できるようにする。

## リスクとフォローアップ
- **シナリオ不足**: Chapter 1 の境界例が不足する場合は `docs/notes/core-library-outline.md` を参照し、追加ケースを `phase4-scenario-matrix.csv` に `priority=high` として登録。リードタイムが足りない場合は `run_examples.sh --suite spec-core` をスキップできるガードを `4-2` タスクと調整する。
- **分類不一致**: `category` や `spec.chapter` が統一されていない場合は `scripts/validate_phase4_matrix.py`（仮）で lint を走らせ、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の「表記崩れ」リスクとして報告。
- **リンク切れ**: `examples/` リネーム時には `README.md` / `SUMMARY.md` / `phase4-scenario-matrix.csv` を同時更新し、`docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` の「相互参照維持」要件を満たす。
