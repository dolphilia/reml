# 3.5 Core IO & Path リメディエーション計画

docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md の進捗を踏まえて、Rust Runtime の Core.IO/Core.Path 実装に残る差分（Reader/Writer 効果記録、BufferedReader の Capability 連携、Path glob、計画書資産の同期）を段階的に解消する。既存のテストや監査ログを活用しつつ、診断メタデータと Capability 要件を確実に満たすことを目標とする。

## 0. 前提と参照

- 仕様: `docs/spec/3-5-core-io-path.md`、`docs/spec/3-6-core-diagnostics-audit.md`、`docs/spec/3-8-core-runtime-capability.md`
- 現状実装: `compiler/runtime/src/io/{reader.rs,writer.rs,mod.rs,buffered.rs,effects.rs,context.rs}`、`compiler/runtime/src/path/{mod.rs,string_utils.rs}`
- 計画資産: `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md`、`docs/plans/bootstrap-roadmap/assets/{core-io-path-api-diff.csv,core-io-effects-matrix.md,core-io-capability-map.md}`、`docs/notes/stdlib/core-io-path-gap-log.md`
- テスト: `compiler/runtime/tests/{io_diagnostics.rs,file_ops.rs,path_normalize.rs,path_security.rs}` ほか

## 1. Reader/Writer 効果トラッキング整備（Phase3 W50）

### 1.1 設計・差分整理
- `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` の Reader/Writer 行を最新実装に合わせて更新し、`impl_status` に必要な項目（`Bytes` サポート・`IoContext.bytes_processed` 等）を列挙。
- `docs/notes/stdlib/core-io-path-gap-log.md` に「Reader/Writer 効果記録」ギャップを追加し、Blocking 優先度で追跡する。

### 1.2 実装
- `compiler/runtime/src/io/reader.rs`
  - トレイトに `fn read_exact_bytes(&mut self, size: usize) -> IoResult<Bytes>`、`fn read_to_end(&mut self) -> IoResult<Bytes>` を仕様どおり公開し、`Bytes` 型を返す API を整備。
  - `impl<T: Read> Reader for T` に `take_io_effects_snapshot()` を挿入し、`IoContext` に `bytes_processed` を設定。
- `compiler/runtime/src/io/writer.rs`
  - `Writer::write_all` で `IoContext` に経過バイトを反映しつつ `record_io_operation` を一括計測。
  - `write_bytes`/`write_all_bytes` を `Bytes` を直接受け取る実装へ変更。
- `compiler/runtime/src/io/mod.rs`
  - `copy` 実装で Thread Local バッファ（次節の `IoCopyBuffer`）を利用し、`IoContext.bytes_processed` を更新。
  - `with_reader` で `take_io_effects_snapshot()` と `IoContext` を `IoError` へ接続。

### 1.3 テスト/検証
- `compiler/runtime/tests/io_diagnostics.rs` に `Reader/Writer` 経由の `IoContext.bytes_processed` 検証ケースを追加。
- `docs/guides/tooling/audit-metrics.md` に `core_io.reader_write_effects_pass_rate` 指標を追記し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario reader_writer` を CI 手順に組み込む。

## 2. BufferedReader とヘルパ API の強化（Phase3 W50）

### 2.1 IoCopyBuffer とメモリ効果
- `compiler/runtime/src/io/buffered.rs`
  - `thread_local!` で 64KiB の `IoCopyBuffer` を用意し、`BufferedReader::new`／`copy` などから再利用。
  - バッファ確保時に `FsAdapter::global().ensure_security_policy()` 等の Capability が不要なことを明示するコメントを追記。
- `compiler/runtime/src/io/effects.rs`
  - `record_buffer_allocation`／`record_buffer_usage` を `IoCopyBuffer` 再利用に対応させる。

### 2.2 Capability 連携
- `compiler/runtime/src/io/buffered.rs` に `memory.buffered_io` Capability ID を取り込み、初期化時に `FsAdapter::global().ensure_buffered_io_capability()` を呼び `IoContext.with_capability` に設定。
- `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` `memory.buffered_io` 行を「Rust 実装: BufferedReader.new」で更新。

### 2.3 テスト/監査
- `compiler/runtime/tests/buffered_reader.rs`（新規）で `read_line`/`capacity`/`IoContext.buffer` の JSON 期待値を検証。
- `tests/data/core_io/buffered_reader/*.json` を整備し、`scripts/validate-diagnostic-json.sh --pattern core.io.buffered` を CI に追加。

> 実施結果（2025-11-30, §2 完了）  
> - `compiler/runtime/src/io/buffer.rs` に `IoCopyBuffer` を実装し、`io/mod.rs::copy` と `buffered.rs` の両方で 64KiB Thread-Local バッファを再利用できるようにした。`record_buffer_allocation`/`record_buffer_usage` で `effect {mem}` を共有し、`IoContext.buffer` へ capacity/fill を転写。  
> - `FsAdapter::ensure_buffered_io_capability()`（`memory.buffered_io`）を追加し、`BufferedReader::new` で Stage 検証と `IoContext.capability` 記録を行うよう更新。`core-io-path-api-diff.csv` と `core-io-effects-matrix.md` の該当行を「Implemented」に更新し、Capability マップへ `memory.buffered_io` を追記した。  
> - `compiler/runtime/tests/buffered_reader.rs` を新設して `tests/data/core_io/buffered_reader/context_snapshot.json` と突合するゴールデンテストを追加。`docs/guides/tooling/audit-metrics.md` に `core_io.buffered_reader_buffer_stats_pass_rate` 指標を登録し、CI で `collect-iterator-audit-metrics.py --section core_io --scenario buffered_reader` を実行する運用を明記した。

## 3. Core.Path glob 実装（Phase3 W51）

### 3.1 API 設計
- `compiler/runtime/src/path/glob.rs` を新規作成し、`glob(pattern: Str<'_>) -> PathResult<Vec<PathBuf>>` を `globset` crate ベースで実装。
- `effect {io, io.blocking}` を `FsAdapter::global().ensure_read_capability()` と `IoContext` で記録し、診断では `core.path.glob.*` を発行。

### 3.2 テスト
- `compiler/runtime/tests/path_glob.rs` に POSIX/Windows 別のゴールデンテストを追加（`tests/data/core_path/glob_{posix,windows}.json`）。
- `docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` の `Core.Path.glob` 行を更新し、`collect-iterator-audit-metrics.py --section core_io --scenario path_glob` の計測手順を `docs/guides/tooling/audit-metrics.md` に追記。

### 3.3 セキュリティ連携
- glob 経由でディレクトリ traversal が起きないよう `PathSecurityError` と `SecurityPolicy` を組み合わせた検証を行い、`docs/notes/runtime/runtime-capability-stage-log.md` に結果を記録。

> 実施結果（Phase3 W51, §3 完了）  
> - `compiler/runtime/src/path/glob.rs` を追加し、`glob` crate での列挙結果を `PathBuf` へ正規化してソート、`FsAdapter::ensure_read_capability()` を経由した `io.fs.read` Capability 検証、`PathErrorKind::{InvalidPattern,Io}` でのエラー報告を実装。`PathError` へ新種別を追加し、`glob` API が `effect {io, io.blocking}` を担う経路を整理した。  
> - フィクスチャ `tests/fixtures/path_glob/*` とゴールデン `tests/data/core_path/glob_{posix,windows}.json`、統合テスト `compiler/runtime/tests/path_glob.rs` を作成し、`cargo test --manifest-path compiler/runtime/Cargo.toml path_glob` を通して POSIX/Windows 両ケースでの一致を確認。  
> - `core.path.glob.*` 診断コードと `metadata.io.glob.*` を `PathError`/`IoError` に接続し、`IoContext` へ glob メタデータを保持できるようにした。`compiler/runtime/tests/path_glob.rs` と `tests/path_glob.rs` に診断テストを追加し、`docs/guides/tooling/audit-metrics.md` の KPI 説明を更新。  
> - `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`・`core-io-effects-matrix.md`・`docs/guides/tooling/audit-metrics.md` に `path_glob` シナリオと CI 実行手順を追記し、`docs/notes/stdlib/core-io-path-gap-log.md` へギャップ解消ログを登録。`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` / Remediation 章双方で W51 タスク完了を明記した。

## 4. ドキュメント/資産同期（Phase3 W51）

### 4.1 API Diff 更新
- `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`
  - Reader/Writer/BufferedReader/File/Watcher 行の `impl_status` を最新状態へ更新し、残タスク（`glob`・`watcher` オプション）を明確化。
- `docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` に `metadata.io.helper`、`io.buffer.*`、`core.path.glob` の検証項目を追記。

### 4.2 ギャップログ整理
- `docs/notes/stdlib/core-io-path-gap-log.md`
  - Reader/Writer/BufferedReader のギャップを「In Progress → Closed」へ移動し、新たに glob/CI 指標のフォローアップを登録。

### 4.3 仕様・ガイド連携
- `docs/spec/3-5-core-io-path.md` の Reader/Writer サンプルと Path glob 節を Rust 実装に合わせて微調整。
- `docs/guides/runtime/runtime-bridges.md` に `IoContext`/`WatcherAudit`/`glob` の利用例と Capability チェック手順を追記。

## 5. スケジュールと成果物

| 週 | タスク | 主要成果物 | 完了条件 |
| --- | --- | --- | --- |
| W50 | §1 Reader/Writer、§2 BufferedReader | `reader.rs`/`writer.rs`/`buffered.rs` 更新、`tests/buffered_reader.rs` | `cargo test --manifest-path compiler/runtime/Cargo.toml reader_writer buffered_reader` 緑、`collect-iterator-audit-metrics.py --section core_io --scenario reader_writer` 成功 |
| W51 | §3 glob、§4 ドキュメント整理 | `path/glob.rs`、テストデータ、`core-io-*` 資料更新 | `scripts/validate-diagnostic-json.sh --pattern core.path.glob` 通過、`core-io-path-api-diff.csv`/`gap-log.md` 同期 |

## 6. リスクとフォローアップ

- **性能劣化**: `IoCopyBuffer` 再利用で GC しにくくなる恐れがあるため `criterion` ベンチを `compiler/runtime/benches/bench_core_io.rs` に追加し、差分が ±15% を超えた場合は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に記録。
- **クロスプラットフォーム差分**: glob/BufferedReader の挙動が OS 依存になる可能性があるため、CI で `matrix = {linux, macos, windows}` を必須化し、差分が出た場合は `docs/plans/bootstrap-roadmap/2-6-windows-support.md` のフォローアップへリンク。
- **Capability 未定義**: `memory.buffered_io` や `watcher.resource_limits` が Registry 未登録の場合は `docs/notes/runtime/runtime-capability-stage-log.md` に TODO を追加し、`docs/plans/rust-migration/3-8-core-runtime-capability-plan.md` の調整を依頼する。
