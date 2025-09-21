# Reml 関連研究調査ノート

> 2024-2025 年の公開研究・OSS 動向を踏まえ、Reml 仕様の根拠と改良アイデアを整理した作業メモです。既存仕様の参照箇所は `*.md` のファイル名で示しています。

## 1. Reml が目指す方向性と中核特徴
- **設計ゴール**: 実用性能・短く書ける宣言的構文・高可読性・説明的エラー・Unicode 前提の文字モデル（`0-1-overview.md:7-157`）
- **横断テーマ**: 型安全な設定 DSL／IDE・LSP 連携／プラグイン拡張によるエコシステム（`0-1-overview.md:19-27`, `1-1-syntax.md:200-253`）
- **言語機能**: パイプライン、名前付き引数、ADT/パターンマッチ、Traits + HM 推論（`0-1-overview.md:33-168`, `1-2-types-Inference.md:8-200`）
- **効果安全**: 属性ベースの効果契約、mut/io/ffi/panic/unsafe 分類、`?` による Result/Option 伝播（`1-3-effects-safety.md:8-200`）
- **Unicode モデル**: Byte/Char/Grapheme の三層と NFC 正規化、安全な境界 API（`1-4-test-unicode-model.md:8-173`）
- **パーサ基盤**: `Parser<T>` + `Reply{consumed, committed}`、Packrat/左再帰/Streaming 切替、`cut/label/recover/trace` による診断（`2-1-parser-type.md:11-188`, `2-2-core-combinator.md:9-198`, `2-5-error.md:8-195`）

## 2. Reml を支える主要技術領域
- **パーサーコンビネーター & 実行戦略**: Packrat 最適化、左再帰処理、LL(*)、インクリメンタル実行、ストリーミング入力
- **エラー報告 & 回復**: 期待集合の縮約、FixIt、自動修正候補、エラーノードの扱い
- **型推論 & Traits**: Hindley–Milner + 制約収集、コヒーレンス、双方向型付け、将来的なサブタイピング
- **効果システム**: 属性契約と値制限、今後の行多相効果への拡張余地
- **Unicode/テキスト**: グラフェム境界、confusable 検査、表示幅、セキュリティガード
- **設定 DSL / プラグイン**: `schema` DSL、差分適用と監査線、Capability & バージョン管理
- **IDE 連携**: `SpanTrace`、JSON 診断、LSP との情報整合

## 3. 最新研究・OSS 動向サマリ（2022-2025）

| 分野 | 出典 (年 / 媒体) | 概要 | Reml への示唆 |
| ---- | ---------------- | ---- | -------------- |
| Parser基盤 | Warth, Douglass, Millstein “Packrat Parsers Can Support Left Recursion” (2008 / PLDI) | Packrat でも左再帰を扱う seed-growing 手法 | `left_recursion` 実装の理論的裏付けとメモリ制御方針の検証材料 |
| Parser基盤 | Adams “Pika parsing: reformulating packrat parsing as a dynamic programming algorithm” (2020) | 逆向き DP による Packrat 最適化・左再帰統合 | ハイブリッド実行 (`2-6-execution-strategy.md`) の長期的選択肢。メモリ使用の上限制御に応用可 |
| エラー回復 | Mascarenhas et al. “LPegLabel: PEGs with Labeled Failures” (2016 / SBLP) | ラベル付き失敗と回復の PEG 拡張 | `label/cut/recover` の期待集合統合・FixIt 提案の見直しに利用 |
| エラー回復 | Rust 製パーサライブラリ Chumsky (2023-2024) | エラーノード挿入と「エラー不可能」方針を実装 | `ParseError` を `(AST, Diagnostics)` に分離し、`Error` ノードを標準 AST へ組み込む検討指針 |
| エラー回復 | Elm / Lezer / Tree-sitter コミュニティの “error impossible” 論考 (2022-) | 失敗時も部分 AST を生成し続ける設計 | Reml の IDE 連携 (`2-5-error.md`) で補完・書き換えを継続させる際の根拠 |
| インクリメンタル解析 | Tree-sitter 0.20 (2023) / 0.21 (2024予定) | GLR ベース増分パーシングとエラー回復 | `SpanTrace` や Streaming 実行から IDE 再解析を行う際の指針。差分適用 API の雛形 |
| インクリメンタル解析 | Salsa 0.17 (2024) | クエリベースのインクリメンタル計算、early cutoff / durability | Packrat メモ表や型検査結果を再利用する仕組みの設計参考。`RunConfig` 拡張案に反映 |
| 型システム | Dunfield, Krishnaswami “Complete and Easy Bidirectional Typechecking” (2013 / ICFP) | 双方向型付けで注釈・エラーの粒度向上 | 公開 API への注釈推奨 (`1-2-types-Inference.md`) と IDE 補助の根拠 |
| 型システム | Serrano et al. “A Principled Approach to OCaml Type Errors” (2020 / JFP) | 期待/実際・候補提示を整理した型エラー改善 | `ParseError.expected` を型診断に拡張する際のメッセージ設計参考 |
| 型システム | “The Simple Essence of Algebraic Subtyping” (2020 / ICFP) / Simple-sub 実装報告 (2023 RustConf) | ML 系での簡潔なサブタイピング導入 | Reml に段階的サブタイピング (整数昇格やレコード幅) を追加するための出発点 |
| 効果システム | Leijen “Koka: Programming with Row-Polymorphic Effect Types” (2022 / POPL) | 行多相効果と段階的導入戦略 | `@pure` を超えた効果トラッキング拡張の設計検討に利用 |
| 効果システム | Dolan et al. “Effect Handlers in OCaml” (2020 / POPL) / Multicore OCaml Notes (2023) | effect handler と安全境界 | `unsafe`/FFI 区画の整理と将来の async/handler 機能の足場 |
| Unicode | Unicode Standard 15.1 (2023), draft 15.1.1 (2024 Q4), ICU 74 (2023) + ICU 75 roadmap | 最新グラフェム境界・confusable 検査・ICU4X API の更新計画 | `graphemes()`・`display_width`・confusable 警告を 2024-2025 の仕様に追随。ICU4X/ugrapheme を組込候補に |
| Unicode | Rust `ugrapheme` crate (2024) | UAX #29 準拠のナノ秒レベル grapheme 反復 | Streaming 入力時の高速境界判定と `Lex` コンビネータの性能改善に活用 |
| 設定 DSL | Dhall 1.41 (2023), CUE 0.5 (2023) | 型付き設定言語による差分検証とポリシー管理 | `schema` の `compute`/`requires`、監査用 fingerprint (`0-1-overview.md`, `1-1-syntax.md`) の具体化 |
| プラグイン運用 | Nix Flakes RFC 122 (2022), Bazel Starlark design updates (2021-2024) | ロックファイルと capability 宣言の運用 | `@plugin` メタデータと `PluginLock` 出力の整合性検証に利用 |
| IDE & 診断 | GHC 9.6 typed holes (2023), Rust `clippy` must_use 拡張 (2022), VSCode LSP 3.17 (2022) | 修正提案と LSP Metadata の実装事例 | `FixIt`, `@must_use`, JSON 診断 (`2-5-error.md`) の表現と LSP 連携を具体化 |

*備考: 2025 年に向けた情報（Unicode 16 など）は現時点でドラフト段階のものを含む。確定版を採用する際はリリースノートで要確認。*

## 4. 改良提案の統合ビュー

| フェーズ | 主な改良テーマ | 内容 | 根拠資料 |
| -------- | -------------- | ---- | -------- |
| Phase 1 (短期: 0-3 ヶ月) | Unicode 処理刷新 | ICU/ICU4X, `ugrapheme` を評価し `grapheme_len`, `display_width`, confusable 検査をアップデート | Unicode 15.1, ICU roadmap, `ugrapheme` |
|  | エラー不可能指向 | パーサ結果を `(AST, Diagnostics)` に分離し、`Error` ノードを AST 型に追加。`ParseError` から FixIt/Notes を抽出 | Chumsky, LPegLabel, Tree-sitter recovery |
|  | 期待集合の人間語化 | `Expectation` に `Alternative`/`Context`/`UserFriendly` を導入し、IDE 向け文脈付きを生成 | `2-5-error.md`, LPG/Elm error 文献 |
| Phase 2 (中期: 3-12 ヶ月) | 型推論拡張 | 双方向型付けの部分導入、Simple-sub を参考に整数昇格・レコード幅によるサブタイプ関係を段階導入 | Dunfield & Krishnaswami, Simple-sub 報告 |
|  | インクリメンタル解析 | `RunConfig` に `incremental` オプション、差分適用 API、Salsa 互換メモ化を試作 | Tree-sitter, Salsa |
|  | IDE 連携強化 | `SpanTrace` の JSON 出力スキーマ定義、LSP 3.17 対応の DiagnosticTag/FixIt マッピング | VSCode LSP 仕様, `2-5-error.md` |
| Phase 3 (長期: 1-2 年) | 高度な Packrat | Pika parsing の DP 戦略や選択的メモ化で Packrat のメモリ/速度最適化を評価 | Pika parsing 論文 |
|  | インクリメンタル型検査 | Salsa 風クエリ DB で型検査結果を再利用し、hover/definition など IDE 機能を提供 | Salsa, rust-analyzer 設計資料 |
|  | スマート回復 | `smart_recover` など修復ポリシーを備えたコンビネータを実験導入 | Tree-sitter, IDE error recovery 研究 |

## 5. 推奨アクション
1. **文献追記**: 上記資料の要点と参照リンクを該当仕様書 (`2-5-error.md`, `2-6-execution-strategy.md`, `1-4-test-unicode-model.md` など) に追記し、設計根拠を明文化する。
2. **プロトタイプ実験**: チーム内で `ugrapheme` / Chumsky / Salsa を用いた小規模 PoC を作成し、性能・API 一貫性を評価する。
3. **テスト整備**: Unicode 15.1/ICU 74 のテストデータを収集し、`grapheme_len`, confusable 検査, Packrat 左再帰ケースの回帰テストを追加する。
4. **IDE スキーマ策定**: `SpanTrace` と `Diagnostic` の JSON スキーマ草案を用意し、LSP 3.17 の `diagnosticCodeDescription`, `data` フィールドと整合させる。
5. **優先度レビュー**: Phase 1 項目を仕様更新計画 (`spec-update-plan.md`) に組み込み、影響・工数を評価するミーティングを設定する。

## 6. リスクと留意事項
- **破壊的変更リスク**: エラー不可能指向の API 変更は既存利用者への影響が大きいため、互換モードや段階的移行策を検討する。
- **性能回帰**: Unicode 処理とインクリメンタル機構はメモリ/CPU を増加させる可能性がある。ベンチマークとプロファイリングの自動化が必要。
- **情報鮮度**: Unicode 16 や ICU 75 以降は現時点でドラフトが多いため、正式リリースを確認してから仕様へ反映する。

---
Reml 仕様更新や関連ガイド改訂時は本ノートを参照し、設計に対する根拠と最新動向のトレーサビリティを確保すること。
