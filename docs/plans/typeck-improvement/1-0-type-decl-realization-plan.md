# 1.0 型宣言実体化計画（Type Alias / Newtype / Sum）

## 目的と背景

- `type` 宣言の内容（alias/newtype/合成型）を **型環境へ反映** し、型推論・診断・IR 生成に一貫した基盤を用意する。
- 現状の Rust Frontend は `type` 宣言の本文を AST へ保持しておらず、`TypeKind::Ident` の解決や union 変種ペイロードが型検査段階で無視される。
- 仕様例（`docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`）と実装の乖離を解消し、正準例の復元を継続できる状態にする。

## スコープ

- **含む**
  - AST/Parser の `type` 宣言本文保持
  - 型環境（type scope）への登録と参照解決
  - 型エイリアス展開、循環検出、診断
  - `type` ベースの合成型/変種ペイロードを型検査へ接続
- **含まない**
  - ランタイムの表現最適化や ABI 安定化
  - LSP 連携や外部フォーマット生成

## 仕様参照

- `docs/spec/1-1-syntax.md`（型宣言構文）
- `docs/spec/1-2-types-Inference.md`（型推論ルール）
- `docs/spec/1-3-effects-safety.md`（newtype の効果契約）

## 現状整理（ギャップ）

- `DeclKind::Type` が **本文を保持しない** ため、エイリアスの中身が型環境に反映されない。
- `TypeKind::Union` の `VariantPayload` が型推論で部分的に無視され、合成型の型付けに漏れがある。
- `type alias` / `type ... = new ...` の差異（名義型 or 展開型）が未定義。

## 実装方針

- `type` 宣言を **型環境のエントリ** として登録し、参照時に alias 展開または名義型として扱う。
- alias 展開は **遅延（参照時）** を基本とし、循環検出と深さ制限を導入する。
- `newtype` は **名義型（distinct）** として扱い、alias とは別の型分類に分ける。

## フェーズ

### フェーズ 1: AST/Parser の本文保持
- `DeclKind::Type` に **本文の AST** を追加（例: `TypeDeclBody::{Alias, Newtype, Sum}`）。
- `type` の `= ...` 部分をパースして AST に保持する。
- `render` のデバッグ出力を更新し、本文が確認できることを保証。

### フェーズ 2: 型環境と参照解決
- 型環境へ `type` 宣言の本文を登録（名前・ジェネリクス・本文）。
- `TypeKind::Ident` 解決で alias を展開（ジェネリクスを置換）。
- 循環検出（`type A = B`, `type B = A` など）を検出して診断を発火。

### フェーズ 3: 合成型と変種ペイロード
- `type Foo = | Bar { ... } | Baz(...)` を型環境で **識別子付き合成型** として扱う。
- `VariantPayload` の型情報を型推論に反映し、パターン/コンストラクタの束縛に反映する。
- `EnumDecl` と `type` 合成型の差異（名義/展開）を仕様に沿って整理する。

### フェーズ 4: 型検査・診断統合
- `type` の未解決参照、循環、過剰展開に対する診断コードを追加。
- alias 展開と newtype 名義型の違いを Typeck の診断に明示。

### フェーズ 5: Backend/IR 影響の整理
- 型環境に新しい表現が追加された場合の IR 表現（区別タグ）を整理。
- Backend で必要な場合のみ type mapping を拡張（名義型の保持/剥離ルール）。

## テスト計画

- `compiler/rust/frontend/tests/typeck_*` に alias/newtype/合成型のユニットテストを追加。
- `examples/docs-examples/spec/` の正準例で `type` を含むケースを追加し、`reml_frontend --emit-diagnostics` が 0 件になることを確認。
- 循環 alias / 参照未定義 / 名義型の一致判定など、診断の期待値を追加。

## 成果物（出口条件）

- `type` 宣言の本文が AST に保存され、型環境に反映されている。
- alias 展開・循環検出・newtype 名義型の区別が typeck で動作。
- 代表サンプル（spec コード）が診断 0 件で通る。

## リスクと対応

- **循環 alias**: 深さ制限と訪問記録を導入し、診断で明示する。
- **互換性**: 既存の型推論結果が変化する可能性があるため、影響範囲を `docs/plans/bootstrap-roadmap/` に報告する。
- **Backend 影響**: 型表現の名義化により layout/ABI が変わる可能性があるため、必要なら別計画を起票する。

## 次のアクション（起票候補）

- `TypeDeclBody` の AST 追加と parser 更新（フェーズ 1）
- alias 展開と循環診断の実装（フェーズ 2）
- 合成型/変種ペイロードの型推論拡張（フェーズ 3）
