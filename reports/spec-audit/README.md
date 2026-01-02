# spec-audit ディレクトリ構造

Phase 2-8 仕様完全性監査で使用する成果物を Chapter 単位で保管する。Rust 版 Reml コンパイラを唯一のアクティブ実装とし、以下のポリシーでログや JSON を整理する。

| ディレクトリ | 役割 | 例 | 備考 |
|--------------|------|----|------|
| `ch0/` | Chapter 0（索引・リンク・脚注）の検証ログ | `links.md`, `overview-diff.md` | `docs/spec/0-0-overview.md` や `docs/spec/README.md` の差分を貼付し、リンクチェッカーの出力を添付する |
| `ch1/` | Chapter 1（構文・型・効果）の CLI/JSON ベースライン | `syntax-samples-*.json`, `effects-poc.log` | `cargo run --bin poc_frontend --emit-*` の結果を保存 |
| `ch2/` | Chapter 2（Parser API / Streaming）のストリーミングログ | `streaming/*.json`, `recover/*.log` | Streaming Runner・Recover を Rust Frontend で実行し、`ERR-001/002` を追跡 |
| `ch3/` | Chapter 3（Diagnostics / Capability / Runtime）の監査ログ | `diagnostics/*.json`, `runtime/*.log` | `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml` 等の結果と監査 JSON を格納 |
| `diffs/` | `rust-gap` 差分メモと回避策 | `<ID>-<chapter>-rust-gap.md` | `docs/notes/process/spec-integrity-audit-checklist.md#rust-gap` と対応 |
| `summary.md` | 監査実行ログ（日時・コマンド・結果） | `2025-11-17 Phase 2-8 kickoff` | `0-3-audit-and-metrics.md#0.3.4a` と相互参照 |

- それぞれのサブディレクトリには README を置き、記録形式と責任者を明記する。
- ログや JSON を保存する際は UTC ではなく JST のタイムスタンプをファイル名に含め、`reports/spec-audit/summary.md` に実行コマンドを追記する。
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` §1.3 から本ディレクトリを参照し、監査フェーズ開始前のベースライン整備証跡とする。

## ログ命名と運用（合意版）
- 命名基本: `reports/spec-audit/<chapter>/<sample>-YYYYMMDD-<kind>.json`（JST）
- Streaming 実行: `reports/spec-audit/<chapter>/streaming_<sample>-YYYYMMDD-<kind>.json`
- 付随ログ: `reports/spec-audit/<chapter>/<sample>-YYYYMMDD-trace.md` / `<sample>-YYYYMMDD-dualwrite.md`
- 命名に含める `<kind>` は `diagnostics` / `typeck` / `impls` / `trace` / `dualwrite` を基本とする
- 実行コマンド、`CI_RUN_ID`、結果メモは `reports/spec-audit/summary.md` に集約する

## Core Collections
- 永続コレクションの構造共有ベンチ (`ListPersistentPatch` / `MapPersistentMerge`) は `compiler/rust/runtime/ffi/src/core_collections_metrics.rs` → `examples/core_collections_metrics.rs` で実行し、CSV を `docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv` に保存する。直近の記録（git `4745e19e` 由来）は `List` 1.7158 倍 / `Map` 1.3903 倍で、入力サイズ比 1.8 以下を満たした。
- 再取得手順: `cargo run --manifest-path compiler/rust/runtime/ffi/Cargo.toml --features core_prelude --example core_collections_metrics -- docs/plans/bootstrap-roadmap/assets/metrics/core_collections_persistent.csv`。コマンド出力と CSV を `reports/spec-audit/ch3/core_collections_persistent-<YYYYMMDD>.log`（必要に応じて新設）へ保存し、本 README と `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#0.3.1a-core.collections-永続構造メトリクス` からリンクする。
