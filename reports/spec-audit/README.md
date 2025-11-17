# spec-audit ディレクトリ構造

Phase 2-8 仕様完全性監査で使用する成果物を Chapter 単位で保管する。Rust 版 Reml コンパイラを唯一のアクティブ実装とし、以下のポリシーでログや JSON を整理する。

| ディレクトリ | 役割 | 例 | 備考 |
|--------------|------|----|------|
| `ch0/` | Chapter 0（索引・リンク・脚注）の検証ログ | `links.md`, `overview-diff.md` | `docs/spec/0-0-overview.md` や `docs/spec/README.md` の差分を貼付し、リンクチェッカーの出力を添付する |
| `ch1/` | Chapter 1（構文・型・効果）の CLI/JSON ベースライン | `syntax-samples-*.json`, `effects-poc.log` | `cargo run --bin poc_frontend --emit-*` の結果を保存 |
| `ch2/` | Chapter 2（Parser API / Streaming）のストリーミングログ | `streaming/*.json`, `recover/*.log` | Streaming Runner・Recover を Rust Frontend で実行し、`ERR-001/002` を追跡 |
| `ch3/` | Chapter 3（Diagnostics / Capability / Runtime）の監査ログ | `diagnostics/*.json`, `runtime/*.log` | `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml` 等の結果と監査 JSON を格納 |
| `diffs/` | `rust-gap` 差分メモと回避策 | `<ID>-<chapter>-rust-gap.md` | `docs/notes/spec-integrity-audit-checklist.md#rust-gap` と対応 |
| `summary.md` | 監査実行ログ（日時・コマンド・結果） | `2025-11-17 Phase 2-8 kickoff` | `0-3-audit-and-metrics.md#0.3.4a` と相互参照 |

- それぞれのサブディレクトリには README を置き、記録形式と責任者を明記する。
- ログや JSON を保存する際は UTC ではなく JST のタイムスタンプをファイル名に含め、`reports/spec-audit/summary.md` に実行コマンドを追記する。
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` §1.3 から本ディレクトリを参照し、監査フェーズ開始前のベースライン整備証跡とする。
