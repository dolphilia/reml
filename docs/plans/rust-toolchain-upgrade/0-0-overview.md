# 0.0 Rust ツールチェーン更新計画: 概要

## 背景
- `reml_frontend` の再ビルドで `icu_normalizer_data` が `rustc 1.83+` を要求し、現行の `rustc 1.69.0` ではビルドできない。
- docs-examples の再検証や Rust Frontend のパーサ更新を継続するため、Rust を最新安定版へ更新し、関連クレートの整合を確保する必要がある。

## 目的
- Rust を最新安定版へ更新し、`compiler/` 配下の全バイナリ/クレートを再ビルド可能にする。
- 依存クレートの更新・互換性確認を行い、ビルドと検証の記録を `reports/` と計画書に残す。
- 更新完了後、`docs/plans/docs-examples-audit/` のフェーズ作業へ復帰できる状態にする。

## 更新対象の Rust バージョン
- `rust-toolchain.toml` を導入し、`channel = "stable"` で最新安定版を追従する。
- `icu_normalizer_data` の `rustc 1.83+` 要件を満たすため、最低条件を `stable (>= 1.83)` とする。
- 実作業時は `rustup show` で取得した stable の具体バージョンを `reports/spec-audit/summary.md` と本計画書に記録する。

## 対象範囲
- Rust ツールチェーン（`rustc` / `cargo` / `rustfmt` / `clippy`）の更新。
- `compiler/` 配下のバイナリとライブラリ（`reml_frontend`、runtime、tooling、tests）。
- Cargo.lock の更新と、必要に応じた依存クレートのバージョン調整。

## 非対象
- Reml 言語仕様そのものの変更。
- `docs/spec/` や `examples/` の内容変更（本計画の範囲外）。

## 関連資料
- `docs/plans/rust-migration/overview.md`
- `docs/plans/docs-examples-audit/1-2-impl-gap-plan-20251223.md`
- `docs/spec/0-1-project-purpose.md`
