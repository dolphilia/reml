# 0.1 Rust ツールチェーン更新計画

## フェーズ 1: 現状調査と影響整理
- 現行 `rustc` / `cargo` バージョンを記録する。
- `compiler/rust/` 配下の Cargo ワークスペース構成とビルド対象を棚卸しする。
- `Cargo.lock` と主要クレートの MSRV 要件を確認し、更新による影響をリスト化する。

### 実施結果
- 現行ツールチェーン: `rustc 1.69.0 (84c898d65 2023-04-16)` / `cargo 1.69.0 (6e9a83356 2023-04-12)`。
- `rust-toolchain` / `rust-toolchain.toml` はリポジトリ内に未配置（ツールチェーン固定なし）。
- `compiler/rust/` 配下は Cargo ワークスペース未定義。リポジトリ直下の `Cargo.toml.ws` は `tests/reml_e2e` のみをメンバーに持つ。
- Cargo プロジェクト一覧とビルド対象:
  - `compiler/rust/adapter`（`reml_adapter`）: `lib` のみ。
  - `compiler/rust/frontend`（`reml_frontend`）: `lib` + `bin` (`reml_frontend`, `remlc`)。
  - `compiler/rust/backend/llvm`（`reml-llvm-backend`）: `lib` のみ。
  - `compiler/rust/runtime`（`reml_runtime`）: `lib`（`rlib`/`cdylib`） + `bin` (`text_stream_decode`, `reml_capability`) + `bench` 3 件。
  - `compiler/rust/runtime/ffi`（`reml_runtime_ffi`）: `lib` + `bench` 1 件（`build.rs` あり）。
  - `compiler/rust/ffi_bindgen`（`reml_ffi_bindgen`）: `lib` + `bin` (`reml-bindgen`)。
  - `compiler/rust/xtask`（`xtask`）: `bin`（`src/main.rs`）。

### Cargo.lock と MSRV 影響メモ
- ルートの `Cargo.lock` は `tests/reml_e2e` のみを対象としており、`compiler/rust/` 配下の依存は含まれない。
- `compiler/rust/` 配下の `Cargo.lock` 配置:
  - あり: `compiler/rust/adapter/Cargo.lock`, `compiler/rust/frontend/Cargo.lock`, `compiler/rust/runtime/Cargo.lock`, `compiler/rust/runtime/ffi/Cargo.lock`, `compiler/rust/backend/llvm/Cargo.lock`, `compiler/rust/xtask/Cargo.lock`
  - なし: `compiler/rust/ffi_bindgen`（ロックファイル未作成）
- 主要クレートのバージョン（抜粋）:
  - `frontend`: `logos 0.13.0`, `chumsky 0.9.3`, `icu_normalizer_data 2.1.1`, `icu_properties_data 2.1.2`, `uuid 1.18.1`, `wasmtime 6.0.2`, `time 0.3.30`, `notify 6.1.1`
  - `runtime`: `wasmtime 6.0.2`, `time 0.3.30`, `rust_decimal 1.39.0`, `num-bigint 0.4.6`, `num-rational 0.4.2`, `proptest 1.2.0`, `criterion 0.3.6`, `wat 1.0.68`
  - `runtime/ffi`: `time 0.3.30`, `notify 6.1.1`, `ordered-float 4.6.0`
  - `adapter`: `getrandom 0.2.16`, `serde 1.0.228`, `thiserror 1.0.69`
  - `backend/llvm`: `serde 1.0.228`, `serde_json 1.0.145`
  - `xtask`: `serde 1.0.228`, `toml 0.5.11`
- MSRV については各 `Cargo.toml` に `rust-version` が未設定のため、依存クレート側の要件に依存する。`reml_frontend` の `icu_normalizer_data` が `rustc 1.83+` を要求する事例が既知（`0-0-overview.md` の背景）なので、更新時は ICU 系と `wasmtime`/`time` などの依存で MSRV の再確認が必要。

## フェーズ 2: ツールチェーン更新
- Rust を最新安定版へ更新する（`rustup` 利用を想定）。
- `rustfmt` / `clippy` も同じツールチェーンに揃える。
- `rust-toolchain.toml` を導入する場合は、ワークスペース方針と合わせて明記する。

### 方針決定（確定）
- `rust-toolchain.toml` を導入し、`channel = "stable"` を採用する（最新安定版を常時追従）。
- `components = ["rustfmt", "clippy"]` を指定し、ツール群のバージョンを固定する。
- `icu_normalizer_data` の要件に合わせ、`stable (>= 1.83)` を最低条件として明記する。
- 更新当日に確定した stable の具体バージョンは、`reports/spec-audit/summary.md` と本計画書のフェーズ 2 実施記録に残す。

## フェーズ 3: 依存クレートの更新と整合
- `cargo update` で依存を更新し、MSRV を超えるクレートがあれば互換方針を決定する。
- `Cargo.lock` の差分を記録し、更新理由をメモに残す。
- 破壊的変更がある場合は、個別にピン留めまたは修正を行う。

## フェーズ 4: 再ビルドと検証
- `compiler/rust/` 配下の主要バイナリを順にビルドする。
- ビルドログと結果を `reports/` に記録する（詳細は 0-2 を参照）。

## フェーズ 5: 影響の整理と復帰準備
- 更新結果を本計画書と `reports/spec-audit/summary.md` に記録する。
- `docs/plans/docs-examples-audit/` の作業に戻るための差分確認を行う。
