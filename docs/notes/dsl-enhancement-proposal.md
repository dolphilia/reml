# Reml DSL 機能強化提案

## 1. はじめに

Reml は "Readable & Expressive Meta Language" を掲げ、DSL ファーストアプローチを核としています。現状の仕様（Phase 2-5時点）は、パーサーコンビネーターの強力なコア（`Core.Parse`）、詳細な診断システム（`Core.Diagnostics`）、そして安全性を担保する効果システム（Effects & Capability）を備えており、言語としての基盤は非常に堅牢です。

しかし、「プログラミング言語を作るための言語」としてさらに体験を向上させるために、以下の4つの領域で強化を提案します。これらは特に「DSL開発の立ち上がり（Prototyping）」から「実用化（Production）」への移行をスムーズにすることを目的としています。

## 1.1 調査結果の要約（既存仕様との照合）

- **Core.Test は既に仕様化済み**: `docs/spec/3-11-core-test.md` でテーブル駆動テストやスナップショットが定義されている。
- **Core.Lsp は最小 API を定義済み**: `docs/spec/3-14-core-lsp.md` と `docs/guides/lsp-authoring.md` により、DSL 作者が LSP を構築するための土台はある。
- **フォーマッタ基盤はあるが CST は未定義**: `docs/spec/3-13-core-text-pretty.md` と `docs/spec/5-3-developer-toolchain.md` がフォーマッタの基盤を示す一方、ユーザー DSL の CST/Trivia 保持は仕様化されていない。
- **Lite 体験の入口はテンプレート前提**: `docs/spec/5-1-package-manager-cli.md` / `docs/spec/5-4-community-content.md` が `reml new --template` を規定するが、「Lite プロファイル」の仕様は未定義。

## 2. 提案内容

### 2.1 DSL Test Kit (`Core.Test.Dsl`) の標準化

**現状:**
`Core.Test` は `docs/spec/3-11-core-test.md` で仕様化済みであり、テーブル駆動テスト (`table_test`) やスナップショット (`assert_snapshot`) を提供します。`docs/guides/testing.md` でも DSL のゴールデン運用が示されています。

**課題:**
パーサーのテストは「入力文字列」と「期待されるAST（またはエラー）」のペアを大量に検証する必要があります。手書きのユニットテストでは記述量が増え、網羅的なテスト（特にエラーケースや境界値）がおろそかになりがちです。

**提案:**
`Core.Test` の拡張レイヤとして、パーサー専用ヘルパをまとめた `Core.Test.Dsl`（または `Core.Test.Parser`）を追加します。`Core.Parse.Reply` と `Core.Diagnostics` の構造を前提にした簡易アサーションを提供し、`table_test` と連動できる形で DSL 特有のボイラープレートを吸収します。

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
*   **実装:** `Core.Parse` の `Reply` 構造と `Diagnostic` を検証する専用アサーションを提供し、`Core.Test` の `table_test`/`assert_snapshot` を内部利用します。

### 2.2 Auto-LSP Derivation (`Core.Lsp.Derive`)

**現状:**
`Core.Lsp` は `docs/spec/3-14-core-lsp.md` で JSON-RPC と診断ブリッジを規定していますが、`Core.Parse` の構造から LSP 情報を自動導出する仕組みは定義されていません。ガイド (`docs/guides/lsp-authoring.md`) も最小例に留まっています。

**課題:**
DSL を作った直後から、その DSL 用のエディタ支援（補完、ハイライト、ホバー）が欲しいという需要は高いですが、LSP の実装はハードルが高いです。

**提案:**
`conductor` 定義と `Parser` コンビネーターの構造から、LSP 機能（の一部）を自動導出する仕組みを導入します。

*   **Keyword Completion:** `symbol("if")` や `keyword("function")` などのコンビネーターから、自動的に補完候補リストを生成します。
*   **Structure Outline:** `rule("func_def", ...)` のような名前付きルールから、アウトライン（シンボルツリー）を自動生成します。
*   **Hover Docs:** `Parser` に付与されたドキュメントコメントを、DSL 利用時のホバー情報として露出させます。
*   **Streaming 連携:** `expected_tokens`（`docs/spec/2-7-core-parse-streaming.md`）から補完候補を拡張する導出経路を追加します。

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
*   **Pretty 連携:** `Core.Text.Pretty`（`docs/spec/3-13-core-text-pretty.md`）と組み合わせ、DSL フォーマッタが CST から `Doc` を生成できる API の標準化が必要です。

これにより、DSL 作者は「AST用パーサー」を書くだけで、自動的に「フォーマッタ用パーサー」も手に入れることができます。

### 2.4 "Reml Lite" プロファイル

**現状:**
Reml の仕様は非常に高機能（監査、セキュリティ、効果システム、Capability Registry）ですが、単純な設定ファイルパーサーや小規模な DSL を作りたいユーザーにとっては、学習コストと記述コストが高い（Overkill）可能性があります。

**課題:**
「ちょっとした設定ファイルをパースしたいだけ」というユーザーが、`AuditEnvelope` や `EffectTag` の詳細を理解しないと書き始められない場合、採用の障壁となります。

**提案:**
学習・導入のハードルを下げるための "Lite" プロファイル（またはガイドライン）を整備します。

*   **デフォルト設定の隠蔽:** 監査やセキュリティポリシーを意識しなくて済むデフォルト設定（`AuditPolicy::None`, `SecurityPolicy::Permissive`）を用意し、定型文（Boilerplate）を削減します。
*   **簡易テンプレート:** `reml new --template lite` のように、最小構成のプロジェクトテンプレートを提供します（`docs/spec/5-1-package-manager-cli.md` のテンプレート機構に追加）。
*   **安全性の明示:** 0-1 §1.2 の安全性方針に抵触しないよう、Lite は「学習/試作」向けであることと、監査/Capability を段階的に有効化する導線を併記します。

### 2.5 DSL Lifecycle & Observability (Phase 4.1+)

**現状:**
`RunConfig.trace` による実行追跡や `Core.Diagnostics` による診断出力は備わっていますが、DSL の「実行性能」や「複雑な不一致の原因」を開発者が視覚的に把握する手段が不足しています。

**提案:**
DSL のライフサイクル全体を支援するための「観測性」と「可搬性」の強化を追加します。

#### 1. Interactive Parser Explorer (`Core.Parse.Visualizer`)
*   **内容**: `RunConfig.trace` が出力する詳細なトレースログ（どのルールがどこで試行され、成功/失敗したか）を可視化するツール、または LSP 経由のインタラクティブ・デバッガ。
*   **メリット**: 巨大なバックトラックや、予期せぬルールでの停滞（Performance Leak）を視覚的に特定できます。

#### 2. DSL Performance Metrics (`Core.Benchmark.Dsl`)
*   **内容**: `0-1 §1.1` の性能基準（10MB/s, 低メモリ消費）を DSL 単位で自動計測し、回帰を検知する標準テストスイート、および `collect-iterator-audit-metrics.py` との連携。
*   **メリット**: 「実用に耐える性能」を客観的に保証し続けることができます。

#### 3. Portable DSL Runtime (WASM / Zero-dep C)
*   **内容**: Reml で記述した DSL パーサーを、ランタイムライブラリなし（または極小のランタイム付き）で WebAssembly や C 言語から呼び出し可能な形式でエクスポートする機能。
*   **メリット**: 「プログラミング言語を作るための言語」として、作成した DSL をブラウザや他言語のエコシステムへ手軽に持ち出せるようになります。

#### 4. Effect-Aware DSL Contracts
*   **内容**: DSL 内で使用可能な「効果（Effects）」を制限し、Capability Registry と連動させる仕組み。
*   **メリット**: 「この DSL はファイル IO を行わない」といった安全性の契約をコンパイル時に保証し、サンドボックス実行を容易にします。

## 3. 仕様反映の整理（更新）

| 項目 | 既存仕様の有無 | ギャップ | 本提案での扱い |
| --- | --- | --- | --- |
| DSL テスト | `Core.Test`（3-11） | パーサー専用アサーションがない | `Core.Test.Dsl` として拡張 |
| LSP 自動導出 | `Core.Lsp`（3-14） | `Core.Parse` からの導出が未定義 | `Core.Lsp.Derive` 追加 |
| CST/Lossless | `ConfigTriviaProfile` あり | Trivia 保持の共通規約/CST が未定義 | `Core.Parse` 拡張（CST モード） |
| Lite プロファイル | `reml new` テンプレートのみ | 最小構成の言語プロファイルが未定義 | 新テンプレート/ガイドを追加 |
| 可視化・計測 | `RunConfig.trace` のみ | 標準の分析ツール/ベンチャーマークがない | `Core.Parse.Visualizer` / `Core.Benchmark` |
| ポータビリティ | `Core.Ffi` はある | DSL 単位での独立出力（WASM等）が未定義 | `reml build --target wasm` 等の整備 |

## 4. まとめ

Reml の強力な基盤の上に、これらの「開発者体験（DX）」と「実用的な運用性（Ops）」を向上させるレイヤーを追加することで、Reml は真に "DSL-First" な言語となります。

1.  **書く (Write):** `Core.Parse` + "Lite" Profile で手軽に開始。
2.  **試す (Test):** `Core.Test.Dsl` で即座に検証。
3.  **使う (Use):** `Core.Lsp.Derive` でエディタ支援、WASM 出力で配布。
4.  **守る (Safe):** Effect-Aware な契約で安全なサンドボックス実行。
5.  **磨く (Polish):** CST サポートでのフォーマッタ提供と、計測ツールによる性能向上。

このサイクルを Reml エコシステム内で完結させることで、言語開発の民主化を加速できると考えます。
