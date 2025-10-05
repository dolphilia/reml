# 3.1 Reml Parser 再実装計画

## 目的
- Phase 3 マイルストーン M1 を達成するため、OCaml 実装の Parser を Reml で再実装し、Core.Parse API (`guides/core-parse-streaming.md`) を備えたセルフホスト用フロントエンドを構築する。
- ストリーミング API とバッチ API を両立させ、プラグイン・DSL が利用できる拡張ポイントを整える。

## スコープ
- **含む**: Core.Parse モジュール実装、ストリーミング制御 (`run_stream`, `FlowController`)、AST 生成、Span 保持、CLI 連携。
- **含まない**: DSL 専用拡張、ユーザー定義演算子の動的登録（必要であれば後続タスクに委譲）。
- **前提**: Phase 2 までに確定した構文・診断仕様が存在し、OCaml 版で参照できる状態であること。

## 作業ブレークダウン

### 1. Core.Parse 型システム設計（35-36週目）
**担当領域**: 基本型とAPI設計

1.1. **Parser型の定義**
- `2-1-parser-type.md` の `Parser<T>` 型を Reml の代数的データ型で実装
- 入力モデル: `Input<L>` (Byte/Char/Grapheme レイヤ対応)
- 状態型: `State { input: Input, pos: usize, consumed: bool, committed: bool }`
- 結果型: `Result<(T, State), ParseError>`

1.2. **入力抽象化の実装**
- `1-4-test-unicode-model.md` に基づく 3 層入力モデル (Byte/Char/Grapheme)
- ストリーミング用の `InputSource` trait 設計
- バッファ管理とバックトラック用の位置追跡機構
- UTF-8 検証とエラーハンドリング

1.3. **基本コンビネーター定義**
- `2-2-core-combinator.md` の 12-15 コンビネーターを Reml で実装
- `map`, `then`, `or`, `many`, `some`, `optional` 等
- モナディックインタフェース (`>>=`, `>>`, `<|>`) の提供
- 型安全性の保証とゼロコスト抽象化

**成果物**: `Core.Parse.Types`, `Core.Parse.Combinators` モジュール、型定義テスト

### 2. ストリーミング実行エンジン（36-37週目）
**担当領域**: 実行制御とフロー管理

2.1. **run_stream API 実装**
- `guides/core-parse-streaming.md` §3.1 の仕様に準拠
- `FlowController` の状態遷移: `NeedInput → Running → Paused → Done`
- バックプレッシャ制御とバッファ管理
- 部分入力の処理とエラー伝播

2.2. **consumed/committed セマンティクス**
- consumed フラグによるバックトラック制御
- committed フラグによるエラー回復戦略
- `2-5-error.md` の cut/label/recover との統合
- メモリ効率的な状態管理

2.3. **バッチ実行との互換**
- `run_batch` APIの実装（全入力一括処理）
- ストリーミング/バッチの切り替え機構
- 性能特性の測定（ストリーム vs バッチ）
- CLI フラグ (`--parse-mode=stream|batch`) の追加

**成果物**: `Core.Parse.Stream` モジュール、実行エンジンテスト、性能ベンチマーク

### 3. Lexer/Parser コンビネーター移植（37-38週目）
**担当領域**: 構文解析実装

3.1. **字句解析レイヤ**
- `2-3-lexer.md` の字句ユーティリティを Reml で再実装
- 空白処理 (`whitespace`, `lexeme`) とコメント (`line_comment`, `block_comment`)
- リテラル解析 (`int_literal`, `float_literal`, `string_literal`)
- 識別子とキーワード (`identifier`, `keyword`, `reserved`)

3.2. **演算子優先順位パーサ**
- `2-4-op-builder.md` の `OpBuilder` を Reml で実装
- 固定演算子テーブルの移植（Phase 1 の優先順位定義）
- left/right/nonassoc の結合規則処理
- 演算子式の AST 生成

3.3. **式・宣言パーサ**
- OCaml 版の構文定義を Reml コンビネーターに変換
- 式パーサ: リテラル、関数適用、ラムダ、パイプ、match
- 宣言パーサ: let/var/fn/type/use
- パターンパーサ: 変数、タプル、レコード、ワイルドカード

**成果物**: `Core.Parse.Lexer`, `Core.Parse.Expr`, `Core.Parse.Decl` モジュール、構文テスト

### 4. AST 構築と Span 統合（38-39週目）
**担当領域**: メタデータ管理

4.1. **AST データ構造の移植**
- Phase 1 の AST 定義 (`Expr`, `Decl`, `Pattern`, `Type`) を Reml で再実装
- 各ノードへの `Span { start: Pos, end: Pos, file: SourceId }` 付与
- ファイル管理: `SourceMap` による複数ファイル対応
- ネストした式での位置情報の正確な計算

4.2. **Span 追跡機構**
- パーサコンビネーター内での自動 Span 付与
- `with_span` コンビネーターの実装
- ソースコード参照の保持（診断用）
- デバッグ情報の埋め込み

4.3. **AST 比較と互換性**
- OCaml 版との AST 構造比較
- シリアライズ形式の定義 (JSON/Bincode)
- ゴールデンテスト用のスナップショット生成
- 差分レポート生成ツール

**成果物**: `Core.Parse.AST` モジュール、Span 統合、AST 比較ツール

### 5. エラーハンドリングと診断（39-40週目）
**担当領域**: 診断システム統合

5.1. **期待値収集**
- `2-5-error.md` の期待値集合 (`Expected`) の実装
- パーサ失敗時の期待トークン自動収集
- 優先度付き期待値のマージ
- コンテキスト依存の提案生成

5.2. **cut/label/recover 統合**
- `cut` による committed 状態の制御
- `label` による期待値のカスタマイズ
- `recover` による同期ポイントの定義
- エラー回復戦略の実装

5.3. **診断メッセージ生成**
- `3-6-core-diagnostics-audit.md` の Diagnostic 形式への変換
- Span 情報を使った正確な位置表示
- ソースコードスニペットの抽出
- OCaml 版との診断整合テスト

**成果物**: `Core.Parse.Error` モジュール、診断生成、整合テスト

### 6. プラグインフックと拡張ポイント（40-41週目）
**担当領域**: 拡張性設計

6.1. **DSL フック設計**
- `guides/DSL-plugin.md` を参照した拡張ポイントの定義
- カスタム構文登録のインタフェース設計
- 演算子優先順位の動的拡張（将来対応の準備）
- プラグイン用の AST 注釈機構

6.2. **プラグイン API 仕様**
- `ParserPlugin` trait の定義
- 登録・解決・実行のライフサイクル
- 安全性制約（Capability Stage の適用）
- サンプルプラグインの実装

6.3. **ドキュメント整備**
- プラグイン API の使用例とガイド
- `notes/dsl-plugin-roadmap.md` への連携事項追記
- 制約事項と今後の拡張計画の明示
- Phase 4 以降での本格対応への引き継ぎ

**成果物**: `Core.Parse.Plugin` モジュール、API 仕様書、サンプルプラグイン

### 7. 並行稼働とテスト整備（41-42週目）
**担当領域**: 品質保証

7.1. **並行稼働インフラ**
- CLI フラグ `--frontend=ocaml|reml` の実装
- 両実装の AST 出力比較ツール
- CI での並行実行設定（GitHub Actions マトリクス）
- フォールバック機構（Reml 失敗時に OCaml へ）

7.2. **ゴールデンテスト拡充**
- `samples/language-impl-comparison/` のサンプル活用
- AST スナップショット比較
- 差分許容基準の定義（構造的等価性）
- 自動レポート生成

7.3. **性能計測**
- ストリーミング vs バッチの性能比較
- Phase 2 ベースライン ±10% の検証
- メモリ使用量プロファイリング
- `0-3-audit-and-metrics.md` への記録

**成果物**: 並行稼働 CLI、ゴールデンテスト、性能レポート

### 8. ドキュメント整備とレビュー準備（42週目）
**担当領域**: ドキュメント

8.1. **技術文書更新**
- `3-0-phase3-self-host.md` への実装詳細追記
- ストリーミング API の使用例追加
- OCaml 版との差異一覧の作成
- 移植ガイドの作成

8.2. **メトリクス記録**
- AST 比較結果の集計
- 性能・メモリ使用量の記録
- 診断整合率の計測
- `0-3-audit-and-metrics.md` への反映

8.3. **レビュー資料作成**
- M1 マイルストーン達成報告書
- AST/診断のサンプル出力
- 既知の制限事項と TODO リスト
- Phase 3 次タスク（3-2 TypeChecker）への引き継ぎ事項

**成果物**: 完全なドキュメント、メトリクス記録、レビュー資料

## 成果物と検証
- `Core.Parse.*` モジュール群が実装され、CI で `reml test Core.Parse` が通過すること。
- AST スナップショット比較で OCaml 版との差分が許容範囲内（構造的等価性 95% 以上）に収束し、差分理由が記録される。
- ストリーミングテストが CI で通過し、性能が Phase 2 のベースライン ±10% 以内であること。
- プラグイン API の設計メモが作成され、`notes/dsl-plugin-roadmap.md` に連携事項が登録される。

## リスクとフォローアップ
- Reml 実装の性能が低下した場合、OCaml 版をフォールバックとして維持し、`0-4-risk-handling.md` に最適化タスクを記録。
- ストリーミング API の設計が固定される前に DSL 要件が追加される場合、`notes/guides-to-spec-integration-plan.md` と再調整。
- 並行稼働期間中はテストを二重に走らせる必要があるため、CI 負荷増大に備える（キャッシュ戦略の最適化）。
- UTF-8 以外のエンコーディング対応が必要になった場合は Phase 4 以降に延期し、仕様更新を `1-4-test-unicode-model.md` に反映。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [guides/core-parse-streaming.md](../../guides/core-parse-streaming.md)
- [2-1-parser-type.md](../../2-1-parser-type.md)
- [2-2-core-combinator.md](../../2-2-core-combinator.md)
- [2-3-lexer.md](../../2-3-lexer.md)
- [2-4-op-builder.md](../../2-4-op-builder.md)
- [2-5-error.md](../../2-5-error.md)
- [1-4-test-unicode-model.md](../../1-4-test-unicode-model.md)
- [guides/DSL-plugin.md](../../guides/DSL-plugin.md)
- [notes/guides-to-spec-integration-plan.md](../../notes/guides-to-spec-integration-plan.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)
