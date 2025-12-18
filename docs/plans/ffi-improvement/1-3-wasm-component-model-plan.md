# Phase 4: WASM Component Model 連携計画（ドラフト）

## 背景
- WASM Component Model / WIT は高レベル型で相互運用でき、
  FFI の安全性を大きく向上させる可能性がある。
- Reml の FFI を長期的に拡張するための方針整理が必要。

## スコープ
- WIT によるインターフェース定義の取り込み方針を整理。
- Canonical ABI を Reml の型システムにどう接続するかを検討。

## 成果物
- WIT 連携の設計メモ（仕様化前の草案）
- Reml の型表現との対応表（一次案）
- `docs/notes/` への調査ログ追記案

## 仕様検討項目
1. **WIT 型対応**
   - `string` / `record` / `variant` / `list` を Reml 型へ写像
2. **境界安全性**
   - FFI と異なるメモリ管理モデル（Shared Nothing）への適応
3. **ツール連携**
   - WIT バインディング生成ツールとの連携方針

## 実装ステップ（ドラフト）
1. `docs/notes/` に WIT 連携の調査ログを追加
2. `docs/spec/3-9-core-async-ffi-unsafe.md` に将来拡張セクションを追記
3. `docs/guides/` に PoC 手順を整理（必要時）

## 依存関係
- `docs/notes/ffi-improvement-survey.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`

## リスクと対策
- **仕様過多**: Phase 4 は調査と設計整理のみとし、実装は別計画に分離する。

## 完了判定
- WIT 連携の調査メモと対応表の一次案が `docs/notes/` に整理されている。
- `docs/spec/3-9-core-async-ffi-unsafe.md` に将来拡張の備考が追記されている。
