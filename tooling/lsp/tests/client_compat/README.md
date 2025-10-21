# LSP 互換テスト（ドラフト）

Phase 2-4 で導入する診断 V2 フィールドを CLI/LSP 双方で検証するための試験クライアント雛形。

## ディレクトリ構成

- `client-v1.ts` — 既存クライアント互換性チェック。V2 追加フィールドを安全に無視できるか確認する。
- `client-v2.ts` — 新フィールドを積極的に利用するクライアントの検証。`diagnostic-v2.schema.json` によるスキーマバリデーションを行う。
- `fixtures/` — CLI から出力した診断 JSON のサンプルを配置する予定。

## 今後のタスク

1. `package.json` とテストランナー（`vitest` / `tsx` など）を追加し、CI から `npm test` で互換性チェックを実施。
2. `client-v2.ts` が参照する AJV スキーマを確定させ、生成ステップを CLI パイプラインへ統合。
3. Windows/macOS 向けの CLI 出力を fixtures に追加し、プラットフォーム差異をレビューできるようにする。
