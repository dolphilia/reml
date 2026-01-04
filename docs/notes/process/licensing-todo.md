# TODO: FFI ヘッダ生成とライセンス整理

> 目的: Phase 2-3 における自動ヘッダ生成・外部ツール導入時のライセンス要件を整理し、`reports/ffi-bridge-summary.md` で追跡する作業と連動させる。

## 1. 対象範囲

- `compiler/runtime/native/include/reml_ffi_bridge.h` を起点としたヘッダ群
- 生成スクリプト（例: `scripts/gen-ffi-headers.reml` 仮称）
- 外部ツール（`cbindgen`, `clang -CC` など）を採用する場合のライセンス互換性

## 2. 未解決タスク

- [ ] SPDX 識別子と生成手順の標準化  
      - `reports/ffi-bridge-summary.md` に生成日時とコミットハッシュを記録する運用を定義  
      - Phase 3 で自動生成へ移行する際のレビュー手順をドラフト化
- [ ] 外部ツール利用時の依存ライセンス調査  
      - `cbindgen` / `bindgen` / `llvm-tblgen` など候補ツールのライセンス確認  
      - OSS 利用時の再頒布条件とソース開示要件をメモ化
- [ ] 監査ログへのライセンス情報連携  
      - `bridge.license` など追加キーの必要性を評価  
      - 監査ダッシュボードでの表示方法を Diagnostics チームと調整
- [ ] ランタイム計測ログ (`reml_ffi_bridge_get_metrics`, `reml_ffi_bridge_pass_rate`) を外部ストレージへ保存する場合のライセンス影響を確認し、必要に応じて `bridge.license` へ追記
- [ ] 仕様書更新時に SPDX / 著作権表記の差分チェックリストを追加し、`docs/spec/3-9-core-async-ffi-unsafe.md`・`docs/spec/3-6-core-diagnostics-audit.md` へ反映した内容を `reports/ffi-bridge-summary.md` §5 にリンクする

## 3. 関連ドキュメント

- docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md
- docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md
- docs/spec/3-9-core-async-ffi-unsafe.md
- reports/ffi-bridge-summary.md

## 4. 次のアクション

- [ ] ライセンス方針レビュー用の議題を次回定例に登録
- [ ] Phase 2-3 中盤までに調査メモを更新し、外部コントリビュータへの周知資料を草案化
