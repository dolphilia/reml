# Phase 3: reml build 統合設計

## 背景
- 既存 FFI はライブラリパス解決やリンクが手作業で、運用負荷が高い。
- Zig のようにビルド設定で FFI 依存を一元管理する方針が必要。

## スコープ
- `reml build` と `reml.json` の FFI 依存記述仕様を定義。
- バインディング生成・リンク設定・監査ログの流れを設計。

## 成果物
- `reml.json` の FFI セクション案
- `reml build` 実行時のフロー図
- 監査ログのキー定義案

## 仕様検討項目
1. **マニフェスト構造（案）**
   ```reml
   // 概念例
   ffi {
     libraries = ["m", "ssl"]
     headers = ["/usr/include/openssl/ssl.h"]
     bindgen = { enabled = true, output = "generated/openssl.reml" }
   }
   ```
2. **ビルドフロー**
   - `reml build` → `reml-bindgen` → 生成物キャッシュ → コンパイル/リンク
3. **監査とキャッシュ**
   - `ffi.build.*` / `ffi.bindgen.*` の監査キー
   - 生成物の再現性（入力ハッシュの記録）

## 実装ステップ
1. `reml.json` の FFI セクション（`libraries`/`headers`/`bindgen`/`linker`）のキー定義と検証ルールを整理する。
2. `reml build` の実行フロー（ヘッダ解析→生成→キャッシュ→リンク）を図示し、`docs/spec/3-9-core-async-ffi-unsafe.md` に統合セクションとして追加する。
3. `ffi.build.*` / `ffi.bindgen.*` の監査キーと入力ハッシュの記録方針を `docs/spec/3-6-core-diagnostics-audit.md` に連携記述する。
4. `docs/guides/ffi/ffi-build-integration-guide.md` に FFI ビルド運用ガイドを追加し、失敗時の再生成条件とキャッシュ破棄手順を記載する。

## 依存関係
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-10-core-env.md`

## リスクと対策
- **プラットフォーム差分**: Linux/macOS/Windows の設定差分を
  `docs/spec/3-10-core-env.md` と連携して管理する。
- **キャッシュ不整合**: 入力ハッシュと生成物の組み合わせを監査ログに必須化する。

## 完了判定
- `reml.json` の FFI セクション仕様が文書化されている。
- `reml build` のヘッダ解析・リンク定義・監査ログのフローが文書化されている。
- `ffi.build.*` / `ffi.bindgen.*` の監査キー案が整理されている。
