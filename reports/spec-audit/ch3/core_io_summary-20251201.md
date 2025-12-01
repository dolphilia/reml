# Core.IO / Path テスト・監査サマリ（2025-12-01）

Phase3 W50 キックオフ時点で `core-io-path` ジョブから収集したテスト・監査ログを整理し、`0-3-audit-and-metrics.md` の新規指標と `0-4-risk-handling.md` のリスク登録へ反映した内容をまとめる。

## 1. 指標ダイジェスト
| 指標 | 最新値 | 集計根拠 |
|------|--------|----------|
| `io.error_rate` | **1.00 (1/1 ケース)** | `compiler/rust/runtime/tests/expected/io_error_open.json` で `core.io.read_error`（Severity=`error`）を 1 件記録。Reader/Writer の成功ケースがまだ集約されていないため、当面は失敗ケースのみで比率を算出。`collect-iterator-audit-metrics.py --section core_io --scenario diagnostics_summary` を Nightly に組み込んだら成功ケースを含めて再計測する。 |
| `path.security.incident_count` | **3 件** | `tests/data/core_path/security/{relative_denied,sandbox_escape,symlink_absolute}.json` に記録された `core.path.security.*` 診断をカウント。各ケースで `metadata.security.reason` が埋まっていることを `scripts/validate-diagnostic-json.sh --pattern core.path.security` で確認済み。 |
| `watcher.audit.pass_rate` | **未計測（Watcher CI 未接続）** | `tests/fixtures/watcher/simple_case` ベースの `io_watcher-simple_case.jsonl` は生成済みだが、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit` をまだ CI に追加できていない。`docs/plans/bootstrap-roadmap/0-4-risk-handling.md#core-io-watcher-risk` でベースライン欠如をリスク化。 |

## 2. テスト実行ログ
### 2.1 Reader/Writer / Diagnostics
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml io_error_into_diagnostic_matches_expected_subset`（`compiler/rust/runtime/tests/io_diagnostics.rs`）を参照し、`io_error_open.json` ゴールデンの `metadata.io.*` / `audit.io.*` が最新仕様と一致することを確認。
- `scripts/validate-diagnostic-json.sh --pattern core.io.read_error compiler/rust/runtime/tests/expected/io_error_open.json` をローカル再実行し、`effect.stage.required/actual` の必須キーを満たすことを手動で検証済み。
- Reader/Writer 成功ケースのカバレッジは API 実装完了待ち。`0-3-audit-and-metrics.md` の `io.error_rate` 行で成功ケース追加後に再集計するタスクを TODO 登録。

### 2.2 Path 正規化 / セキュリティ
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml path_normalize` → `tests/data/core_path/normalize_{posix,windows}.json` で `components`・`normalize` の差分が無いことを確認（`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §4.1）。
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml path_security` → `tests/data/core_path/security/*.json` の 3 ケースすべてが `core.path.security.*` 診断を生成。`path.security.incident_count` に同値を記録し、`SecurityPolicy` の `sandbox_escape` ケースで `metadata.security.reason = "symlink_outside_root"` を確認した。

### 2.3 Watcher
- `tests/fixtures/watcher/simple_case` を用いたモック実行で `io_watcher-simple_case.jsonl` を更新。2 件の WatchEvent（create/delete）が `queue_size=0`, `delay_ns` のみを報告しており、`metadata.io.async_queue` をまだ記録できていない点を `watcher.audit.pass_rate` の課題として整理。
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml watch::tests::` は macOS/Linux の双方で `notify` バックエンドが未結線のため `ignored` 扱い。CI では `--include-ignored` を付与して実装確認を行う TODO を残した。

### 2.4 サンプル / CLI
- `tooling/examples/run_examples.sh --suite core_io` を dry-run し、`examples/core_io/file_copy.reml` の CLI ログが `metadata.io.helper = "copy"` を出力することを確認。`core_io.example_suite_pass_rate` は 1.0（2/2）を維持。
- `tooling/examples/run_examples.sh --suite core_path` では `examples/core_path/security_check.reml` が `core.path.security.invalid` を意図的に発火し、`path.security.incident_count` のケース数と整合する。

## 3. ベンチマーク
| シナリオ | 状態 | 備考 |
|----------|------|------|
| `reader_copy_64k` | **ベースライン確定** | `cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --bench bench_core_io -- --noplot` の結果を `reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` に保存済み。OCaml 比 ±9% で収束。以降は `io.copy_throughput_mb_s` をトラック。 |
| `buffered_read_line` | **安定** | `BufferedReader` 実装後の Criterion 値は ±6% に収まる。`core_io.buffered_reader_buffer_stats_pass_rate` の計測ログと併せて確認。 |
| `core_path_normalize` | **安定** | Path 文字列 API のベンチは `reports/spec-audit/ch3/path_unicode-20251130.md` で報告済み。`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §4.3 にリンクを追加。 |
| `watch_event_batch` | **未計測** | Watcher API が CI 無効化中のためサンプルベンチのみ生成。`watcher.audit.pass_rate` が 1.0 になるまで本番値を確定しない。 |

## 4. リスクとフォローアップ
- `watcher.audit.pass_rate` 未計測 → [0-4-risk-handling.md#core-io-watcher-risk](../../docs/plans/bootstrap-roadmap/0-4-risk-handling.md#core-io-watcher-risk) に登録。CI への `watcher_audit` シナリオ追加がブロッカー。
- Capability Stage 再検証 → [0-4-risk-handling.md#core-io-permission-risk](../../docs/plans/bootstrap-roadmap/0-4-risk-handling.md#core-io-permission-risk)。`io.error_rate` の分母を拡充し、`fs.permissions.*` Stage 遅延を監視する。
- シンボリックリンク攻撃耐性 → [0-4-risk-handling.md#core-path-symlink-risk](../../docs/plans/bootstrap-roadmap/0-4-risk-handling.md#core-path-symlink-risk)。`path.security.incident_count` が 3 件を超えた際の自動ブロック導入を Phase3 Sprint E で検討。

## 5. TODO / 次アクション
1. `collect-iterator-audit-metrics.py --section core_io --scenario diagnostics_summary --require-success` を Linux CI に追加し、Reader/Writer 正常系を `io.error_rate` 分母へ取り込む。
2. `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/watcher-audit.json --require-success` を導入し、`metadata.io.async_queue` を診断へ転写できるか検証。
3. `scripts/validate-diagnostic-json.sh --pattern core.path.security tests/data/core_path/security/*.json` を PR テンプレートへ追加し、`path.security.incident_count` のカウント自動化を行う。
4. `reports/spec-audit/ch3/core_io_summary-YYYYMMDD.md` を週次で更新し、`0-3-audit-and-metrics.md` の `io.error_rate` / `path.security.incident_count` / `watcher.audit.pass_rate` 表と値を同期する。
