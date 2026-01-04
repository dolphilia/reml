# 2.0 標準パーサーAPI 概要

## 概要
標準パーサーAPI章は `Core.Parse` ファミリーに含まれる型・コンビネーター・エラー・実行戦略を体系化し、Reml の言語仕様を実用的なパーサーとして具体化するための指針をまとめています。入力モデルからエラー報告、演算子ビルダーや実行最適化までを段階的に整理し、DSL 拡張やプラグインが安定した基盤上で構築できるようにします。

実装状況は Rust 版が 4.1 フェーズでバッチ用 `Parser<T>`/Packrat/期待集合生成まで実装済みであり、Lex プロファイル共有・Streaming/Plugin 連携は未着手です。

## セクションガイド
- [2.1 パーサ型と入力モデル](2-1-parser-type.md): `Parser<T>` と `Input` モデル、コミット/消費セマンティクス、`RunConfig` が担う実行時オプションを定義します。
- [2.2 コアコンビネーター](2-2-core-combinator.md): 直列・選択・繰り返し・先読みなど最小公理系のコンビネーターと、エラー品質を高めるための慣習を示します。`expect_keyword`/`expect_symbol` など派生 API による診断メッセージ統一方針も含みます。
- [2.3 字句レイヤユーティリティ](2-3-lexer.md): 空白・コメント処理や識別子・数値・文字列のトークン化、Unicode 安全性を備えた Lex ヘルパ群を整理します。`leading`/`trim` により `skipL`/`skipR` に依存しない空白処理の糖衣も追加されています。
- [2.4 演算子優先度ビルダー](2-4-op-builder.md): fixity 宣言による演算子テーブル構築、曖昧性解消、内部アルゴリズムとエラー整合性を解説します。
- [2.5 エラーハンドリング](2-5-error.md): 期待集合や `cut` の効果、翻訳可能な診断モデル、`recover` 戦略などエラー生成・表示ポリシーを定義し、`Core.Text.display_width` を利用して `ParseState` と `Diagnostic` の列情報を同期させる手順を整備します。
- [2.6 実行戦略](2-6-execution-strategy.md): トランポリンによる末尾最適化、Packrat/左再帰ガード、計測とターゲット適応を含むランナー設計をまとめます。
- [2.7 ストリーミング実行](2-7-core-parse-streaming.md): `run_stream`/`resume` API、継続メタデータ、バックプレッシャ制御、RunConfig 連携を定義し、バッチランナーと同等の診断品質を保つストリーミング処理の契約を示します。

## Phase 4〜10 の拡張ドラフト（脚注）
- `RunConfig.extensions["parse"].cst`（Phase 4）: CST/Trivia 収集の opt-in フラグ。既定は OFF とし、`run_with_cst` / `run_with_cst_shared` が `CstOutput` を返す際にのみ収集する。AST-only 経路は影響を受けない。
- `RunConfig.extensions["parse"].operator_table`（Phase 8）: `Core.Parse.expr_builder`/`OpBuilder` が利用する優先度・結合性テーブルを外部から注入するためのオプション。未指定時は各パーサーが埋め込む `levels` を採用し、既存のチェーン/ビルダー挙動は変わらない。
- `RunConfig.extensions["lex"].profile` / `layout_profile`（Phase 9）: autoWhitespace/Layout が空白・コメント・オフサイド仮想トークンを共有するための Lex ブリッジ。未設定時は簡易空白へフォールバックし構文意味は変えない。`phase4-scenario-matrix.csv` の `CH2-PARSE-901` でレイアウト共有経路を監視する。
- `RunConfig.profile` / `RunConfig.extensions["parse"].profile` / `profile_output`（Phase 10）: Packrat/バックトラック/回復の計測を opt-in で有効化し、`ParseResult.profile` に集計する観測フラグ。`profile_output` を与えると JSON を `reports/` 等へ書き出す（デフォルト OFF、安全側フォールバック）。`phase4-scenario-matrix.csv` の `CH2-PARSE-902` でベストエフォート書き出し経路を監視する。
