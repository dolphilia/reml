# 第5章 調査メモ: 構文解析 (Parsing)

## 参照した資料
- `compiler/frontend/src/parser/mod.rs:64-469`（ParsedModule、ParserTraceEvent、ParserDriver、lex→parse→diagnostic の流れ）
- `compiler/frontend/src/parser/mod.rs:486-736`（parse_tokens、parse_result_from_module、期待トークン生成）
- `compiler/frontend/src/parser/mod.rs:2094-5791`（`module_parser` の構文定義と `module_item` の集約）
- `compiler/frontend/src/parser/mod.rs:5794-6255`（トップレベル prefix 解析、module/use トークン解釈）
- `compiler/frontend/src/parser/mod.rs:6908-7017`（StreamingRecoverController と診断集約）
- `compiler/frontend/src/parser/api.rs:20-280`（RunConfig / Input / State / Reply / ParseResult の API）
- `compiler/frontend/src/parser/ast.rs:5-200`（Module/ModuleBody/UseDecl など AST 主要型）
- `compiler/frontend/src/parser/streaming_runner.rs:1-165`（run_stream/resume のラッパー）
- `compiler/frontend/src/streaming/mod.rs:1-167`（Packrat/SpanTrace の状態と統計）
- `docs/spec/2-0-parser-api-overview.md`（パーサ API の全体像）
- `docs/spec/2-1-parser-type.md`（Parser/RunConfig/Reply の仕様）
- `docs/spec/2-2-core-combinator.md`（コンビネータの設計方針）
- `docs/spec/2-4-op-builder.md`（演算子ビルダー）
- `docs/spec/2-7-core-parse-streaming.md`（ストリーミング実行）

## 調査メモ

### 入口と責務
- `ParserDriver::parse_with_options` が字句解析から構文解析までのメイン経路。`lex_source_with_options` でトークン化し、`parse_tokens` で構文解析、診断収集、SpanTrace/Packrat を束ねた `ParsedModule` を返す。(`compiler/frontend/src/parser/mod.rs:318-424`)
- `ParserOptions::from_run_config` は RunConfig の packrat/trace/lex 設定をパーサ用に変換する。(`compiler/frontend/src/parser/mod.rs:260-313`)
- `ParsedModule` は AST と診断に加え、packrat 統計、span trace、stream flow 状態、trace events を保持して CLI/LSP へ渡す。(`compiler/frontend/src/parser/mod.rs:64-78`)

### パーサ API と実行設定
- `RunConfig` は `require_eof`、`packrat`、`left_recursion`、`trace`、`extensions` を保持し、`with_extension` で拡張設定を合成する。(`compiler/frontend/src/parser/api.rs:47-115`)
- `Input` は `source` と `offset` のみを持つ軽量ビューで、`advance` で新しいビューを作る。(`compiler/frontend/src/parser/api.rs:118-139`)
- `Reply<T>` が成功/失敗時の `consumed`/`committed` を表し、`ParseResult<T>` が CLI/LSP 返却用の結果（診断や packrat 統計込み）になる。(`compiler/frontend/src/parser/api.rs:210-279`)

### AST 構造
- 構文解析の出力は `parser/ast.rs` に定義された AST で、`Module` が `header`/`uses`/`decls`/`functions`/`exprs` をまとめる。(`compiler/frontend/src/parser/ast.rs:5-54`)
- `Module`/`UseDecl` には `render()` があり、AST を文字列へ戻す最小ユーティリティとして使える。(`compiler/frontend/src/parser/ast.rs:17-178`)
- `FixityKind` など演算子の結合性や構文要素が AST 上に残る。(`compiler/frontend/src/parser/ast.rs:181-200`)

### 文法定義とトップレベル前処理
- `module_parser` は Chumsky のコンビネータで文法を組み立て、`TokenKind` を入力として AST を構成する。識別子や文脈キーワードを `map_with_span` で Span 付き `Ident` に変換する。(`compiler/frontend/src/parser/mod.rs:2094-2155`)
- `parse_tokens` は `parse_top_level_prefix` で `module`/`use` 宣言をトークン列から事前抽出し、残りを `module_parser` に渡す。(`compiler/frontend/src/parser/mod.rs:486-539`, `compiler/frontend/src/parser/mod.rs:5794-5830`)
- `parse_module_header_tokens` / `parse_use_decl_tokens` が `module`/`use` を直接トークンから読み、トップレベル AST に差し込む。`module {` 形式は除外するなど、CST 的な前処理を含む。(`compiler/frontend/src/parser/mod.rs:6169-6255`)

### エラー処理と期待トークン
- lexer 由来の `FrontendError` は `error_to_diagnostic` で `FrontendDiagnostic` に変換し、`CODE_*` を付与する。(`compiler/frontend/src/parser/mod.rs:426-465`)
- Chumsky の `Simple` エラーは `format_simple_error` と `build_expected_summary` を通して `ExpectedTokensSummary` に集約される。(`compiler/frontend/src/parser/mod.rs:645-719`)
- `StreamingRecoverController` が streaming モードの診断をチェックポイントごとに集約し、過剰なエラーを抑制する。(`compiler/frontend/src/parser/mod.rs:6908-7017`)

### Streaming/Packrat と trace
- `StreamingStateConfig` は packrat と span trace の有効化、バジェットを管理する。(`compiler/frontend/src/streaming/mod.rs:30-47`)
- packrat は `PackratEntry`/`PackratCacheEntry` を収集し、`PackratStats`/`StreamMetrics` として parse 結果に反映される。(`compiler/frontend/src/streaming/mod.rs:50-159`)
- `StreamingRunner` は `run_stream`/`resume` API を提供し、`RunConfig.extensions["stream"]` の `chunk_size`/`resume_hint` を参照する。(`compiler/frontend/src/parser/streaming_runner.rs:1-165`)

### 仕様との照合メモ
- 対応 spec: `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-1-parser-type.md`, `docs/spec/2-2-core-combinator.md`, `docs/spec/2-4-op-builder.md`, `docs/spec/2-7-core-parse-streaming.md`。
- `Input` は仕様の `line/column` を持たず、`source` + `offset` の簡易ビューのみ。Span も `u32` のバイト範囲のみなので、仕様との差分がある。(`compiler/frontend/src/parser/api.rs:118-139`, `docs/spec/2-1-parser-type.md`)
- spec は `Parser<T>` の純粋 API を想定しているが、実装は Chumsky で token stream をパースしており、`State`/`Reply` は外部 API の型として整理中。(`compiler/frontend/src/parser/api.rs:152-223`, `docs/spec/2-1-parser-type.md`)
- spec で触れる CST/trivia 収集は現行実装では見当たらず、`AST` のみを構築している。(`docs/spec/2-0-parser-api-overview.md`)

### 未確認事項 / TODO
- `RunConfig.left_recursion` が Chumsky 文法でどの程度機能しているか、実装上の接続が見当たらないため調査が必要。
- `ParseResult.trace_events` の用途（CLI 出力や diagnostics との関連）を追跡する必要がある。(`compiler/frontend/src/parser/mod.rs:86-200`, `compiler/frontend/src/parser/api.rs:225-279`)
