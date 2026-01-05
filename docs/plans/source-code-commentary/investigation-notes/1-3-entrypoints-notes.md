# 第1部 第3章: 実行の入口 調査メモ

## 参照資料
- `compiler/frontend/README.md`: CLI の位置づけ（reml_frontend / remlc）。
- `compiler/frontend/src/bin/reml_frontend.rs`: Rust フロントエンド CLI の実装。
- `compiler/frontend/src/bin/remlc.rs`: 設定・マニフェスト・テンプレート CLI の実装。

## reml_frontend の入口
- ファイル冒頭コメントが目的を明示する（入力を解析して JSON を出力）。
  - `compiler/frontend/src/bin/reml_frontend.rs:1`
- `main` は FFI 実行エンジン初期化 → plugin/capability コマンドのショートカット → 引数解析 → パイプライン監査の開始 → `run_frontend` 実行 → CLI 出力/終了コード返却の流れ。
  - `compiler/frontend/src/bin/reml_frontend.rs:545-625`
- `run_frontend` は入力ファイル読み込み → 解析モード分岐（parse driver/通常/ストリーミング）→ 型推論 → 診断生成 → 出力構築までを直列に実行。
  - `compiler/frontend/src/bin/reml_frontend.rs:628-760`
- CLI 引数は `parse_args` で手書きパースされ、`RunSettings`/`StreamSettings`/出力系フラグを組み立てる。
  - `compiler/frontend/src/bin/reml_frontend.rs:2175-2295`

## reml_frontend のサブコマンド系ショートカット
- `--capability describe` は CLI 本体に入る前に処理される。
  - `compiler/frontend/src/bin/reml_frontend.rs:108-147`
- `plugin install`/`plugin verify`/`plugin list` などは `try_run_plugin_command` で判定し、成功時は `main` を終了する。
  - `compiler/frontend/src/bin/reml_frontend.rs:150-250`（冒頭の分岐ロジック）
  - `compiler/frontend/src/bin/reml_frontend.rs:545-568`（main 側の終了判定）

## reml_frontend の主要データ構造
- `CliRunResult` が CLI で必要な出力要素（診断エンベロープ、終了コード、監査情報）を束ねる。
  - `compiler/frontend/src/bin/reml_frontend.rs:91-99`
- 監査は `PipelineDescriptor` と `StageAuditPayload` を組み立てて通知している。
  - `compiler/frontend/src/bin/reml_frontend.rs:571-590`

## remlc の入口
- `main` は FFI 実行エンジン初期化と `try_main` のエラーハンドリングを担当する。
  - `compiler/frontend/src/bin/remlc.rs:21-31`
- `try_main` が `new`/`manifest`/`config`/`build` をディスパッチする。
  - `compiler/frontend/src/bin/remlc.rs:34-52`

## remlc の主要サブコマンド
- `new` はテンプレートコピーを行う。
  - `compiler/frontend/src/bin/remlc.rs:93-118`
- `manifest dump` はマニフェストを読み込んで JSON 出力する。
  - `compiler/frontend/src/bin/remlc.rs:55-90`
- `config lint/diff` はマニフェストやスキーマを検証して診断出力する。
  - `compiler/frontend/src/bin/remlc.rs:121-208`
- `build` は build 設定の検証と bindgen 実行可否の判定を行い、レポートを出力する。
  - `compiler/frontend/src/bin/remlc.rs:139-173`
