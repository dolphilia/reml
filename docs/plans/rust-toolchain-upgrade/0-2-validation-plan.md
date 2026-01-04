# 0.2 再ビルドと検証計画

## 目的
- ツールチェーン更新後に `compiler/` 配下の主要バイナリがビルドできることを確認する。

## ビルド対象（最低限）
- `compiler/frontend`（`reml_frontend`）
- `compiler/runtime`
- `compiler/tooling`（存在する場合）

## 推奨コマンド例
- `cargo build --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend`
- `cargo build --manifest-path compiler/runtime/Cargo.toml`
- `cargo build --manifest-path compiler/tooling/Cargo.toml`

## 検証ログの記録
- 実行コマンドと結果を `reports/spec-audit/summary.md` に追記する。
- 重大な依存更新があれば `reports/spec-audit/ch0/` などにメモを作成し、更新理由と差分を記録する。

## 修正優先の運用
- ビルド失敗時はロールバックを前提とせず、最新ツールチェーンのまま修正を試みる。
- 修正の進め方:
  - 失敗ログから原因を特定し、対象クレート単位で切り分ける。
  - 既存コードの修正で解消できるかを最優先で検討する。
  - 依存クレート側の問題は `patch` / `replace` 等の手段で一時的にパッチ適用を検討する。
  - 代替クレートの調査やバージョン固定（限定的ピン留め）も検討する。
- 対応内容と判断理由は `reports/spec-audit/summary.md` に追記し、必要なら `reports/spec-audit/ch0/` に詳細メモを残す。

### フェーズ 4 のログ項目（修正対応）
- 原因切り分け: 失敗したクレート名、再現コマンド、ログ抜粋。
- 修正内容: 変更ファイル、対応方針（コード修正/パッチ/代替/ピン留め）。
- パッチ有無: `patch` / `replace` / フォーク利用の有無と対象クレート名。

### フェーズ 4 ログ記録サンプル
- JST 時刻: 10:30
- 対象クレート: `compiler/frontend`
- 症状/ログ要約: `error[E0599]` でビルド停止
- 原因切り分け: `frontend` のみ再現、依存更新が要因
- 修正内容: `parser/lexer.rs` の型変換を修正
- パッチ有無: なし
- 結果: ✅ 成功
- 備考: `cargo build --manifest-path compiler/frontend/Cargo.toml`

## 判定基準
- 上記ビルドが全て成功すること。
- `Cargo.lock` の差分が計画書に記録され、更新理由が説明されていること。
