# 2.2 アダプタ層ガイドライン

本章は Phase P2 バックエンド統合の一環として、Rust 実装が各プラットフォーム差異（ファイルシステム、ネットワーク、時刻、乱数、プロセス、環境変数等）を吸収するアダプタ層の設計方針をまとめる。`docs/spec/3-10-core-env.md`・`docs/guides/runtime/portability.md`・`compiler/runtime/native/include/reml_os.h` に定義された契約と整合し、`2-0-llvm-backend-plan.md`・`2-1-runtime-integration.md` で必要となる環境依存 API を安全に提供することが目的である。

## 2.2.1 目的
- 各プラットフォーム固有 API を抽象化する Rust アダプタ層（`adapter` モジュール）を整備し、バックエンド・ランタイム連携・CI から同一 API で利用できるようにする。
- `RunConfig`, `CapabilityRegistry`, `Audit` と連携し、`effect {io.async, io.blocking, runtime, security}` などの効果タグを守ったまま環境操作を行う。
- Windows / Linux / macOS / 将来ターゲット（WASI, embedded）の差異を明示し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` に記載された制約をアダプタ層で吸収する。

## 2.2.2 スコープと前提
- **対象範囲**
  - FS（ファイル／パス操作）、Network（TCP/UDP/HTTP クライアント基盤）、Time（モノトニック時計・壁時計）、Random（暗号安全乱数・擬似乱数）、Process（サブプロセス／環境変数）、System（システム情報、CPU/メモリ統計）。
  - `RunConfig` 連携（ターゲットプロファイル、環境 Capability）、`target.config.*` 診断との協調。
  - CI / CLI から利用する `adapter` API の公開設計（Feature フラグ・Stage ポリシー含む）。
- **除外**
  - DSL プラグインや拡張 Capability 導入。該当項目は Chapter 5・`docs/notes/dsl/dsl-plugin-roadmap.md` で扱う。
  - ハードウェア固有（GPU, GPIO 等）の詳細実装。必要に応じて後続フェーズで計画する。
  - パフォーマンス最適化（P4 スコープ）。
- **前提**
  - `docs/guides/runtime/portability.md` が定義するターゲットプロファイル／条件付きコンパイル (`@cfg`) ポリシーが共有されている。
  - `compiler/runtime/native/include/reml_os.h` に OS 抽象 API が存在し、Rust 側から FFI で呼び出せる。
  - P0/P1 で `RunConfig`／`Diagnostic` の拡張フィールドが整備済み。

## 2.2.3 完了条件
- Rust アダプタ層が FS/Network/Time/Random/Process の最小 API を提供し、主要 3 プラットフォームでの単体テストと統合テストをパスする。
- `RunConfig.extensions["target"]` / `Diagnostic.extensions["cfg"]` へのメタデータ出力が揃い、`target.config.mismatch` や `target.profile.missing` 診断がゼロ件になる。
- アダプタ層の Stage 要件（Capability Stage, Feature flag）が `CapabilityRegistry` と同期され、`effects.contract.stage_mismatch` を発生させない。
- CI（P3）で利用するアダプタ API ドキュメントが整備され、`docs/plans/rust-migration/README.md` から参照可能になっている。

## 2.2.4 主成果物

| 成果物 | 内容 | 依存資料 |
| --- | --- | --- |
| `compiler/adapter/` | プラットフォーム差異吸収モジュール（FS/Network/Time/Random/Process/Env）。エラーモデルは `Result<T, AdapterError>`。 | `docs/guides/runtime/portability.md`, `docs/spec/3-10-core-env.md`, `compiler/runtime/native/include/reml_os.h` |
| ターゲット能力マップ | プロファイル別（`desktop-x86_64`, `windows-gnu`, `windows-msvc`, `darwin-aarch64` 等）の Capability ↔ Adapter API 対応表。 | `docs/plans/rust-migration/0-2-windows-toolchain-audit.md`, `docs/guides/tooling/audit-metrics.md` |
| テストハーネス | クロスプラットフォーム Adapter テスト（ファイル操作、ソケット、時間計測、乱数品質、プロセス起動）。CI 連携（GitHub Actions matrix）。 | `tooling/ci/`, `compiler/runtime/native/tests/test_os.c`, `docs/guides/runtime/runtime-bridges.md` |
| ポリシーガイド | Stage/Feature フラグ、`@cfg` ルール、監査ログテンプレート（`adapter.*`）。 | `docs/guides/runtime/portability.md`, `docs/spec/3-8`, `docs/spec/3-9` |

## 2.2.5 サブシステム別方針

| サブシステム | API 例 | 主効果タグ | 対応 Capability | 監査ログキー | 備考 |
| --- | --- | --- | --- | --- | --- |
| FS | `open`, `read`, `write`, `stat`, `canonicalize` | `effect {io.blocking}` | `IoCapability.fs` | `adapter.fs.*` | Windows パスの UNC 対応、`reml_os_path_*` FFI を利用 |
| Network | `connect`, `listen`, `accept`, `send`, `recv` | `effect {io.async, security}` | `NetworkCapability` | `adapter.net.*` | TLS/HTTP は Phase 3 以降。`RuntimeBridge` 連携 |
| Time | `monotonic_now`, `system_now`, `sleep`, `deadline` | `effect {io.timer}` | `RuntimeCapability.timer` | `adapter.time.*` | Rust 標準の `Instant`, `SystemTime` をラップ |
| Random | `random_bytes`, `rng_stream`, `seed_from_os` | `effect {security}` | `SecurityCapability.random` | `adapter.random.*` | `getrandom` / Windows `BCryptGenRandom` 落とし込み |
| Process | `spawn`, `wait`, `env_get`, `env_set`, `current_dir` | `effect {process}` | `ProcessCapability` | `adapter.proc.*` | `RunConfig` と環境変数同期、`security.audit` 連携 |

### FS
- `reml_os.h` の `reml_os_path_normalize` を Rust ラッパ (`PathAdapter`) で利用し、Windows では UTF-16 変換・macOS/Linux では UTF-8 を維持する。  
- シンボリックリンク／特殊パスは `Audit` に `adapter.fs.symlink` を記録。`@cfg(target_os = "windows")` で UNC パス・ショートネーム対策を実行する。  
- ファイルハンドルは `ForeignPtr<FileHandle>` として管理し、`Drop` で `reml_os_file_close` を呼ぶ。

### Network
- 第一段階ではブロッキング TCP/UDP をサポートし、`AsyncCapability` と統合する非同期 API は Phase 3 で拡張。  
- `NetworkCapability` 登録時に `stage = Experimental` から開始し、`audit.log("adapter.net.connect", ...)` を生成。TLS は `SecurityCapability` に委譲し、証明書の検証結果を監査する。  
- Windows では Winsock 初期化 (`WSAStartup`) の差異をアダプタ層で吸収。

### Time
- `Instant`/`Duration` を `RunConfig.extensions["target"].time` と同期し、`monotonic` と `system` を区別。  
- `sleep` は `effect {io.blocking}` を要求し、`Core.Async` と統合する `sleep_async` は `2-1-runtime-integration.md` の範囲で実装。  
- 監査ログには `adapter.time.skew_ns` を出力し、`docs/guides/tooling/audit-metrics.md` の SLA に従う。

### Random
- `getrandom` crate と Windows `BCryptGenRandom` をラップし、`SecurityCapability` から `audit.log("adapter.random", ...)` を出力。  
- 擬似乱数は Phase 2 では提供せず、`docs/guides/runtime/portability.md` のガイドに従って `feature = "prng"` で opt-in。  
- 秘匿情報を扱う場合は `Diagnostic` に `security.redaction = true` を附帯する。

### Process / Env
- `spawn`/`wait` は Rust 標準の `std::process` を利用しつつ、`RunConfig` で設定された `env` を前処理する。  
- `env_get`/`env_set` は `effect {process}` と `SecurityCapability` を要求し、ログに `adapter.proc.env_mutation` を残す。  
- Windows と POSIX の差異（コマンドライン引数の解析、UTF-16/UTF-8）をアダプタ層で統一。

## 2.2.6 マイルストーン（目安）

| 週 | マイルストーン | 主タスク | 検証方法 |
| --- | --- | --- | --- |
| W1 | アダプタ API 骨子 | サブシステム別インタフェース定義、`Result` 型設計、Stage ポリシー案作成 | ADR（内部メモ）、仕様リンク整理 |
| W2 | FS/Time 実装 | パス操作・時間 API の実装、`RunConfig` と連携、Basic テスト | クロスプラットフォーム単体テスト、監査ログ確認 |
| W3 | Network/Random 実装 | TCP/UDP/乱数 API 実装、`SecurityCapability` 連携、監査ログ整備 | GitHub Actions matrix、`adapter.net.*` ログ収集 |
| W4 | Process/Env と統合テスト | プロセス生成、環境変数同期、`target.config.*` 診断検証 | CI `adapter-smoke` ジョブ、`docs/notes/` へ検証結果記録 |
| W4.5 | レビュー & P3 連携 | API の安定化、CI 連携仕様書作成、`docs/plans/rust-migration/README.md` 更新 | ドキュメントレビュー、`docs/notes/docs-migrations.log` |

## 2.2.7 検証・メトリクス
- **単体テスト**: `cargo test adapter_fs`, `adapter_time`, `adapter_net`, `adapter_random`, `adapter_process` を用意し、主要プラットフォームで動作確認。  
- **監査ログ**: 各サブシステムで `audit.log("adapter.fs", ...)` 等を出力し、`collect-iterator-audit-metrics.py` に `adapter.*` 項目を追加。  
- **診断検証**: `reml target validate` → `RunConfig` 初期化 → `adapter` 呼び出しの順で `target.config.*` 診断が発生しないことを確認。  
- **CI**: GitHub Actions に `adapter-integration` ジョブを追加し、Windows GNU/MSVC・macOS・Linux の 4 マトリクスでテストを実行。

## 2.2.8 リスクと対応
- **プラットフォーム API の仕様差**  
  - *リスク*: Windows/POSIX API の差異で挙動がずれる。  
  - *対応*: FFI で `reml_os_*` ラッパを最大限活用し、差異は `#[cfg]` で局所化。差異が大きい場合は `docs/notes/portability-gap.md`（新設予定）へ記録。

- **監査ログ影響の見落とし**  
  - *リスク*: 新 API が監査要件に未対応。  
  - *対応*: すべての Adapter API に `AuditContext` を受け渡す設計とし、ログ未出力は lint で検知。Stage 昇格時に `audit.log` サンプルをレビュー。

- **依存クレートの選定遅延**  
  - *リスク*: ネットワーク/乱数実装で外部 crate 選定が遅れる。  
  - *対応*: Phase 2 では標準ライブラリ + OS FFI の最小構成で開始し、外部 crate 採用は `feature = "external"` で opt-in。選定ノートを `docs/notes/core-runtime-outline.md` に残す。

- **WASI / Embedded との整合不足**  
  - *リスク*: 将来ターゲットでアダプタが動作しない。  
  - *対応*: `adapter` API を最小インタフェースに限定し、未対応ターゲットは `UnsupportedCapability` として `Diagnostic` を発行。詳細調査は `docs/notes/backend/cross-compilation-spec-intro.md` と連携。

## 2.2.9 P1 W4.5 ハンドオーバー
- `reports/dual-write/front-end/P1_W4.5_frontend_handover/diag/streaming/20280410-w4-diag-streaming-r21/` から `runconfig.extensions.stream.*` / `flow.backpressure.max_lag_bytes` / `expected_tokens.*` を抽出し、アダプタ API の Streaming Flow 設定（`AdapterStreamConfig`）へ取り込む。`appendix/w4-diagnostic-case-matrix.md#W4.5-ハンドオーバーメモ` で `Pending(W4.5)` とされた項目は P2 着手前に補完必須。
- CLI/LSP ケース (`diag/cli-lsp/20280430-w4-diag-cli-lsp/`) に `extensions.config.*` / `extensions.cli.*` / `extensions.lsp.*` の基準値が記録されている。アダプタ層で RunConfig を構築する際にこの JSON をゴールデンとし、`p1-front-end-checklists.csv` の `HandedOver` 列に記載された Run ID を参照する。
- Type/Effect/FFI ケース (`diag/effects/20280418-w4-diag-effects-r3/`, `20280601-w4-diag-type-effect-rust-typeck-r7/`) では `StageAuditPayload` と `--runtime-capabilities` の欠落が `Pending(W4.5)` として残っているため、アダプタ層の Capability 宣言にも TODO として登録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-DIAG-RUST-06` とリンクする。
- `w4-diagnostic-cases.txt` に追加した `#handed_over` コメントを読み取り、Recover（Pass）以外のケースを優先的に再実行する `adapter-smoke` ターゲットを用意する。

## 2.2.10 関連ドキュメント更新
- 本章の API 追加時には `docs/plans/rust-migration/README.md`・`docs/plans/README.md` に P2 セクションを追記する。  
- `RunConfig` や `Diagnostic` の拡張フィールドを変更した場合は `docs/spec/2-6-execution-strategy.md`・`docs/spec/3-6-core-diagnostics-audit.md` の参照箇所を確認し、必要なら更新提案を行う。  
- Windows ツールチェーン関連の変更は `0-2-windows-toolchain-audit.md` と `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` にフィードバックし、`docs/notes/docs-migrations.log` を更新する。

## 2.2.11 次フェーズ連携
- P3 CI 統合計画（`3-0-ci-and-dual-write-strategy.md`）でアダプタ API を活用し、dual-write テストや監査メトリクス収集の基盤を提供する。  
- P4 リスク登録 (`4-0-risk-register.md`) ではアダプタ層の未対応ターゲット・依存クレート更新リスクを追跡する。  
- DSL プラグイン／Capability 拡張（Chapter 5）の準備として、アダプタ API の公開仕様を `docs/guides/dsl/plugin-authoring.md` 等へ展開する。

---
**参照**: `docs/guides/runtime/portability.md`, `docs/spec/3-10-core-env.md`, `docs/guides/runtime/runtime-bridges.md`, `compiler/runtime/native/include/reml_os.h`, `docs/plans/rust-migration/0-2-windows-toolchain-audit.md`, `docs/plans/rust-migration/2-0-llvm-backend-plan.md`, `docs/plans/rust-migration/2-1-runtime-integration.md`
