# Phase 2-5 仕様差分レビュー チェックリスト

このテンプレートは Phase 2-5「仕様差分補正」タスクのレビュー担当者が記録を残すためのものです。各セクションを埋め、チェック項目を確認したのち、関連ドキュメント（差分リスト、`0-3-audit-and-metrics.md`）と紐付けて保存してください。

## 基本情報
- レビュー対象領域（例: Chapter 1 言語コア / パーサー API / 標準ライブラリ / 補助資料 / 用語索引）:
- 対象ドキュメント（相対パスで列挙）:
- レビュア（氏名/ロール）:
- レビュー日:
- 差分リスト ID（`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` 内の参照）:

## チェックリスト
各項目を確認したら `[ ]` を `[x]` に更新し、補足があれば備考欄に記入してください。

### 用語整合
- [ ] `docs/spec/0-2-glossary.md` と照合し、用語・表記ゆれが無いことを確認した。  
  - 備考:
- [ ] Glossary 更新が必要な場合、追加・修正案を差分リストへ記録した。  
  - 備考:

### コードサンプル検証
- [ ] `reml` タグ付きコードブロックを抽出し、`compiler/ocaml` のサンプルランナーまたは等価ツールでパース・型検証した。  
  - 実行ログ／コマンド（例: `scripts/run-sample.sh <file>`）:
- [ ] 失敗したサンプルがある場合、再現手順と原因を差分リストに記載した。  
  - 備考:

### データ構造対照
- [ ] ドキュメント記載のレコード/enum と実装（例: `compiler/ocaml/src/diagnostic_serialization.ml`, `runtime/native/capability_stage.ml`）のフィールド差分を比較し、結果を表形式で整理した。  
  - 差分比較メモ（ファイル/行参照付き）:
- [ ] 差異が存在する場合、修正案または TODO として差分リストに追加した。  
  - 備考:

### リンク・参照
- [ ] 相互参照・脚注・章内リンクが `README.md` / `docs/README.md` の導線と一致している。  
  - 検証方法（例: `markdown-link-check`、手動確認）:
- [ ] リンク切れや誤リンクがあれば、対象 URL と修正方法を記録した。  
  - 備考:

### 診断・監査フィールド
- [ ] `schema.version`, `audit.timestamp`, `bridge.stage.*`, `effect.stage.*`, `ffi_bridge.audit_pass_rate` など主要フィールドが仕様と実装で一致している。  
  - 参照したファイル／ログ:
- [ ] `scripts/validate-diagnostic-json.sh` を実行し、出力結果を差分リストに添付した。  
  - 実行日時と結果概要:
- [ ] Phase 2-7 の未完了タスク（技術的負債 ID 22/23）が原因で欠落している項目があれば、担当窓口へエスカレーションした。  
  - エスカレーションメモ:

### 技術的負債トラッキング
- [ ] `compiler/ocaml/docs/technical-debt.md` を参照し、関連 ID（特に 22/23）についてレビュー内容と影響度を更新した。  
  - 追記した内容:
- [ ] 新たに発見した負債がある場合、優先度と対応方針を `0-4-risk-handling.md` に登録した。  
  - 登録内容:

## 追補メモ
- 追加コメント・観察事項:
- 次週アクションアイテム（該当する場合）:

---
**提出先**: レビュー完了後はバージョン履歴を残すため、`docs/plans/bootstrap-roadmap/checklists/` 配下に日付・領域別ファイル（例: `spec-drift-review-YYYYMMDD-ch1.md`）として保存し、関連する差分リストおよび `0-3-audit-and-metrics.md` エントリへリンクしてください。
