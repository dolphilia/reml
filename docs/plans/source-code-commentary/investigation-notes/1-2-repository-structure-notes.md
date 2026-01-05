# 第2章 調査メモ: リポジトリ構造

## 参照した資料
- `README.md:21-65`（リポジトリの役割と主要ディレクトリ）
- `README.md:32-44`（`Cargo.toml.ws` と `--manifest-path` 前提）
- `compiler/README.md:1-19`（`compiler/` 配下のサブディレクトリ一覧と入口 README）
- `compiler/frontend/README.md:3-15`（frontend の責務と CLI）
- `compiler/backend/README.md:3-6`（backend の位置づけ）
- `compiler/runtime/README.md:3-9`（runtime の構成）
- `compiler/adapter/README.md:3-13`（adapter の責務とサブシステム）
- `compiler/ffi_bindgen/README.md:3-27`（ffi_bindgen の役割と構成）
- `compiler/xtask/README.md:3-13`（xtask の用途）
- `tooling/README.md:1-17`（tooling の役割とサブディレクトリ）
- `docs/README.md:1-55`（docs 配下のカテゴリ構成）
- `benchmarks/Cargo.toml:1-35`（ベンチマーク用クレートとベンチ対象）
- `benchmarks/src/lib.rs:1-2`（ベンチマーク用の共有クレート）
- `tooling/examples/run_examples.sh:7-17`（expected ディレクトリの参照）
- `expected/dsl_paradigm/README.md:1-11`（expected のスナップショット説明）
- `third_party/proc_macro_crate/README.md:1-24`（third_party に置かれた外部クレート）
- `docs/spec/3-6-core-diagnostics-audit.md:318-326`（tmp 配下のテレメトリ出力）
- `docs/spec/3-6-core-diagnostics-audit.md:1579-1583`（tmp 配下の監査ログ出力先）

## 調査メモ

### リポジトリ全体の役割と主要ディレクトリ
- ルート README では、仕様策定 (`docs/spec/`)、言語実装 (`compiler/` と `tooling/`)、実務ガイドや計画書 (`docs/`)、サンプル/テスト (`examples/`, `tests/`)、監査レポート (`reports/`) を主要領域として挙げている。 (`README.md:21-65`)
- ルートは `Cargo.toml.ws` のみを置く構成で、`cargo` 実行は `--manifest-path` 指定が前提。 (`README.md:32-44`)

### compiler/ 配下の構造
- `compiler/` は Rust 実装の集約領域で、`frontend` / `backend` / `runtime` / `adapter` / `ffi_bindgen` / `xtask` に分割される。 (`compiler/README.md:5-19`)
- `frontend` は字句解析〜型検査、診断、ストリーミング実行と CLI (`reml_frontend`, `remlc`) を提供する。 (`compiler/frontend/README.md:3-15`)
- `backend` は LLVM バックエンドのスケルトンを `llvm/` に置く。 (`compiler/backend/README.md:3-6`)
- `runtime` は Rust ランタイム本体に加えて `native/` と `ffi/` を含む構成。 (`compiler/runtime/README.md:3-9`)
- `adapter` は Env/FS/Network/Time/Random/Process/Target を束ねるプラットフォーム差異吸収層。 (`compiler/adapter/README.md:3-13`)
- `ffi_bindgen` は C ヘッダから Reml の FFI シグネチャを生成するツール群。 (`compiler/ffi_bindgen/README.md:3-27`)
- `xtask` は監査や補助タスクの実行用クレート。 (`compiler/xtask/README.md:3-13`)

### tooling/ 配下の構造
- tooling は CI/検証、監査、LSP、リリースなど周辺ツール資産を集約し、`benchmarks/`, `ci/`, `examples/`, `json-schema/`, `lsp/`, `release/`, `review/`, `runtime/`, `scripts/`, `telemetry/`, `templates/`, `toolchains/` を持つ。 (`tooling/README.md:1-17`)

### docs/ 配下の構造
- `docs/` は公式仕様 (`docs/spec/`)、実務ガイド (`docs/guides/`)、調査ノート (`docs/notes/`)、計画書 (`docs/plans/`) を中心に整理されている。 (`docs/README.md:1-55`)

### ルート直下の補助ディレクトリ
- `benchmarks/` は `reml_text_benchmarks` というベンチマーク用クレートで、`text/*` と `parse/profile.rs` のベンチを登録している。 (`benchmarks/Cargo.toml:1-35`) 共有データのための空ライブラリを保持する。 (`benchmarks/src/lib.rs:1-2`)
- `expected/` は `examples/` の出力スナップショットを保持し、`run_examples.sh` で必要ディレクトリとして参照される。 (`tooling/examples/run_examples.sh:7-17`) `expected/dsl_paradigm/README.md` では DSL パラダイム例の stdout/監査ログスナップショットを定義している。 (`expected/dsl_paradigm/README.md:1-11`)
- `third_party/` には外部クレートを配置している。現時点では `proc_macro_crate` が `README` を持ち、外部由来コードであることが明示されている。 (`third_party/proc_macro_crate/README.md:1-24`)
- `tmp/` は一時生成物の置き場として運用され、診断テレメトリの既定出力先や監査ログの一時保存先として参照される。 (`docs/spec/3-6-core-diagnostics-audit.md:318-326`, `docs/spec/3-6-core-diagnostics-audit.md:1579-1583`)

### 未確認事項 / TODO
- `expected/` 配下の各サブスイートの更新手順や生成コマンドは、`tooling/examples` 側のスクリプトを精査して追記する必要がある。
