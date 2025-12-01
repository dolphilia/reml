# 0.3 測定・監査・レビュー記録

本章では Phase 1〜4 に共通する測定指標、診断と監査ログの収集方法、レビュー記録フォーマットを定義する。[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) と `docs/notes/llvm-spec-status-survey.md` のフォーマットを継承し、各フェーズの完了条件を定量的に確認できるようにする。

## 0.3.1 指標セット
| カテゴリ | 指標 | 定義 | 収集タイミング | 仕様参照 |
|----------|------|------|----------------|----------|
| 性能 | `parse_throughput` | 10MB ソースの解析時間 (ms) | フェーズごとに最低 3 回計測 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §1.1 |
| 性能 | `memory_peak_ratio` | ピークメモリ / 入力サイズ | 各フェーズ主要マイルストーン後 | 同上 |
| 安全性 | `stage_mismatch_count` | Capability Stage ミスマッチ件数 | CI (PR ごと) | [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) |
| 安全性 | `ffi_ownership_violation` | FFI 所有権警告件数 | CI + 週次レビュー | [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) |
| 安全性 | `iterator.stage.audit_pass_rate` | `typeclass.iterator.stage_mismatch` 診断で必須監査キーが揃った割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §2.4 |
| 安全性 | `diagnostic.audit_presence_rate` | 診断 JSON で `audit` / `cli.audit_id` / `cli.change_set` / `schema.version` / `timestamp` が欠落なく出力された割合（0.0〜1.0、欠落時は 0.0） | CI（週次レビュー、PRごと） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| コレクション | `collections.audit_bridge_pass_rate` | `Core.Collections` の `Map.diff`/`Map.merge`/`Set.diff`/`Table.to_map` が `Collections.audit_bridge` を経由して `AuditEnvelope.change_set`（`collections.diff.*` キーを含む）を出力できた割合。`change_set.total>0` のケースは `collector.effect.audit=true` であることを確認する。 | CI (`tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario map_set_persistent --require-audit`) + 週次レビュー | [3-7-core-config-data.md](../../spec/3-7-core-config-data.md) §2, [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §2.2 |
| コレクション | `collector.effect.audit_presence` | `CollectOutcome` が `AuditEnvelope.metadata["collections.diff.*"]` を生成した際に `collector.effect.audit`/`collector.effect.mem_bytes` が必須キーとして埋まっている割合 (0.0〜1.0)。ゼロ差分（`change_set.total=0`）の場合は `effect.audit=false` であることを確認し、非ゼロ差分時は `true` を義務付ける。 | CI (`collect-iterator-audit-metrics.py --section collectors --scenario map_set_persistent --require-audit`) | [reports/spec-audit/ch1/core_iter_collectors.audit.jsonl](../../reports/spec-audit/ch1/core_iter_collectors.audit.jsonl), [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §2.2 |
| コレクション | `vec.effect.mem_bytes` | `collect_vec_mem_reservation` で `collector.effect.mut=true` かつ `collector.effect.mem_bytes > 0` を `AuditEnvelope.metadata` が報告することを確認する指標。`python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --scenario vec_mem_exhaustion --require-success` と `scripts/validate-diagnostic-json.sh --pattern collector.effect.mem_bytes reports/spec-audit/ch1/core_iter_collectors.json` を実行して `collector.effect.mem_bytes` キーが欠落せず正の値であることを保証する。 | CI (`vec_mem_exhaustion` シナリオ) | [reports/spec-audit/ch1/core_iter_collectors.json](../../reports/spec-audit/ch1/core_iter_collectors.json), [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.1.2 |
| コレクション | `collections.persistent_mem_ratio` | `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に記録した `ListPersistentPatch`/`MapPersistentMerge` シナリオのピークメモリ ÷ 入力サイズ。Phase 3 M2 の完了判定で 1.8 以下を確認し、次フェーズの監査ベースラインとして残す。 | `cargo run --manifest-path compiler/rust/runtime/ffi/Cargo.toml --features core_prelude --example core_collections_metrics` を Phase3 Week12 で実行 / 再測時は Phase4 Kickoff | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §2.3 |
| コレクション | `vec_mut_ops_per_sec` | `compiler/rust/runtime/ffi/benches/core_collections_mutable.rs` の `VecMutOpsPerSec` を 3 回計測した平均（ops/sec）。`CoreVec` 実装差分で ±15% 以内を維持できているか確認する。 | Phase3 Week12 の `cargo bench --manifest-path compiler/rust/runtime/ffi/Cargo.toml --bench core_collections_mutable`、および CI ナイトリー | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.1 |
| コレクション | `cell_mutations_total` | `CollectOutcome` / `AuditEnvelope.metadata` に記録された `collector.effect.cell=true` の件数。`collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation --require-cell` で実行し、ゼロならブロッカーとする。 | CI（`tooling/ci/collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation --require-success --require-cell`） | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.2、および `assets/metrics/core_collections_persistent.csv` の `RefInternalMutation` 行 |
| コレクション | `ref_borrow_conflict_rate` | `CollectError::BorrowConflict` を `collector.effect.rc=true` ケースで発生させた割合。`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `ref_internal_mutation` セクションから `conflicts / total borrow_mut` を算出し、CSV にも記録。 | 週次レビュー時に `python3 tooling/ci/collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation --output reports/iterator-collector-metrics.json --require-cell` を実行してログを更新 | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.2 および `assets/metrics/core_collections_persistent.csv` |
| コレクション | `table_insert_throughput` | `TableCollector` + `EffectfulTable` を用いた `collect_table_csv` ケースで 1 秒あたりに処理できた挿入件数。`collect-iterator-audit-metrics.py --scenario table_csv_import` の `metrics.table.insert_per_sec` を取得する。 | CI（Phase3 self-host 判定 / 週次レビュー） | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.3 |
| コレクション | `csv_load_latency` | `Table.load_csv` が `Core.IO.CsvReader` から 1,000 行 CSV をロードする際の平均遅延。`reports/spec-audit/ch3/table_csv_load.json` を `scripts/validate-diagnostic-json.sh --suite collectors --pattern csv_load` 経由で検証する。 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_collections_table` 完走後 / フェーズマイルストーン | [3-2-core-collections-plan.md](./3-2-core-collections-plan.md) §3.3, [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md) |
| テキスト | `text.mem.zero_copy_ratio` | `Bytes::from_vec` / `String::into_bytes` / `TextBuilder::finish` など `Vec<u8>` をムーブする経路で `collector.effect.transfer=true` かつ `collector.effect.mem_bytes=0` になった割合（0.0〜1.0）。`python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json --output reports/text-mem-metrics.json --require-success` を実行し、ZeroCopy 入力バイト数 / 全監視入力バイト数を記録する。2027-03-31 時点で `EffectSet::mark_transfer`（`compiler/rust/runtime/src/prelude/iter/mod.rs`）と `collector.effect.transfer` の計測を Rust Runtime に実装済み。 | CI（Phase3 Week41 以降の `phase3-core-text` ジョブ、週次レビュー） | [3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §1.2, [docs/notes/text-unicode-ownership.md](../../docs/notes/text-unicode-ownership.md) |
| テキスト | `text.mem.copy_penalty_bytes` | `Bytes::from_slice` / `Str::to_bytes` / `String::to_bytes` が `collector.effect.mem_bytes>0` を報告した際の平均バイト数。`reports/text-mem-metrics.json` の `cases[]` を `collect-iterator-audit-metrics.py --section text --scenario bytes_clone` で解析し、1KB 入力あたりのメモリ増分（B/KB）を KPI として保存する。目標値は Phase3 で ≤1024B/KB（複製無し）とし、超過時はブロッカー。 | 同上 | [3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §1.2, [docs/spec/0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §1.1 |
| テキスト | `text.grapheme.cache_hit` | `log_grapheme_stats` が出力する `cache_hits / (cache_hits + cache_miss)`（UC-02/UC-03 のヒットケースを対象）。`python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --text-source reports/spec-audit/ch1/core_text_grapheme_stats.json --output reports/text-grapheme-metrics.json --require-success` を実行し、0.80 以上を目標値とする。`version_mismatch_evictions` も併せて JSON に保存し、閾値超過時は `unicode-cache-cases.md` を参照して原因を特定する。 | CI（`phase3-core-text` ジョブ、UC-02/UC-03 完了時） | [docs/spec/3-3-core-text-unicode.md](../../docs/spec/3-3-core-text-unicode.md) §4.1.1/§5, [docs/plans/bootstrap-roadmap/checklists/unicode-cache-cases.md](./checklists/unicode-cache-cases.md) |
| テキスト | `text.grapheme.script_mix_ratio` | `reports/spec-audit/ch1/core_text_grapheme_stats.json` の UC-02 行を解析し、`script_mix_ratio >= 0.55` かつ `rtl_ratio >= 0.4` であることを確認する指標。`python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --text-source reports/spec-audit/ch1/core_text_grapheme_stats.json --require-success --check script_mix` を実行し、`primary_script`/`rtl_ratio` が欠落または閾値未満の場合は CI を失敗させる。 | UC-02/UC-03 再計測時（`text_internal_cache` テスト完走後） | [docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §2.2, [docs/notes/text-unicode-segmentation-comparison.md](../notes/text-unicode-segmentation-comparison.md), [docs/notes/text-unicode-ownership.md](../notes/text-unicode-ownership.md) |
| テキスト | `text.normalize.mb_per_s` | `tests/data/unicode/UAX15/NormalizationTest-15.1.0.txt` を入力として `normalize_{nfc,nfd,nfkc,nfkd}` が処理した総バイト数 ÷ 実行時間 (MB/s)。`cargo run --manifest-path compiler/rust/runtime/Cargo.toml --example text_normalization_metrics -- --output reports/text-normalization-metrics.json` で測定し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario normalization_conformance --text-normalization-source reports/text-normalization-metrics.json --require-success` で閾値（各フォーム 2.0 MB/s 以上、平均 3.0 MB/s 以上）を検証する。 | CI（`phase3-core-text` ジョブ、週次レビュー） | [3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §3.1, [docs/spec/3-3-core-text-unicode.md](../../spec/3-3-core-text-unicode.md) §4.2 |
| テキスト | `text.bench.normalization_mb_per_s` | `cargo bench --manifest-path benchmarks/Cargo.toml text::normalization -- --save-baseline phase3-core-text` が出力する `criterion` スループット（MB/s）の平均。`reports/benchmarks/core_text/phase3-baseline.md` に転記し、Phase2 ベースライン比 ±15% 以内を確認する。逸脱時は `docs/notes/text-unicode-performance-investigation.md` へ記録する。 | Phase3 `phase3-core-text` ジョブ完走後の手動ベンチ、リリース判定前 | [3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §6.2, [reports/benchmarks/core_text/README.md](../../reports/benchmarks/core_text/README.md) |
| テキスト | `text.bench.grapheme_ns_per_char` | `benchmarks/text/grapheme.rs` の `segment_cold` / `segment_cached` が報告する `ns/iter` を 1 文字あたりに換算し、`reports/benchmarks/core_text/phase3-baseline.md` に `cold`/`cached`/`log_stats` を記録。`cache hit %` は併せて `reports/spec-audit/ch1/core_text_grapheme_stats.json` を確認し、±15% 超過時にリスク登録する。 | Phase3 `phase3-core-text` ジョブ、Cache KPI 更新時 | [3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §6.2, [docs/notes/text-unicode-performance-investigation.md](../../docs/notes/text-unicode-performance-investigation.md) |
| 数値/時間 | `numeric_time.effect_matrix_pass_rate` | `docs/plans/bootstrap-roadmap/assets/core-numeric-time-effects-matrix.md` に列挙した API 行を `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario effects_matrix --matrix ... --output reports/spec-audit/ch3/core_numeric_time_effects.json --require-success` で検証し、`effect {time}`/`{unicode}`/`{audit}` のタグと `CapabilityRegistry::verify_capability_stage` 結果が欠けていない割合。`AuditEnvelope.metadata["numeric_time.api"]` とマトリクス表の `代表 API` を突合させる。 | Phase3 `core-numeric-time` ジョブ → `effects_matrix` シナリオ実行時、週次レビュー | [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md), [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md), [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md), [assets/core-numeric-time-effects-matrix.md](assets/core-numeric-time-effects-matrix.md) |
| IO/Path | `core_io.effect_matrix_pass_rate` | `docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` に列挙した API 行を `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario effects_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/core_io_effects.json --require-success` で検証し、`effect {io}` / `{io.blocking}` / `{io.async}` / `{security}` と `CapabilityRegistry::verify_capability_stage` (`IoCapability` / `SecurityCapability` / `AsyncCapability`) の結果が欠落していない割合。`metadata.io.*` / `metadata.security.*` / `effect.stage.*` がマトリクスの要件と一致していることを突合する。 | Phase3 `core-io-path` ジョブ（Watcher・Security 含む全ケース）および週次レビュー | [3-5-core-io-path.md](../../spec/3-5-core-io-path.md), [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md), [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md), [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md) |
| IO/Path | `core_io.reader_writer_effects_pass_rate` | `Reader`/`Writer`/`copy`/`with_reader` 経路で `metadata.io.bytes_processed`, `extensions.effects.io_blocking_calls`, `metadata.io.helper`（`copy`）、`effect.stage.*` が欠落しない割合。`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario reader_writer --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/reader_writer-effects.json --require-success` を実行し、`tests/io_diagnostics.rs` の Reader/Writer ケースと `docs/plans/bootstrap-roadmap/3-5-core-io-path-remediation.md#1` で定義した監査キーを突合する。 | Phase3 `core-io-path` ジョブ（Reader/Writer Remediation W50 完了後は PR ごと） | [3-5-core-io-path-remediation.md](./3-5-core-io-path-remediation.md)#1-readerwriter-効果トラッキング整備, [assets/core-io-path-api-diff.csv](assets/core-io-path-api-diff.csv), [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md) |
| IO/Path | `core_io.buffered_reader_buffer_stats_pass_rate` | `BufferedReader`/`buffered`/`read_line` の診断および `IoContext` に `metadata.io.buffer.capacity/fill`, `capability = "memory.buffered_io"`, `effect.mem=true` が揃った割合。`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario buffered_reader --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/buffered_reader-metrics.json --require-success` を実行し、`compiler/rust/runtime/tests/buffered_reader.rs` + `tests/data/core_io/buffered_reader/context_snapshot.json` のゴールデンと突合する。 | Phase3 `core-io-path` ジョブ（BufferedReader Remediation W50 完了後は PR ごと） | [3-5-core-io-path-remediation.md](./3-5-core-io-path-remediation.md)#2-bufferedreader-とヘルパ-api-の強化, [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md), [assets/core-io-capability-map.md](assets/core-io-capability-map.md) |
| IO/Path | `core_io.file_ops_pass_rate` | `tests/data/core_io/file_ops/{posix,windows}` のゴールデンと `docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` の「ファイルハンドル / メタデータ」行を対象に、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario file_ops --platform linux --platform windows --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/file_ops-metrics.json --require-success` を実行した際の pass_rate。`core.io.file.*` 診断で `metadata.io.operation`, `metadata.io.path`, `metadata.io.capability`, `metadata.security.policy`, `effect.stage.required/actual` が揃っていること、`IoErrorKind` と OS 固有エラー（`errno`, `ERROR_ACCESS_DENIED` 等）が `extensions.io.*` で報告されていることを確認する。 | Phase3 `core-io-path` ジョブ（POSIX/Windows のファイル操作テスト。`reports/spec-audit/ch3/file_ops-YYYYMMDD.md` を週次レビューで確認） | [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md) §3.1, [3-5-core-io-path.md](../../spec/3-5-core-io-path.md) §3, [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md), [assets/core-io-path-api-diff.csv](assets/core-io-path-api-diff.csv) |
| IO/Path | `core_io.path_glob_pass_rate` | `tests/path_glob.rs` + `tests/data/core_path/glob_{posix,windows}.json` + `tests/fixtures/path_glob/*` を対象に、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario path_glob --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/path_glob-metrics.json --require-success` を実行した際の pass_rate。`Core.Path.glob` が `FsAdapter::ensure_read_capability()` を通じて `io.fs.read` Stage を検証し、`PathErrorKind::{InvalidPattern,Io}` でパターン/IO エラーを捕捉したうえで POSIX/Windows ゴールデンと一致すること、`core.path.glob.*` 診断／`metadata.io.glob.*` が JSON/Audit に出力されることを確認する。 | Phase3 `core-io-path` ジョブ（Path glob Remediation W51 完了以降は PR ごとに回す） | [3-5-core-io-path-remediation.md](./3-5-core-io-path-remediation.md)#3-corepath-glob-実装phase3-w51, [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md), [assets/core-io-path-api-diff.csv](assets/core-io-path-api-diff.csv), [tests/path_glob.rs](../../compiler/rust/runtime/tests/path_glob.rs) |
| IO/Path | `core_io.example_suite_pass_rate` | `tooling/examples/run_examples.sh --suite core_io` および `--suite core_path` を実行し、`cargo run --bin reml` が `examples/core_io/file_copy.reml` / `examples/core_path/security_check.reml` で成功した割合（0.0 or 1.0）。Reader/Writer/Path セキュリティの監査キー欠落が再発した場合はこの指標が 0.0 になり、`core-io-path-plan.md` §6 ドキュメント更新タスクの回帰として扱う。 | Phase3 `core-io-path` ジョブ（Nightly と PR の両方） | [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md)#6-ドキュメントサンプル更新49-50週目, [examples/core_io/file_copy.reml](../../examples/core_io/file_copy.reml), [examples/core_path/security_check.reml](../../examples/core_path/security_check.reml), [tooling/examples/run_examples.sh](../../tooling/examples/run_examples.sh) |
| IO/Path | `io.error_rate` | `core-io-path` CI ジョブ完了時に `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario diagnostics_summary --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --source reports/spec-audit/ch3/file_ops-metrics.json --source reports/spec-audit/ch3/reader_writer-effects.json --output reports/spec-audit/ch3/core_io_summary.json --require-success` を実行し、`core.io.*` 診断（Severity=`error`）件数 ÷ `core_io` ケース総数を集計する。結果は `reports/spec-audit/ch3/core_io_summary-YYYYMMDD.md` と同 JSON に転記し、1.0% 超の増加を検出したら `0-4-risk-handling.md` へ再登録する。 | Phase3 `core-io-path` ジョブの週次レビュー、Self-host 判定前 | [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md)#73-テスト結果とリスク整理, [3-5-core-io-path.md](../../spec/3-5-core-io-path.md) §2-§3, [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md), [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md) |
| IO/Path | `path.security.incident_count` | `tests/data/core_path/security/*.json`・`tests/data/core_path/normalize_{posix,windows}.json` を対象に `scripts/validate-diagnostic-json.sh --pattern core.path.security --pattern metadata.security.reason tests/data/core_path/security/*.json reports/spec-audit/ch3/path_security-*.json` と `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario path_security --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/path_security-metrics.json --require-success` を連携実行し、Severity=`error` の `core.path.security.*` 診断件数を集計する。件数は `reports/spec-audit/ch3/core_io_summary-YYYYMMDD.md` へ追記し、累積が 5 件を超えた場合は `3-5-core-io-path-remediation.md` §4.2 TODO を再開する。 | Phase3 `core-io-path` ジョブ（Nightly）と週次レビュー | [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md)#42-セキュリティヘルパ, [3-5-core-io-path-remediation.md](./3-5-core-io-path-remediation.md)#4, [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md), [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md) |
| IO/Path | `watcher.audit.pass_rate` | `compiler/rust/runtime/benches/bench_core_io.rs` と `tests/data/core_io/watcher/*.json`（WatchEvent 合成ケース）を `cargo test --manifest-path compiler/rust/runtime/Cargo.toml watch::tests:: -- --include-ignored` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --source reports/spec-audit/ch3/watch_event-metrics.json --output reports/spec-audit/ch3/watcher-audit.json --require-success` の順で検証し、`Watcher` 系診断 (`core.io.watch.*`) が `metadata.io.async_queue`, `metadata.io.capability`, `effect {io.async}` を欠落なく出力した割合。0.95 未満になった場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#core-io-watcher-risk` を再開し、`watch.queue_size` のプロファイル収集を強化する。 | Phase3 `core-io-path` ジョブ（Watcher 実装が有効化される W49 以降）、週次レビュー | [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md)#51-ファイル監視api, [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)#5, [assets/core-io-effects-matrix.md](assets/core-io-effects-matrix.md), [compiler/rust/runtime/benches/bench_core_io.rs](../../compiler/rust/runtime/benches/bench_core_io.rs) |
| IO/Path | `core_io.benchmark.copy_throughput_mb_s` | `reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` の `core_io.reader_copy_*` から 1 秒あたりの転送 MB を算出し、Phase 2 ベースラインとの差分が ±15% を超えた場合に失敗扱いとする。`cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --features \"core-io core-path\" --bench bench_core_io -- --noplot` を夜間ジョブで実行し、`watch_event_batch`/`buffered_read_line`/`path_normalize` の値も同 JSON に書き戻す。 | Phase3 `core-io-path` 週次ジョブ、およびリリース前レビュー時 | [3-5-core-io-path-plan.md](./3-5-core-io-path-plan.md)#72-io-性能ベンチマーク, [docs/plans/rust-migration/3-2-benchmark-baseline.md](../rust-migration/3-2-benchmark-baseline.md), [reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json](../../reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json) |
| 数値/時間 | `numeric.quantiles.mem_bytes` | `take_numeric_effects_snapshot()` が報告する `effect {mem}` / `mem_bytes` を `quantiles` 実行後に観測し、サンプル数とポイント数に応じたコピー量が記録されているかを確認する指標。`python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario statistics_accuracy --quantiles-source tests/expected/numeric_quantiles.json --require-success` を実行し、`reports/spec-audit/ch3/numeric_statistics_metrics.json` に `mem_bytes` と `points`/`samples` を保存する。 | Phase3 `core-numeric-time` ジョブ（`statistics_accuracy` シナリオ）および週次レビュー | [docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md](./3-4-core-numeric-time-plan.md) §3.1, [compiler/rust/runtime/src/numeric/effects.rs](../../compiler/rust/runtime/src/numeric/effects.rs) |
| 数値/時間 | `numeric.finance.audit_success_rate` | `currency_add`/`net_present_value` が `numeric.finance.*` メタデータ（`currency_code`/`scale`）と `NumericErrorKind::UnsupportedCurrency` を欠落なく出力した割合。`tests/data/numeric/finance/unsupported_currency.json` を `scripts/validate-diagnostic-json.sh --suite numeric` で検証し、結果を `reports/spec-audit/ch3/numeric_finance-metrics.json` に保存して週次レビュー時に確認する。 | Phase3 `core-numeric-time` ジョブ（finance ケース）＋週次レビュー | [docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md](./3-4-core-numeric-time-plan.md) §2.4, [compiler/rust/runtime/src/numeric/finance.rs](../../compiler/rust/runtime/src/numeric/finance.rs) |
| 数値/時間 | `time.timezone.lookup_consistency` | `tests/data/time/timezone_cases.json` の `cases[].platform_offsets` と `offset_seconds` が一致しているかを `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario timezone_lookup --tz-source tests/data/time/timezone_cases.json --output reports/spec-audit/ch3/time_timezone_lookup.json --require-success` で検証する指標。Linux/macOS/Windows の offset 差異が検出された場合は `docs/notes/runtime-capability-stage-log.md` へ即時記録し、Capability Stage の再確認を行う。 | Phase3 `core-numeric-time` ジョブ（`timezone_lookup` シナリオ）＋週次レビュー | [docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md](./3-4-core-numeric-time-plan.md) §4.2, [compiler/rust/runtime/src/time/timezone.rs](../../compiler/rust/runtime/src/time/timezone.rs), [docs/notes/runtime-capability-stage-log.md](../../docs/notes/runtime-capability-stage-log.md) |
| 数値/時間 | `time.syscall.latency_ns` | `tests/expected/time_{now,sleep}.json` を入力に `python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario clock_accuracy --time-source tests/expected/time_now.json --time-source tests/expected/time_sleep.json --output reports/spec-audit/ch3/time_clock_accuracy.json --require-success` を実行し、`TimeSyscallMetrics` が観測した `total_latency_ns/calls`（平均）と `max_latency_ns` を記録。`time.env.*` を含むメタデータを同レポートと `reports/spec-audit/ch3/time_env-bridge.md` に保存し、`now`/`sleep` API のレイテンシが許容範囲（1ms 以内）であることを確認する。 | Phase3 `core-numeric-time` ジョブ（`clock_accuracy` シナリオ）＋週次レビュー | [docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md](./3-4-core-numeric-time-plan.md) §4.1, [docs/plans/bootstrap-roadmap/3-4-core-numeric-time-gap-plan.md](./3-4-core-numeric-time-gap-plan.md) §2, [reports/spec-audit/ch3/time_env-bridge.md](../../reports/spec-audit/ch3/time_env-bridge.md) |
| 数値/時間 | `metrics.emit.success_rate` | `MetricPoint` → `AuditSink` 経路が `metric_point.*` / `effect.capability = "metrics.emit"` / `effect.stage.required = stable` を欠落なく出力した割合。`python3 tooling/ci/collect-iterator-audit-metrics.py --section numeric_time --scenario emit_metric --metric-source tests/data/metrics/metric_point_cases.json --output reports/spec-audit/ch3/metric_point-emit_metric.json --require-success` と `scripts/validate-diagnostic-json.sh --pattern metrics.emit` を実行し、`reports/spec-audit/ch3/metric_point-emit_metric.json` および `reports/audit/metric_point/*.audit.jsonl` の `cases[].missing_keys` / `audit.metadata` が `null` であることを確認する。 | Phase3 `core-numeric-time` ジョブ（`emit_metric` シナリオ）および週次レビュー | [docs/spec/3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md) §4, [docs/spec/3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md), [compiler/rust/runtime/src/diagnostics/audit_bridge.rs](../../compiler/rust/runtime/src/diagnostics/audit_bridge.rs), [compiler/rust/runtime/src/diagnostics/metric_point.rs](../../compiler/rust/runtime/src/diagnostics/metric_point.rs), [docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md](./3-4-core-numeric-time-plan.md) §5.1 |
| テキスト | `text.builder.mem_throughput` | `benchmarks/text/builder.rs` の `push_{str,bytes,grapheme}_finish` が計測した `criterion` Throughput（MB/s）と `take_text_effects_snapshot()` による `collector.effect.mem_bytes` をセットで記録する指標。`reports/benchmarks/core_text/phase3-baseline.md` に 3 ケースの値を保存し、`text.mem.zero_copy_ratio` と併せて ±15% 内を確認する。 | Phase3 `phase3-core-text` ジョブ、TextBuilder 回帰テスト後 | [3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §6.2, [reports/benchmarks/core_text/phase3-baseline.md](../../reports/benchmarks/core_text/phase3-baseline.md) |
| テキスト | `unicode.conformance.pass_rate` | `cargo test unicode_conformance --features unicode_full --manifest-path compiler/rust/runtime/Cargo.toml` の結果を解析し、`tests/data/unicode/UAX29/*` / `UAX15/*` の全ケースが合格した割合。失敗時は `reports/spec-audit/ch1/unicode_conformance_failures.md` に記録し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` へも登録する。 | CI（`phase3-core-text` ナイトリー＋週次レビュー） | [reports/spec-audit/ch1/unicode_conformance_failures.md](../../reports/spec-audit/ch1/unicode_conformance_failures.md), [docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md](./checklists/unicode-conformance-checklist.md) |
| 型クラス | `typeclass.metadata_pass_rate` | `extensions.typeclass` / `audit_metadata.typeclass.*` が完全に埋まっている割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | 同 §1.4 |
| 型クラス | `typeclass.dictionary_pass_rate` | `extensions.typeclass.dictionary.*` と `AuditEnvelope.metadata["typeclass.dictionary.*"]` が `kind != "none"` で揃っている割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) §B, [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| 型推論 | `type_inference.value_restriction_violation` | Strict モードで `type_inference.value_restriction_violation` 診断が 0 件であることを確認（検出時はブロッカー） | CI（`tooling/ci/collect-iterator-audit-metrics.py --require-success` 実行時） | [1-2-types-Inference.md](../../spec/1-2-types-Inference.md) §C.3, [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §B |
| 型推論 | `type_inference.value_restriction_legacy_usage` | Legacy モードを用いた回数（Strict モード移行中の監視指標。値は報告のみでゲート対象外） | CI（`collect-iterator-audit-metrics.py --summary` 出力） | 同上 |
| 安全性 | `effect_analysis.missing_tag` | 効果解析で検出漏れとなったタグ数（0件が合格、1件以上でブロッカー） | CI（`dune runtest` 実行後にメトリクス集計） | [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §3 |
| 安全性 | `effect.capability_array_pass_rate` | 複数 Capability を要求する `effects` 診断で `required_capabilities` / `actual_capabilities` 配列が CLI/LSP/監査ログすべてに揃って出力された割合（0.0〜1.0）。`diagnostics.effect_stage_consistency` は Stage ミスマッチの存在検知を担い、本指標は配列欠落の有無を確認する。 | `tooling/ci/collect-iterator-audit-metrics.py --require-success` 実行後に `iterator` セクションへ併記 | [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §I, [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) §1.2 |

### 0.3.1a Core.Collections 永続構造メトリクス
- `compiler/rust/runtime/ffi/src/core_collections_metrics.rs` が生成する `ListPersistentPatch`（要素 100k）と `MapPersistentMerge`（キー 50k）の測定値を `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に保存する。ピークメモリはそれぞれ 1.7158 倍 / 1.3903 倍で、Phase 3 M2 の閾値（≤1.8）を満たした。`List`/`PersistentMap` のノード共有率はともに 1.0 で、共有ノードを 50% コストで算入する推定式を `compiler/rust/runtime/src/collections/persistent/list.rs:151` と `compiler/rust/runtime/src/collections/persistent/btree.rs:189` に実装済み。
- 再計測時は `cargo run --manifest-path compiler/rust/runtime/ffi/Cargo.toml --features core_prelude --example core_collections_metrics -- docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` を実行し、CSV を `git` に保存した上で `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md#23-構造共有メトリクスとレポート化` と本章にリンクする。
| 効果 | `syntax.effect_construct_acceptance` | 効果構文（`perform`/`handle`）が PoC 仕様どおりに受理された割合。`effect_syntax.constructs` のうち `diagnostics` にエラーが含まれないケースを分子とし、Phase 2-5 では 0.0（PoC 未実装）を許容値、Phase 2-7 以降は 1.0 を合格条件とする。 | `tooling/ci/collect-iterator-audit-metrics.py --section effects --require-success`（Phase 2-7 で実装予定） | [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §G–I, [docs/plans/bootstrap-roadmap/2-5-proposals/SYNTAX-003-proposal.md](./2-5-proposals/SYNTAX-003-proposal.md) §S3 |
| 効果 | `effects.syntax_poison_rate` | 効果構文の検証で未捕捉タグ（`diagnostics` に `effects.contract.residual` などが含まれるケース）が発生していない割合。PoC 期間は 1.0 を維持することを確認し、正式実装で回帰した場合はブロッカー扱いとする。 | `tooling/ci/collect-iterator-audit-metrics.py --section effects --require-success`（Phase 2-7 で実装予定） | [1-3-effects-safety.md](../../spec/1-3-effects-safety.md) §I, [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §2 |
| 標準ライブラリ | `core_prelude.missing_api` | `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `spec` 定義と Rust 実装の対応表で未実装となっている件数。0 件で合格、1 件以上は WBS 2.1b の未完了扱い。2025-11-20 実測値: 0（`reports/spec-audit/ch0/links.md#prelude-実装ログ` に `cargo xtask prelude-audit --strict` の結果を保存）。 | `cargo xtask prelude-audit --strict --wbs 2.1b --baseline docs/spec/3-1-core-prelude-iteration.md`（Nightly + リリース判定前） | [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md), [3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §2.1 |
| 標準ライブラリ | `iterator.api.coverage` | Iter/Collector API の実装率（`implemented_entries / total_entries`）。2025-11-20 実測値: 1.0（`reports/spec-audit/ch1/iter.json` の `missing_entries = []`）。 | `cargo xtask prelude-audit --section iter --strict --baseline docs/spec/3-1-core-prelude-iteration.md`（Nightly + リリース判定前） | [reports/spec-audit/ch1/iter.json](../../reports/spec-audit/ch1/iter.json), [reports/spec-audit/ch0/links.md](../../reports/spec-audit/ch0/links.md#iter-f3) |
| 標準ライブラリ | `iterator.adapter.coverage` | Adapter API (map/filter/flat_map/zip/buffered/collect_* など 12 件) の実装率。2027-02-24 実測値: 1.0（`reports/spec-audit/ch1/core_iter_adapters.json` の `missing_entries = []` を `reports/iterator-adapter-metrics.json` で再計測）。 | `cargo xtask prelude-audit --section iter --filter adapter --strict` ＋ `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case adapters --source reports/spec-audit/ch1/core_iter_adapters.json --output reports/iterator-adapter-metrics.json --require-success` | [reports/spec-audit/ch1/core_iter_adapters.json](../../reports/spec-audit/ch1/core_iter_adapters.json), [reports/iterator-adapter-metrics.json](../../reports/iterator-adapter-metrics.json), [reports/spec-audit/ch0/links.md](../../reports/spec-audit/ch0/links.md#iter-adapters-g4) |
| 標準ライブラリ | `core_prelude.guard.failures` | `core.prelude.ensure_failed` 診断が `tooling/ci/collect-iterator-audit-metrics.py --section prelude-guard` で検出された件数。0 件が合格で、検出時は Phase 3 M1 ブロッカーとして `0-4-risk-handling.md` へ登録する。 | Nightly CI + `scripts/validate-diagnostic-json.sh` 実行後（`reports/spec-audit/ch0/links.md` に JSON ログを残す） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §3, [3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §2.2 |
| 標準ライブラリ | `core_prelude.panic_path` | `compiler/rust/frontend/tests/panic_forbidden.rs` と `RUSTFLAGS="-Dnon-fmt-panics -Z panic-abort-tests"` で検出した `panic!`/`unwrap_unchecked` 経路数（`effect {debug}` 以外）。0 件が合格で、1 件以上は `4-5-backward-compat-checklist.md` の回帰手順で再検証する。 | Nightly CI + リリース前の `cargo test panic_forbidden`・`scripts/validate-diagnostic-json.sh` 実行後 | [1-3-effects-safety.md](../../spec/1-3-effects-safety.md), [3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §2.3 |
| 標準ライブラリ | `iterator.range.overflow_guard` | `Iter::range` のオーバーフロー検出が `IterRangeError::Overflow` 経由で `iterator.range.overflow_guard` に 1 以上記録されているか（未記録ならブロッカー）。 | `collect-iterator-audit --section iter --case range --output reports/spec-audit/ch1/iter.json` 実行後に KPI を確認 | [3-1-core-prelude-iteration.md](../../spec/3-1-core-prelude-iteration.md) §3, [reports/spec-audit/ch1/iter.json](../../reports/spec-audit/ch1/iter.json) |
| 標準ライブラリ | `iterator.repeat.flagged` | `Iter::repeat` が `diagnostic.extensions["iterator.repeat"]=true` を監査ログでも報告できているか（true で合格）。 | `collect-iterator-audit --section iter --case repeat`（F1-3 生成 API テスト） | 同上 |
| 標準ライブラリ | `iterator.once.length` / `iterator.empty.items` | `Iter::once` が 1 要素、`Iter::empty` が 0 要素として集計されているか。`iterator.once.length=1`、`iterator.empty.items=0` 以外はブロッカー。 | `collect-iterator-audit --section iter --case once --case empty` を Nightly で実行 | [reports/spec-audit/ch0/links.md#iter-generators](../../reports/spec-audit/ch0/links.md#iter-generators), [docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §3.3b |
| 標準ライブラリ | `iterator.map.latency` | `Iter::from_list |> Iter::map |> Iter::collect_vec` パイプライン（`core_iter_adapters.rs::map_pipeline`）の平均レイテンシ (ms)。Phase 2 ベースライン（`docs/plans/rust-migration/3-2-benchmark-baseline.md`）比 ±10% 以内が目標。 | Nightly CI (`cargo test core_iter_adapters -- --nocapture` + `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case map --output reports/iterator-map-filter-metrics.json`) | [docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §4, [reports/spec-audit/ch0/links.md#iter-g1-map-filter](../../reports/spec-audit/ch0/links.md#iter-g1-map-filter) |
| 標準ライブラリ | `iterator.filter.predicate_count` | `Iter::filter` の `EffectLabels::predicate_calls` が入力要素数と一致しているかを示す指標（`core_iter_adapters.rs::filter_effect` snapshot で測定）。`IterDriver` が `EffectSet` を更新するため、逸脱は効果タグ配線の不備としてブロッカー扱い。 | Nightly CI（`python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case filter --output reports/iterator-map-filter-metrics.json` + `scripts/validate-diagnostic-json.sh --pattern iterator.filter`） | 同上 |
| 標準ライブラリ | `iterator.mem.window` | `Iter::buffered` のリングバッファ容量（`EffectLabels.mem_bytes`）とバックプレッシャ率（`dropped / produced`）。`core_iter_adapters.rs::buffered_window` snapshot と `cargo bench -p compiler-rust-frontend iter_buffered` の結果を `reports/iterator-buffered-metrics.json` / `reports/benchmarks/iter_buffered-YYYY-MM-DD.json` に保存し、±10% 以内の性能維持を確認する。 | Nightly + 週次ベンチ（`cargo test ... buffered_window` → `cargo bench -p compiler-rust-frontend iter_buffered` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success`） | [docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §4.a G3, [reports/spec-audit/ch0/links.md#iter-buffered](../../reports/spec-audit/ch0/links.md#iter-buffered), [docs/plans/rust-migration/3-2-benchmark-baseline.md](../rust-migration/3-2-benchmark-baseline.md) §3.2.4 |
| 標準ライブラリ | `iterator.collect.bridge_parity` | `Iter::collect_list`/`collect_vec`/`collect_string` が `Collector` 直接呼びと同じ Stage/Effect/診断拡張（`Diagnostic.extensions["prelude.collector.*"]`、`AuditEnvelope.metadata["prelude.collector.*"]`）を出力するか。2027-03-06 実測値: `iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem_reservation = 4`（Vec）、`collector.error.invalid_encoding = 1`（String）、`diagnostic.audit_presence_rate = 1.0`。 | Nightly CI で `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_terminators` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case terminators --source reports/spec-audit/ch1/core_iter_terminators.json --output reports/iterator-collector-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.collect --pattern prelude.collector` を直列実行 | [reports/spec-audit/ch0/links.md#iter-terminators-h1](../../reports/spec-audit/ch0/links.md#iter-terminators-h1), [docs/notes/core-library-outline.md#collector-f2-監査ログ](../notes/core-library-outline.md#collector-f2-監査ログ), [docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §4.b H1 |
| 標準ライブラリ | `iterator.flat_map.mem_reservation` | `core_iter_adapters.rs::flat_map_vec` で `Iter::flat_map` がネストした `Iter` を展開する際に確保するメモリ量（`EffectLabels::mem_reservation_bytes`）を監査。`reports/iterator-flatmap-metrics.json` の `adapter_metrics.flat_map_vec.effects.mem` と連動。 | Nightly CI（`cargo test core_iter_adapters -- --include-ignored flat_map_vec` + `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case flat_map --output reports/iterator-flatmap-metrics.json --require-success`） | [docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md](./3-1-core-prelude-iteration-plan.md) §4.a, [reports/spec-audit/ch0/links.md#iter-adapters](../../reports/spec-audit/ch0/links.md#iter-adapters) |
| 標準ライブラリ | `iterator.zip.shorter_error_rate` | `Iter::zip` が長さ差を検出した際に `iterator.error.zip_shorter` を確実に診断へ書き込めているかを示す指標。`reports/iterator-zip-metrics.json` の `adapter_metrics.zip_mismatch.iterator.error.zip_shorter` を 0（未検出）/1（検出）で監視し、`reports/diagnostic-format-regression.md#iterator.zip_mismatch` と整合させる。 | Nightly CI（`cargo test core_iter_adapters -- --include-ignored zip_mismatch` + `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case zip --output reports/iterator-zip-metrics.json --require-success` + `scripts/validate-diagnostic-json.sh --pattern iterator.zip`） | 同上 |
| 標準ライブラリ | `iterator.api.coverage` | `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の `module="Iter"` エントリが仕様 3-1 の生成 API を網羅し、`rust_status`/`wbs`/`pending_entries` を `reports/spec-audit/ch1/iter.json` に同期できている割合。`cargo xtask prelude-audit` の出力で 1.0 を維持する。 | `cargo xtask prelude-audit --section iter --baseline docs/spec/3-1-core-prelude-iteration.md --wbs 3.1c-F1-5`（Nightly + M1 Go/No-Go 判定） | [reports/spec-audit/ch1/iter.json](../../reports/spec-audit/ch1/iter.json), [reports/spec-audit/ch0/links.md#iter-generators](../../reports/spec-audit/ch0/links.md#iter-generators), [docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md#33b-生成-api-実装ステップ（wbs-31c-f1）](./3-1-core-prelude-iteration-plan.md#33b-生成-api-実装ステップ（wbs-31c-f1）) |
| 効果 | `diagnostics.effect_row_stage_consistency` | CLI/LSP/監査の効果行出力が一致した割合。`extensions["effects"].row.*` と `AuditEnvelope.metadata["effect.row.*"]` が完全一致したケースを分子とし、1.0 未満の場合は効果行統合をロールバックする。 | `tooling/ci/collect-iterator-audit-metrics.py --require-success --section effects`（Phase 2-7 Sprint B で実装） | [docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md](./2-5-proposals/TYPE-002-proposal.md) Step4, [docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md](./2-7-deferred-remediation.md)#type-002-effect-row-integration |
| 型推論 | `type_effect_row_equivalence` | 効果行を含む型等価テスト（`type_effect_row_equivalence_*`）が成功した割合。宣言順差異・残余効果差分・`@handles` 整合の各ケースを分母とし、失敗時は `Type_unification` 実装をブロッカー扱いにする。 | `dune runtest compiler/ocaml/tests/test_type_inference.ml` 実行後に `collect-iterator-audit-metrics.py --section effects` で集計 | [docs/plans/bootstrap-roadmap/2-5-review-log.md](./2-5-review-log.md#type-002-step4-実装ロードマップとテスト観点2026-04-24), [compiler/ocaml/tests/test_type_inference.ml](../../compiler/ocaml/tests/test_type_inference.ml) |
| 効果 | `effect_row_guard_regressions` | `RunConfig.extensions["effects"].type_row_mode` 切替時に `effects.type_row.integration_blocked` が発火した件数。`metadata-only` 解除前は 0 件を維持し、発生時は自動で `ty-integrated` ロールアウトを停止する。 | `tooling/ci/collect-iterator-audit-metrics.py --require-success --section effects`（Phase 2-7 Sprint B） | [docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md](./2-7-deferred-remediation.md)#type-002-effect-row-integration, [docs/plans/bootstrap-roadmap/2-5-proposals/TYPE-002-proposal.md](./2-5-proposals/TYPE-002-proposal.md) Step4 |
| 安全性 | `ffi_bridge.audit_pass_rate` | `ffi.contract.*` 診断で `AuditEnvelope.metadata.bridge.*` と拡張フィールドが揃った割合 (0.0〜1.0) | CI（週次レビュー、PRごと） | [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md), [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) |
| Parser | `parser.parse_result_consistency` | `Parser_driver.run` と `run_partial` が生成する `ParseResult`（`value`/`span`/`diagnostics`/`consumed`/`committed`）の一致率。1.0 未満の場合は `parser_driver_tests.ml` の差分レポートを添付。 | CI（`dune runtest tests`、parser_driver シナリオ） | [2-1-parser-type.md](../../spec/2-1-parser-type.md) §A, [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §G |
| Parser | `parser.core_comb_rule_coverage` | Core Parse 経由で発行された構文診断が `extensions.parse.parser_id` と `parser.core.rule.*` 監査メタデータを揃えている割合 (0.0〜1.0)。欠落時はブロッカー。 | CI（`tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-2-core-combinator.md](../../spec/2-2-core-combinator.md) §A, [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §B |
| Parser | `parser.stream.outcome_consistency` | `run_stream` と `run` の結果（AST・診断・`stream_meta`）が一致する割合。1.0 未満の場合は `streaming_runner_tests.ml` の差分を添付し、PoC 時点での逸脱をブロッカー扱いとする。 | CI（`dune runtest compiler/ocaml/tests/streaming_runner_tests.ml` + `tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-7-core-parse-streaming.md](../../spec/2-7-core-parse-streaming.md) §A, [docs/guides/core-parse-streaming.md](../../guides/core-parse-streaming.md) §10 |
| Parser | `parser.stream.demandhint_coverage` | CLI/LSP/監査ログに `stream_meta` と `DemandHint` の必須フィールド（`min_bytes` / `preferred_bytes` / `resume_hint` / `last_reason`）が欠落なく出力された割合 (0.0〜1.0)。欠落時は CI を失敗させる。 | CI（`scripts/validate-diagnostic-json.sh` 実行後／`tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §B, [2-7-deferred-remediation.md](2-7-deferred-remediation.md) §6 |
| Parser | `parser.stream.bridge_backpressure_diagnostics` | `PendingReason::Backpressure` 発生時に `bridge.stage.backpressure` 診断が欠落なく出力された割合 (0.0〜1.0)。0.0 の場合は Runtime Bridge Stage または `stream_signal` の配線不備としてブロッカー登録する。 | 週次 CI（`tooling/ci/collect-iterator-audit-metrics.py --section streaming --platform windows-msvc --require-success`） | [docs/spec/3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) §10.5, [docs/guides/runtime-bridges.md](../../guides/runtime-bridges.md) §10.5 |
| Parser | `parser.stream.bridge_stage_propagation` | Backpressure Signal を起点とした Stage 差異が `effects.contract.stage_mismatch` 診断へ伝搬した割合 (0.0〜1.0)。未達時は `0-4-risk-handling.md` に再調整タスクを登録する。 | 週次 CI（同上。必要に応じて `--platform macos-arm64` を追加実行） | 同上および [docs/spec/3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §8 |
| Parser | `parser.packrat_cache_hit_ratio` | Packrat メモ化のヒット率（`hits / queries`）。計測値が存在しない場合は警告として扱い、比率が 0.85 未満で要調査。 | CI（同上） | [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §E |
| Parser | `parser.farthest_error_offset` | `DiagState.farthest_error_offset` が報告する最遠エラー位置（バイトオフセット）。`None` または 0 の場合は回復ロジックが無効化されているとみなしブロッカー登録。 | CI（`test_parse_result_state.ml` / CLI 失敗シナリオ） | 同上 |
| Parser | `parser.use_nested_support` | 多段ネストを含む `use` 宣言が AST で再帰展開され、`Module_env.flatten_use_decls` が 100% 成功する割合（0.0〜1.0）。 | CI（`dune runtest compiler/ocaml/tests/test_parser.exe` と `test_module_env.exe` 完了後、`tooling/ci/collect-iterator-audit-metrics.py --summary`） | [1-1-syntax.md](../../spec/1-1-syntax.md) §B.1, [1-5-formal-grammar-bnf.md](../../spec/1-5-formal-grammar-bnf.md) §1 |
| Parser | `parser.expected_summary_presence` | 構文エラー診断で `expected.alternatives` が空でない割合（0.0〜1.0）。検出件数が 0 の場合は CI を失敗させる。 | CI（`scripts/validate-diagnostic-json.sh` 実行後／`tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-5-error.md](../../spec/2-5-error.md) §B, [docs/spec/3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| Parser | `parser.recover_fixit_coverage` | `extensions["recover"]` が `sync_tokens`/`hits`/`strategy`/`has_fixits`/`notes` を揃えて出力し、CLI/LSP/ストリーミング経路が同一 FixIt を提示する割合 (0.0〜1.0)。欠落時はブロッカー。 | CI（`dune runtest parser_recover_tests`・`streaming_runner_tests` → `scripts/validate-diagnostic-json.sh` → `tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-5-error.md](../../spec/2-5-error.md) §D, [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| Parser | `parser.expected_tokens_per_error` | `expected.alternatives` に含まれる候補数の平均値。0.0 の場合は期待集合が欠落しているため即時調査する。 | 同上 | 同上 |
| Parser | `parser.runconfig_switch_coverage` | RunConfig スイッチ（packrat/left_recursion/trace/merge_warnings）が JSON と監査ログに記録された割合（0.0〜1.0、4 項目すべて観測で 1.0）。 | CI（`dune runtest parser` → `scripts/validate-diagnostic-json.sh` → `tooling/ci/collect-iterator-audit-metrics.py`） | [2-1-parser-type.md](../../spec/2-1-parser-type.md) §D, [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §B |
| Parser | `parser.runconfig_extension_pass_rate` | `RunConfig.extensions["lex"|"recover"|"stream"]` が CLI/LSP JSON と監査ログへ伝搬した割合（0.0〜1.0）。 | 同上 | [2-1-parser-type.md](../../spec/2-1-parser-type.md) §D, [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §C |
| Parser | `parser.packrat_cache_hit_ratio` | Packrat キャッシュヒット数 / Packrat 参照回数。`RunConfig.packrat=true` のテスト実行時に 0.6 以上を目安とし、未達時は `Core_parse.Packrat_cache` のキー衝突を調査する（PARSER-003 Step4 で設計合意、実装は Step5）。 | Packrat 有効化テスト（`dune runtest parser` → `tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-6-execution-strategy.md](../../spec/2-6-execution-strategy.md) §B, [docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md](./2-5-proposals/PARSER-003-proposal.md) §5 |
| Parser | `parser.recover_sync_success_rate` | `recover` 適用時に同期トークンへ到達できた割合（成功回数 / recover 呼出回数）。期待値は 0.8 以上。`RunConfig.extensions["recover"]` で同期トークンを設定したケースのみ対象（Step5 で収集実装予定）。 | 回復テスト（`dune runtest tests/test_cli_diagnostics.exe` → `tooling/ci/collect-iterator-audit-metrics.py --require-success`） | [2-5-error.md](../../spec/2-5-error.md) §D, [docs/plans/bootstrap-roadmap/2-5-proposals/PARSER-003-proposal.md](./2-5-proposals/PARSER-003-proposal.md) §5 |
| Parser | `lexer.shared_profile_pass_rate` | `RunConfig.extensions["lex"].profile` が `run_config` 出力と `audit_metadata`／`extensions.runconfig.extensions.lex` で一致し共有される割合（0.0〜1.0）。`space_id` が記録されたサンプルはメトリクス出力に併記する。 | CI（`tooling/ci/collect-iterator-audit-metrics.py --summary`） | [2-3-lexer.md](../../spec/2-3-lexer.md) §G, [2-1-parser-type.md](../../spec/2-1-parser-type.md) §D |
| Parser | `lexer.identifier_profile_unicode` | `parser.runconfig.lex.profile` と `AuditEnvelope.metadata["parser.runconfig.lex.profile"]` が `unicode` に揃ったサンプル比率（0.0〜1.0）。Phase 2-5 は `--lex-profile=ascii` 既定のため 0.0 を記録し、Phase 2-7 `lexer-unicode` 着手時に `--lex-profile=unicode` を有効化して 1.0 達成を確認する。`tooling/ci/collect-iterator-audit-metrics.py` が `parser` セクションで計測し、ASCII/Unicode の件数内訳と CLI スイッチ設計（`--lex-profile=ascii|unicode`）を同レポートへ出力する。 | Phase 2-7 受け入れテスト（`lexer-unicode`）と週次レビュー／Phase 2-5 は指標ログのみ | [2-3-lexer.md](../../spec/2-3-lexer.md) D-1, [docs/plans/bootstrap-roadmap/2-5-proposals/LEXER-001-proposal.md](./2-5-proposals/LEXER-001-proposal.md), [docs/plans/bootstrap-roadmap/2-5-review-log.md](2-5-review-log.md#lexer-001-step2-仕様脚注と索引の整備2026-02-18) ／ [同 Step4 記録](2-5-review-log.md#lexer-001-step4-測定指標と-ci-スイッチの定義2026-03-21) |
| Parser/Diagnostics | `unicode.diagnostic.display_span` | `UnicodeError` を原因とする診断が `unicode.error.kind`/`unicode.error.offset` と `primary.span`（`Span::new(offset, offset+grapheme_len)`）を両方出力できた割合 (0.0〜1.0)。`compiler/rust/frontend/tests/lexer_unicode_identifier.rs` と `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` を突合し、`scripts/validate-diagnostic-json.sh --pattern unicode.error.kind --pattern unicode.identifier.raw reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` が成功することをもって合格とする。`display_width` は `reports/spec-audit/ch1/unicode_diagnostics-20270330.json` を `scripts/validate-diagnostic-json.sh --pattern unicode.display_width ...` で検証し、`extensions["unicode"]` と `AuditEnvelope.metadata["unicode.display_width"]` の両方に記録されていることを確認する。 | Phase3 `phase3-core-text` ジョブで Unicode 識別子テスト（`cargo test --manifest-path compiler/rust/frontend/Cargo.toml lexer_unicode_identifier -- --nocapture`）を実行した際、および `reports/spec-audit/ch1/lexer_unicode_identifier-*.json` 更新時 | [docs/spec/2-5-error.md](../../spec/2-5-error.md) §B, [docs/spec/3-3-core-text-unicode.md](../../spec/3-3-core-text-unicode.md) §5.1, [docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md](./3-3-core-text-unicode-plan.md) §3.3 |
| DX | `diagnostic_regressions` | 診断差分の件数 | PR ごと | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) |
| DX | `diagnostic.info_hint_ratio` | `Severity = Info` または `Hint` の診断件数 / 全診断件数。情報診断が十分に発行されているか、ヒント診断が過剰になっていないかを監視する。 | CI（`scripts/validate-diagnostic-json.sh` 実行後／`tooling/ci/collect-iterator-audit-metrics.py --summary`） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1, [2-5-error.md](../../spec/2-5-error.md) |
| DX | `diagnostic.hint_surface_area` | `Severity = Hint` の診断が指し示す `primary` + `secondary` Span の総バイト長 / 解析対象バイト長。Phase 2-7 で JSON 集計を実装し、ヒントが局所的に集中していないかを測定する。 | 週次レビュー（Phase 2-7 以降に本集計を有効化） | [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §1 |
| DX | `audit_diff.regressions` | `tooling/review/audit-diff.py` が算出した `diagnostic.regressions + metadata.changed` 件数 | PR ごと | 同上／`reports/diagnostic-format-regression.md` |
| DX | `audit_dashboard.generated` | 直近の `audit_dashboard` ジョブが成功し `reports/audit/dashboard/index.html` を出力した回数 | 週次レビュー | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §5.2 |
| DX | `audit_query.coverage` | DSL プリセット（Stage/FFI/型クラス）でヒットした監査ログ数 / 全監査ログ数 | PR ごと | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §5.3 |
| DX | `error_resolution_latency` | 重大バグの修正までの日数 | 月次 | [0-1-project-purpose.md](../../spec/0-1-project-purpose.md) §2.2 |

- `collections.audit_bridge_pass_rate` および `collector.effect.audit_presence` は `REML_COLLECTIONS_CHANGE_SET[_PATH]` を介して `ChangeSet` JSON を CLI に注入し、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の `collections.diff.*` を生成することが前提である。`collect-iterator-audit-metrics.py --section collectors --scenario map_set_persistent --require-audit` が `collections.diff.items` や `collector.effect.audit`/`collector.effect.mem_bytes` の整合性をチェックし、`scripts/validate-diagnostic-json.sh --pattern collections.diff --pattern collector.effect.audit --pattern collector.effect.mem_bytes reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` で `kind`/`key`/`value` とメタデータが揃っていることを検証する工程を設ける必要がある【F:../docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md†L316-L333】【F:../scripts/validate-diagnostic-json.sh†L1-L200】。

- CI 集計スクリプト: `tooling/ci/collect-iterator-audit-metrics.py` を用いて診断 JSON を検査し、結果を `tooling/ci/iterator-audit-metrics.json` に書き出す。`metrics[]` 配列には `diagnostic.audit_presence_rate`・iterator・FFI ブリッジ指標に加えて `review` セクション（`audit_diff.regressions`, `audit_query.coverage`）が含まれ、`diagnostics.summary` では `info_fraction` / `hint_fraction` / `info_hint_ratio` を併記する。必須フィールド欠落時は `pass_rate = 0.0` に丸める。Linux / macOS / Windows いずれの CI ワークフローでも `--require-success` オプションを有効化し、`pass_rate` が 1.0 未満または DX 指標が閾値を超えた場合にジョブ全体を失敗させる。

### 0.3.1a レビュー支援ツール連携
- 差分検知: `tooling/review/audit-diff.py --base reports/audit/baseline.jsonl --target reports/audit/current.jsonl --output reports/audit/review/<commit>/diff` を CI/ローカル双方で実行し、`diff.json` に記録された `diagnostic.regressions`, `metadata.changed`, `pass_rate.delta` を `audit_diff.regressions` の計測値として採用する。PR テンプレートでは差分サマリを貼り付け、`reports/diagnostic-format-regression.md` のチェックリストに沿ってレビューする。CI 側では `collect-iterator-audit-metrics.py --section review --review-diff reports/audit/review/<commit>/diff.json` を呼び出し、差分指標を取得する。
- コメント投稿: `tooling/ci/publish-audit-diff.py --diff reports/audit/review/<commit>/diff.json` で Markdown コメントを生成し、`actions/github-script` などから PR へ投稿する。`--max-details` で差分テーブルの件数を調整可能。
- 可視化: `tooling/review/audit_dashboard.py --metrics <metrics.json> --render --output reports/audit/dashboard/` により `index.{html,md}` / `metrics.timeseries.csv` / `metrics.snapshot.json` を生成し、`audit_dashboard` CI ジョブでアーティファクト化する。マイルストーン固定値は `reports/audit/dashboard/releases/<milestone>/` に保存し、週次レビュー後に `audit_dashboard.generated` のカウントを更新する。CI 計測は `collect-iterator-audit-metrics.py --section review --review-dashboard reports/audit/dashboard/index.html` を介して行い、生成ファイルが存在しない場合は `failures[]` に記録される。
- クエリ: `tooling/review/audit-query --query-file tooling/review/presets/stage-regressions.dsl --from reports/audit/review/<commit>/target.jsonl --output reports/audit/review/<commit>/query/stage.json` のように DSL プリセットを用いて重点領域を抽出し、ヒット件数を `audit_query.coverage` の計算に利用する。プリセット一覧は `tooling/review/presets/README.md` で管理し、更新時は `docs/spec/3-6-core-diagnostics-audit.md` 付録へ同期する。CI では `collect-iterator-audit-metrics.py --section review --review-coverage reports/audit/review/<commit>/query/stage.json` を通じて集計する。
- いずれのツールも共通の正規化ライブラリ `tooling/review/audit_shared.py` を使用し、スキーマ更新時は当該モジュールと `tooling/json-schema/audit-diff.schema.json` を併せて更新する。更新差分は `reports/audit/review/<commit>/diff.md` に記録し、計測値を `0-3-audit-and-metrics.md` へ転記する。

### メタデータキー定義表（Diagnostic.extensions / AuditEnvelope.metadata）
| ドメイン | キー | `Diagnostic.extensions` | `AuditEnvelope.metadata` | 必須フェーズ | 備考 |
|----------|------|-------------------------|---------------------------|--------------|------|
| CLI 実行 | `cli.audit_id` | `diagnostic.extensions.cli.audit_id` | `metadata["cli.audit_id"]` | Phase 2-3 以降 | `audit_id` を CLI 実行単位で共有し、診断・監査の突合に利用する。 |
| CLI 実行 | `cli.change_set` | `diagnostic.extensions.cli.change_set` | `metadata["cli.change_set"]` | Phase 2-3 以降 | 差分適用対象の識別子。スキーマ v1.1 で追加。 |
| 型クラス | `typeclass.trait` | `extensions.typeclass.trait` | `metadata["typeclass.trait"]` | Phase 2-4 | 制約に対応するトレイト名。 |
| 型クラス | `typeclass.type_args[]` | `extensions.typeclass.type_args[]` | `metadata["typeclass.type_args"]` | Phase 2-4 | 文字列表現された型引数。 |
| 型クラス | `typeclass.constraint` | `extensions.typeclass.constraint` | `metadata["typeclass.constraint"]` | Phase 2-4 | `trait<args...>` 形式の制約表示。 |
| 型クラス | `typeclass.resolution_state` | `extensions.typeclass.resolution_state` | `metadata["typeclass.resolution_state"]` | Phase 2-4 | `resolved` / `stage_mismatch` / `unresolved` / `ambiguous` / `unresolved_typevar` / `cyclic` / `pending`。 |
| 型クラス | `typeclass.dictionary` | `extensions.typeclass.dictionary` | `metadata["typeclass.dictionary"]` | Phase 2-4 | 採用辞書の JSON 表現。`kind = "none"` で辞書欠落を明示。 |
| 型クラス | `typeclass.candidates[]` | `extensions.typeclass.candidates[]` | `metadata["typeclass.candidates"]` | Phase 2-4 | 候補辞書の配列。要素は `typeclass.dictionary` と同構造。 |
| 型クラス | `typeclass.pending[]` | `extensions.typeclass.pending[]` | `metadata["typeclass.pending"]` | Phase 2-4 | 後続処理へ委ねた制約の一覧。 |
| 型クラス | `typeclass.generalized_typevars[]` | `extensions.typeclass.generalized_typevars[]` | `metadata["typeclass.generalized_typevars"]` | Phase 2-4 | 一般化・未解決の型変数。 |
| 型クラス | `typeclass.graph.export_dot` | `extensions.typeclass.graph.export_dot` | `metadata["typeclass.graph.export_dot"]` | Phase 2-4 | 制約グラフ DOT ファイルのパスまたは `null`。 |
| 型クラス | `typeclass.span.start` / `.end` | `extensions.typeclass.span.start` 等 | `metadata["typeclass.span.start"]` 等 | Phase 2-4 | 制約導入位置のオフセット。 |
| 効果 | `effect.stage.required` | `extensions.effect.stage.required` | `metadata["effect.stage.required"]` | Phase 2-2 | Stage 宣言の期待値。 |
| 効果 | `effect.stage.actual` | `extensions.effect.stage.actual` | `metadata["effect.stage.actual"]` | Phase 2-2 | 実測 Stage。 |
| 効果 | `effect.stage.residual` | `extensions.effect.stage.residual` | `metadata["effect.stage.residual"]` | Phase 2-4 | 残余効果の JSON。 |
| 効果 | `effect.handler_stack` | `extensions.effect.handler_stack[]` | `metadata["effect.handler_stack"]` | Phase 2-4 | ハンドラ適用順。 |
| FFI | `bridge.platform` | `extensions.bridge.platform` | `metadata["bridge.platform"]` | Phase 2-3 | `linux-gnu` / `windows-msvc` / `macos-arm64` 等。 |
| FFI | `bridge.abi` | `extensions.bridge.abi` | `metadata["bridge.abi"]` | Phase 2-3 | 呼出規約の識別子。 |
| FFI | `bridge.ownership` | `extensions.bridge.ownership` | `metadata["bridge.ownership"]` | Phase 2-3 | 引数の所有権。 |
| FFI | `bridge.audit_pass_rate` | `extensions.bridge.audit_pass_rate` | `metadata["bridge.audit_pass_rate"]` | Phase 2-4 | CI での合格率。 |
| 解析 | `parser.input_name` | `extensions.parse.input_name` | `metadata["parse.input_name"]` | Phase 1-4 | ソース名。 |
| 解析 | `parser.stage_trace[]` | `extensions.parse.stage_trace[]` | `metadata["parse.stage_trace"]` | Phase 2-4 | レキサー→パーサ→補助解析の順序。 |

- キー追加時は本表を更新し、`docs/spec/3-6-core-diagnostics-audit.md` の付録と差異が無いか確認する。新規キーは `schema.version` をインクリメントした上で CI の `jsonschema` 検証対象に追加する。
- 監査永続化ストアの健全性チェックは `tooling/ci/verify-audit-metadata.py --index reports/audit/index.json --root . --history-dir reports/audit/history` を用いる。CI では `--strict` オプションの有効化を検討し、`retained_entries` の再計算結果と拡張キー欠落の双方を検証に含める。
- CI で監査ログを収集した後は `tooling/ci/create-audit-index.py --output reports/audit/index.json --audit <profile:target:path[:status[:level[:pass_rate]]]>` を実行して index を生成する。生成済み index を `verify-audit-metadata.py` に渡すことで一貫したゲートフローを構築する。

### `schema.version` 更新履歴
| バージョン | 反映日 | 主な変更点 | 参照ドキュメント | CI 導入 |
|-----------|--------|------------|-------------------|---------|
| v1.0 | 2025-09-30 | CLI `audit_id` / `change_set` の導入、`bridge.platform` キー確定 | `docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md` §3 | Linux のみ (`bootstrap-linux.yml`) |
| v1.1 | 2025-10-24 | `extensions.bridge.*` 拡張、`schema_version` フィールドの必須化、macOS 監査サンプル追加 | `docs/plans/bootstrap-roadmap/2-3-completion-report.md` §5, `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` | Linux / macOS CI (`bootstrap-linux.yml`, `bootstrap-macos.yml`) |
| v1.1+phase2-4 | 2025-10-29（予定） | 型クラス・効果メタデータの必須化、Windows 監査ジョブ (ID 22) 対応、`bridge.audit_pass_rate` 追加 | `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` §4 | Linux / Windows / macOS CI へ順次反映予定 |

- 履歴の更新と同時に `reports/audit/index.json` の `schema_version` フィールドを確認し、`tooling/ci/collect-iterator-audit-metrics.py` が最新値を指していることを確認する。
- メジャー更新（`v2.0` 等）時は `docs/migrations/audit-schema-history.md` に詳細な移行手順を残し、互換ウィンドウと移行スクリプトの有無を記載する。

### macOS 追加指標（Phase 1-8 以降）
| カテゴリ | 指標 | 定義 | 収集タイミング | 計画参照 |
|----------|------|------|----------------|----------|
| CI | `ci_build_time_macos` | `bootstrap-macos` ワークフローにおける `dune build` の実行時間（分） | push/pr ごと | [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) §5 |
| CI | `ci_test_time_macos` | `bootstrap-macos` ワークフローにおける `dune runtest` の実行時間（分） | push/pr ごと | 同上 |
| 品質 | `llvm_verify_macos` | `llvm-as` → `opt -verify` → `llc -mtriple=arm64-apple-darwin` の成否（0=成功,1=失敗） | CI 実行ごと | 同上 |
| 成果物 | `runtime_macho_size` | `libreml_runtime.a` (Mach-O) のファイルサイズ（KB） | 週次 | 同上 |
| 運用 | `macos_runner_queue_time` | GitHub Actions macOS ランナーの待機時間（分） | 週次レビュー | 同上 |

> **補足**: macOS 指標は Linux 指標との比較を想定し、`metrics.json` にターゲット別セクションを設けて記録する。乖離が 15% を超えた場合は `0-4-risk-handling.md` に登録して原因調査を開始する。

### Phase 1-8 実測値（macOS Apple Silicon ARM64）

**測定日**: 2025-10-11
**環境**: macOS 14.x / Apple Silicon (ARM64) / LLVM 18.1.8 / OCaml 5.2.1

| 指標 | 実測値 | 備考 |
|------|--------|------|
| `ci_build_time_macos` | 2.4秒 | `dune build` フルビルド（クリーンビルド後） |
| `ci_test_time_macos` | ~30秒 | `dune runtest` + ランタイムテスト + AddressSanitizer |
| `llvm_verify_macos` | 成功 (0) | ARM64 ターゲットで全サンプル検証成功 |
| `runtime_macho_size` | 56 KB | `libreml_runtime.a` (ARM64 Mach-O) |
| `macos_runner_queue_time` | 未計測 | GitHub Actions の実運用開始後に記録 |

**LLVM IR 検証結果**:
- ターゲット: `arm64-apple-darwin`
- 検証パイプライン: `llvm-as` → `opt -verify` → `llc -mtriple=arm64-apple-darwin`
- 全テストサンプル検証成功（examples/cli/*.reml）

### Phase 2-5 Step4 記録（2026-03-31）
- `lexer.identifier_profile_unicode`: 0.0（ASCII プロファイルのみ稼働）。`tooling/ci/collect-iterator-audit-metrics.py --summary` の `parser.runconfig.lex.profile` 集計結果と `reports/audit/summary.md` のログを確認し、Phase 2-5 時点で Unicode プロファイルを有効化したジョブが存在しないことを再確認した。
  - レビュー: 2026-03-31 仕様差分補正週次（Phase 2-5 Week32）。
  - フォローアップ: Phase 2-7 `lexer-unicode` タスク着手時に `REML_ENABLE_UNICODE_TESTS=1` を有効化し、同指標が 1.0 へ遷移することを確認する（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` 参照）。

### 0.3.1a Core Prelude KPI 収集手順（WBS 2.3b）
- **API インベントリ**: `cargo xtask prelude-audit --strict --wbs 2.1b --baseline docs/spec/3-1-core-prelude-iteration.md` を `compiler/rust/frontend` ルートで実行し、`core_prelude.missing_api` が 0 であることを確認する。Iter/Collector 向けは `cargo xtask prelude-audit --section iter --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を同じジョブで実行し、`reports/spec-audit/ch1/iter.json` と `reports/spec-audit/ch0/links.md#iter-f3` に結果を保存する。Nightly CI では必要に応じて `--machine-readable --output reports/spec-audit/ch0/core_prelude_kpi.csv` を付与し、`timestamp,missing_api` を追記する。
- **panic 経路監査**: `RUSTFLAGS="-Dnon-fmt-panics -Z panic-abort-tests" cargo test core_prelude_option_result panic_forbidden` を必須化し、`scripts/validate-diagnostic-json.sh` → `reports/diagnostic-format-regression.md` で差分が無いことを確認する。`core_prelude.panic_path = 0` のログは `reports/spec-audit/ch0/links.md#core_prelude` に貼り付け、失敗時は `4-5-backward-compat-checklist.md` の回帰項目へ移送する。
- **Guard 診断メトリクス**: `tooling/ci/collect-iterator-audit-metrics.py --section prelude-guard --require-success --export-csv reports/spec-audit/ch0/core_prelude_kpi.csv` を Nightly CI に追加し、`core_prelude.guard.failures` と `core_prelude.guard.ensure_not_null` を同じ CSV に集約する。値が 0 でない場合は `0-4-risk-handling.md#core-prelude-guard` にリスク登録し、`docs/spec/3-6-core-diagnostics-audit.md` の `core.prelude.ensure_failed` セクションに影響範囲を追記する。
- **Iter 生成 KPI**: `collect-iterator-audit --section iter --case range --case repeat --case once --case empty --output reports/spec-audit/ch1/iter.json` を週次で実行し、`iterator.range.overflow_guard`（>=1）、`iterator.repeat.flagged=true`、`iterator.once.length=1`、`iterator.empty.items=0` を満たしているかを `reports/spec-audit/ch0/links.md#iter-generators` へ記録する。未達の場合は `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md#33b-生成-api-実装ステップ（wbs-31c-f1）` を参照し補修タスクを登録する。
- **Iter API 在庫カバレッジ**: `cargo xtask prelude-audit --section iter --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を Nightly で実行し、`reports/spec-audit/ch1/iter.json` の `iterator.api.coverage`（2025-11-20 = 1.0 / pending: なし）と `core_prelude.missing_api` を同時に監視する。差分や未実装が検出された場合は `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml` の該当行と `docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md` の補修タスクを更新し、`reports/spec-audit/ch0/links.md#iter-f3` に再実行ログを残す。
- **Iter Adapter KPI（G1 map/filter）**: `cargo test core_iter_adapters -- --nocapture map_pipeline filter_effect map_filter_chain_panic_guard` を実行し、直後に `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case map --case filter --output reports/iterator-map-filter-metrics.json --require-success` を実行する。`iterator.map.latency` と `iterator.filter.predicate_count` の両方を `reports/spec-audit/ch0/links.md#iter-g1-map-filter` に転記し、逸脱時は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ登録する。
  - 2026-02-18 実測結果: `reports/iterator-map-filter-metrics.json` に `adapter_metrics.map_pipeline.latency_ns = 16750`、`adapter_metrics.filter_effect.latency_ns = 2875`、`adapter_metrics.filter_effect.effects.predicate_calls = 4`（filter 入力 4 件に一致）を保存。`collect-iterator-audit-metrics.py` の実行ログと JSON は `reports/spec-audit/ch0/links.md#iter-g1-map-filter` 経由で参照できる。
- **Iter Adapter KPI（G3 buffered/backpressure）**: `cargo test core_iter_adapters -- --include-ignored buffered_window` → `cargo bench -p compiler-rust-frontend iter_buffered` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case buffered --output reports/iterator-buffered-metrics.json --require-success` を同一 Nightly で実行し、`iterator.mem.window.bytes` と `iterator.mem.window.backpressure` を `reports/iterator-buffered-metrics.json` に追記する。Criterion の結果は `reports/benchmarks/iter_buffered-YYYY-MM-DD.json` へ保存し、`docs/plans/rust-migration/3-2-benchmark-baseline.md` の ±10% 基準と突き合わせる。
  - 2027-02-22 実測結果: `reports/iterator-buffered-metrics.json` に `adapter_metrics.buffered_window.effects.mem_bytes = 2` / `adapter_metrics.buffered_window.backpressure.dropped = 2` / `produced = 6` / `ratio = 0.33` を記録し、`iterator.mem.window.bytes = 2`・`iterator.mem.window.backpressure = 0.33` を KPI として追加。Criterion ベンチ `reports/benchmarks/iter_buffered-2027-02-22.json` は `windows_per_sec` が +3.8% で±10%以内、`mem_bytes_per_window = 2` を確認した。Run-ID は `2027-02-22-iter-adapter-g3` とし、リンクは `reports/spec-audit/ch0/links.md#iter-buffered` 経由で参照できる。
- **Iter Adapter KPI（G2 flat_map/zip）**: `cargo test core_iter_adapters -- --include-ignored flat_map_vec zip_mismatch` を実行し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case flat_map --case zip --output reports/iterator-flatmap-metrics.json --secondary-output reports/iterator-zip-metrics.json --require-success` を続けて実行する。診断面は `scripts/validate-diagnostic-json.sh --pattern iterator.flat_map --pattern iterator.zip reports/spec-audit/ch1/core_iter_adapters.json` で確認し、`iterator.flat_map.mem_reservation` と `iterator.zip.shorter_error_rate` を `reports/spec-audit/ch0/links.md#iter-adapters`・`docs/notes/core-library-outline.md#iter-g2-flat-zip`・`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#iterator-adapter-esc` に記録する。
  - 2026-02-21 実測結果: `reports/iterator-flatmap-metrics.json` に `adapter_metrics.flat_map_vec.effects.mem = true` / `mem_reservation_bytes = 3` / `stage.required = Exact(\"beta\")`、`reports/iterator-zip-metrics.json` に `adapter_metrics.zip_mismatch.iterator.error.zip_shorter = 1` / `stage.actual = \"stable\"` を記録し、`reports/diagnostic-format-regression.md#iterator.zip_mismatch` に同じ Run-ID を追記。`prelude_api_inventory.toml` と `reports/spec-audit/ch1/core_iter_adapters.json` を同期済み。

## 0.3.2 レポートテンプレート
- **週次レポート**: `reports/week-YYYYMMDD.md`（将来追加予定）に以下の項目を記録する。
  - 主要マイルストーン進捗
  - 指標の最新値
  - リスク/ブロッカー（`0-4-risk-handling.md` へのリンク）
- **フェーズ終了レビュー**: 各 Phase 文書末尾のチェックリストと合わせて、以下を必須記録とする。
  - 指標表（最新値と目標）
  - レビュア署名（Parser/Type/Runtime/Toolchain）
  - 仕様変更一覧（ファイル/節/概要）

## 0.3.3 診断・監査ログ整合性
- `Diagnostic` オブジェクトの拡張フィールド (`extensions`) は [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) に定義されたキー (`effect.stage.required`, `bridge.target` など) を使用する。
- `Diagnostic` と `AuditEnvelope` のフィールド差分と移行計画は [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md#diagnostic-field-table-draft) の比較表ドラフトを参照する。
- `tooling/runtime/audit-schema.json` のバージョン管理は [2-4-diagnostics-audit-pipeline.md](2-4-diagnostics-audit-pipeline.md#audit-envelope-draft) の移行ステップ案に従い、`schema.version` フィールドを更新した際は本節と `docs/notes/audit-migrations.md`（新規予定）に履歴を残す。
- 監査ログ (`AuditEnvelope`) は JSON Lines 形式で保存し、以下を必須フィールドとする。
  - `metadata.effect.stage.required`
  - `metadata.bridge.target`
  - `metadata.bridge.abi`
  - `metadata.bridge.ownership`
- スキーマ検証: `tooling/runtime/audit-schema.json`（ドラフト）を基準に `bridge.*` フィールドを検証するツールを Phase 2-3 で整備する。仮段階では `tooling/ci/collect-iterator-audit-metrics.py` の `ffi_bridge.audit_pass_rate` を用いて欠落を検知する。
- ログ検証用に `tools/audit-verify`（将来実装予定）を準備し、CI で `--strict` フラグを用いて検証。

### 監査ログ収集・永続化フロー
1. **実行コマンド**  
   - ローカル検証: `remlc <target>.reml --emit-audit --audit-store=local --audit-level=full [追加オプション]`  
     実行後に `tooling/audit-store/local/<timestamp>/` 下へ `*.jsonl` と `index.json` が生成される。`index.json.latest` は最後のビルド ID を指すシンボリックリンクとして維持する。  
   - CI 実行: `remlc ... --emit-audit --audit-store=ci --audit-level=summary` を推奨し、`reports/audit/<target>/<YYYY>/<MM>/<DD>/<commit>_<target>_<build-id>.jsonl` を生成する。効果・FFI 検証ジョブでは `--audit-level=full` を併用する。
2. **インデックス更新**  
   - すべてのプロファイルで `AuditEnvelope.build_id` を `<utc timestamp>-<commit sha>` 形式で発行し、`reports/audit/index.json`（CI）または `tooling/audit-store/local/index.json`（ローカル）へ追記する。  
   - CI ではビルドごとのメタデータ（`target`, `pass_rate`, `audit_level`, `artifact_path`）を必須フィールドとし、`reports/audit/index.json` の `pruned` 配列で削除済みビルド ID を管理する。
3. **履歴・失敗ログ**  
   - `tooling/ci/collect-iterator-audit-metrics.py --prune` を週次で実行し、`tooling/ci/audit-retention.toml` に定義した `retain = {ci = 100, local = 30}` を超える履歴を削除する。削除対象は `reports/audit/history/<target>.jsonl.gz` へ圧縮退避し、失敗ビルドは `reports/audit/failed/<commit>/` に完全保存する。  
   - 圧縮履歴を更新した際は `reports/audit/usage.csv` に容量を追記し、500MB を超えた場合は `0-4-risk-handling.md` に対応策を登録する。
4. **メトリクス集計**  
   - `tooling/ci/collect-iterator-audit-metrics.py --summary reports/audit/index.json --output reports/audit/summary.md` を実行し、`ffi_bridge.audit_pass_rate` と `iterator.stage.audit_pass_rate` の推移を Markdown サマリとして生成する。  
   - サマリ生成後は CI アーティファクトとして保存し、レビュー時に `reports/ffi-bridge-summary.md` からリンクする。
5. **レビューチェックリスト**  
   - PR で `reports/audit/index.json` または `tooling/audit-store/local/index.json` が更新された場合は、レビュアが `audit-retention.toml` の閾値・`usage.csv` の容量推移・`summary.md` のメトリクス変化を確認する。  
   - 必須フィールド欠落や pass_rate < 1.0 を検出した場合はブロッカーとして `0-4-risk-handling.md` に登録し、修正完了後に削除する。

## 0.3.4 レビュア体制
| 領域 | 主担当 | 副担当 | レビュー頻度 |
|------|--------|--------|--------------|
| Parser/Core.Parse | Rust Parser WG（owner: #rust-frontend-parser） | Spec Core WG（owner: #spec-core） | 週次（Phase 2-8 W36〜W38 は毎週火曜） |
| Type/Effects | Effects Taskforce（owner: #effects-runtime） | Typeclass WG（owner: #type-systems） | 週次（Phase 2-8 期間は木曜夕方） |
| Runtime/Capability | Rust Runtime WG（owner: #rust-runtime） | Diagnostics WG（owner: #diagnostics-audit） | 隔週（Phase 2-8 中は W36/W38 に集中レビュー） |
| Toolchain/CI | CI Platform WG（owner: #ci-tooling） | Docs Platform WG（owner: #docs-hub） | 隔週（Phase 2-8 W36 後半と W38 前半で臨時レビュー） |

レビュアの割当が変更された場合は、この表と各 Phase 文書のレビュア欄を更新する。担当者が空欄の場合は `0-4-risk-handling.md` にリスクとして記録し、埋めるまでフェーズ進行を停止する。

### 0.3.4a Phase 2-8 仕様監査スプリント（Rust フォーカス）
| 週（ISO W） | 範囲 | 主担当（Slack Channel） | Rust 成果物 / 実行コマンド | 記録先 |
|-------------|------|--------------------------|-----------------------------|--------|
| W36 後半 | 差分リスト統合 + Chapter 0 索引用語 | Spec Core WG（#spec-core） × Docs Platform WG（#docs-hub） | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml` ログの概要と `reports/spec-audit/ch0/links.md` でのリンク確認結果 | `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#phase28-diff-class`, `reports/spec-audit/ch0/` |
| W37 前半 | Chapter 1（構文・型・効果） | Rust Parser WG（#rust-frontend-parser） × Effects Taskforce（#effects-runtime） | `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --emit-diagnostics --input docs/spec/1-1-syntax/examples/*.reml` | `reports/spec-audit/ch1/`, `docs/notes/spec-integrity-audit-checklist.md#phase-2-8` |
| W37 後半 | Chapter 2（Parser API / Streaming） | Parser API WG（#parser-api） × Diagnostics WG（#diagnostics-audit） | `compiler/rust/frontend/tests/streaming_runner.rs` の結果と Streaming JSON を `reports/spec-audit/ch2/streaming/*.json` へ保存 | `reports/spec-audit/ch2/`, `docs/notes/spec-integrity-audit-checklist.md#err-001` |
| W38 前半 | Chapter 3（Diagnostics / Capability / Runtime） | Rust Runtime WG（#rust-runtime） × CI Platform WG（#ci-tooling） | `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml` / `compiler/rust/adapter/Cargo.toml` のログと `tooling/ci/collect-iterator-audit-metrics.py --section diagnostics --require-success` 出力 | `reports/spec-audit/ch3/`, `reports/spec-audit/summary.md`, `reports/spec-audit/diffs/` |

> メモ: 各担当はレビュー終了後 24 時間以内に `reports/spec-audit/summary.md` へ抜粋を追記し、`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` §1 のベースライン報告にリンクする。

## 0.3.5 仕様差分追跡
- 仕様ファイルに変更が入った際は、以下の形式で記録する。
  - `YYYY-MM-DD / ファイル:節 / 変更概要 / 参照コミット`
- 記録は Phase ごとにセクションを分け、フェーズ終了時にレビューアが確認する。
- 差分が複数フェーズに跨る場合は、各フェーズで影響範囲を明記し、必要に応じて追加タスクを `0-4-risk-handling.md` に登録する。
- 2025-10-24 / docs/spec/3-6-core-diagnostics-audit.md:§2.4 / CLI・監査ゴールデン出力で `schema.version`, `audit.timestamp`, `AuditEnvelope.metadata.bridge.*` が欠落していることを確認。Phase 2-7 §1 で `collect-iterator-audit-metrics.py --platform <target> --require-success` を導入し、2025-11-06 時点で `ffi_bridge.audit_pass_rate = 1.0` を達成。`reports/ffi-bridge-summary.md` および `reports/iterator-stage-summary-windows.md` に検証ログを保存し、技術的負債 ID22 をクローズ。 / completed (Phase 2-7)
- 2025-10-24 / docs/spec/3-8-core-runtime-capability.md:§10 / Stage 監査ログで `extensions.bridge.stage.*` と `effect.stage.*` が未出力のため `iterator.stage.audit_pass_rate = 0.0` を維持。Phase 2-7 の監査ゲート整備で 2025-11-06 に `iterator.stage.audit_pass_rate = 1.0` を確認し、`reports/iterator-stage-summary-macos.md` と `reports/audit/index.json` に記録。技術的負債 ID23 をクローズ。 / completed (Phase 2-7)
- 2025-11-04 / docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md:§0 / Phase 2-7 キックオフで KPI ベースラインを設定。`lexer.identifier_profile_unicode = 0.0`, `syntax.effect_construct_acceptance = 0.0`, `diagnostics.effect_row_stage_consistency = null` を測定前の起動値として登録し、`tooling/ci/collect-iterator-audit-metrics.py` Phase 2-7 プロファイルと `scripts/validate-diagnostic-json.sh` Phase 2-7 プロファイルの整備状況を `docs/plans/bootstrap-roadmap/2-5-review-log.md#phase2-7-キックオフレビュー2025-11-04` に記録。 / baseline (Phase 2-7)
- 2025-11-06 / docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md:§1 / Windows/macOS CI に `collect-iterator-audit-metrics.py --platform windows-msvc|macos-arm64 --require-success` を導入し、`ffi_bridge.audit_pass_rate = 1.0`・`iterator.stage.audit_pass_rate = 1.0`・`diagnostic.audit_presence_rate = 1.0` を `reports/iterator-stage-summary-windows.md` / `reports/iterator-stage-summary-macos.md` で確認。技術的負債 ID22/23 をクローズし、`reports/audit/index.json` を最新化。 / completed (Phase 2-7)
- 2025-11-20 / docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md:§3 / `.github/workflows/bootstrap-linux.yml#rust-prelude-tests` で `RUSTFLAGS="-Zpanic-abort-tests" cargo +nightly test --test core_iter_pipeline|core_iter_effects` を実行し、`reports/spec-audit/ch1/iter.json`・`reports/iterator-stage-summary.md`・`reports/iterator-stage-metrics.json` を実測値 (`iterator.stage.audit_pass_rate = 1.0`, `collector.effect.mem = 0`) で更新。`scripts/validate-diagnostic-json.sh --pattern iterator --pattern collector` も同ジョブで併走させ、`docs/notes/core-library-outline.md#iterator-f3` および `reports/spec-audit/ch0/links.md#iterator-f3` へ記録。 / completed (Phase 3-1)
- 2025-11-20 / docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md:§3 / WBS 3.1b F2-3 で `reports/spec-audit/ch1/core_iter_collectors.json` を生成し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section collectors --module iter --case wbs-31b-f2 --source reports/spec-audit/ch1/core_iter_collectors.json --audit-source reports/spec-audit/ch1/core_iter_collectors.audit.jsonl --output reports/iterator-collector-metrics.json` を実行。`collector.stage.audit_pass_rate=1.0`・`collector.effect.mem=2/7`・`collector.effect.mut=4/7`・`collector.error.duplicate_key=2`・`collector.error.invalid_encoding=1` を `reports/iterator-collector-summary.md` と本章に記録し、`docs/notes/core-library-outline.md#collector-f2-監査ログ` に検証手順を転記。 / completed (Phase 3-1)
- 2027-03-06 / docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md:§4.b H1 / `Iter::collect_*` と `Collector` の監査ログ整合性を検証するため `cargo test --manifest-path compiler/rust/frontend/Cargo.toml core_iter_terminators` → `python3 tooling/ci/collect-iterator-audit-metrics.py --section iterator --case terminators --source reports/spec-audit/ch1/core_iter_terminators.json --output reports/iterator-collector-metrics.json --require-success` → `scripts/validate-diagnostic-json.sh --pattern iterator.collect --pattern prelude.collector reports/spec-audit/ch1/core_iter_terminators.json` を直列で実行。`iterator.stage.audit_pass_rate = 1.0`、`collector.effect.mem_reservation = 4`（Vec）、`collector.error.invalid_encoding = 1`（String）、`diagnostic.audit_presence_rate = 1.0` を `reports/spec-audit/ch0/links.md#iter-terminators-h1` と `reports/iterator-collector-summary.md#collect_vec_reserve-h1` に記録し、`docs/notes/core-library-outline.md#collector-f2-監査ログ` および `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#collector-f2-監査ログ` へリンクした。 / completed (Phase 3-1)
- 2026-12-12 / docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md:§8 / 効果構文 PoC 週次レビューで `collect-iterator-audit-metrics.py --section effects --require-success` を実行し、`syntax.effect_construct_acceptance = 1.0` と `effects.syntax_poison_rate = 0.0` を確認。計測ログは `reports/audit/phase2-7/effects/` に保管し、レビュー結果を `docs/notes/effect-system-tracking.md#2026-12-12-h-o1〜h-o5-進捗レビュー` に記録した。 / monitoring (Phase 2-7)
  - レビュー: 2026-12-12 Effects チーム週次レビュー（Phase 2-7 Week42）。
  - フォローアップ: フラグ運用（H-O3）と Stage 監査整備（H-O4/H-O5）が未完了のため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §8.2〜§8.3 を継続更新する。
- 2026-12-21 / docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md:§5 / Phase 2-7 最終週に `collect-iterator-audit-metrics.py --section diagnostics --require-success` を導入し、`diagnostics.domain_coverage = 1.0`, `diagnostics.plugin_bundle_ratio = 0.98`, `diagnostics.effect_stage_consistency = 1.0` を確認。集計結果は `reports/audit/phase2-7/diagnostics-domain-20261221.json` と `reports/audit/dashboard/diagnostics.md` に保存し、Phase 2-8 監査で利用するベースラインとして登録。 / completed (Phase 2-7)
  - レビュー: 2026-12-21 Diagnostics/Plugin 合同レビュー（Phase 2-7 Week43）。
  - フォローアップ: 指標が閾値を下回った場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-domain-metrics` を再オープンし、Phase 2-8 仕様監査で追加テストを実施する。
- 2026-12-02 / tooling/ci / GitHub Actions `bootstrap-linux`・`bootstrap-windows`・`bootstrap-macos` ワークフローで `REML_ENABLE_UNICODE_TESTS=1` を既定化し、`tooling/ci/collect-iterator-audit-metrics.py --require-success` の `parser.runconfig.lex.profile` 集計が `unicode` 比率 100% で推移していることを `reports/audit/summary.md#parser` へ記録。`lexer.identifier_profile_unicode = 1.0` を確認し、本章の週次ログへ追加した。 / completed (Phase 2-7)
  - レビュー: 2026-12-02 Parser/Lexer 週次レビュー（Phase 2-7 Week41）。
  - フォローアップ: KPI が 1.0 を下回った場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の「Unicode XID 識別子実装未完了」を再エスカレーションし、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §7.2 の手順でテストを再検証する。
- 2026-11-21 / reports/audit/dashboard/streaming.md / `parser.stream.outcome_consistency`, `parser.stream.backpressure_sync`, `parser.stream.flow.auto_coverage`, `parser.stream.demandhint_coverage`, `parser.stream.bridge_backpressure_diagnostics`, `parser.stream.bridge_stage_propagation` を週次集計し、`reports/audit/phase2-7/streaming/` に実行ログを保存。各指標が `1.0` であることを確認し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` §6.6 に転記した。 / monitoring (Phase 2-7)
- 2027-01-19 / docs/plans/bootstrap-roadmap/3-1-core-prelude-iteration-plan.md:§2 / WBS 2.3b (`Option/Result` panic 排除) の完了確認として `core_prelude.missing_api = 0`・`core_prelude.panic_path = 0` を KPI 表へ登録し、`reports/spec-audit/ch0/core_prelude_kpi.csv` と `reports/spec-audit/ch0/links.md#core_prelude` に `cargo xtask prelude-audit --strict`・`cargo test panic_forbidden`・`scripts/validate-diagnostic-json.sh` のログを保存。`reports/diagnostic-format-regression.md` には差分なしでスナップショットを追記した。 / monitoring (Phase 3-1)
  - レビュー: 2027-01-19 Core Library × QA/PM 週次レビュー（Phase 3-1 Week04）。
  - フォローアップ: `core_prelude.panic_path > 0` または `core_prelude.missing_api > 0` の場合は `4-5-backward-compat-checklist.md#core-prelude-regressions` と `0-4-risk-handling.md#core-prelude-guard` を即再開し、`reports/spec-audit/ch0/core_prelude_kpi.csv` へ失敗サンプルを残す。

## 0.3.6 最適化パス統計（Phase 3 Week 10-11）

### 実装統計
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| コード規模 | Core IR 総行数 | 5,642行 | ir.ml, desugar.ml, cfg.ml, const_fold.ml, dce.ml, pipeline.ml, ir_printer.ml |
| テスト | テストケース総数 | 42件 | test_core_ir, test_desugar, test_cfg, test_const_fold (26件), test_dce (9件), test_pipeline (7件) |
| テスト | 成功率 | 100% (42/42) | 回帰なし |
| 最適化 | 定数畳み込み式数 | 変動 | パイプラインテストで計測 |
| 最適化 | 削除束縛数 | 変動 | DCEテストで計測 |
| 最適化 | 削除ブロック数 | 変動 | DCEテストで計測 |
| 性能 | ConstFold実行時間 | <0.001秒 | テストケース平均 |
| 性能 | DCE実行時間 | <0.001秒 | テストケース平均 |

### 最適化効果の例
- **定数畳み込み**: `10 + 20` → `30`（26件のテストで検証）
- **死コード削除**: `let x = 42 in 10` → `10`（9件のテストで検証）
- **パイプライン統合**: 不動点反復で複数パスを自動適用（7件のテストで検証）

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `stage_mismatch_count` | 0件 | 0件 | ✅ 達成 |
| テストカバレッジ | 100% | 95%以上 | ✅ 達成 |

## 0.3.7 RuntimeCapability 運用と効果診断ゴールデン

### JSON 管理手順
- Capability Registry は `tooling/runtime/capabilities/` に配置する。デフォルト設定は `default.json`、プラットフォーム差分は `{platform}.json`（例: `linux.json`, `windows.json`）で管理し、コミット時に必ず本節へ変更履歴を追記する。
- JSON フォーマット（暫定）は以下を必須キーとする。`stage` は `experimental` / `beta` / `stable` のいずれか、`capabilities` は `RuntimeCapability` 列挙子文字列、`overrides` はターゲットトリプル別の上書き設定。
  ```json
  {
    "stage": "stable",
    "capabilities": ["io", "panic", "runtime"],
    "overrides": {
      "x86_64-pc-windows-msvc": ["ffi", "process"]
    }
  }
  ```
- JSON の編集手順:
  1. 変更箇所を `tooling/runtime/README.md`（Phase 2-2 で追加予定）に記録し、出典となる仕様 (`docs/spec/3-8-core-runtime-capability.md`) を併記する。
  2. `scripts/validate-runtime-capabilities.sh`（Phase 2-2 で整備）を実行し、スキーマ検証と Stage 解釈トレースの再計算を行う。スクリプトは `reports/runtime-capabilities-validation.json` に `stage_summary`（CLI/JSON/環境変数/Runtime 判定の一覧）を出力し、CI で `jq` フォーマットチェックを通過することを確認する。
  3. 差分を `0-3.9 進捗ログ` に追記し、`stage_summary` から抜粋した Stage 変更点（例: `default.json: beta → stable`）を合わせて記録する。レビュアには JSON とサマリの両方を提示する。
- Stage が変更された場合は、必ず効果診断ゴールデンと `AuditEnvelope` ゴールデンを再生成し、`stage_trace` の差分が Typer/Runtime で一致していることを確認する。
- CLI/環境変数による Stage 指定を検証する場合は、`--cli-stage <stage>` / `--env-stage <stage>` を併用し、`stage_trace` の冒頭エントリ（`cli_option` / `env_var`）と整合を確認する。

#### Windows / 追加ターゲット差分検証
- `tooling/runtime/capabilities/default.json` では `overrides.x86_64-pc-windows-msvc` に Windows 専用の Stage と Capability を定義している。新しいターゲットを追加する場合も同じ `overrides` セクションか、個別の `{platform}.json` に追記し、本節へ差分を記録する。
- 検証手順:
  1. `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json --output reports/runtime-capabilities-validation.json` を実行し、`stage_summary.runtime_candidates` に `target: x86_64-pc-windows-msvc` が含まれることを確認する。
  2. 同ファイルの `overrides` に `target: arm64-apple-darwin` が追加された場合は同コマンドで再検証し、`runtime_candidates` に `arm64-apple-darwin` が `stage: beta` として出力されること、および `stage_trace` に同ターゲットのエントリが追加されていることを確認する。検証ログと併せて `reports/ffi-macos-summary.md` に記録し、レビューコメントで共有する。
  3. Stage や Capability を更新した場合は、`reports/runtime-capabilities-validation.json` の `stage_summary.json[].overrides` と `stage_trace` を 0.3.9 進捗ログへ抜粋し、レビューで参照できるようにする。
  4. 追加ターゲット（例: `aarch64-pc-windows-msvc` や `x86_64-unknown-linux-gnu` の派生）を導入した際は、同コマンドに `--cli-stage` / `--env-stage` を付与して優先度を再確認し、`tooling/ci/sync-iterator-audit.sh --metrics tooling/ci/iterator-audit-metrics.json --verify-log tooling/ci/llvm-verify.log --output reports/iterator-stage-summary.md` を再実行して `iterator.stage.audit_pass_rate = 1.0` ・`ffi_bridge.audit_pass_rate = 1.0` を維持しているかを確かめる。
- 検証の結果、`pass_rate < 1.0` となった場合や `stage_trace` に欠落が発生した場合は、影響段階が解消されるまで `0-4-risk-handling.md` に TODO を登録し、ロールバック方針と併せて共有する。

### CLI オプション優先度と検証
- Stage 解決は「CLI `--effect-stage` → JSON `--runtime-capabilities` → 環境変数 `REMLC_EFFECT_STAGE`」の優先順を採用し、`RuntimeCapabilityResolver`（Phase 2-2 で導入予定）で一元化する。
- 動作確認フロー:
  1. `remlc examples/effects/demo.reml --effect-stage beta --format=json` を実行し、`Diagnostic.extensions["effect.stage.required"]` が `beta` になることを確認。
  2. 同一コマンドに `--runtime-capabilities tooling/runtime/capabilities/linux.json` を追加し、JSON の `stage` が採用されることを `effect.stage.actual` で確認。
  3. どちらも指定せず `REMLC_EFFECT_STAGE=stable` を設定し、環境変数が採用されることを確認。
  4. 上記 3 ケースで `Diagnostic.extensions["effect.stage_trace"]` に出力される `source` / `stage` / `capability` の配列が CLI 指定 → JSON → 環境変数の順序で記録されていることを確認し、Runtime 側の `AuditEnvelope.metadata.stage_trace` も同一配列であることを `grep` などで突き合わせる。
- 上記 3 ケースの出力を `compiler/ocaml/tests/golden/diagnostics/effects/stage-resolution.json.golden`（新設）でスナップショット化し、`dune runtest compiler/ocaml/tests/test_diagnostics.ml` に統合する。

### 監査ログと CI 指標
- Stage 判定および FFI ブリッジ検証は `RuntimeCapabilityResolver` → `AuditEnvelope` → `tooling/ci/collect-iterator-audit-metrics.py` → `iterator.stage.audit_pass_rate` / `ffi_bridge.audit_pass_rate` の順で連携する。各段階で `stage_trace` または `bridge.*` が欠落した場合は CI を失敗させる。
- `remlc examples/effects/demo.reml --emit-audit --audit-store=local --audit-level=full --effect-stage beta` を実行し、`AuditEnvelope.metadata.stage_trace` に Typer 判定と Runtime 判定が連続して格納されていることを確認する。監査ゴールデンは `compiler/ocaml/tests/golden/audit/effects-stage.json.golden`（新設）に保存し、`tooling/audit-store/local/` に生成された `index.json` がビルド ID を記録しているか確認する。
- CI では `tooling/ci/sync-iterator-audit.sh --metrics /tmp/iterator-audit.json --audit compiler/ocaml/tests/golden/audit/effects-stage.json.golden` を実行し、`iterator.stage.audit_pass_rate` と `ffi_bridge.audit_pass_rate` がいずれも 1.0 であることをゲート条件とする。Stage 判定差分が発生した場合は `stage_trace` の乖離内容を Markdown サマリに追記し、FFI 契約差分が発生した場合は `bridge.*` 欠落項目をサマリへ明記してレビューへ共有する。
- 監査ログの更新後は `reports/runtime-capabilities-validation.json` の `stage_summary`・`iterator-stage-summary.md` および FFI ブリッジ用サマリ（導入後に `reports/ffi-bridge-summary.md` 予定）を本節へリンクする。

### 監査スキーマのバージョン管理ポリシー
- 管理対象: `tooling/runtime/audit-schema.json`（監査 JSON Lines スキーマ）を単一の真実源とし、更新時は `schema.version` フィールドを必ずインクリメントする。命名規約は `v<major>.<minor>`。  
- 変更手順:
  1. スキーマに差分が生じる場合は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` および関連仕様 (`docs/spec/3-6-core-diagnostics-audit.md`) に反映内容を追記し、レビュー依頼の際に `schema.version` の変更理由を記録する。
  2. スキーマ更新と同じブランチで `scripts/ci/verify-audit-schema.sh`（Phase 2-4 で導入予定）を実行し、`python3 -m jsonschema --instance <audit.jsonl> --schema tooling/runtime/audit-schema.json` で生成ログを検証する。CI へ導入後は Linux / Windows / macOS ジョブで同スクリプトを実行し、`schema-report.json` をアーティファクト化する。
  3. スキーマ変更が行われた場合は `docs/migrations/audit-schema-history.md`（新設予定）または既存レポートに差分サマリを追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の本節にリンクを追加する。
- リリース前チェック: `tooling/ci/collect-iterator-audit-metrics.py` は `schema_version` の不整合を検出した場合に失敗するよう設定する。CI での失敗は Phase 2-4 のゴール条件に含め、`compiler/ocaml/docs/technical-debt.md` で追跡する。
- 互換性ウィンドウ: `schema.version` がメジャー更新 (`v2.x` → `v3.0` 等) の場合は、旧バージョンログを 2 フェーズ分保持し、`scripts/audit/upgrade-schema.py`（導入予定）で自動移行できることを確認してから旧バージョンの受理を停止する。

### 効果診断ゴールデンの整備
- ゴールデン配置: `compiler/ocaml/tests/golden/diagnostics/effects/`（`*.golden`）に JSON スナップショットを保存し、必須キー `effect.stage.required` / `effect.stage.actual` / `effect.stage.residual` / `effect.stage.source` および `diagnostic.extensions.effect.stage_trace` / `diagnostic.extensions.effect.attribute` / `diagnostic.extensions.effect.residual` を全て検証する。
- 更新手順:
  1. `remlc` を `--format=json --emit-diagnostics` モードで実行し、一時ファイルを生成。
  2. `scripts/update-effects-golden.sh`（Phase 2-2 で追加予定）を用いて対象ゴールデンのみを上書きする。自動プロモートは使用しない。スクリプトでは `stage_trace` の差分を検知し、Typer / Runtime フェーズの順序が正しいかを静的チェックする。
  3. 更新後に `tooling/ci/collect-iterator-audit-metrics.py` を実行し、`iterator.stage.audit_pass_rate` / `ffi_bridge.audit_pass_rate` が 1.0 を維持していることを確認する。
  4. 差分と検証結果を本節に追記し、Phase 2-2 の週次レビュー議事録と同期する。
- ゴールデン差分がまだ確認されていない場合や Stage 検証が未完了の場合は、`0-4-risk-handling.md` に TODO を登録して Phase 2-2 の完了条件に含める。

### 0.3.7a 効果構文メトリクス運用（Phase 2-5 追加）
- Phase 2-5 では効果構文が PoC ステージに留まるため、指標 `syntax.effect_construct_acceptance` / `effects.syntax_poison_rate` は **基準値記録のみ** を目的とする。Phase 2-7 で実装が入るまでは `collect-iterator-audit-metrics.py` が値を算出しないため、PoC テストでは人工ログを用いてフォーマットを固定化する。
- サンプル JSON（`SYNTAX-003` 計画書と `docs/notes/effect-system-tracking.md` にも掲載）:

  ```json
  {
    "effect_syntax": {
      "constructs": [
        {
          "kind": "perform",
          "tag": "Console.log",
          "sigma_before": ["Console"],
          "sigma_after": ["Console"],
          "diagnostics": []
        },
        {
          "kind": "handle",
          "tag": "Console.log",
          "sigma_before": ["Console"],
          "sigma_handler": ["Console"],
          "sigma_after": [],
          "diagnostics": ["effects.contract.residual"]
        }
      ],
      "metrics": {
        "syntax.effect_construct_acceptance": 0.0,
        "effects.syntax_poison_rate": 1.0
      }
    }
  }
  ```
- Phase 2-7 で `collect-iterator-audit-metrics.py --section effects --require-success` を実装し、`syntax.effect_construct_acceptance = 1.0` 未満または `effects.syntax_poison_rate < 1.0` の場合は CI を失敗させる。失敗時は `0-4-risk-handling.md` に記録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の効果構文タスクへリンクする。
- CLI/LSP ゴールデン更新時は `reports/diagnostic-format-regression.md` §2 のチェックリストに従い、効果構文サンプルを `tooling/lsp/tests/client_compat/fixtures/` と `compiler/ocaml/tests/golden/diagnostics/` に追加する。PoC 期間は `syntax.effect_construct_acceptance = 0.0` / `effects.syntax_poison_rate = 1.0` を維持し、正式実装で値が変化した場合はレビューで意図を説明する。

### 0.3.7b 効果行統合メトリクス運用（2026-12-18 更新）
- `type_row_mode` の既定値が `"ty-integrated"` となり、Linux/Windows/macOS すべての CI で `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success --section effects --platform <target>` を必須化する。レガシーツールチェーンが `metadata-only` を強制する場合は CI で明示的に `type_row_mode` を切り替え、互換ジョブとして扱う。
- 監視項目と合格基準:
  - `diagnostics.effect_row_stage_consistency = 1.0`: CLI/LSP 診断と監査ログで宣言順・残余集合・正規化集合が完全一致する。  
  - `type_effect_row_equivalence = 1.0`: `compiler/ocaml/tests/test_type_inference.ml` の `type_effect_row_equivalence_*` ケースが全て成功し、収集メトリクスも 1.0 を返す。  
  - `effect_row_guard_regressions = 0`: `RunConfig.extensions["effects"].type_row_mode` を `ty-integrated` へ切り替えてもガード診断が発生しない。互換モードで `metadata-only` を要求したジョブはガード件数に含めず、結果には註釈を残す。
- 監査ログには常に `effect.type_row.{declared,residual,canonical}` を出力し、`reports/diagnostic-format-regression.md` §2 のチェックリストで CLI/LSP ゴールデンとの差分をレビューする。  
- 指標が基準値から逸脱した場合は `0-4-risk-handling.md` の `TYPE-002-ROW-INTEGRATION` を再オープンし、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の Type セクションへフォローアップタスクを登録する。

### 0.3.7c 診断ドメイン可視化メトリクス運用（2026-12-21 更新）
- `collect-iterator-audit-metrics.py --section diagnostics --require-success` を CI ゲート化し、CLI/LSP/監査ログの三チャネルで新しい診断ドメイン語彙が揃っているかを自動検証する。集計結果は `reports/audit/phase2-7/diagnostics-domain-20261221.json` に保存し、ダッシュボード表示は `reports/audit/dashboard/diagnostics.md` を更新する。
- 監視項目と閾値:
  - `diagnostics.domain_coverage` ≥ 0.95: `Diagnostic.error_domain` が仕様で定義された語彙（`Effect`/`Plugin`/`Lsp`/`Capability` 等）をカバーしている割合。欠落ドメインがある場合は `docs/spec/3-6-core-diagnostics-audit.md` の索引と CLI/LSP ゴールデンを突き合わせて原因を特定する。
  - `diagnostics.plugin_bundle_ratio` ≥ 0.95: Plugin ドメイン診断で `extensions.plugin.bundle` と `AuditEnvelope.metadata["plugin.bundle.signature"]` が同時に出力されている割合。逸脱時は `docs/notes/dsl-plugin-roadmap.md` の診断連携節を参照し、バンドル署名の再発行を検討する。
  - `diagnostics.effect_stage_consistency = 1.0`: 効果ドメイン診断で `effect.stage.required` / `effect.stage.actual` が CLI/LSP/監査ログで一致しているかを確認する。値が下がった場合は `collect-iterator-audit-metrics.py --section effects` の結果と付き合わせ、Stage 検証テストを再実行する。
- エスカレーション: いずれかの指標が閾値を下回った場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#diagnostic-domain-metrics` を再オープンし、Phase 2-8 仕様監査で追加テスト (`tooling/lsp/tests/client_compat/diagnostics_*`) を設定する。

## 0.3.8 LLVM ABI テスト統計（Phase 3 Week 15）

### 実装統計
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| コード規模 | ABI実装総行数 | 211行 | abi.ml/mli（System V ABI判定・LLVM属性設定） |
| コード規模 | テストコード総行数 | 518行 | test_abi.ml（拡張後） |
| テスト | テストケース総数 | 61件 | 既存45件 + 新規16件（境界値9件、エッジケース7件） |
| テスト | 成功率 | 100% (61/61) | 回帰なし |
| カバレッジ | 型サイズテスト | 20件 | プリミティブ9件、タプル8件、エッジケース3件 |
| カバレッジ | ABI判定テスト | 26件 | 戻り値13件、引数13件（境界値・エッジケース含む） |
| カバレッジ | LLVM属性テスト | 6件 | sret 3件、byval 3件 |
| カバレッジ | デバッグ関数テスト | 4件 | 文字列表現検証 |

### ABI判定精度
| 項目 | 詳細 | 検証結果 |
|------|------|----------|
| 16バイト境界 | (i64, i8) 15バイト以下 → DirectReturn/DirectArg | ✅ 正確 |
| 16バイト境界 | (i64, i64) 16バイト → DirectReturn/DirectArg | ✅ 正確 |
| 16バイト境界超過 | (i64, i64, i8) 17バイト超 → SretReturn/ByvalArg | ✅ 正確 |
| ネスト構造 | ((i64, i64), i64) 24バイト → SretReturn/ByvalArg | ✅ 正確 |
| エッジケース | () 空タプル 0バイト → DirectReturn/DirectArg | ✅ 正確 |
| FAT pointer | {data: String, count: i64} 24バイト → SretReturn/ByvalArg | ✅ 正確 |

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `stage_mismatch_count` | 0件 | 0件 | ✅ 達成 |
| テストカバレッジ | 100% | 95%以上 | ✅ 達成 |
| 境界値検証 | 3ケース | 2ケース以上 | ✅ 達成 |

### 技術的発見
- **パディング挙動**: (i64, i8)は実際には16バイトにパディングされ、境界値以下として正しく扱われる
- **ネストタプル**: ((i64, i64), i64)はフラット化され24バイトとして正しくABI判定される
- **関数型**: 現在の実装では関数ポインタ（8バイト）として扱われ、将来的にクロージャ（16バイト）への拡張が必要

## 0.3.9 進捗ログ

### phase2.week31
| マイルストーン | 期限 | 担当（ロール） | 成果物 | 状態 | 備考 |
|----------------|------|----------------|--------|------|------|
| Kick-off レビュー会議 | 31週目 Day1 午前 | 仕様差分補正チームリード、Phase 2-7 代表 | レビュースコープ承認メモ、連絡窓口一覧 | 予定 | `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` §1.1、技術的負債 ID22/23 最新状況共有 |
| Chapter/領域別レビュー | 31週目 Day3 終了 | Chapter 1/2/3 担当、診断ログ担当 | 差分リスト初版（章別）、チェックリスト記入結果 | 予定 | `docs/plans/bootstrap-roadmap/checklists/spec-drift-review-checklist.md` を使用し、`scripts/validate-diagnostic-json.sh` 実行ログを添付 |
| スケジュール確定報告 | 31週目 Day5 終了 | 仕様差分補正チーム PM、Phase 2-7 調整役 | 週次レビュー計画（Week32-34）、`0-3-audit-and-metrics.md` 更新 | 予定 | Kick-off/領域レビュー結果を反映し、遅延時は `0-4-risk-handling.md` 登録 |

- 関連チェックリスト: `docs/plans/bootstrap-roadmap/checklists/spec-drift-review-checklist.md`（Phase 2-5 仕様差分レビュー）
- ログ記録手順: 各マイルストーン完了後に本表の状態欄を更新し、差分リスト ID とリンクを追記する。未達の場合は理由とリカバリ計画を備考欄へ記載し、必要に応じて `0-4-risk-handling.md` へエスカレーションする。

### phaseP0.review-2025-11-06
| 文書 | 主な確認事項 | 状態 | フォローアップ | Reviewer | Due |
|------|--------------|------|----------------|----------|-----|
| `docs/plans/rust-migration/0-0-roadmap.md` | セクション欠落と参照先の妥当性確認（問題なし） | 完了 | - | Codex (2025-11-06) | - |
| `docs/plans/rust-migration/0-1-baseline-and-diff-assets.md` | `collect-iterator-audit-metrics.py` 引数表記の更新（`--platform` へ修正） | 完了 | - | Codex (2025-11-06) | - |
| `docs/plans/rust-migration/0-2-windows-toolchain-audit.md` | PowerShell スクリプト引数と CI アーティファクト手順の現行化 | 完了 | - | Codex (2025-11-06) | - |
| `docs/plans/rust-migration/appendix/glossary-alignment.md` | Rust 固有語彙の不足確認（`Dual-write` 追記） | 完了 | - | Codex (2025-11-06) | - |

- 2025-11-06 / docs/plans/rust-migration/0-0-roadmap.md 他 / P0 文書セットアップのセルフレビューを実施し、`collect-iterator-audit-metrics.py --platform windows-msvc` など最新のコマンド体系へ更新。`0-2-windows-toolchain-audit.md` の PowerShell 引数と glossary の `Dual-write` 行を修正して差分を解消。 / completed (Phase P0)
  - レビュー: 2025-11-06 Rust Migration WG セルフレビュー（Codex）
  - フォローアップ: 追加課題なし。Phase P1 着手時に `reports/dual-write/` 配下のログ運用を開始する。

- 2025-10-06 / compiler/ocaml / パーサードライバーを `Result<Ast, Diagnostic>` へ移行し、`tests/test_parser.ml` に診断メタデータ検証を追加。`diagnostic_regressions` 指標は `dune test` による差分チェックで監視。
- 2025-10-07 / compiler/ocaml / Phase 3 Week 10-11 完了: Core IR 最適化パス（定数畳み込み、死コード削除、パイプライン統合）を実装。総コード行数: 約5,642行、テスト: 42件全て成功。
- 2025-10-09 / compiler/ocaml / Phase 3 Week 15 完了: ABI判定・属性設定のユニットテスト実装。総テストケース: 61件（既存45件 + 新規16件）、成功率: 100%。16バイト境界の正確な判定を検証済み。
- 2025-10-09 / tooling/ci/docker / `ghcr.io/reml/bootstrap-runtime:dev-local` を linux/amd64 でビルド（所要 ~530 秒、圧縮前 4.09GB）。`scripts/docker/run-runtime-tests.sh` と `scripts/docker/smoke-linux.sh` を実行し、既知の失敗（Let Polymorphism A2、LLVM ゴールデン差分、`basic_interpreter.reml` の構文エラー）を確認。計測値を `tooling/ci/docker/metrics.json` に記録。
- 2025-10-10 / .github/workflows / ランタイム CI 統合完了: `bootstrap-linux.yml` に Valgrind 検証・アーティファクト収集を追加し、Lint/Build/Test/Artifact の 4 ジョブ構成で Phase 1-5 §8 の CI 自動化を達成。
- 2025-10-16 / compiler/ocaml / `compiler/ocaml/scripts/benchmark_typeclass.sh --static-only` を実行し、辞書渡し／モノモルフィゼーションの静的比較レポート (`compiler/ocaml/benchmark_results/static_comparison.json`) を生成。現時点では while/for 未実装のため IR/ビットコード生成がスキップされメトリクスは 0 だが、Phase 3 でループ実装後に再計測予定。
- 2025-10-16 / tooling/ci / `collect-iterator-audit-metrics.py` → `sync-iterator-audit.sh` を手動実行し、`iterator.stage.audit_pass_rate = 1.0` を確認。`/tmp/iterator-summary.md` に生成した Markdown を次回 CI から `reports/` 階層へ保存し、週次で本ドキュメントへ転記する運用を開始。
- 2025-10-18 / tooling/runtime / `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を再実行し、`reports/runtime-capabilities-validation.json` の `runtime_candidates` に Windows (`x86_64-pc-windows-msvc`) の Stage `beta` が存在することを確認。運用手順を §0.3.7 に追記し、Phase 2-2 の Windows override 検証フローを確定。
- 2025-10-19 / tooling/runtime / `tooling/runtime/capabilities/default.json` に `arm64-apple-darwin` override（Stage `beta`, Capabilities: `ffi.bridge`, `process.spawn`）を追加。`reports/runtime-capabilities-validation.json`・`stage_trace` を手動更新し、`reports/ffi-macos-summary.md` を計測ログテンプレートとして新設。スクリプト再実行と CI ログ収集は Phase 2-3 macOS 計測タスクで実施予定。
- 2025-10-24 / docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md / `reports/diagnostic-format-regression.md` の手順に従い `npm ci --prefix tooling/lsp/tests/client_compat` と `scripts/validate-diagnostic-json.sh` を実行。ゴールデンは schema v2 に適合したが `ffi_bridge.audit_pass_rate`・`iterator.stage.audit_pass_rate` は 0.0 のままで、差分リスト初期エントリとして Phase 2-7 タスク (ID22/23) を参照する運用を開始。

## 0.3.10 ランタイムテスト統計（Phase 1-5 Week 16）

### CI 統合実装
| カテゴリ | 指標 | 値 | 備考 |
|----------|------|-----|------|
| CI ワークフロー | ステップ追加数 | 5件 | Valgrind インストール、ビルド、テスト、Valgrind 検証、アーティファクト収集 |
| テスト | 実行テストケース | 14件 | メモリアロケータ（6件）、参照カウント（8件） |
| テスト | 成功率 | 100% | 全テスト成功（ローカル検証済み） |
| メモリ検証 | Valgrind 統合 | 有効 | `--leak-check=full --error-exitcode=1` で実行 |
| メモリ検証 | AddressSanitizer | 有効 | `DEBUG=1` ビルドで自動有効化 |
| アーティファクト | 保持期間（成功時） | 30日 | `libreml_runtime.a` および `.o` ファイル |
| アーティファクト | 保持期間（失敗時） | 7日 | テストバイナリおよびログファイル |

### メモリ安全性検証
| 項目 | 検証方法 | 結果 |
|------|----------|------|
| リーク検出 | Valgrind `--leak-check=full` | ✅ 0件（全テスト通過） |
| ダングリングポインタ | AddressSanitizer | ✅ 0件（全テスト通過） |
| 二重解放 | AddressSanitizer | ✅ 0件（全テスト通過） |
| 境界チェック | AddressSanitizer | ✅ 0件（全テスト通過） |

### 自動化範囲
- ✅ ランタイムビルド（`make runtime`）
- ✅ 基本テスト実行（`make test`）
- ✅ Valgrind メモリ検証（全テストバイナリ）
- ✅ アーティファクト自動収集（成功時・失敗時）
- ✅ ローカル再現手順のドキュメント化（`runtime/native/README.md`）

### 品質指標
| 指標 | 値 | 目標 | 状態 |
|------|-----|------|------|
| `diagnostic_regressions` | 0件 | 0件 | ✅ 達成 |
| `memory_leak_count` | 0件 | 0件 | ✅ 達成 |
| `test_coverage` | 100% | 95%以上 | ✅ 達成 |
| CI 実行時間（追加分） | 約3-5分 | 10分以内 | ✅ 達成 |

### 今後の課題（Phase 2 以降）
- [ ] Windows 環境での Valgrind 代替（Dr. Memory など）
- [ ] macOS 環境でのメモリリーク検証（leaks コマンド）
- [ ] CI 実行時間の最適化（キャッシュ戦略の改善）
- [ ] クロスプラットフォームでのアーティファクト統合

---

本章で定義した指標とログフォーマットは、計画書全体の共通基盤として扱う。各 Phase 文書はここで定義した指標を利用し、進行状況と品質を定量的に管理する。
