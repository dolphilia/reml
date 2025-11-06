# 0.0 Rust 移植ロードマップ

本章は `unified-porting-principles.md` で定義した方針に基づき、Phase P0（ベースライン整備）の作業順序と完了条件を明文化する。Windows 対応停滞で蓄積した課題を Rust 移植計画へ移譲しつつ、OCaml 実装との整合を確保するための起点となる。

## 0.0.1 目的
- OCaml 実装の資産・メトリクス・診断基準を棚卸しし、Rust 移植の比較対象（ゴールデン/ベンチ/監査）を固定する。
- Windows 向けツールチェーンと CI 基盤の健康状態を明らかにし、Rust 版が参照すべき環境チェックリストを整備する。
- Phase P1 以降の計画書（フロントエンド移植、診断互換、Runtime 統合）に引き継ぐための共通語彙・成果物テンプレートを確立する。

## 0.0.2 スコープと前提
- **含む作業**: 計画書ひな形整備 (`0-1`/`0-2`/`appendix`)、メトリクス収集観点の整理、OCaml 側の基準線更新、Windows 環境監査手順の確立。
- **含まない作業**: Rust 実装コードの作成、CI ジョブの実装変更、仕様書そのものの改訂。必要な場合は Phase P1〜P3 の計画書で扱う。
- **前提**:
  - `docs/spec/0-1-project-purpose.md` に定義された性能・安全性・段階的拡張の価値観を優先順位付けに適用する。
  - Phase 2 の成果物（`reports/diagnostic-format-regression.md`, `tooling/ci/collect-iterator-audit-metrics.py` 等）が最新状態で参照可能である。
  - Phase 2-6 Windows 対応の経緯は `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` に集約されており、本計画は同資料の選定結果（Rust 採用）を前提とする。

## 0.0.3 作業マイルストーン

| 週番号（目安） | マイルストーン | 主成果物 | 検証方法 |
| --- | --- | --- | --- |
| W1 | ベースライン指標の固定 | `0-1-baseline-and-diff-assets.md` 初稿 | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の指標を引用して確認 |
| W2 | 環境監査の手順化 | `0-2-windows-toolchain-audit.md` 初稿 + `tooling/toolchains` 参照チェック | `setup-windows-toolchain.ps1` / `check-windows-bootstrap-env.ps1` の実行要件レビュー |
| W3 | 用語・参照体系の統合 | `appendix/glossary-alignment.md` 初稿 | `docs/spec/0-2-glossary.md` との整合レビュー |
| W3.5 | P0 総合レビュー完了 | 本章を含む P0 文書群のレビュー記録 | `0-3-audit-and-metrics.md` へレビューログ登録、`docs/plans/rust-migration/README.md` 更新 |

## 0.0.4 タスクブレークダウン
1. **ベースライン整備**（`0-1` 参照）  
   - OCaml 実装のディレクトリ構造・ゴールデンテスト・診断レポートの現状をまとめる。  
   - Rust 版で再現すべき観測点（AST/IR、診断 JSON、ベンチマーク）と収集手段を明記する。
2. **環境監査**（`0-2` 参照）  
   - Windows (MSVC/GNU) 両系統で Rust 移植時に必要となるツールセットを列挙し、既存 PowerShell スクリプトの検証手順を付与する。  
   - CI ログに残すべき証跡と `collect-iterator-audit-metrics.py` のフラグ設定方針を整理する。
3. **用語整合**（`appendix` 参照）  
   - Rust 固有概念（所有権、Borrow Checker 等）と Reml 仕様語彙のマッピングを提示する。  
   - P1 以降の計画書で新用語を導入する際のレビュー手順を定義する。

## 0.0.5 依存関係とハンドオーバー
- Phase 2 の残課題（`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`）で収集した診断メトリクスが P0 の初期値となる。差分が発生した場合は `0-1` の追跡表と `0-3-audit-and-metrics.md` を同時更新する。
- Windows 移行オプション検討（`2-6-windows-support-migration-options.md`）に記録されたリスクは `0-2` および `4-0-risk-register.md`（P4 計画予定）で再利用する。
- P0 文書レビュー完了後、Phase P1 計画 (`1-0-front-end-transition.md`) へ `dual-write` 戦略とベースライン値を引き継ぐ。

## 0.0.6 完了条件
- `0-1` `0-2` `appendix/glossary-alignment.md` の各ドキュメントに初稿が存在し、レビューコメントを `0-3-audit-and-metrics.md` に記録済みである。
- OCaml 実装との比較指標（解析性能、診断キー、監査メトリクス）が `0-1` に明記され、Rust 実装で再測定するための手順が定義されている。
- Windows ツールチェーン監査チェックリストが `0-2` にまとまり、PowerShell スクリプトで出力すべきログ項目が列挙されている。
- 用語整合表に Phase P0 で必要なキーワードがすべて含まれ、`docs/spec/0-2-glossary.md` 参照が付与されている。

## 0.0.7 関連資料
- `docs/plans/rust-migration/unified-porting-principles.md`
- `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md`
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`
- `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md`
- `reports/diagnostic-format-regression.md`
- `tooling/toolchains/README.md`

---

> **レビューノート**: P0 作業で得られた補足調査や TODO は `docs/notes/` 配下に追記案を残し、`docs-migrations.log` へ記録すること。レビュー完了後に脚注リンクを追加予定（コミット時点では保留）。
