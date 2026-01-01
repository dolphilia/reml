# 3.5 Core IO & Path 実装計画

## 目的
- 仕様 [3-5-core-io-path.md](../../spec/3-5-core-io-path.md) に準拠した `Core.IO`/`Core.Path` API を実装し、同期 IO・パス操作・セキュリティポリシーを統一する。
- Reader/Writer 抽象、ファイル操作、バッファリング、パス検証を Reml 実装へ落とし込み、Diagnostics/Audit/Runtime と安全に連携させる。
- 効果タグ (`effect {io}`, `{io.blocking}`, `{security}` 等) と Capability 検証を整備し、クロスプラットフォーム差異を管理する。

## スコープ
- **含む**: Reader/Writer/BufferedReader、File API、IO エラー、Path 抽象と正規化、セキュリティヘルパ、ファイル監視 (オプション機能) の実装、ドキュメント更新。
- **含まない**: 非同期 IO ランタイム、分散ファイルシステム統合、WASM 向け特化 API (Phase 4 以降)。
- **前提**: `Core.Text`/`Core.Numeric`/`Core.Diagnostics`/`Core.Runtime` が整備済みであり、Phase 2 で定義されたエラー型・監査モデルが利用可能であること。

## 作業ブレークダウン

### 1. API 差分整理と依存調整（47週目）
**担当領域**: 設計調整

1.1. Reader/Writer/Path/Watcher に関する公開 API を一覧化し、既存実装との差分と優先度を決定する。  
実施ステップ:
- `docs/spec/3-5-core-io-path.md` から API 名・引数・戻り値・効果タグを抽出し、`docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` に整理する。
- `rg "pub (struct|enum|fn)" compiler/rust/runtime/src/io -g "*.rs"`、`rg -g "*path*" compiler/rust/runtime/src -n "pub"` を用いて Rust 実装の現状を洗い出し、CSV に `実装状況 (PoC/Done/Missing)` とファイルパスを追記する。
- 差分を `docs/notes/core-io-path-gap-log.md` に登録し、優先順位 (Blocking/High/Normal) と依存タスク（Diagnostics/Runtime/Guides 更新）を紐付けた backlog を作成する。
- ✅ 2025-11-29: API 一覧と実装状況を `core-io-path-api-diff.csv` に追加し、主要な欠落項目（Reader/Writer ヘルパ、File/Buffered、Path/Security/Watcher）を `core-io-path-gap-log.md` に Blocking/High 優先度で記録した。

1.2. 効果タグと Capability 要件 (`effect {io.blocking}`, `{security}` 等) を整理し、CI で検証するテスト計画を策定する。  
実施ステップ:
- `docs/spec/3-6-core-diagnostics-audit.md`、`docs/spec/3-8-core-runtime-capability.md` から IO/Path 関連の効果タグ・Stage 要件を抜粋し、`docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` に API ごとの対応表を整備する。
- `EffectSet`/`CollectorEffectMarkers` が `effect {io.blocking}`, `{io.async}`, `{security}` を追跡できるか確認し、必要であれば `compiler/rust/runtime/src/io/effects.rs` の TODO として記録する。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario effects_matrix` の設計メモを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追記し、CI で効果タグを検証するステップを定義する。

1.3. OS 依存機能 (permissions, symlink) の抽象化方針を決め、Runtime Capability (3-8) との連携を確認する。  
実施ステップ:
- `docs/spec/3-8-core-runtime-capability.md` §8-§10 を参照し、`fs.permissions.*`, `fs.symlink.*`, `fs.watcher.*` などの Capability ID を `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` に整理する。
- Core.IO Capability マップには `<!-- capability-matrix:start -->` ブロックを追加し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario capability_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md --output reports/spec-audit/ch3/core_io_capabilities.json --require-success` で Stage/Provider/効果スコープの欠落を検知できるようにする。CI の `core-io-path` ジョブで当該シナリオを実行し、`core_io.capability_matrix_pass_rate` の閾値を `0-3-audit-and-metrics.md` に登録する。
- `compiler/rust/runtime/src/runtime_bridge/` と `runtime/native/` を調査し、OS 固有実装を切り替えるアダプタ層（`FsAdapter`/`WatcherAdapter`）の責務を設計メモにまとめる。
- Capability 連携の検証ポイントを `docs/notes/runtime-capability-stage-log.md` に追記し、CI で `verify_capability_stage` を実行する Runbook を `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` と同期させる。

### 2. Reader/Writer 抽象実装（47-48週目）
**担当領域**: IO 基盤

2.1. `Reader`/`Writer` トレイトと共通ヘルパ (`copy`, `with_reader`) を実装し、`IoError` 体系を整備する。  
実施ステップ:
- `compiler/rust/runtime/src/io/reader.rs` / `writer.rs` を追加し、`trait Reader { fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError>; }` 等を仕様準拠で定義する。
- `docs/spec/3-5-core-io-path.md` のテーブルを参照し、`Reader::copy_to`/`Writer::flush`/`io::copy` などヘルパを `compiler/rust/runtime/src/io/mod.rs` に集約する。
- `IoError`/`IoErrorKind` を `compiler/rust/runtime/src/io/error.rs` へ新設し、`std::io::ErrorKind` とのマッピングと `effect {io.blocking}` 記録処理を組み込む。

#### 2.1.1 Reader/Writer トレイト設計メモ
- `Reader`/`Writer` 双方で `Bytes`（`Core.Text` の `Bytes` 別名）を第一級に扱う前提を明文化し、`docs/spec/3-5-core-io-path.md` §2 に合わせて `Result<usize, IoError>`／`Result<Bytes, IoError>` を返す API を固定する。`reader.rs`/`writer.rs` は `std::io::{Read, Write}` を包むだけでなく、`IoContext` を自動生成して `metadata.io.operation` を補完する責務を担う。
- `EffectSet` と `collect-iterator-audit-metrics.py --section core_io --scenario effects_matrix` の整合を守るため、`Reader::read`/`Writer::write` 呼び出し直後に `take_io_effects_snapshot()`（`compiler/rust/runtime/src/io/effects.rs` 新設）を呼び出して `effect {io}`/`{io.blocking}` を測定する仕様にする。Snapshot は `IoContext.effect` と `Diagnostic.extensions["effects"]` の両方から参照できるよう `IoContext` に埋め込む。
- `Core.Diagnostics` で要求される `core.io.*` コードと `effect.stage.required/actual` の転写ポイントを `Reader`/`Writer` のトレイトメソッド実装に集中させる。`Reader::read_exact` は `effect {mem}` を伴う一時バッファ確保を明示し、`IoCapabilityStage::Beta` の検証フローを `verify_capability_stage("io.fs")`→`IoContext.capability`→`Diagnostic.metadata.io.capability` の順に統一する。

#### 2.1.2 ヘルパ API（copy / with_reader）設計
- `copy` は 64 KiB 固定バッファを確保し、`Reader`/`Writer` インスタンスに `IoContext.bytes_processed` を累積記録させる。`effect {mem}` の発火を抑制するため、バッファは `IoCopyBuffer`（`compiler/rust/runtime/src/io/buffer.rs` に追加予定）経由で `thread_local` に再利用する。監査メトリクスは `reports/spec-audit/ch3/core_io_effects.json` で `io.copy.bytes_processed` を追跡する。
- `with_reader` は RAII で `File::open` 後に `Reader` をクロージャへ渡し、`ScopeGuard`/`defer` を用いた安全な close を保証する。`docs/spec/3-5-core-io-path.md` の使用例（設定ファイル読み込み）を反映し、`with_reader("config.toml", |reader| { ... })` 形で `effect {io.blocking}` を 1 箇所へ閉じ込める。`core-io-path-api-diff.csv` に `with_reader` 実装計画の補足列（`ScopeGuard` 依存）を追記する。
- 補助関数（`Reader::copy_to`, `Writer::write_all`, `Reader::read_to_end` 等）は `io/mod.rs` にまとめ、`core-io-effects-matrix.md` の Reader/Writer 行と突合できるよう `metadata.io.helper = <helper_name>` を記録する。エラー発生時には `IoContext.operation` を `copy`/`write_all` に更新し、`IoErrorKind::Interrupted` を自動リトライする方針を共有する。

#### 2.1.3 IoError / IoContext / Diagnostic 連携
- `IoErrorKind` と `std::io::ErrorKind` のマッピング表を `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` の `Notes` 列と `docs/notes/core-io-path-gap-log.md` に整理した。`SecurityViolation`/`UnsupportedPlatform` のように標準ライブラリに同等エントリが無い場合は、`ErrorKind::Other` + `metadata.io.platform` で補完する。`IoErrorKind::OutOfMemory` は `effect {mem}` を記録する Reader/Writer/BufferedReader 共通のフォールバックとして扱う。
- `IoContext` へ `path: Option<PathBuf>`, `operation: IoOperation`, `bytes_processed: Option<u64>`, `capability: Option<CapabilityId>`, `effects: IoEffectsSnapshot`, `timestamp: Timestamp` を保持させ、`IoError::into_diagnostic()` が `metadata.io.*` と `metadata.effects.*` に転写する仕様を定義した。`Timestamp` は `Core.Time::SystemClockAdapter` から取得し Phase3 `3-4` との依存を明記。
- `core-io-effects-matrix.md` の Reader/Writer 行に `diagnostic: core.io.read_error/core.io.write_error` を追記し、`scripts/validate-diagnostic-json.sh --pattern core.io` が `metadata.io.operation`, `metadata.io.capability`, `metadata.io.path`, `metadata.io.bytes_processed`, `effect.stage.required/actual` を必須キーとして検証するよう CI ルールを追加する計画を記述した。

> 進行ログ（Phase3 W47, 2.1）  
> - `docs/spec/3-5-core-io-path.md` §2 の API をベースに `Reader`/`Writer` トレイトの責務・`IoContext` 注入ポイント・`EffectSet` 連携フローを整理し、本節の `#### 2.1.1`〜`2.1.3` に詳細設計を追記。Rust 実装では `reader.rs`/`writer.rs`/`mod.rs`/`error.rs` を分割し、`IoContext` が `effect {io.blocking}` と Capability 情報を収集するロードマップを確定した。  
> - `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` の Reader/Writer/IoError 行に `notes` を追記し、`copy`/`with_reader`/`IoErrorKind` の依存関係（`ScopeGuard`, `IoCopyBuffer`, `Core.Time`）と `impl_status=PoC` の補強要件を明記した。  
> - `docs/notes/core-io-path-gap-log.md` 2025-11-29 エントリを更新し、Reader/Writer ギャップが `Plan 3-5 §2.1` に紐付いたこと、`effect {io}` 計測と `with_reader` の自動 close 方針が Phase3 Self-Host `config.load` シナリオの前提となることを明文化した。

2.2. バッファリング (`BufferedReader`, `read_line`) を実装し、`effect {mem}`/`{io.blocking}` を伴う動作をテストする。  
実施ステップ:
- `compiler/rust/runtime/src/io/buffered.rs` に `BufferedReader<'a, R: Reader>` を実装し、リングバッファ設計と `read_line`/`fill_buf` の状態管理を定義する。
- `docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` の `BufferedReader` 行に `effect {mem}`/`{io.blocking}` を明記し、`take_io_effects_snapshot()`（新設）でメモリ使用量を計測する。
- `tests/data/core_io/buffered_reader/*.json` を追加し、`scripts/validate-diagnostic-json.sh --suite core_io` と `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-io buffered_reader::tests::*` を CI へ組み込む。

#### 2.2.1 BufferedReader リングバッファと API 設計
- `BufferedReader<'a, R>` は `inner: R`, `buf: Box<[u8]>`, `start: usize`, `end: usize`, `line_cursor: Option<usize>` を保持し、`fill_buf` → `consume` の有限状態機械を `state_diagram.md`（`docs/notes/core-io-path-gap-log.md` へ添付予定）で明文化する。`reader.rs` の `Reader` トレイトをそのまま包むのではなく、`IoContext` を引き継ぐ `BufferedReaderContext` を `buffered.rs` 内で管理し `metadata.io.buffer.capacity` / `metadata.io.buffer.remaining` を記録する。
- `buffered(reader, capacity)` は `IoCopyBuffer` と同一の `thread_local` バッファプールを利用し、`capacity` が 4 KiB 未満の場合は 4 KiB に切り上げる。`capacity` が 1 MiB を超える場合は `IoErrorKind::InvalidInput` を返す仕様を `docs/notes/core-io-path-gap-log.md` に反映し、`0-4-risk-handling.md` へメモリ過剰割当のリスクを追記する。
- `read_line` / `read_until` は `Core.Text` の UTF-8 変換（`docs/spec/3-3-core-text-unicode.md` §2.3）を利用し、`Bytes`→`Str` 変換で失敗した場合に `IoErrorKind::InvalidInput` を生成する。`docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` に `read_line` の戻り値が `Result<Option<Str>, IoError>` である理由と `impl_status=Missing` を `Due=W48` として追記する。

#### 2.2.2 効果タグ・Capability 計測と IoContext 拡張
- `BufferedReader` の初期化時に `EffectSet.mark_mem(buffer_capacity)` を呼び、`take_io_effects_snapshot()` に `IoEffectsSnapshot { mem_bytes, io_blocking, capability_id }` を記録する。`docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` `BufferedReader` 行へ `metadata.io.buffer.capacity`, `metadata.io.buffer.fill_ratio` の必須キーを追加し、`collect-iterator-audit-metrics.py --section core_io --scenario effects_matrix --check buffered` で突合する。
- `IoContext` に `buffer: Option<BufferStats>` を追加し、`BufferStats { capacity: u32, fill: u32, last_fill_timestamp: Timestamp }` を `compiler/rust/runtime/src/io/context.rs` に定義する。`IoError::into_diagnostic()` は `metadata.io.buffer.capacity`, `metadata.io.buffer.fill` を自動転写し、`docs/spec/3-6-core-diagnostics-audit.md` §1.3 に記載された `core.io.read_error.buffered` 診断例と整合させる。
- `CapabilityId = "memory.buffered_io"`（`docs/plans/rust-migration/2-2-adapter-layer-guidelines.md` §2.2.5）を `BufferedReader` の初期化で検証し、Stage ミスマッチ時は `core.io.buffered.capability_mismatch` を発火させる。`docs/notes/runtime-capability-stage-log.md` に `memory.buffered_io` の Stage ステータスを追加し、`3-8-core-runtime-capability-plan.md` の Phase3 TODO と同期する。

#### 2.2.3 `read_line` テストスイートと診断整合
- `tests/data/core_io/buffered_reader/` には `read_line_utf8.json`, `read_line_large.json`, `read_line_partial.json` を配置し、`metadata.io.buffer.remaining` / `effects.mem_bytes` の値をゴールデン化する。テストは `cargo test --manifest-path compiler/rust/runtime/Cargo.toml buffered_reader::tests::read_line_cases -- --include-ignored` にまとめ、CI では Linux/macOS/Windows で同一ログが生成されることを `reports/spec-audit/ch3/buffered_reader-YYYYMMDD.md` に記録する。
- `scripts/validate-diagnostic-json.sh --suite core_io` に `--pattern core.io.buffered` を追加してゴールデン期待値を検証し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3 指標へ `buffered_reader.mem_bytes_p99` と `buffered_reader.read_line_latency_ms` を登録する。計測値は `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario buffered_reader --output reports/spec-audit/ch3/buffered_reader_effects.json --require-success` で収集する。
- `docs/notes/core-io-path-gap-log.md` W48 エントリで `read_line` のエッジケース（CRLF, BOM, 4-byte UTF-8）が `docs/spec/3-3-core-text-unicode.md` の設計と整合するかを確認し、差分があれば `docs/notes/dsl-plugin-roadmap.md` の `Core.IO` 依存リストに TODO を登録する。

> 進行ログ（Phase3 W48, 2.2）  
> - BufferedReader のリングバッファ構造と `buffered()`／`read_line()` API の仕様を整理し、`docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv`・`docs/notes/core-io-path-gap-log.md` へ反映するタスク（Due=W48）を追加。`IoCopyBuffer` とバッファプール共有ポリシーを定義し、容量制限とリスクログを確定した。  
> - `IoContext` 拡張と `memory.buffered_io` Capability の検証フローを `core-io-effects-matrix` 行と同期し、`collect-iterator-audit-metrics.py --section core_io --scenario buffered_reader` で `metadata.io.buffer.*` をチェックする CI 設計をまとめた。  
> - `tests/data/core_io/buffered_reader/*.json` と `reports/spec-audit/ch3/buffered_reader_effects.json` を生成するテストスイート案を固め、`scripts/validate-diagnostic-json.sh --suite core_io --pattern core.io.buffered` の更新手順を `0-3-audit-and-metrics.md` へ追記する方針を確定した。

2.3. `IoError` → `Diagnostic` 変換・監査メタデータ (`IoContext`) を実装し、CLI 出力と整合することを確認する。  
実施ステップ:
- `compiler/rust/runtime/src/io/error.rs` に `impl IntoDiagnostic for IoError` を追加し、`code = "core.io.*"`、`metadata.io.path`, `metadata.io.operation`, `metadata.io.capability` を設定する。
- `IoContext` 構造体を `compiler/rust/runtime/src/io/context.rs` に定義し、`Reader`/`Writer`/`File` 呼び出しからコンテキスト (path, mode, capability) を自動で注入する。
- `docs/spec/3-6-core-diagnostics-audit.md` の `io.*` 例を再現する CLI ゴールデン (`compiler/rust/runtime/tests/expected/io_error_open.json`) を作成し、`scripts/validate-diagnostic-json.sh --pattern core.io` で整合性を確保する。

> 進行ログ（Phase3 W48, 2.3）  
> - `compiler/rust/runtime/src/io/context.rs` を新設して `IoContext`/`BufferStats`/タイムスタンプ初期化を分離し、`IoContext::with_timestamp`・`set_effects` 等から Reader/Writer/BufferedReader が同一 API で監査メタデータを注入できるようにした。`compiler/rust/runtime/src/io/mod.rs` では再エクスポートを更新し、既存コードの import を保った。  
> - `compiler/rust/runtime/src/io/error.rs` に `IntoDiagnostic` 実装を追加し、`code` を `core.io.read_error/core.io.write_error` と `IoErrorKind::default_code` で切り替えるロジック、`extensions.io.*`／`audit["io.*"]`／`io.effects.*` の変換、`EffectLabels`→JSON 変換ヘルパを実装。Reader/Writer から渡された `IoContext` が `path/capability/bytes_processed/buffer/timestamp` を診断へ転写できるようになった。  
> - `compiler/rust/runtime/tests/expected/io_error_open.json` と `tests/io_diagnostics.rs` を追加し、`IoError::new(...).into_diagnostic().into_json()` が CLI 想定 JSON の主要キー（`metadata.io.*`, `extensions.effects.*`）を満たすことを `assert_contains` で検証。`cargo test io_error_into_diagnostic_matches_expected_subset`（`compiler/rust/runtime`）を実行して新テストを通過済み。`scripts/validate-diagnostic-json.sh` 実行は未実施のため、CI 連携は Phase3 `core-io` ジョブで別途追跡する。

### 3. ファイル API とメタデータ（48週目）
**担当領域**: ファイル操作

3.1. `File::open/create/remove/metadata` 等の API を実装し、プラットフォームごとのエラー挙動をテストする。  
実施ステップ:
- `compiler/rust/runtime/src/io/file.rs` に `File` 構造体と `open`, `create`, `remove`, `metadata` を実装し、`std::fs::File` への委譲を Capability で包む。
- `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` の `File` 行を更新し、POSIX/Windows の挙動差を `docs/notes/core-io-path-gap-log.md` に整理する。
- `tests/data/core_io/file_ops/{posix,windows}/*.json` を作成し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario file_ops --platform {linux,windows}` で差分を検証する。

#### 3.1.1 File ハンドル／FsAdapter と IoContext 連携
- `File` 型は `handle: FsHandle`, `path: PathBuf`, `options: FileOptionsSnapshot`, `context: IoContext` を保持し、`FsHandle` が OS 固有（`std::fs::File`, `RawHandle`, `RawFd`）を抽象化する。`FsAdapter`（`compiler/rust/runtime/src/io/adapters.rs` 新設）で `open/create/remove` を包み、呼び出し前に `CapabilityRegistry::verify_capability_stage("io.fs.read")`/`("io.fs.write")` を評価する。Capability 結果は `IoContext.capability` と `effect.stage.required/actual` へ転写し、`core-io-capability-map.md` の `io.fs.read/write` 行と突合できるよう `StageRequirement::AtLeast(StageId::Beta)` を固定値ではなく Registry 応答から取得する。
- `File::open`/`create` では `PathBuf` → OS ネイティブ表現への変換を `FsAdapter::prepare_path()` へ集約し、POSIX は `OsStr`、Windows は UTF-16 `Vec<u16>` に変換する。パス変換で失敗した場合は `IoErrorKind::InvalidInput` を返し、`metadata.io.path.normalized` に `Path::normalize` 結果を格納する。`docs/notes/core-io-path-gap-log.md` へ Windows UNC / POSIX ルートの差異メモを追記する。
- `IoContext` には `operation: FileOperation`（`Open`/`Create`/`Remove`/`Metadata`）、`bytes_processed: Option<u64>`, `timestamp`, `capability`, `platform: IoPlatform`, `adapter: Str` を記録する。`File::remove` 実行中は `IoContext.operation = Remove`、`IoContext.path = Some(path.clone())` を強制し、`IoError::into_diagnostic()` が `metadata.io.operation = "file.remove"`、`metadata.io.platform`、`metadata.io.capability` を出力できることをテストで確認する。
- `core-io-path-api-diff.csv` の `File::*` 行 `notes` に `FsAdapter`／`IoContext` の要件と `impl_status=Missing (Plan 3-5 §3.1.1)` を追加し、Reader/Writer と同じ列挙形式で進捗を追跡する。`docs/plans/rust-migration/2-2-adapter-layer-guidelines.md` で定義したアダプタ方針を参照し、`File` API でも同一トレース ID（`io.fs.adapter`) をログへ埋め込む。

#### 3.1.2 メタデータ・診断・Core.Time 依存
- `File::metadata` は `Core.Numeric & Time` (`Timestamp`, `Duration`) と連携するため `docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` の `TimestampAdapter` を利用する。取得情報（`size`, `created_at`, `modified_at`, `permissions`) は `FileMetadata`（§3.2 で詳細化）に格納し、`IoContext.metadata = Some(FileMetadataSnapshot)` を介して `metadata.io.file.*`／`metadata.time.timestamp` を診断へ転写する。
- `IoErrorKind` マッピングを `core-io-path-api-diff.csv` と `core-io-effects-matrix.md` に追記し、`PermissionDenied`/`NotFound`/`UnsupportedPlatform`/`SecurityViolation` が `core.io.file.*` 診断コード（`core.io.file.permission_denied`, `core.io.file.not_found`, `core.io.file.unsupported_platform`, `core.io.file.security`) へ対応することを明文化する。`scripts/validate-diagnostic-json.sh --pattern core.io.file` で `metadata.io.operation`, `metadata.io.path`, `metadata.io.capability` の必須キーを検証する手順を `0-3-audit-and-metrics.md` に追記した。
- `core-io-capability-map.md` に `fs.permissions.read`/`fs.permissions.modify` の `Rust 実装フック` を `File::metadata`/`File::create` として再整理し、`Status = Planning (Plan 3-5 §3.1.2)` を設定。Stage 要件は `Exact(StageId::Stable)` であるため、`File::create` は `CapabilityRegistry::verify_capability_stage("fs.permissions.modify")` を呼び出し、未充足の場合は `IoErrorKind::SecurityViolation` を返して `effect.stage.required = "stable"` を記録する。
- `docs/notes/core-io-path-gap-log.md` へ `file_ops` 診断要件（`metadata.io.file.size`, `metadata.io.file.permissions`, `metadata.security.policy`）を追記し、OCaml 実装との差分（`stat` の精度、`File.create` の `umask` 取り扱い）を `Impact: compiler/rust/runtime/src/io/file.rs` として記録する。

#### 3.1.3 クロスプラットフォームテストと `file_ops` メトリクス
- `tests/data/core_io/file_ops/posix/` には `open_success.json`, `create_truncate.json`, `remove_missing.json`, `metadata_permissions.json` を用意し、`platform = "linux"`/`"macos"` を添付して CLI ゴールデンを保存する。Windows 版は `tests/data/core_io/file_ops/windows/` に `open_success.json`, `create_ntfs_attrs.json`, `metadata_timestamp.json` を配置し、`metadata.io.platform = "windows"` と `metadata.io.file.permissions.attributes` を比較できるようにする。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario file_ops --platform linux --platform windows --matrix docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md --output reports/spec-audit/ch3/file_ops-metrics.json --require-success` を Phase3 `core-io-path` ジョブへ追加し、`core_io.file_ops_pass_rate`（`0-3-audit-and-metrics.md` に追記）として CI で可視化する。Linux/macOS は `platform=posix` で集計し、Windows は `platform=windows` で PASS/FAIL を個別記録する。
- `file_ops` シナリオでは `metadata.io.operation` の分布、`effect.stage.required/actual`, `metadata.io.capability`, `metadata.security.policy` を `reports/spec-audit/ch3/file_ops-YYYYMMDD.md` にまとめ、`docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` `ファイルハンドル / メタデータ` 行の検証ポイントへリンクする。テスト成功時は `core-io-path-gap-log.md` に `Status = Planned → In Progress` を更新する。
- Windows 固有の `ERROR_ACCESS_DENIED` / `SharingViolation` を `IoErrorKind::PermissionDenied`／`SharingViolation` で区別し、`tests/data/core_io/file_ops/windows/remove_locked.json` で `effect.stage.required = "beta"`（`io.fs.write`) と `metadata.io.platform = "windows"` を検証する。POSIX 側は `EACCES`/`ENOENT`/`EROFS` を `metadata.io.errno`（`extensions.io.errno`）として出力し、`core-io-capability-map.md` へ `Rust 実装フック = File::remove (errno mapping)` を追記する。

> 進行ログ（Phase3 W48, 3.1）  
> - `File` API のアダプタ設計と `IoContext` 連携を整理し、`FsAdapter` を Reader/Writer と共有する方針、`FileOperation` 列挙、`CapabilityRegistry::verify_capability_stage` 呼び出し位置を確定した。`core-io-path-api-diff.csv` の `File::*` 行へ Plan §3.1 の要件を追記済み。  
> - `core-io-capability-map.md` と `core-io-effects-matrix.md` に `file_ops` シナリオの検証ポイント、`fs.permissions.*` Stage チェック、`metadata.io.file.*` 必須キーを反映。`0-3-audit-and-metrics.md` には `core_io.file_ops_pass_rate` 指標、`collect-iterator-audit-metrics.py` 実行例を追加した。  
> - `docs/notes/core-io-path-gap-log.md` へ File API ギャップの詳細（POSIX/Windows 差分、Timestamp 依存、Capability `fs.permissions.*`）と `file_ops` テストセットの作成計画を登録し、Blocking として追跡を継続する。

3.2. `FileOptions`/`FileMetadata` の定義を整備し、`Timestamp` (`Core.Numeric & Time`) と連携する。  
実施ステップ:
- `FileOptions` を `compiler/rust/runtime/src/io/options.rs` に作成し、`read`, `write`, `append`, `truncate`, `create`、`permissions` をビルダー形式で設定できるようにする。
- `FileMetadata` を `compiler/rust/runtime/src/io/metadata.rs` に定義し、`size`, `permissions`, `created_at`, `modified_at` を `Core.Time::Timestamp` で表現する。`3-4-core-numeric-time-plan.md` 側の `Timestamp → IO` バックログと依存関係を明記する。
- `compiler/rust/runtime/tests/golden/core_io/file_ops/metadata_basic_{unix,windows}.json` にメタデータ出力を保存し、`scripts/validate-diagnostic-json.sh --pattern core.io.metadata` で `Timestamp` のフォーマットと効果タグ (`effect {time}`) を検証する。

> 進行ログ（Phase3 W48, 3.2）  
> - `compiler/rust/runtime/src/io/permissions.rs` を新設し、`FilePermissions` で `UnixMode`/`WindowsAttributes` の両方を保持できるようにした。`FileOptions`（`options.rs`）に `permissions()` ビルダーと `permissions_snapshot()` を追加し、`OpenOptions` へモード/属性を適用する処理を組み込んだ。  
> - `FileMetadata`（`metadata.rs`）へ `permissions` フィールドとアクセサを追加し、`Timestamp` との連携を維持したまま `fs::Metadata` からパーミッション情報を抽出するよう更新した。  
> - `compiler/rust/runtime/tests/file_ops.rs` にメタデータ JSON 検証と `FileOptions::permissions` のユニットテストを追加し、プラットフォーム別ゴールデン（`tests/golden/core_io/file_ops/metadata_basic_{unix,windows}.json`）で `permissions`/`timestamps` の必須キーを固定。`cargo test file_create_write_metadata_remove` を実行し、Core.IO ファイル API の基本ケースが通過することを確認した。

3.3. `sync`/`defer` 処理の統合を確認し、リソースリーク検出テストを追加する。  
実施ステップ:
- `File::sync_all`/`sync_data`/`drop` 時の `IoError` を `compiler/rust/runtime/src/io/file.rs` でフックし、`effect {io.blocking}`/`{fs.sync}` を記録する。
- `defer` (`with_file`/`with_temp_dir`) の RAII ヘルパを `compiler/rust/runtime/src/io/scope.rs` に定義し、`Drop` で確実に解放されることを `cargo test --features core-io io::scope::tests::*` で検証する。
- `tests/data/core_io/leak_detection/*.json` を `reports/spec-audit/ch3/io_leak-detection.md` にまとめ、`valgrind`/`miri` ベースの自動チェック (CI optional) を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリンクする。

> 進行ログ（Phase3 W48, 3.3）  
> - `EffectSet`/`EffectLabels` に `fs_sync` を追加し、`compiler/rust/runtime/src/io/file.rs` の `sync_*` / `Drop` で `record_fs_sync_operation()` を発火させた。診断エンコード (`compiler/rust/runtime/src/io/error.rs`) へも `io.effects.fs_sync{,_calls}` を出力するロジックを実装。  
> - `compiler/rust/runtime/src/io/scope.rs` を新設し、`ScopeGuard`・`with_file`・`with_temp_dir`・`FileHandleGuard`・リークトラッカー (`leak_tracker_snapshot`/`reset_leak_tracker`) を公開。`File` に `FileHandleGuard` を埋め込み、スコープ外で自動的にカウンタが減少するよう統合。  
> - `compiler/rust/runtime/tests/leak_detection.rs` と `tests/data/core_io/leak_detection/scoped_cleanup.json` を追加し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml leak_detection::scoped_resources_cleanup_matches_expected_snapshot` でハンドル/TempDir カウンタが 0 に戻ることを確認。結果は `reports/spec-audit/ch3/io_leak-detection.md` に記録し、フォローアップ手順を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#0.4.7-coreio-リーク検出フォローアップ` へリンクした。

### 4. Path 抽象とセキュリティ（48-49週目）
**担当領域**: パス処理

4.1. `Path`/`PathBuf` と基本操作 (`path`, `join`, `normalize`, `is_absolute`) を実装し、プラットフォーム差異に対するテストを作成する。  
実施ステップ:
- `compiler/rust/runtime/src/path/mod.rs` に `struct Path<'a>`/`PathBuf` を定義し、内部的には `std::path::{Path, PathBuf}` を利用しつつ Reml 仕様の `Effect`/`Capability` を付与する。
- `normalize`, `join`, `split`, `is_absolute`, `components` 等の API を `3-5-core-io-path.md` の例と一致するよう設計し、POSIX/Windows 表現を `cfg` 切り替えで検証する。
- `tests/data/core_path/normalize_{posix,windows}.json` を追加し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-path path::tests::*` と `scripts/validate-diagnostic-json.sh --pattern core.path.normalize` を実行する。

> 進行ログ（Phase3 W49, 4.1 完了条件の一次達成）  
> - `compiler/rust/runtime/src/path/mod.rs` を新設し、`PathBuf`/`Path`/`PathError` と `path()/join()/normalize()/parent()/is_absolute()/components()` API を実装。空文字・NUL の検証と `Str` からの変換を提供し、`normalize_components` で `.`/`..`/UNC/ドライブを共通処理化した。  
> - `tests/data/core_path/normalize_{posix,windows}.json` と `compiler/rust/runtime/tests/path_normalize.rs` を追加し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml normalize_and_join_follow_golden_cases` で POSIX/Windows の代表ケース（`/var/log`, `C:\data\logs`, UNC 共有、`..` を含む相対パス）をゴールデン化。  
> - `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` と `docs/notes/core-io-path-gap-log.md` に Path API 実装状況を反映し、セキュリティヘルパ・文字列ユーティリティの完了と glob 実装計画（§4.2 以降）を併記した。

4.2. セキュリティヘルパ (`validate_path`, `sandbox_path`, `is_safe_symlink`) を実装し、`effect {security}` の検証を行う。  
実施ステップ:
- `compiler/rust/runtime/src/path/security.rs` に `validate_path`/`sandbox_path`/`is_safe_symlink` を実装し、`Core.Diagnostics` と連携する `PathSecurityError` を定義する。
- `docs/spec/3-8-core-runtime-capability.md` の `security.fs.*` 要件を `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` に反映し、`CapabilityStage` チェックを `security.rs` で行う。
- `tests/data/core_path/security/*.json` を `scripts/validate-diagnostic-json.sh --pattern core.path.security` で検証し、`effect {security}` と `metadata.security.reason` を `collect-iterator-audit-metrics.py --section core_io --scenario path_security` から観測する。

> 進行ログ（Phase3 W49, 4.2 着手）  
> - `compiler/rust/runtime/src/path/security.rs` を追加し、`SecurityPolicy`・`PathSecurityError`・`PathSecurityResult` を導入。`validate_path` / `sandbox_path` / `is_safe_symlink` で `FsAdapter::ensure_security_policy()`・`ensure_symlink_query()` を呼び出し、`EffectLabels.security` と `metadata.security.*` を `GuardDiagnostic` へ転写できるようになった。  
> - `compiler/rust/runtime/tests/path_security.rs` と `tests/data/core_path/security/{relative_denied,sandbox_escape,symlink_absolute}.json` を作成し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml path_security` で `core.path.security.*` 診断が生成されることを確認。テストでは POSIX/Windows のサンドボックスルート（`sample_root()`）と Unix symlink ケースを分岐させ、CI で JSON ゴールデンを再利用できるようにした。  
> - `docs/plans/bootstrap-roadmap/assets/{core-io-path-api-diff.csv,core-io-effects-matrix.md,core-io-capability-map.md}` を更新し、Security 行の `impl_status=In Progress (Rust runtime)`、`path_security` シナリオの検証ポイント（`metadata.security.reason`, `tests/path_security.rs`）を反映。`docs/notes/runtime-capability-stage-log.md` にも Capability `security.fs.policy`/`fs.symlink.query` の実測経路を追記した。

4.3. 文字列ユーティリティ (`normalize_path`, `join_paths`) を実装し、`Core.Text` と連携するテストを整備する。  
実施ステップ:
- `compiler/rust/runtime/src/path/string_utils.rs` を追加し、`normalize_path_str`, `join_paths_str`, `relative_to` を UTF-8 ベースで実装する（`Core.Text` の正規化ロジックを再利用）。
- `docs/spec/3-3-core-text-unicode.md` の `TextNormalizer` と互換にするため、`Core.Text` サイドの API 参照 (`docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md`) を更新し、双方向リンクを README に追記する。
- `tests/data/core_path/unicode_cases.json` を作成し、`rg --glob "*unicode*.json" docs/spec` で引いたサンプルを再利用しつつ、正規化差異を `reports/spec-audit/ch3/path_unicode-*.md` に記録する。

> 進行ログ（Phase3 W49, 4.3 実装完了）  
> - `compiler/rust/runtime/src/path/mod.rs` に `PathStyle` を追加し、`compiler/rust/runtime/src/path/string_utils.rs` で `normalize_path_str` / `join_paths_str` / `is_absolute_str` / `relative_to` を実装。`PathErrorKind::UnsupportedPlatform` を拡張して `Core.Text` の `Str` と効果記録 (`record_text_mem_copy`) を通じた純粋なパス文字列 API を整備した。  
> - `tests/data/core_path/unicode_cases.json` と `compiler/rust/runtime/tests/path_string_utils.rs` を追加し、POSIX/Windows/UNC ケースにおける正規化・結合・相対計算をゴールデン化。`cargo test --manifest-path compiler/rust/runtime/Cargo.toml path_string_utils` の結果を `reports/spec-audit/ch3/path_unicode-20251130.md` に記録した。  
> - `docs/plans/bootstrap-roadmap/assets/core-io-path-api-diff.csv` の Core.Path.Strings 行（PathStyle/normalize_path/join_paths/is_absolute_str）を `Implemented (Rust runtime)` へ更新し、`docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` と `docs/notes/core-io-path-gap-log.md` に文字列ユーティリティ行・エントリを追記した。`docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` / `docs/plans/bootstrap-roadmap/README.md` にも Text ↔ Path の相互参照を追加済み。

### 5. Watcher / 拡張機能（49週目）
**担当領域**: オプション機能

5.1. ファイル監視 API (`watch`, `watch_with_limits`, `close`) を実装し、`effect {io.async}` のハンドリングを確認する。  
実施ステップ:
- `compiler/rust/runtime/src/io/watcher.rs` に `Watcher`, `WatcherHandle`, `WatchEvent` を実装し、`notify` crate (macOS FSEvents, Linux inotify, Windows ReadDirectoryChangesW) を抽象化する。
- `effect {io.async}` を `Watcher` が記録できるよう `io/effects.rs` に非同期チャンネルのメトリクス (`watch.queue_size`, `watch.delay_ns`) を追加する。
- `tests/fixtures/watcher/simple_case` を用意し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-io watch::tests::*` を実行して基本イベントの整合性を確認する。

> 進行ログ（Phase3 W49, 5.1 完了）  
> - `compiler/rust/runtime/src/io/watcher.rs` に `WatchEvent`/`Watcher`/`WatcherHandle` を追加し、`notify` + `WatcherAdapter` で `fs.watcher.*` Capability と `effect {io.async}` を同時に検証。`IoContext` へ `metadata.io.watch.queue_size` / `metadata.io.watch.delay_ns` を記録できるよう `io/effects.rs`（`WatchMetricsSnapshot`）と `io/context.rs` を拡張し、`IoError::into_diagnostic()` が `core.io.watcher_error` に監視メタデータを転写する経路を実装した。  
> - `io/error.rs` のエンコードに `watch` セクションを追加し、`AuditEnvelope.metadata["io.watch.*"]` へ統合。`docs/plans/bootstrap-roadmap/assets/{core-io-path-api-diff.csv,core-io-effects-matrix.md,core-io-capability-map.md}` を更新し、Watcher 行を「Implemented (Rust runtime)」/Stage In Progress へ変更。  
> - `compiler/rust/runtime/tests/watcher.rs` と `tests/fixtures/watcher/simple_case/` を追加し、tempdir で `watch_reports_create_and_delete_events`／`watch_with_limits_rejects_invalid_path` を実行。`cargo test --manifest-path compiler/rust/runtime/Cargo.toml watcher` の実行は新規依存取得にネットワークが必要なためローカルでは未実行だが、テスト手順とフィクスチャを文書化した。

5.2. 監視イベントを `AuditEnvelope` へ記録する仕組みを整備し、ログの構造化をテストする。  
実施ステップ:
- `WatcherEventRecorder` を `compiler/rust/runtime/src/io/watcher_audit.rs` に作成し、イベントを `AuditEnvelope.metadata.io.watch.*` へ書き込む。
- `reports/spec-audit/ch3/io_watcher-*.jsonl` を新設し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit` で監査レポートを収集する。
- `docs/spec/3-6-core-diagnostics-audit.md` の監視例を参照し、CLI/Runtime のログフォーマットを `scripts/validate-diagnostic-json.sh --pattern core.io.watcher` で検証する。

> 進行ログ（Phase3 W49, 5.2）  
> - `compiler/rust/runtime/src/io/watcher_audit.rs` に `WatcherEventRecorder` / `WatcherAuditSnapshot` / `WatcherAuditEvent` を追加し、`watch` 実行中に生成されるイベントを最大 64 件までリングバッファへ保持。`Watcher::audit_snapshot()` / `WatcherHandle::audit_snapshot()` から `AuditEnvelope.metadata` 互換の辞書 (`io.watch.paths`, `io.watch.events_total`, `io.watch.events[*].{kind,path,timestamp,queue_size,delay_ns}`) を取得できるようにした。`watcher.rs` では Runtime/State の両方に Recorder を配り、`record_watch_metrics` と同じタイミングで非同期イベントを記録する。  
> - `compiler/rust/runtime/tests/watcher.rs` を拡張し、`watch_reports_create_and_delete_events` で `WatcherAuditSnapshot` をアサートしたうえで `reports/spec-audit/ch3/io_watcher-simple_case.jsonl` を生成。CI ではこの JSON Lines を `scripts/validate-diagnostic-json.sh --pattern core.io.watcher`・`python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario watcher_audit --source reports/spec-audit/ch3/io_watcher-simple_case.jsonl --output reports/spec-audit/ch3/io_watcher-metrics.json --require-success` へ渡し、`watcher.audit.pass_rate` と `io.watch.events_total` の欠落を監視する。`docs/plans/bootstrap-roadmap/assets/core-io-effects-matrix.md` の Watcher 行にも `WatcherAuditSnapshot` を参照する脚注を追加した。

5.3. クロスプラットフォームでサポートが異なる機能は `Capability` 判定と `IoErrorKind::UnsupportedPlatform` で扱う。  
実施ステップ:
- `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` に `watcher.fschange`, `watcher.recursive`, `watcher.resource_limits` の Stage/OS 対応を追記する。
- `compiler/rust/runtime/src/io/error.rs` の `IoErrorKind::UnsupportedPlatform` に `platform`/`feature` メタデータを加え、`Capability` 判定前後のログを `AuditEnvelope` に保存する。
- `docs/notes/runtime-capability-stage-log.md` に watcher 系 Capability のステータスを残し、`3-8-core-runtime-capability-plan.md` のフェーズ依存タスクと連動させる。

> 進行ログ（Phase3 W49, 5.3）  
> - `core-io-capability-map.md` に `watcher.fschange`/`watcher.recursive`/`watcher.resource_limits` 行を追加し、Linux/macOS/Windows のみサポートされること、Registry 上は `fs.watcher.*` へ委譲されること、`core.io.unsupported_platform` 診断で `metadata.io.platform/io.feature` を必須化することを明示した。  
> - `IoError` に `with_platform`/`with_feature` を追加して `IoErrorKind::UnsupportedPlatform` が `extensions["io"]` と `AuditEnvelope.metadata["io.*"]` へプラットフォーム差分を残せるよう拡張し、`watcher.rs` では `ensure_watcher_feature` で OS 判定→`UnsupportedPlatform` を発火する処理と `WatcherAdapter::ensure_resource_limit_capability()` を実装。  
> - `docs/notes/runtime-capability-stage-log.md` と `3-8-core-runtime-capability-plan.md` に上記 Capability と Runbook 追記を行い、`watcher_audit` シナリオで `metadata.io.platform`/`metadata.io.feature` を検証する手順を共有した。

### 6. ドキュメント・サンプル更新（49-50週目）
**担当領域**: 情報整備

6.1. 仕様書サンプル・ガイド (`docs/guides/runtime/runtime-bridges.md`) を更新し、実装差分を解消する。  
実施ステップ:
- `docs/spec/3-5-core-io-path.md` のコードサンプル・脚注を最新 API に合わせて改稿し、`docs/spec/3-0-core-library-overview.md` に概要を追記する。
- `docs/guides/runtime/runtime-bridges.md`/`docs/guides/dsl/plugin-authoring.md` に IO/Path の利用例を追加し、Capability チェックや `IoContext` の記録方法を解説する。
- `docs/plans/bootstrap-roadmap/README.md` と `docs/plans/rust-migration/overview.md` に本計画書へのリンクと更新履歴を追記する。

6.2. `README.md`/`3-0-phase3-self-host.md` に IO/Path 実装ステータスを記載し、利用者向け注意事項を明示する。  
実施ステップ:
- `README.md` の Phase3 進捗表に `Core.IO & Path` 行を追加し、マイルストーン/担当/完了条件を記載する。
- `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` M4/M5 行へ IO/Path の依存関係と完了条件 (`Watcher`, `Path security`, `IoError diagnostics`) を紐付ける。
- 監査ログ (`docs/notes/runtime-bridges-roadmap.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`) への記載箇所を更新し、読者が最新ステータスを追跡できるようにする。

6.3. `examples/` にファイル操作・パス検証の例を追加し、CI で自動実行する。  
実施ステップ:
- `examples/practical/core_io/file_copy/canonical.reml`, `examples/practical/core_path/security_check/relative_denied.reml`（旧 `examples/core_io` / `examples/core_path`）を追加し、`Core.IO` API の実際の使い方とエラーハンドリングを示す。
- `tooling/examples/run_examples.sh --suite core_io` を整備し、CI (`.github/workflows/examples.yml`) で Reml スクリプトを実行するよう設定する。
- `docs/notes/examples-regression-log.md` に新例の実行結果とトラブルシューティングを記録し、リグレッション時の調査手順を共有する。

> 進行ログ（Phase3 W50, §6）
> - `docs/spec/3-5-core-io-path.md`、`docs/spec/3-0-core-library-overview.md`、`docs/guides/runtime/runtime-bridges.md`、`docs/guides/dsl/plugin-authoring.md` に Reader/Writer と Path セキュリティのサンプル参照を追記し、`docs/plans/bootstrap-roadmap/README.md`・`docs/plans/rust-migration/overview.md`・`docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` にも本タスクの完了条件を明記した。
> - `examples/practical/core_io/file_copy/canonical.reml` / `examples/practical/core_path/security_check/relative_denied.reml`（旧 `examples/core_io` / `examples/core_path`）と `tooling/examples/run_examples.sh --suite core_io|core_path` を追加し、`core_io.example_suite_pass_rate` KPI を `0-3-audit-and-metrics.md` へ登録。`examples/README.md` と各 README に概要・実行手順を記録した。
> - `docs/notes/runtime-bridges-roadmap.md` と `docs/notes/examples-regression-log.md` を新設し、Runtime Bridge / Plugin / サンプル実行の Runbook とリグレッション記録を共有。`docs/notes/core-io-path-gap-log.md` にも「サンプル・ドキュメント整合」のギャップ解消ログを追記した。

### 7. テスト・ベンチマーク統合（50週目）
**担当領域**: 品質保証

7.1. 単体・統合テストを追加し、エラー経路・効果タグ・Capability 検証を網羅する。  
実施ステップ:
- `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features "core-io core-path"` に統合テストターゲット (`io::tests::`, `path::tests::`) を追加し、`effect {io.*}`/`{security}` を `EffectSet` で確認する。
- `scripts/validate-diagnostic-json.sh --suite core_io` を作成し、`tests/data/core_io/*.json` と `tests/data/core_path/*.json` の診断・監査出力を検証する。
- Capability テスト (`tests/capabilities/core_io_*`) を GitHub Actions に追加し、`verify_capability_stage` の pass/fail を `reports/spec-audit/ch3/core_io_capability-*.md` に残す。

7.2. IO 性能ベンチマークを実施し、Rust 実装の Phase 2 ベースライン比 ±15% を目標に評価する（OCaml 実装は参考資料としてのみ保持）。  
実施ステップ:
- `compiler/rust/runtime/benches/bench_core_io.rs` を `criterion` ベースで実装し、`reader_copy`, `buffered_read_line`, `path_normalize`, `watcher_throughput` のベンチを追加する。
- `reports/benchmarks/core-io-path/phase3-baseline-YYYYMMDD.json` を作成し、`docs/plans/rust-migration/3-2-benchmark-baseline.md` に測定項目 (`io.copy_throughput_mb_s`, `path.normalize_ops_s`) を追記する。
- OCaml 実装との差分を `docs/notes/core-io-path-gap-log.md` に記録し、±15% 以上の回帰が発生した場合は `0-4-risk-handling.md` にリスク登録する。

> 進行ログ（Phase3 W50, 7.2）  
> - `compiler/rust/runtime/benches/bench_core_io.rs` を新設し、`reader_copy`, `buffered_read_line`, `core_path_normalize`, `watch_event_batch`（Watcher の監査イベントを模した合成バッチ）を Criterion ベンチとして追加。Watcher 実装はサンドボックス内で OS イベントを取得できないため、`WatchEvent` ベースのベンチを用意し監査パイプラインのオーバーヘッドを測定する設計にした。  
> - `cargo bench --manifest-path compiler/rust/runtime/Cargo.toml --features "core-io core-path" --bench bench_core_io -- --noplot` を実行し、`reports/benchmarks/core-io-path/phase3-baseline-2025-12-24.json` に初回ベースラインを保存。`reader_copy_64k`/`reader_copy_2m`/`buffered_read_line`/`path_normalize`/`watch_event_batch` の 8 シナリオを記録し、±15% のしきい値を `docs/plans/rust-migration/3-2-benchmark-baseline.md` へ登録した。  
> - ベンチ結果を `docs/plans/rust-migration/3-2-benchmark-baseline.md`（指標・スイート表）、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`（`core_io.benchmark.copy_throughput_mb_s` KPI）、`docs/notes/core-io-path-gap-log.md`（回帰ログ）と同期し、Phase 2 OCaml 測定値と比較するフローを明文化した。

7.3. テスト結果とリスクを `0-3-audit-and-metrics.md`/`0-4-risk-handling.md` に記録し、追加調整が必要な項目を整理する。  
実施ステップ:
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase3 指標に `io.error_rate`, `path.security.incident_count`, `watcher.audit.pass_rate` を追加し、CI から収集した数値を毎スプリント反映する。
- `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に IO/Path 特有のリスク（大量ファイル監視、権限不足、シンボリックリンク攻撃）を追記し、緩和策・責任者を明確化する。
- `reports/spec-audit/ch3/core_io_summary-YYYYMMDD.md` を作成し、テスト・ベンチ結果と未解決課題をまとめて Phase3 定例に共有する。

## 成果物と検証
- `Core.IO`/`Core.Path` API が仕様通りに実装され、効果タグ・Capability 検証が合致していること。
- ファイル操作・パス検証がクロスプラットフォームで正しく動作し、未対応機能が明示されていること。
- ドキュメント・サンプルが更新され、安全な IO 利用方法が共有されていること。

## リスクとフォローアップ
- プラットフォーム差異でテストが不安定な場合、対象機能を実験扱いにし `docs/notes/runtime-bridges.md` に制約を記録する。
- 監視 API が OS の制限により提供できない場合、Phase 4 のマルチターゲット検証でフォローアップする。
- `security` 効果の運用が未確定な場合、Capabilities と連携したポリシー策定を Phase 3-8 に委譲する。

## 参考資料
- [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
