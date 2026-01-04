# 0.1 Rust ツールチェーン更新計画

## フェーズ 1: 現状調査と影響整理
- 現行 `rustc` / `cargo` バージョンを記録する。
- `compiler/` 配下の Cargo ワークスペース構成とビルド対象を棚卸しする。
- `Cargo.lock` と主要クレートの MSRV 要件を確認し、更新による影響をリスト化する。

### 実施結果
- 現行ツールチェーン: `rustc 1.69.0 (84c898d65 2023-04-16)` / `cargo 1.69.0 (6e9a83356 2023-04-12)`。
- `rust-toolchain` / `rust-toolchain.toml` はリポジトリ内に未配置（ツールチェーン固定なし）。
- `compiler/` 配下は Cargo ワークスペース未定義。リポジトリ直下の `Cargo.toml.ws` は `tests/reml_e2e` のみをメンバーに持つ。
- Cargo プロジェクト一覧とビルド対象:
  - `compiler/adapter`（`reml_adapter`）: `lib` のみ。
  - `compiler/frontend`（`reml_frontend`）: `lib` + `bin` (`reml_frontend`, `remlc`)。
  - `compiler/backend/llvm`（`reml-llvm-backend`）: `lib` のみ。
  - `compiler/runtime`（`reml_runtime`）: `lib`（`rlib`/`cdylib`） + `bin` (`text_stream_decode`, `reml_capability`) + `bench` 3 件。
  - `compiler/runtime/ffi`（`reml_runtime_ffi`）: `lib` + `bench` 1 件（`build.rs` あり）。
  - `compiler/ffi_bindgen`（`reml_ffi_bindgen`）: `lib` + `bin` (`reml-bindgen`)。
  - `compiler/xtask`（`xtask`）: `bin`（`src/main.rs`）。

### Cargo.lock と MSRV 影響メモ
- ルートの `Cargo.lock` は `tests/reml_e2e` のみを対象としており、`compiler/` 配下の依存は含まれない。
- `compiler/` 配下の `Cargo.lock` 配置:
  - あり: `compiler/adapter/Cargo.lock`, `compiler/frontend/Cargo.lock`, `compiler/runtime/Cargo.lock`, `compiler/runtime/ffi/Cargo.lock`, `compiler/backend/llvm/Cargo.lock`, `compiler/xtask/Cargo.lock`
  - なし: `compiler/ffi_bindgen`（ロックファイル未作成）
- 主要クレートのバージョン（抜粋）:
  - `frontend`: `logos 0.13.0`, `chumsky 0.9.3`, `icu_normalizer_data 2.1.1`, `icu_properties_data 2.1.2`, `uuid 1.18.1`, `wasmtime 6.0.2`, `time 0.3.30`, `notify 6.1.1`
  - `runtime`: `wasmtime 6.0.2`, `time 0.3.30`, `rust_decimal 1.39.0`, `num-bigint 0.4.6`, `num-rational 0.4.2`, `proptest 1.2.0`, `criterion 0.3.6`, `wat 1.0.68`
  - `compiler/runtime/ffi`: `time 0.3.30`, `notify 6.1.1`, `ordered-float 4.6.0`
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

### 実施結果
- `rust-toolchain.toml` をリポジトリ直下へ追加し、`channel = "stable"` と `components = ["rustfmt", "clippy"]` を設定した。
- `rustup` を導入して `rustup update stable` を実行し、`rustc 1.92.0 (ded5c06cf 2025-12-08)` を最新安定版として確認した。
- `stable (>= 1.83)` の最低条件は維持し、具体バージョンは上記の通りとする。

## フェーズ 3: 依存クレートの更新と整合
- `cargo update` で依存を更新し、MSRV を超えるクレートがあれば互換方針を決定する。
- `Cargo.lock` の差分を記録し、更新理由をメモに残す。
- 破壊的変更がある場合は、個別にピン留めまたは修正を行う。

### 実施結果
- `cargo update` を以下で実行し、該当する `Cargo.lock` を更新した。
  - `compiler/adapter`
  - `compiler/frontend`
  - `compiler/backend/llvm`
  - `compiler/runtime`
  - `compiler/runtime/ffi`
  - `compiler/xtask`
- 主要な更新差分（抜粋）:
  - 共通更新: `itoa 1.0.16`, `ryu 1.0.21`, `serde_json 1.0.146`, `syn 2.0.111`
  - `frontend`: `uuid 1.19.0`, `cc 1.2.50`, `insta 1.45.0`, `log 0.4.29`, `redox_syscall 0.6.0`, `zerocopy 0.8.31`, `windows-sys 0.61.2` 追加
  - `runtime`: `csv 1.4.0`, `url 2.5.7`, `rayon 1.11.0`, `wasm-encoder 0.243.0`, `wast 243.0.0`, `wat 1.243.0` に更新し、`icu_*` 系と `wasmparser 0.243.0` などの新規追加を確認
- `Cargo.toml.ws` は `cargo` の manifest として認識されないため、`Cargo.lock`（リポジトリ直下）は更新できなかった。`tests/reml_e2e` のロック更新は `Cargo.toml` の復帰手順（`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` 参照）を含めて方針決定が必要。
- 破壊的変更の影響はフェーズ 4 のビルド検証で確認し、必要に応じてピン留め方針を追記する。

## フェーズ 4: 再ビルドと検証
- `compiler/` 配下の主要バイナリを順にビルドする。
- ビルドログと結果を `reports/` に記録する（詳細は 0-2 を参照）。
- ビルド失敗時はロールバックではなく修正優先で対応し、原因切り分け・コード修正・パッチ適用・代替クレート検討の順で解消を試みる。
- 修正対応ログの項目は `docs/plans/rust-toolchain-upgrade/0-2-validation-plan.md` の「フェーズ 4 のログ項目（修正対応）」に従う。

### 実施結果
- `cargo build --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend` を実行し、ビルド成功を確認した。
- `cargo build --manifest-path compiler/runtime/Cargo.toml` を実行し、ビルド成功を確認した（`core_prelude` 未定義の `unexpected_cfgs` など警告は継続）。
- `compiler/tooling` が存在しないため、該当ビルドは対象外とした。
- 失敗対応として `time 0.3.30` の `error[E0282]` を解消するため `time = "0.3.36"` へ緩和し、`cargo update -p time` で `0.3.44` に更新した。
- `compiler/frontend` の `OperationDecl` / extern パラメータ型不整合を修正し、ビルドを通した。
- 実行コマンド・結果・修正対応ログは `reports/spec-audit/summary.md` に追記済み。

## フェーズ 5: 影響の整理と復帰準備
- 更新結果を本計画書と `reports/spec-audit/summary.md` に記録する。
- `docs/plans/docs-examples-audit/` の作業に戻るための差分確認を行う。
