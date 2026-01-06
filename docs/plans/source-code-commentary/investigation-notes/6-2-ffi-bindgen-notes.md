# 調査メモ: 第20章 FFI Bindgen

## 対象モジュール

- `compiler/ffi_bindgen/src/lib.rs`
- `compiler/ffi_bindgen/src/main.rs`
- `compiler/ffi_bindgen/README.md`
- `compiler/ffi_bindgen/Cargo.toml`
- `compiler/frontend/src/bin/remlc.rs`（`remlc build` の bindgen 統合）
- `docs/spec/3-9-core-async-ffi-unsafe.md`（reml-bindgen 仕様）
- `docs/guides/ffi/reml-bindgen-guide.md`

## 入口と全体像

- `compiler/ffi_bindgen/src/lib.rs` が `reml-bindgen` の中核で、設定読み込み・解析・生成・マニフェスト出力を担う。
  - `compiler/ffi_bindgen/src/lib.rs:9-208`
- `compiler/ffi_bindgen/src/main.rs` が CLI エントリで、引数解析・JSON Lines ログ・エラー変換を担当する。
  - `compiler/ffi_bindgen/src/main.rs:6-171`
- `compiler/ffi_bindgen/README.md` はツール概要と CLI/設定の最小要件をまとめている。
  - `compiler/ffi_bindgen/README.md:1-27`
- `remlc build` の `--emit-bindgen` が `reml-bindgen` を起動し、キャッシュと監査メタデータ生成を行う。
  - `compiler/frontend/src/bin/remlc.rs:518-798`

## データ構造

- **BindgenConfig**: `reml-bindgen.toml` の設定モデル。`headers` / `include_paths` / `defines` / `output` / `manifest` / `exclude` を保持する。
  - `compiler/ffi_bindgen/src/lib.rs:9-49`
- **CliOptions**: CLI の上書き設定。`config_path` と各種オプションを保持する。
  - `compiler/ffi_bindgen/src/lib.rs:53-63`
- **DiagnosticEntry**: 生成時の診断データ。`code` / `symbol` / `c_type` / `reason` / `hint` を持つ。
  - `compiler/ffi_bindgen/src/lib.rs:65-76`
- **Manifest / ManifestType**: `bindings.manifest.json` のメタデータ本体。型変換と修飾子を記録する。
  - `compiler/ffi_bindgen/src/lib.rs:78-94`
- **RunResult**: 生成物のパスと `Manifest` を束ねる戻り値。
  - `compiler/ffi_bindgen/src/lib.rs:96-102`
- **ParsedHeader / ParsedFunction / ParsedParam / ParsedType**: 解析パイプライン内の中間表現。
  - `compiler/ffi_bindgen/src/lib.rs:262-287`

## コアロジック

- **設定ロードと検証**: `load_config` が `headers` / `include_paths` / `output` / `manifest` の必須性を検証する。
  - `compiler/ffi_bindgen/src/lib.rs:114-133`
- **生成フロー**: `run_bindgen` が `reml-bindgen.toml` を読み、パス解決・入力ハッシュ計算・ヘッダ解析・`.reml` 生成・`bindings.manifest.json` 生成までを直列に実施する。
  - `compiler/ffi_bindgen/src/lib.rs:136-208`
- **入力ハッシュ**: `calculate_input_hash` は `CARGO_PKG_VERSION` と入力パス/設定を連結して SHA256 から 8 バイトを切り出す。
  - `compiler/ffi_bindgen/src/lib.rs:223-252`
- **ヘッダ解析**: `parse_header` が行単位で正規表現を当て、関数宣言のみ抽出する（プリプロセスは行わない）。
  - `compiler/ffi_bindgen/src/lib.rs:289-347`
- **型解析**:
  - `parse_params` は `...` を検出すると `ffi.bindgen.unknown_type` を記録して解析を中断する。
    - `compiler/ffi_bindgen/src/lib.rs:376-414`
  - `parse_type` は配列・関数ポインタ・空型を未対応型として診断し、基本型を Reml 型に写像する。
    - `compiler/ffi_bindgen/src/lib.rs:454-566`
  - `split_type_and_name` は最後のトークンを引数名として分離する簡易ルールを持つ。
    - `compiler/ffi_bindgen/src/lib.rs:416-451`
- **出力生成**:
  - `render_reml` が `extern "C"` ブロックと関数宣言を生成する。
    - `compiler/ffi_bindgen/src/lib.rs:568-595`
  - `infer_module_name` が出力先パスから `module` 名を推定する（`AGENTS.md` / `.git` をルート判定に利用）。
    - `compiler/ffi_bindgen/src/lib.rs:597-629`
- **CLI ログ**: `main` は `bindgen.start` / `bindgen.parse` / `bindgen.generate` / `bindgen.finish` の JSON Lines を出力する。
  - `compiler/ffi_bindgen/src/main.rs:35-59`
- **remlc 統合**: `run_bindgen_if_enabled` が `--emit-bindgen` 時に `reml-bindgen` を起動し、キャッシュと監査メタデータを管理する。
  - `compiler/frontend/src/bin/remlc.rs:518-798`

## エラー処理

- `BindgenError` が設定/解析/生成の失敗を分類し、CLI で `ffi.bindgen.*` 診断に変換される。
  - `compiler/ffi_bindgen/src/lib.rs:104-112`
  - `compiler/ffi_bindgen/src/main.rs:138-150`
- 既存出力への書き込みは `ffi.bindgen.output_overwrite` を診断に追加しつつ上書きする（CLI 側ではキャッシュ復元時に上書き禁止）。
  - `compiler/ffi_bindgen/src/lib.rs:631-649`
  - `compiler/frontend/src/bin/remlc.rs:800-941`
- `remlc` 側は `ffi.bindgen.generate_failed` を基本エラーとして扱い、監査メタデータを作成する。
  - `compiler/frontend/src/bin/remlc.rs:629-698`
  - `compiler/frontend/src/bin/remlc.rs:915-925`

## 仕様との対応メモ

- `docs/spec/3-9-core-async-ffi-unsafe.md` の `reml-bindgen` 仕様は設定キー・診断キー・`bindings.manifest.json` の必須項目を定義している。
  - `docs/spec/3-9-core-async-ffi-unsafe.md:904-983`
- `reml-bindgen` の診断メタデータ形式と JSON Lines ログはガイドと一致する。
  - `docs/guides/ffi/reml-bindgen-guide.md:10-75`
- **ギャップ候補**:
  - 仕様は `struct` / `enum` の `repr(C)` 生成に言及するが、現実装は関数宣言のみ生成する。
  - 仕様は `ffi.bindgen.unresolved_symbol` を定義するが、実装に該当診断は存在しない。
  - 仕様の入力ハッシュはヘッダ実体・TargetProfile・`reml-bindgen` バージョンを含める想定だが、`reml-bindgen` 自体のハッシュと `remlc` のハッシュ計算が一致していない。
    - `compiler/ffi_bindgen/src/lib.rs:223-252`
    - `compiler/frontend/src/bin/remlc.rs:715-731`

## TODO / 不明点

- `compile_commands` が設定に存在するが、解析フローで利用されていない（将来用途か要確認）。
- `remlc` の監査メタデータに `manifest_path` として `reml.json` のパスが入っている点は仕様との差分になりうる。
- `reml-bindgen` の `--version` 出力は現時点の CLI に未実装（`remlc` は `reml-bindgen --version` を呼ぶ）。
