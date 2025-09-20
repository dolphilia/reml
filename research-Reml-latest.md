# Reml 関連研究調査ノート

## 1. Reml が目指す方向性と中核特徴
- **設計ゴール**: 実用性能、宣言的に短く書ける構文、高可読性、安全かつ説明的なエラー、Unicode 前提の文字モデル（`0-1-overview.md`）
- **横断テーマ**: 型安全な設定 DSL、IDE/LSP 連携、プラグイン拡張によるエコシステム構築（`0-1-overview.md`）
- **言語機能**: パイプライン・名前付き引数・ADT・トレイト + HM 推論（`1-1-syntax.md`, `1-2-types-Inference.md`）
- **効果安全**: 属性ベースの効果契約、mut/io/ffi/panic/unsafe の分類、`?` による Result/Option 伝播（`1-3-effects-safety.md`）
- **Unicode モデル**: Byte/Char/Grapheme の 3 層、NFC 正規化、グラフェム境界 API（`1-4-test-unicode-model.md`）
- **パーサ基盤**: `Parser<T>` + `Reply{consumed, committed}`、Packrat/左再帰サポート、`cut/label/recover/trace` による高品質診断（`2-1-parser-type.md`, `2-2-core-combinator.md`, `2-5-error.md`）

## 2. Reml を支える主要技術領域
- **パーサーコンビネーターと実行戦略**: Packrat、左再帰処理、LL(*)、ストリーミング対応
- **型推論とトレイト解決**: Hindley–Milner + 制約収集、コヒーレンス制御、双方向型付け
- **効果システムと安全性**: 属性契約、行多相効果への拡張余地、`unsafe` 境界管理
- **Unicode 処理**: グラフェム単位の位置管理、正規化、表示幅、セキュリティ警告
- **設定 DSL / プラグインエコシステム**: 型付き設定、差分適用、Capability 管理とロックファイル
- **IDE 連携と診断体験**: JSON 診断、FixIt、自動修正提案、LSP 統合

## 3. 最新研究・資料サマリ

| 分野 | 出典 (年 / 会議・媒体) | 概要 | Reml への示唆 |
| ---- | ---------------------- | ---- | -------------- |
| Parser | Warth, Douglass, Millstein "Packrat Parsers Can Support Left Recursion" (2008 / PLDI) | Packrat でも左再帰を扱えるアルゴリズムを提案 | Reml の `left_recursion` 実装方針の理論的裏付けと、メモリ制御策の検証に活用 |
| Parser | Mascarenhas et al. "LPegLabel: PEGs with Labeled Failures" (2016 / SBLP) | ラベル付き失敗と回復の PEG 拡張 | `label/cut/recover` の挙動や FixIt 設計を比較し、期待集合の調整や同期戦略に反映 |
| Parser | Might, Darais, Spiewak "Parsing with Derivatives" (2011 / JFP) | パーサ微分による停止保証と遅延評価 | `many` 空成功検出、ストリーミング入力向けの代替戦略として検討 |
| Parser | Tree-sitter 0.20 リリースノート (2023) | インクリメンタル GLR とエラー復旧の実装 | `SpanTrace`・IDE 追跡の改善、部分再解析戦略の実証例 |
| 型推論 | Dunfield, Krishnaswami "Complete and Easy Bidirectional Typechecking" (2013 / ICFP) | 双方向型付けでエラー粒度を改善 | 公開 API の型注釈推奨 (`1-2`)、エラー位置の精密化に適用可能 |
| 型推論 | Matsakis 他 "Trait Solving in Rust" (2022-2023 / Rust 専門資料) | 新 Solver とコヒーレンス管理 | Reml のトレイト解決・孤児規則 (`1-2`) におけるアルゴリズム選定、インクリメンタル解決の検討材料 |
| 型推論 | Serrano et al. "A Principled Approach to OCaml Type Errors" (2020 / JFP) | 型エラーの期待/実際提示を体系化 | `ParseError.expected` を型エラー診断へ拡張する際の UI/文面参考 |
| 効果 | Leijen "Koka: Programming with Row-Polymorphic Effect Types" (2022 / POPL) | 行多相効果システムと段階的導入 | Reml の今後の効果拡張案 (`1-3`) を具体化する際の設計テンプレート |
| 効果 | Dolan et al. "Effect Handlers in OCaml" (2020 / POPL) & Multicore OCaml ホワイトペーパー (2023) | effect handler と安全境界の実装 | `unsafe` や `defer` のランタイム保証、将来の async/handler 拡張検討に活用 |
| Unicode | Unicode Standard 15.1 (2023), UAX #29/#39 (2023), ICU 74 (2023) | 最新のグラフェム境界・confusable 検査・正規化仕様 | `graphemes()`, confusable 警告、`display_width` のテストデータ更新基準 |
| Unicode | WHATWG Encoding Standard (Living, 2023) | Web 向け文字エンコーディング規範 | ストリーミング入力時のエンコーディング検証や CLI 連携に反映 |
| 設定 DSL | Dhall Language 1.41.1 (2023), CUE 0.5 (2023) | 型付き設定言語の差分適用・検証モデル | `schema` の `compute` / `requires` 設計、差分適用と監査線形化 (`1-1`, `0-1`) の実証例 |
| プラグイン | Nix Flakes RFC (2022), Bazel Starlark Design (2021) | エコシステム拡張とロックファイル | `@plugin` メタデータと `PluginLock` 生成 (`1-1`) の整合性検証に活用 |
| IDE/診断 | GHC 9.6 typed holes 改善 (2023), Rust `clippy` must_use 拡張 (2022) | 修正提案付き診断の実践例 | `FixIt`, `@must_use` 警告 (`2-5`) の文面・LSP 出力改善に反映 |
| IDE/診断 | VSCode LSP 3.17 (2022) 仕様 | `DiagnosticTag` 等のメタ情報 | `SpanTrace` と JSON 診断の LSP マッピングに直接利用 |

## 4. 推奨アクション
1. 各研究・仕様を精読し、該当節に参考文献リンクと要約を追記（特に `cut/recover`, 効果契約, Unicode API）。
2. Tree-sitter・Dhall・Koka など公開実装を試用し、Reml 仕様とのギャップや追加サンプルを収集。
3. Unicode 15.1 / ICU 74 ベースのテストデータを整備し、`grapheme_len`, `confusable` 検査の検証ケースを追加。
4. IDE/LSP 連携仕様のドラフトを作成し、`SpanTrace`・FixIt の JSON 出力スキーマを確定。

---
本ノートは Reml 仕様書更新および関連ガイドの改訂時に参照し、設計根拠と最新知見のトレーサビリティを確保することを目的とする。
