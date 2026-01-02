# Core.Text 関連ドキュメント同期チェックリスト

## 概要
Core.Text / Unicode 実装に伴って更新すべき文書を列挙し、`README.md`・Phase 3 計画・各種ガイドとのリンク切れを防ぐ。

## チェック表
| ID | 文書/セクション | トリガーイベント | 必須更新内容 | リンク確認 (y/n) | 状況 | 備考 |
| --- | --- | --- | --- | --- | --- | --- |
| DOC-01 | `docs/spec/3-3-core-text-unicode.md` サンプル | API シグネチャ変更 / サンプル更新 | コード例の再実行、`examples/core-text/*.golden` のリンク追加 | Y | Done (2027-03-30) | §9 に `examples/core-text` 脚注・`reports/spec-audit/ch1/core_text_examples-20270330.md` を追加 |
| DOC-02 | `README.md` Phase 3 ハイライト | Text 実装マイルストーン終了 | 新規バッジと README リストの更新、`3-0-phase3-self-host.md` へリンク | Y | Done (2027-03-30) | ルート README/3-0 Self-Host に Core.Text 進捗節を追加 |
| DOC-03 | `docs/guides/compiler/core-parse-streaming.md` | `decode_stream` API 公開 | Streaming 例と Unicode 注意事項の追記 | Y | Done (2027-03-30) | §10 decode_stream + TextBuilder を追加 |
| DOC-04 | `docs/guides/ecosystem/ai-integration.md` | Text 正規化ポリシー決定 | AI 入出力フローに正規化ステップを追加 | Y | Done (2027-03-30) | §6 Normalize/prepare_identifier/`examples/core-text` 参照を追記 |
| DOC-05 | `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` | 新規リスク登録 | リスク ID、緩和策、フォローアップを追加 | Y | Done (2027-03-30) | `R-041 Unicode Data Drift` を登録 |

## 運用ルール
- チェック完了時に `リンク確認` 列へ `Y` を記載し、コミット ID を備考へ残す。
- 追跡対象が増えた場合は ID を採番し、このファイルと `docs/notes/process/docs-update-log.md` の双方に反映する。
