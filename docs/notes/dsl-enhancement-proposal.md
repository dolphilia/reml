# Reml DSL 機能強化提案 (Rev. 2)

## 1. はじめに

Reml は "Readable & Expressive Meta Language" を掲げ、DSL ファーストアプローチを核としています。
本提案は、Reml を「プログラミング言語を作るための言語」として、プロトタイピングから実運用（Production）までの各フェーズにおける開発者体験（DX）を飛躍的に向上させるための機能強化案です。

Phase 2-5 および Phase 4.1 の計画（`4-1-core-parse-combinator-plan-v2`）で整備される基盤を前提とし、その上に積み上げるアプリケーション層の支援機能を定義します。

## 2. 現状と課題の要約

| 領域 | 現状 (As-Is) | 課題 (Issue) |
| --- | --- | --- |
| **テスト** | `Core.Test` でテーブルテスト/スナップショットは可能 | パーサー特有の「AST構造検証」や「エラー位置検証」の記述が冗長。 |
| **LSP** | `Core.Lsp` でプロトコルと診断ブリッジは定義済み | パーサー定義から補完やアウトラインを手動で実装する必要があり、コストが高い。 |
| **フォーマッタ** | `Core.Text.Pretty` あり。`autoWhitespace` 計画あり | AST から元の空白/コメント（Trivia）を復元する CST (Concrete Syntax Tree) の標準が未整備。 |
| **学習コスト** | 高機能だが初期設定が多い (`Audit`, `Effect` 等) | 「設定ファイルパーサー」程度の用途には Overkill で、初学者の参入障壁が高い。 |
| **デバッグ** | `RunConfig.trace` のテキストログのみ | バックトラックや性能ボトルネックを視覚的に把握する手段がない。 |

## 3. 提案内容

### 3.1 DSL Test Kit (`Core.Test.Dsl`)

パーサーのテストを宣言的かつ簡潔に記述するための DSL です。`Core.Test` の拡張として実装します。

**機能:**
*   **AST Matcher:** 構造体や列挙型を簡略記法で記述し、AST とパターンマッチさせる。`...` の部分一致や `List(...)` / `Record(...)` の構文を前提とする。
*   **Error Expectation:** エラーコード、発生位置（行:列）、メッセージの一部を簡潔に検証。`Diagnostic.codes` の別名と複数診断の優先順を含む。
*   **Golden File Support:** 入力ファイルと期待される AST/エラー出力をペアで管理するフローの標準化。`snapshot.updated` に `snapshot.mode` / `snapshot.bytes` を記録する。

```reml
use Core.Test.Dsl

test_parser(my_parser) {
  // 正常系: 簡略記法でのAST比較
  case "1 + 2" => Add(Int(1), Int(2))
  
  // 異常系: エラー位置とコードの検証
  case "1 + " => Error(code="parser.unexpected_eof", at=4)
  
  // 構造的部分一致（... で不要なフィールドを無視）
  case "fn main() {}" => Func(name="main", ...)
}
```

**進捗メモ（2025-12）:**
*   `test_parser { case ... }` の糖衣構文を Rust 側で復帰し、`DslCase` 展開を実装済み。
*   `AstMatcher` に `Pattern(...)` / `List(...)` / `Record(...)` を追加し、部分一致と順序/キー一致をサポート。
*   `ErrorExpectation` の `Diagnostic.codes` 対応、`LineCol` 判定、複数診断の優先順を実装済み。
*   ゴールデン経路は `examples/**/golden/{case_id}.input` と `expected/**/golden/{case_id}.ast|error` を読み込み、`snapshot.updated` を記録する。

### 3.2 Auto-LSP Derivation (`Core.Lsp.Derive`)

`Core.Parse` のコンビネーター定義から、LSP の機能を可能な限り自動導出します。

**機能:**
*   **Completion:** `symbol`, `keyword` コンビネーターからキーワード補完候補を生成。
*   **Outline:** `rule` コンビネーターの名前と階層構造から Document Symbol を生成。
*   **Semantic Tokens:** `token` 定義からシンタックスハイライト情報を導出。
*   **Hover:** コンビネーターに付与された Doc comment を Hover 情報として表示。

```reml
conductor my_dsl_server {
  serve my_parser
    // コンビネーター構造から補完、アウトライン、ハイライトを自動導出
    |> derive_standard_capabilities
}
```

### 3.3 CST Support & Lossless Parsing

DSL のフォーマッタやリファクタリングツールを作成するために、コメントや空白（Trivia）を保持した解析モードを提供します。
Phase 4.1 計画の `autoWhitespace` と連携し、スキップされた空白情報をノードに付着させます。

**機能:**
*   **CST Node:** `Node { kind, children, trivia_leading, trivia_trailing }` 形式の汎用木構造。
*   **Trivia Attachment:** `autoWhitespace` で消費されたトークンを、直近の AST ノードの Trivia として自動記録。
*   **Printer Derivation:** CST から `Core.Text.Pretty` の `Doc` を生成するデフォルトプリンタの導出。

### 3.4 "Reml Lite" プロファイルとテンプレート

小規模なツールや設定ファイルパーサー向けに、Reml の複雑さ（監査、効果システム）を隠蔽した開発体験を提供します。

**機能:**
*   **Lite Template:** `reml new --template lite` で、`project.stage = "lite"`、`AuditPolicy::None`、`SecurityPolicy::Permissive` が設定済みのプロジェクトを作成する。監査ログは省略するが `Diagnostic`/`AuditEnvelope` は生成する。
*   **既定値:** `dsl.lite.capabilities = []`、`dsl.lite.expect_effects = []`、`config.compatibility.json = json-relaxed` を既定とする。
*   **Prelude Lite:** よく使う機能（IO, List操作など）を `Result` のアンラップなしで使える（または `?` だけで済む）ようなエイリアスやヘルパーを提供（教育用）。
*   **Guide:** 「まずは動くもの」を作るための、厳密さを後回しにしたチュートリアル。

**移行導線（Lite → 標準）:**
1. `project.stage` を `beta` / `stable` に更新する。
2. `--audit-log <path>` を指定して監査ログを有効化する。
3. `dsl.lite.expect_effects` と `dsl.lite.capabilities` を実装に合わせて明示する。
4. `config.compatibility.json.profile` を `stable` へ戻し、`feature_guard` を整理する。

**関連計画と回帰資産**
- 計画書: `docs/plans/bootstrap-roadmap/4-1-dsl-lite-profile-plan.md`
- 回帰サンプル: `examples/practical/lite_template/` と `expected/lite_template/`
- Phase 4 シナリオ: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CH5-LITE-001`

### 3.5 Interactive Parser Visualizer (`Core.Parse.Visualizer`)

パース処理の挙動を視覚的に理解・デバッグするためのツールです。LSP の拡張機能、または Web ベースのツールとして提供します。

**機能:**
*   **Trace Visualization:** どのルールが成功/失敗し、どこでバックトラックが発生したかをツリー状に表示。
*   **Step-by-Step Debugging:** 入力ストリーム上のカーソル位置と、対応するパーサールールのスタックを連動表示。
*   **Performance Heatmap:** 処理時間がかかっているルールや、メモ化ミスの多い箇所をヒートマップ表示。

### 3.6 DSL Composability Standard

複数の DSL を組み合わせて使う（例: Reml 内に SQL を埋め込む、Markdown 内に Reml を埋め込む）際の標準パターンを確立します。
特に `conductor` を「DSL 協働の司令塔」と位置付け、解析・実行・診断・LSP を一貫した契約で束ねることを目的とします。
設計指針の補足は [1-1-syntax.md](../spec/1-1-syntax.md) の B.8.3 を参照してください。
ストリーミング時のバックプレッシャ協調は [core-parse-streaming.md](../guides/core-parse-streaming.md) の 4.1 を参照してください。

**機能:**
*   **Embedded Parser Interface:** 異なる DSL パーサーを「サブパーサー」として呼び出す共通インターフェース（境界トークン、復帰位置、エラー回復を含む）。
*   **Context Passing:** 親 DSL から子 DSL へ変数スコープ、型環境、設定、Capability 要求を引き継ぐ仕組み。
*   **Conductor Orchestration:** `conductor` のパイプライン定義内で、DSL 間の依存・評価順序・実行ポリシーを宣言可能にする。
*   **LSP Delegation:** 埋め込み部分のカーソル位置で子 DSL の LSP へ委譲し、診断・補完・フォーマットを合成する。
*   **CST/Trivia Bridge:** 埋め込み DSL が CST を返す場合は Trivia を保持したまま親 DSL の CST へ統合する。
*   **Diagnostic Boundary:** どの DSL が発生源かを `Diagnostic.source_dsl` に明示し、監査ログにも DSL ID を付与する。

**並列処理・ストリーミングの考慮:**
*   **Parallel Parse Windows:** 埋め込み区間が独立している場合、サブパーサを並列実行できるよう `embedded_dsl` が並列安全性フラグを持つ。
*   **Backpressure Coordination:** ストリーミング入力（大規模ファイルやネットワーク入力）では、親子 DSL の `FlowController` を協調させ、メモリ上限と遅延制御を共有する。
*   **Speculative Parsing:** 子 DSL の候補が複数ある場合、最小コストの候補から評価し、失敗時の回復ルールを明確化する。

**Conductor 連携の要件:**
*   **DSL Contract:** `conductor` に登録した DSL は `dsl_id` を持ち、Capability と効果の契約を `with_capabilities` と連動させる。
*   **Execution Plan:** 並列・直列・優先度付きの実行戦略を `execution` ブロックで宣言し、`Core.Async` のスケジューラへ反映する。
*   **Resource Limits:** 各 DSL に `resource_limit`（メモリ、CPU、タイムアウト）を設定でき、逸脱時は `conductor.resource.limit_exceeded` を報告する。

```reml
// Markdown パーサーの中に Reml コードブロックパーサーを埋め込む例
let code_block =
  embedded_dsl(
    dsl_id = "reml",
    start = "```reml",
    end = "```",
    parser = Reml.Parser.main,
    lsp = Reml.Lsp.server,
    mode = EmbeddedMode::ParallelSafe,
    context = ContextBridge::inherit(["scope", "type_env"])
  )

conductor docs_pipeline {
  markdown: Markdown.Parser.main
    |> with_embedded([code_block])
    |> with_capabilities(["core.parse", "core.lsp"])

  execution {
    parallel markdown
  }
}
```

**追加で考慮すべき点:**
*   **Versioned Embedding:** 親 DSL が子 DSL のバージョンを宣言し、互換性の欠落はビルド時に診断する。
*   **Security Sandbox:** 子 DSL の実行が副作用を持つ場合は `conductor` の Capability 契約で制限し、監査ログへエビデンスを残す。
*   **Error Containment:** 子 DSL の失敗が親 DSL 全体を巻き込まないよう、回復モードとフォールバック規約を標準化する。

### 3.7 Error Recovery Combinators

「エラーがあってもパースを続行する」ための回復戦略を、高度な知識なしに実装できるコンビネーター群を提供します。

**機能:**
*   **Sync Points:** `sync_to(symbol(";"))` のように、エラー時にどこまで読み飛ばして復帰するかを指定するコンビネーター。
*   **Panic Mode Helpers:** ブロックの終わりまでスキップする等の定型パターン。
*   **Missing/Inserted Token:** 「ここに `)` があるはず」として仮想トークンを挿入し、パースを続行するリカバリ。

## 4. ロードマップとの整合

本提案の機能は、Bootstrap Roadmap の以下のフェーズと連携して進めます。

*   **Phase 4 (Migration/Extension):** `Core.Test.Dsl` を Rust 実装まで完了し、`Core.Lsp.Derive` はドラフトのまま継続検討。`Core.Parse` v2 (`autoWhitespace`) の完了を待って CST を検討。
*   **Phase 5 (Self-Host):** コンパイラ自身が Reml で書き直される際、これらの機能をドッグフーディングする。特に CST は自身のフォーマッタ実装に必須。
*   **Future:** Visualizer や Composability はエコシステムの成熟に合わせて展開。

## 5. 結論

これらの機能強化により、Reml は単なる「パーサーライブラリ」を超え、**「言語開発のための統合プラットフォーム（Language Workbench）」** へと進化します。
これにより、開発者は「言語仕様の設計」という本質的な課題に集中でき、高品質な DSL を短期間で実用化できるようになります。
