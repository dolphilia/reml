# Core IO & Path ギャップログ

Core.IO/Core.Path 仕様（docs/spec/3-5-core-io-path.md）と Rust Runtime 実装との差分、優先度、依存タスクを整理する。`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` の実行ログおよび Phase3 Self-Host 依存管理に利用する。

## 記入フォーマット
| 日付 | 優先度 | 概要 | 影響範囲 | 状態 | 参照 |
| --- | --- | --- | --- | --- | --- |

## 最新エントリ
| 日付 | 優先度 | 概要 | 影響範囲 | 状態 | 参照 |
| --- | --- | --- | --- | --- | --- |
| 2025-12-06 | High | Reader/Writer/IoError 体系の実装ガイド（effect 記録、`with_reader`、`IoContext`）を 3-5 Plan §2.1 へ反映。`IoCopyBuffer` 設計と `core.io.*` 診断メタデータ要件は整理済みだが、Rust 実装/CI 連携が未完了。 | `compiler/rust/runtime/src/io/{reader.rs,writer.rs,mod.rs,error.rs}`, `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`, `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §2.1, `docs/spec/3-6-core-diagnostics-audit.md` | In Progress (Plan 3-5 §2.1 設計完了、実装待ち) | docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md#21-readerwriter-抽象実装, docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv |
| 2025-11-29 | High | Reader/Writer は `std::io::{Read,Write}` ラッパ (reader.rs / writer.rs) のみで、`copy`/`with_reader`/`IoContext.bytes_processed` が未実装。`effect {io, io.blocking}` 記録と `ScopeGuard` 連携が無く、監査メトリクスと Phase3 Self-Host (3-0) で要求する `config.load` サンプルを再現できない。 | `compiler/rust/runtime/src/io/{reader.rs,writer.rs,mod.rs}`, `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`, `docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` §1.1, `docs/spec/3-6-core-diagnostics-audit.md` | Backlog (Plan 3-5 §2, 要 Diagnostics 連携) | docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv, docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md |
| 2025-11-29 | Blocking | File/Buffered API（`File::*`, `FileOptions`, `FileMetadata`, `BufferedReader`, `read_line`）が Runtime に存在せず、Phase3 Reader/Config ワークロードや `effect {mem}` 計測が着手できない。`Core.Numeric & Time` (3-4) に依存する `Timestamp` との連携も未確立。 | `compiler/rust/runtime/src/io/{file.rs,options.rs,metadata.rs,buffered.rs}`, `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`, `docs/spec/3-4-core-numeric-time.md`, `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` | Backlog (Plan 3-5 §3, §3.1) | docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv, docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md |
| 2025-11-29 | Blocking | Path/Security/Watcher モジュール（`Path`, `validate_path`, `watch`, `WatchEvent`, 文字列ユーティリティ）が未作成。`effect {security}` / `effect {io.async}` 記録、Capability `fs.*` ステージ検証、`docs/guides/runtime-bridges.md` の監査サンプル更新を行えない。 | `compiler/rust/runtime/src/path/*`, `compiler/rust/runtime/src/io/watcher.rs`, `docs/spec/3-8-core-runtime-capability.md`, `docs/notes/runtime-capability-stage-log.md`, `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` | Backlog (Plan 3-5 §4, §5) | docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv, docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md, docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md |

## TODO
- [ ] 各エントリに Owner/Target Sprint を割り当て、Phase3 自動化チェックリストへ転記する。
- [ ] 差分解消後は `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` と相互リンクし、完了履歴を `reports/spec-audit/ch3/` に記録する。
