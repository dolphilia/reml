# FFI WIT PoC ガイド（ドラフト）

## 目的
WASM Component Model / WIT の PoC を行う際の最小手順を整理する。

## 想定読者
- WIT 連携の調査担当者
- PoC による適用可能性を評価したい開発者

## 前提
- 本ガイドは調査フェーズの PoC 想定であり、正式な実装手順ではない。
- 生成ツールやランタイムは外部依存であり、Reml リポジトリ内で完結しない可能性がある。

## PoC のゴール
- WIT 定義から Reml 側の呼び出しまでの最短パスを確認する。
- 共有メモリなし（Shared Nothing）前提で、所有権・コピー境界が破綻しないことを検証する。

## PoC 手順
1. WIT 定義を作成し、対象 API の型として `string` / `record` / `variant` / `list` / `option` / `result` / `resource` を含める。
2. 外部ツールで WIT から Reml 側のバインディング草案を生成し、生成結果を `Core.Ffi.Dsl` のラッパで包む。
3. Reml 側で呼び出し検証を実施し、境界データの整合と所有権移譲の挙動を確認する。
4. 調査ログへ観測結果を整理し、`do../../notes/ffi/ffi-wasm-component-model-log.md` に反映する。

## 検証観点
- `string` と `list<u8>` の境界コピーが期待通りに行われるか。
- `record` / `variant` のフィールド順序とタグが一致するか。
- `Result` マッピング時のエラー種別が診断キーと紐づくか。
- 監査ログに WIT 由来の識別情報を付与できるか。
- `resource` のライフサイクルが `own` / `borrow` で意図通りに管理されるか。
- `own` で受け取った値が境界で二重解放されないか。
- `borrow` がスコープ外へ逃げないよう制約できるか。
- Shared Nothing 前提のため、境界越えのコピーと解放責務が明示できるか。

## 成果物
- WIT 定義と生成物のスナップショット（調査ログ用）。
- PoC 実施時の観察結果（`do../../notes/ffi/ffi-wasm-component-model-log.md` へ反映）。
- WIT 生成、バインディング生成、呼び出し検証の手順メモ。

## PoC 実施記録（追記欄）
- 外部ツール名: （PoC 実施後に追記）
- 生成物パス: WIT 定義（例: `do../../notes/wit/samples/hello.wit`）
- 生成物パス: 生成バインディング（例: `do../../notes/wit/poc/bindings.reml`）
- 生成物パス: 検証ログ（例: `do../../notes/wit/poc/run-log.md`）

## 参考メモ
- Shared Nothing 前提のため、所有権移譲とコピー境界を明確化する。
- ここでは実装ではなく調査結果の整理を目的とする。
