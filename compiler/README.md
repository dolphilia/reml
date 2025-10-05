# compiler ディレクトリ構成（準備中）

Reml 実装の各フェーズを受け入れるための作業領域です。Phase 1 では OCaml 製ブートストラップコンパイラ、Phase 3 以降ではセルフホスト版の構築を想定しています。

## サブディレクトリ
- `ocaml/`: Phase 1 計画（`docs/plans/bootstrap-roadmap/1-x`）に基づく OCaml 実装を配置予定
- `self-host/`: Phase 3 計画（`docs/plans/bootstrap-roadmap/3-x`）で定義される Reml 実装を配置予定

各サブディレクトリには後続フェーズで `src/`, `tests/`, `docs/` などのモジュール構成を整備します。開始時期とタスクはブートストラップ計画書を参照してください。

## TODO
- [ ] Phase 1 移行時に `ocaml/README.md` を作成し、ビルド手順と依存関係を記載
- [ ] Phase 3 開始前に `self-host/` の構成とタスクスケジュールを確定
