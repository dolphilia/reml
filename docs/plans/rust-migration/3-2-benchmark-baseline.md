# 3.2 ベンチマークベースライン計画

本書は Rust 移植 Phase P3 において性能回帰を監視するためのベンチマーク指標・実行手順・CI 連携を定義する。`unified-porting-principles.md` で掲げた「性能回帰 ±10% 以内」の成功指標と、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の測定ガイドラインを Rust 実装へ適用する。

## 3.2.1 目的
- OCaml 実装と Rust 実装の性能比較を体系化し、dual-write CI での可観測な性能回帰を即時検知する。
- `compiler/ocaml/benchmarks/` の既存スイートを Rust 側で再実行できるよう再整備し、結果を統一フォーマット (`reports/benchmarks/*.json`) で保存する。
- ベンチマーク結果を P4 の最適化フェーズに引き渡すため、履歴と閾値設定を `reports/audit/dashboard` と同期する。

## 3.2.2 スコープと前提
- **対象**: CLI ベンチ (`scripts/benchmark.sh`)、LLVM IR ベンチ (`scripts/compare-ir.sh`)、パーサ/型推論のマイクロベンチ (`compiler/ocaml/benchmarks/*.reml`)。
- **除外**: ランタイム/FFI ベンチ（P4 で実施）、セルフホスト後のクロスコンパイル性能（Phase 3 後半の別計画で扱う）。
- **前提**:
  - `cargo criterion` など Rust 側のベンチ環境がセットアップ可能。
  - `scripts/benchmark.sh` が Rust CLI でも利用できるよう `--frontend` オプションを追加済み。
  - ベンチマーク結果を JSON で出力し、`reports/audit/index.json` に記録する枠が `benchmarks` セクションとして確保されている。

## 3.2.3 測定指標
| 分類 | 指標名 | 定義 | 目標値 | 参照 |
| --- | --- | --- | --- | --- |
| パーサ | `parse_throughput` | 10MB ソースの解析時間 (ms) | Rust ≤ 1.1 × OCaml | `docs/spec/0-1-project-purpose.md` §1.1 |
| 型推論 | `type_inference_cpu_time` | 型推論フェーズの CPU 時間 (ms) | Rust ≤ 1.1 × OCaml | `1-0-front-end-transition.md` |
| 効果解析 | `effect_analysis.missing_tag` | 欠落タグ数 | Rust = 0 | `0-3-audit-and-metrics.md` |
| LLVM | `llvm_codegen_time` | MIR→LLVM IR 変換時間 | Rust ≤ 1.1 × OCaml | `2-0-llvm-backend-plan.md` |
| CLI | `diagnostic_render_time` | `remlc` CLI で診断テキスト生成に要する時間 | Rust ≤ 1.1 × OCaml | `diagnostic_formatter.ml`, `diagnostic_formatter.rs` (予定) |
| メモリ | `memory_peak_ratio` | ピークメモリ / 入力サイズ | Rust ≤ 1.0 × OCaml | `0-1-project-purpose.md` §1.1 |
| コレクション | `vec.effect.mem_bytes` | `collect_vec_mem_reservation` が `collector.effect.mut=true` かつ `collector.effect.mem_bytes > 0` を `AuditEnvelope.metadata`/`Diagnostic.extensions` で出力しているか検証する | `python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion --require-success`（`collector.effect.mem_bytes` が欠落しないことを保証） | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.1.2 |
| IO/Path | `io.copy_throughput_mb_s` | `bench_core_io` の `reader_copy_*` を 1 秒あたりの転送 MB として集計し、Phase 2 ベースラインとの差分を ±15% 以内に抑える。`reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` の `core_io.reader_copy_*` を参照し、`core_io.benchmark.copy_throughput_mb_s` KPI で監視する。 | Rust ≤ 1.15 × Phase2 | [3-5-core-io-path-plan.md](../bootstrap-roadmap/3-5-core-io-path-plan.md) §7.2, [reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json](../../reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json) |
| IO/Path | `path.normalize_ops_s` | `bench_core_io` の `core_path.normalize_*` ケースから 1 秒あたりの正規化件数を算出し、Phase 2 比 ±15% を超えた場合にアラート。Baseline は `reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` で管理し、CI では Linux/macOS/Windows で日次収集する。 | Rust ≤ 1.15 × Phase2 | [3-5-core-io-path-plan.md](../bootstrap-roadmap/3-5-core-io-path-plan.md) §7.2 |
| IO/Path | `watch.audit_batch_ns` | Watcher 監査イベント処理 (`watch_event_batch`) の 1 バッチあたり処理時間（ns）を測定し、Phase 2 実装と比較して ±15% を閾値とする。サンドボックスでは合成イベントで測定、実ハードでは `watch` 実行結果に差し替える。 | Rust ≤ 1.15 × Phase2 | [3-5-core-io-path-plan.md](../bootstrap-roadmap/3-5-core-io-path-plan.md) §7.2, [reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json](../../reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json) |

各指標は `reports/benchmarks/benchmark-<date>.json` に `{ "baseline": "...", "candidate": "...", "delta": "...", "delta_pct": ... }` の形式で保存し、`delta_pct` が ±10% を超えた場合は CI を失敗扱いとする。

## 3.2.4 ベンチマークスイート整備
| スイート | 内容 | OCaml 実行コマンド | Rust 実行コマンド | 備考 |
| --- | --- | --- | --- | --- |
| `macro_typeclass.reml` | 大規模型クラス解決 | `scripts/benchmark.sh --suite macro_typeclass --frontend ocaml` | `scripts/benchmark.sh --suite macro_typeclass --frontend rust` | dual-write 比較に利用 |
| `micro_typeclass.reml` | 型クラスマイクロベンチ | 同上 | 同上 | ネスト深度による回帰検知 |
| `simple_bench.reml` | パーサ/型推論基本 | `scripts/benchmark.sh --suite simple --frontend ocaml` | `--frontend rust` | CI ブロッカー設定 |
| `test_simple.reml` | 最小ケース | 同上 | 同上 | smoke テスト |
| LLVM diff | IR 生成 | `scripts/compare-ir.sh ocaml rust --mode bench` | 同コマンド | `tooling/ci/compare-ir.py` と連携 |
| CLI render | 診断文字列 | `scripts/benchmark-diagnostic.sh --frontend ocaml` | `--frontend rust` | 新規スクリプトを追加 |
| Iter buffered | `Iter::buffered` アダプタ + `collect_vec`（backpressure/メモリ測定） | `cargo bench -p compiler-ocaml-frontend iter_buffered` (暫定) | `cargo bench -p compiler-rust-frontend iter_buffered -- warmup-time 3 --measurement-time 10` | KPI: `iterator.mem.window`（`reports/iterator-buffered-metrics.json`）と `windows_per_sec`（`reports/benchmarks/iter_buffered-YYYY-MM-DD.json`）。±10% 以内であれば合格。 |
| `numeric_statistics` | Core.Numeric `mean`/`variance`/`percentile` の数値安定性・性能測定 | （OCaml 実装なし、参考値のみ） | `cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric --bench bench_numeric_statistics -- --noplot` | `reports/benchmarks/numeric-phase3/phase3-baseline-2025-12-04.json` に `mean_large_drift`/`variance_random_walk`/`percentile_heavy_tail` の基準値を保存。Phase3 KPI `numeric.statistics.latency_ms` をここから抽出する。 |
| `time_clock` | Core.Time `now`/`monotonic_now`/`duration_between` の syscall ジッター測定 | （OCaml 実装なし、参考値のみ） | `cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --features core-time --bench time_clock -- --noplot` | `reports/benchmarks/numeric-time/phase3-bench-20250107.json` に `time_now_latency`/`time_monotonic_now_latency`/`duration_between_*` の中央値と outlier 比率を記録し、Phase3 KPI `time.syscall.latency_ns` を監視する。 |
| `bench_core_io` | Core.IO reader_copy / buffered_read_line、Core.Path normalize、Watcher 監査イベント処理（合成ベンチ） | （OCaml 実装なし、参考値のみ） | `cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --features \"core-io core-path\" --bench bench_core_io -- --noplot` | `reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` に `core_io.reader_copy_*` / `core_io.buffered_read_line_*` / `core_path.normalize_*` / `core_io.watch_event_batch` を記録。Watcher は CI サンドボックス制約のため合成イベントで測定し、実ハードでは `watch` 実行値に差し替える。 |

> 更新ログ（2025-12-24）  
> - `bench_core_io` 初回ベースラインを取得し、`io.copy_throughput_mb_s` / `path.normalize_ops_s` / `watch.audit_batch_ns` を `reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` に保存。Watcher ベンチは当面、`WatchEvent` 合成バッチで監査パイプラインを測定し、実環境で OS 監視が解禁され次第に差し替える。  
> - `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `core_io.benchmark.copy_throughput_mb_s` KPI を追加し、Phase 2 → Phase 3 の性能回帰を ±15% で検出するフローを統一した。

Rust 実装向けに `compiler/rust/benchmarks/` を作成し、OCaml スイートと同じ入力を利用する。ベンチ実行時は `cargo run --bin remlc -- --frontend rust ...` を内部で呼び出し、差分を `reports/benchmarks/dual` に保存する。

## 3.2.5 実行フロー
1. `scripts/benchmark.sh --frontend ocaml --output tmp/bench-ocaml.json`
2. `scripts/benchmark.sh --frontend rust --output tmp/bench-rust.json`
3. `python3 tooling/ci/compare-benchmarks.py --baseline tmp/bench-ocaml.json --candidate tmp/bench-rust.json --threshold 0.10 --output reports/benchmarks/benchmark-<date>.json`
4. `collect-iterator-audit-metrics.py --section diagnostics --source reports/benchmarks/benchmark-<date>.json --metric diagnostic.render_time` を実行し、診断レンダリング時間の差分を記録。
5. 差分が閾値を超えた場合は `reports/benchmarks/regression-<date>.md` に原因と暫定対応を記載し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md#rust-performance-regressions` を更新する。

## 3.2.6 CI 連携
- `bootstrap-linux.yml` に `bench` ジョブを追加し、`strategy.matrix.frontend` を利用して OCaml / Rust を並列実行。dual-write 比較は `compare-benchmarks.py` の閾値判定で gating。
- macOS / Windows ではリソース制約を考慮し、週次スケジュール実行 (`workflow_dispatch` + cron) とする。結果は `reports/benchmarks/history/<platform>/benchmark-<date>.json` に保存。
- `tooling/ci/record-metrics.sh` を拡張し、`--benchmark-result` を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の表へ追記する。
- ベンチ結果のアーカイブは GitHub Artifact `rust-benchmark-results` として 30 日保持。長期保存が必要なものは `reports/benchmarks/history/` へ同期。

## 3.2.7 レビューとマイルストーン
| マイルストーン | 期間 | 完了条件 |
| --- | --- | --- |
| M1: ベンチスイート整備 | Sprint 1 | `scripts/benchmark.sh --frontend rust` が全スイートで成功。`reports/benchmarks/benchmark-init.json` を作成 |
| M2: CI 統合 | Sprint 2 | Linux CI に `bench` ジョブが追加され、`parse_throughput` 等が ±10% 以内に収まる |
| M3: プラットフォーム拡張 | Sprint 3 | macOS / Windows で週次ベンチが実行され、履歴が `reports/benchmarks/history/` に蓄積 |
| M4: レビュー完了 | Sprint 4 | 4 週間連続で回帰なし、`docs/notes/benchmark-trend-report.md` にサマリを記録 |

## 3.2.8 リスクと対応
- **測定ノイズ**: CI 環境のばらつきで ±10% を超える可能性がある。複数回測定して中央値を採用し、`--samples` オプションを追加。
- **入力資産の劣化**: `compiler/ocaml/benchmarks/` が最新仕様を反映していない場合は `docs/notes/core-library-outline.md` のテスト入力を転用。更新時は `0-1-baseline-and-diff-assets.md` に追記。
- **ツールチェーン差異**: Windows では LLVM バージョン差で性能差が出る可能性。`reports/windows-env-check-rust.json` を確認し、`0-2-windows-toolchain-audit.md` の fallback を適用。
- **ベンチ実行時間**: dual-write でジョブ時間が長引く場合は `--quick` プリセットを用意し、週次フルベンチと PR 時スモークベンチを分離。

---

ベンチマーク結果は `3-0-ci-and-dual-write-strategy.md` で定義した dual-write CI の一環として取得し、監査ダッシュボードとの整合は [3-1-observability-alignment.md](3-1-observability-alignment.md) に従って記録する。
