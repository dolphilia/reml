# Rust 移植計画ドキュメント集約

このディレクトリには、OCaml 実装から Rust 実装へ移行するための計画書と補助資料をまとめる。Phase 2-6 の Windows 対応停滞を解消し、Phase 3 のセルフホスト移行前に Rust 版コンパイラの足場を固めることが目的である。

## 位置づけ
- `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` の決定（移植言語を Rust に確定）を受けたフォローアップ
- Phase 2-8 以降の仕様監査やセルフホスト準備と並行して進める移行タスク群
- `compiler/rust/` 以下で進める実装成果物との対応関係を管理

## 初期構成（案）
- [`overview.md`](overview.md): Rust 移植プログラムの全体像、マイルストーン、依存関係
- `milestones/`: フェーズ別・モジュール別の詳細計画
- `risk-register.md`: 移植固有のリスクと緩和策
- `ci-strategy.md`: Rust 版のビルド・テスト・CI 方針

ドキュメントの正式な採番は `overview.md` 完成時に設定する。暫定構成は `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` の命名規則を参考に調整する。

## 作業ルール
- 既存仕様との整合を保ち、差分が発生した場合は `docs/spec/` および関連ガイドを更新する
- 新規に作成した計画書は `docs/plans/README.md` から参照可能にする
- 構成変更や大規模改訂を行った際は `docs-migrations.log` に記録する
