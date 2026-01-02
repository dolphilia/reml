# Reml 設計ゴールと実装補遺

このドキュメントは `0-0-overview.md` から分離した詳細メモです。概要からこぼれる設計ゴール、横断テーマ、実装ステップ、簡易 BNF などを保管し、実装者や仕様検証者が参照しやすいようにまとめています。

## 0. 設計ゴール（非機能要件）

**実用性最優先**: 教育用途は結果的に使えれば良い程度の二次的位置付け

1. **実用性能**：末尾最適化、トランポリン、Packrat/左再帰を必要時だけON。FFI・LLVM連携による実用価値の確保。
2. **短く書ける**：演算子優先度や空白処理を"宣言"で終わらせる。
3. **読みやすい**：左→右に流れるパイプ、名前付き引数、推論の強さ。
4. **エラーが良い**：位置・期待集合・cut（コミット）・復旧・トレース。
5. **Unicode前提**：`byte/char/grapheme` の3レイヤを区別。
6. **段階的な抽象化拡張**：高度な機能（代数的効果、ハンドラ、行多相など）は opt-in フラグで試せるようにし、基盤仕様は既存ユーザーの学習負荷を変えない。

## 0.5 横断テーマと配置

Reml はコア哲学（小さく強いコア・宣言的な操作・高品質な診断）を、以下の横断テーマとして全仕様に貫く。

- **型安全な設定**：`Core.Config`（[3-7](../spec/3-7-core-config-data.md)）と CLI ガイドで、宣言 DSL → スキーマ検証 → 差分適用 → 実運用(Audit) の安全線を確立する。
- **ツール連携**：`RunConfig.extensions["lsp"]` / 構造化ログ（[2-6](../spec/2-6-execution-strategy.md)）と IDE/LSP ガイドに加え、監査・メトリクス API（[3-4](../spec/3-4-core-numeric-time.md), [3-6](../spec/3-6-core-diagnostics-audit.md)）で共通 JSON メタデータを揃える。
- **ターゲット適応**：`RunConfig.extensions["target"]` で OS/アーキテクチャ/フィーチャ情報を供給し、`@cfg`（[1-1](../spec/1-1-syntax.md#条件付きコンパイル属性-cfg)）と `platform_info()`（[3-8](../spec/3-8-core-runtime-capability.md)）を連動させて安全に分岐する。CLI/ビルドツールはここに整形済みターゲット情報を登録し、診断は `target.config.*` メッセージで誤設定を即座に指摘する。
- **プラグイン拡張**：`ParserPlugin` / `CapabilitySet`（[2-1](../spec/2-1-parser-type.md) の I 節）と DSL プラグインガイド、Capability Registry（[3-8](../spec/3-8-core-runtime-capability.md)）で外部 DSL の登録・互換・署名検証まで一貫して扱う。
- **段階的な効果導入**：`-Z` 系実験フラグと仕様書付録で新しい効果モデルを公開し、`@pure` と Capability Registry の契約を維持したまま opt-in で試せるようにする。PoC→正式統合のロードマップは `docs/notes/effects/algebraic-effects-implementation-roadmap-revised.md` を参照。

これらの柱は `0-1-project-purpose.md` の目的群と同期し、フェーズ更新時も設計意図を再確認できるよう整理されている。

## 0.6 Chapter 3 連携メモ

標準パーサ API で蓄積した設計と診断ノウハウを、Chapter 3 で標準ライブラリ（`Core.*`）へ拡張する準備を進めている。フェーズ1では標準ライブラリがカバーすべき範囲と優先度を整理し、フェーズ2では Prelude、Collections、Text などの章立てとクロスリファレンスの骨子を固めた。【F:notes/core-library-scope.md†L1-L48】【F:notes/core-library-outline.md†L1-L31】

今後は `Core.Diagnostics`/`Core.Audit`、`Core.Runtime`/Capability Registry など、横断テーマの要素を Chapter 3 内の各節へ落とし込み、既存ガイドとの往復参照を整備する計画である。【F:notes/core-library-outline.md†L20-L31】

## 1. 言語コア仕様（Reml）抜粋

### 1.1 構文（例）

```reml
let x = 42           // 不変（デフォルト）
var y = 0            // 可変
fn add(a: i64, b: i64) -> i64 = a + b
```

```reml
match v with
| Ok(x)  -> println(x)
| Err(e) -> panic(e)
```

### 1.2 型と推論メモ

* Hindley-Milner 系推論（明示注釈は任意、公開APIは型必須推奨）
* ADT + ジェネリクス + 型クラス相当（Traits）

### 1.3 効果と安全性メモ

* 例外なし（`panic` はデバッグ用）。失敗は `Result` or パーサの `Error`。
* 末尾再帰最適化、トランポリン（深い `many` でも安全）。

### 1.4 文字モデル要点

* `Byte` / `Char` / `Grapheme` を区別。
* 文字列は UTF-8。`text.iterGraphemes()` 等を標準装備。

## 2. 標準パーサAPI（Core.Parse）抜粋

```reml
type Parser<T> = fn(&mut State) -> Reply<T>
```

主要コンビネータ（`map`, `then`, `or`, `many`, `recover`, `trace` など）や演算子ビルダー、エラー設計の骨子をここにまとめている。

## 3. 実装アプローチ

### 3.1 MVP（最小実装）

* 基本型: `i64`, `Bool`, 単相関数
* 目標: IR実行器で `main` が走ること

### 3.2 Experimental → Stable の流れ

* 実験フラグ (`-Z`) による段階的な公開。
* ベータ統合・正式安定化までのチェックリスト。

### 3.3 本格実装

* データ型: タプル/配列/文字列（RC管理）、クロージャ
* 型システム: モノモルフィゼーションでジェネリクス

### 3.4 完全実装

* ADT/`match`/型クラス辞書パッシング
* エラー処理: `Result`/`Option` の一級化、`?` 演算子

## 4. ミニ言語仕様（BNF抜粋）

```bnf
Module   ::= { UseDecl | TypeDecl | FnDecl | LetDecl }+
...
```

## 5. 実装ガイド（言語処理系の観点）

* フロントエンド: Reml 自身も `Core.Parse` で自己記述可能。
* エラーフォーマッタ: `Err.pretty(src, e)` の表示方針。
* 最適化・IDE 連携メモ。

## 6. 要点まとめ

* 言語側: パイプ・型推論・ADT・マッチ・末尾最適化・Unicode。
* ライブラリ側: 少数精鋭のコンビネータと宣言的 precedence。
* 運用: Packrat/左再帰の選択的ON、期待集合ベースの診断。

## 関連仕様リンク

### 言語コア仕様

* [1.1 構文](../spec/1-1-syntax.md)
* [1.2 型と推論](../spec/1-2-types-Inference.md)
* [1.3 効果と安全性](../spec/1-3-effects-safety.md)
* [1.4 文字モデル](../spec/1-4-test-unicode-model.md)

### 標準パーサーAPI仕様

* [2.1 パーサ型](../spec/2-1-parser-type.md)
* [2.2 コア・コンビネータ](../spec/2-2-core-combinator.md)
* [2.3 字句レイヤ](../spec/2-3-lexer.md)
* [2.4 演算子優先度ビルダー](../spec/2-4-op-builder.md)
* [2.5 エラー設計](../spec/2-5-error.md)
* [2.6 実行戦略](../spec/2-6-execution-strategy.md)

### 実装関連

* [1.5 形式文法（BNF）](../spec/1-5-formal-grammar-bnf.md)
* [LLVM 連携ノート](../guides/llvm-integration-notes.md)
* [初期設計コンセプト](../guides/early-design-concepts.md)
