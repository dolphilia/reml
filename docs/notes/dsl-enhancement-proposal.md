# Reml DSL 機能強化提案

## 1. はじめに

Reml は "Readable & Expressive Meta Language" を掲げ、DSL ファーストアプローチを核としています。現状の仕様（Phase 2-5時点）は、パーサーコンビネーターの強力なコア（`Core.Parse`）、詳細な診断システム（`Core.Diagnostics`）、そして安全性を担保する効果システム（Effects & Capability）を備えており、言語としての基盤は非常に堅牢です。

しかし、「プログラミング言語を作るための言語」としてさらに体験を向上させるために、以下の4つの領域で強化を提案します。これらは特に「DSL開発の立ち上がり（Prototyping）」から「実用化（Production）」への移行をスムーズにすることを目的としています。

## 2. 提案内容

### 2.1 DSL Test Kit (`Core.Test.Dsl`) の標準化

**現状:**
`1-4-test-unicode-model.md` や `5-3-developer-toolchain.md` にテストランナーの記述はありますが、DSL（パーサー）特有のテスト手法については標準化されていません。現在は各開発者が `assert_eq` 等で手書きする必要があります。

**課題:**
パーサーのテストは「入力文字列」と「期待されるAST（またはエラー）」のペアを大量に検証する必要があります。手書きのユニットテストでは記述量が増え、網羅的なテスト（特にエラーケースや境界値）がおろそかになりがちです。

**提案:**
データ駆動型テストをサポートする `Core.Test.Dsl` モジュールを標準ライブラリに追加します。

```reml
// 使用イメージ
use Core.Test.Dsl

test_parser(my_parser) {
  // 正常系: 入力 -> 期待されるAST構造（簡易記法）
  case "1 + 2" => Add(Int(1), Int(2))
  
  // 異常系: 入力 -> 期待されるエラーコードと位置
  case "1 + " => Error(code="parser.unexpected_eof", at=4)
  
  // ゴールデンテスト: ファイルベースの比較
  golden_case("tests/fixtures/valid/*.input", "tests/fixtures/valid/*.ast")
}
```

*   **メリット:** テスト記述のコストを大幅に下げ、DSL の品質を早期に担保できます。
*   **実装:** `Core.Parse` の `Reply` 構造と `Diagnostic` を検証する専用アサーションを提供します。

### 2.2 Auto-LSP Derivation (`Core.Lsp.Derive`)

**現状:**
`5-3-developer-toolchain.md` にて LSP のサポートが明記されていますが、DSL 開発者が自前で `RemlLanguageServer` と連携するロジックを書く必要があるように見受けられます（`DslExportSignature` の提供など）。

**課題:**
DSL を作った直後から、その DSL 用のエディタ支援（補完、ハイライト、ホバー）が欲しいという需要は高いですが、LSP の実装はハードルが高いです。

**提案:**
`conductor` 定義と `Parser` コンビネーターの構造から、LSP 機能（の一部）を自動導出する仕組みを導入します。

*   **Keyword Completion:** `symbol("if")` や `keyword("function")` などのコンビネーターから、自動的に補完候補リストを生成します。
*   **Structure Outline:** `rule("func_def", ...)` のような名前付きルールから、アウトライン（シンボルツリー）を自動生成します。
*   **Hover Docs:** `Parser` に付与されたドキュメントコメントを、DSL 利用時のホバー情報として露出させます。

```reml
// conductor 内での宣言イメージ
conductor my_dsl_server {
  serve my_parser
    |> derive_completion    // コンビネーターから補完を推論
    |> derive_outline       // rule名からアウトラインを生成
    |> derive_semantic_tokens
}
```

### 2.3 Concrete Syntax Tree (CST) / Lossless Parsing のサポート

**現状:**
`Core.Parse` は AST（抽象構文木）の構築に主眼が置かれています。`Core.Format` は言語仕様の構文木を利用しますが、ユーザー定義 DSL のフォーマッタを作るための標準的な仕組み（Trivia の保持など）が明示されていません。

**課題:**
DSL が普及すると、必ず「フォーマッタ」や「リファクタリングツール」が欲しくなります。これらを作るには、空白やコメント（Trivia）を保持した「完全な構文木（CST）」が必要です。

**提案:**
`Core.Parse` に CST 構築モード（または Green Tree 生成機能）を追加します。

*   **Lossless Mode:** パーサー定義を変更せずに、AST の代わりに CST（`Node { kind, children, trivia }`）を生成するモード。
*   **Trivia Attachment:** `with_space` で指定された空白パーサーが消費した内容を、自動的に直後のトークンノードに Trivia として付着させます。

これにより、DSL 作者は「AST用パーサー」を書くだけで、自動的に「フォーマッタ用パーサー」も手に入れることができます。

### 2.4 "Reml Lite" プロファイル

**現状:**
Reml の仕様は非常に高機能（監査、セキュリティ、効果システム、Capability Registry）ですが、単純な設定ファイルパーサーや小規模な DSL を作りたいユーザーにとっては、学習コストと記述コストが高い（Overkill）可能性があります。

**課題:**
「ちょっとした設定ファイルをパースしたいだけ」というユーザーが、`AuditEnvelope` や `EffectTag` の詳細を理解しないと書き始められない場合、採用の障壁となります。

**提案:**
学習・導入のハードルを下げるための "Lite" プロファイル（またはガイドライン）を整備します。

*   **デフォルト設定の隠蔽:** 監査やセキュリティポリシーを意識しなくて済むデフォルト設定（`AuditPolicy::None`, `SecurityPolicy::Permissive`）を用意し、定型文（Boilerplate）を削減します。
*   **簡易テンプレート:** `reml new --template lite` のように、最小構成のプロジェクトテンプレートを提供します。

## 3. まとめ

Reml の強力な基盤の上に、これらの「開発者体験（DX）」を向上させるレイヤーを追加することで、Reml は真に "DSL-First" な言語となります。

1.  **書く (Write):** `Core.Parse` + "Lite" Profile で手軽に開始。
2.  **試す (Test):** `Core.Test.Dsl` で即座に検証。
3.  **使う (Use):** `Core.Lsp.Derive` でエディタ支援を自動生成。
4.  **整える (Polish):** CST サポートでフォーマッタを提供。

このサイクルを Reml エコシステム内で完結させることで、言語開発の民主化を加速できると考えます。
