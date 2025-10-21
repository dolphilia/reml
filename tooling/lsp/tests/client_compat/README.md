# LSP 互換テスト（ドラフト）

Phase 2-4 で導入する診断 V2 フィールドを CLI/LSP 双方で検証するための試験クライアント雛形。

## ディレクトリ構成

- `client-v1.ts` — 既存クライアント互換性チェック。V2 追加フィールドを安全に無視できるか確認する。
- `client-v2.ts` — 新フィールドを積極的に利用するクライアントの検証。`diagnostic-v2.schema.json` によるスキーマバリデーションを行う。
- `fixtures/` — CLI から出力した診断 JSON のサンプル。`diagnostic-v2-ffi-sample.json` など FFI 監査ケースも含む。

## 実行方法

```bash
cd tooling/lsp/tests/client_compat
npm install
npm test
```

Vitest が `tests/` 以下のシナリオを実行し、V1/V2 両方のフィクスチャ読み込みと JSON Schema 検証を行う。

## 今後のタスク

1. `client-v2.ts` が参照する AJV スキーマを確定させ、生成ステップを CLI パイプラインへ統合。
2. Windows/macOS 向けの CLI 出力を fixtures に追加し、プラットフォーム差異をレビューできるようにする。
3. `npm test` を GitHub Actions（`lsp-contract` ジョブ予定）へ組み込み、JSON Schema 検証を自動化する。
