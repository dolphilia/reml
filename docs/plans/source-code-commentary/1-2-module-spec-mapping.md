# Remlソースコード完全解説: モジュールと仕様対応表

`compiler/frontend/src` と `compiler/runtime/src` の主要モジュールについて、
章の候補と対応する `docs/spec` 節を整理する。

## frontend

| モジュール | 章の候補 | 対応する spec 節 |
| --- | --- | --- |
| `compiler/frontend/src/lexer` | 第4章: 字句解析 | `docs/spec/2-3-lexer.md` |
| `compiler/frontend/src/token.rs` | 第4章: 字句解析（トークン設計） | `docs/spec/2-3-lexer.md` |
| `compiler/frontend/src/unicode.rs` | 第4章: 字句解析（Unicode） | `docs/spec/1-4-test-unicode-model.md` |
| `compiler/frontend/src/span.rs` | 第4章/第6章（Span・位置情報） | `docs/spec/1-4-test-unicode-model.md`, `docs/spec/2-5-error.md` |
| `compiler/frontend/src/parser` | 第5章: 構文解析 | `docs/spec/2-0-parser-api-overview.md`, `docs/spec/2-1-parser-type.md`, `docs/spec/2-2-core-combinator.md`, `docs/spec/2-4-op-builder.md` |
| `compiler/frontend/src/streaming` | 第9章: 実行パイプライン（ストリーミング） | `docs/spec/2-7-core-parse-streaming.md` |
| `compiler/frontend/src/pipeline` | 第9章: 実行パイプライン | `docs/spec/2-6-execution-strategy.md` |
| `compiler/frontend/src/diagnostic` | 第6章: 診断と出力 | `docs/spec/2-5-error.md` |
| `compiler/frontend/src/output` | 第6章: 診断と出力 | `docs/spec/2-5-error.md` |
| `compiler/frontend/src/typeck` | 第7章: 型チェックと型推論 | `docs/spec/1-2-types-Inference.md` |
| `compiler/frontend/src/semantics` | 第8章: 意味解析 | `docs/spec/1-0-language-core-overview.md`, `docs/spec/1-1-syntax.md` |
| `compiler/frontend/src/effects` | 第10章: エフェクトとFFI実行 | `docs/spec/1-3-effects-safety.md` |
| `compiler/frontend/src/ffi_executor.rs` | 第10章: エフェクトとFFI実行 | `docs/spec/3-9-core-async-ffi-unsafe.md` |
| `compiler/frontend/src/error.rs` | 第6章: 診断と出力 | `docs/spec/2-5-error.md` |

## runtime

| モジュール | 章の候補 | 対応する spec 節 |
| --- | --- | --- |
| `compiler/runtime/src/runtime` | 第13章: ランタイムの全体像 | `docs/spec/3-8-core-runtime-capability.md` |
| `compiler/runtime/src/embedding.rs` | 第13章: ランタイムの全体像 | `docs/spec/3-8-core-runtime-capability.md`（要確認） |
| `compiler/runtime/src/run_config.rs` | 第13章: ランタイムの全体像 | `docs/spec/3-7-core-config-data.md` |
| `compiler/runtime/src/stage.rs` | 第13章: ランタイムの全体像 | `docs/spec/3-8-core-runtime-capability.md`（要確認） |
| `compiler/runtime/src/capability` | 第14章: Capability と監査 | `docs/spec/3-8-core-runtime-capability.md` |
| `compiler/runtime/src/audit` | 第14章: Capability と監査 | `docs/spec/3-6-core-diagnostics-audit.md` |
| `compiler/runtime/src/collections` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-2-core-collections.md` |
| `compiler/runtime/src/text` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-3-core-text-unicode.md` |
| `compiler/runtime/src/numeric` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-4-core-numeric-time.md` |
| `compiler/runtime/src/time` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-4-core-numeric-time.md` |
| `compiler/runtime/src/io` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-5-core-io-path.md` |
| `compiler/runtime/src/path` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-5-core-io-path.md` |
| `compiler/runtime/src/diagnostics` | 第16章: 解析・DSL・診断 | `docs/spec/3-6-core-diagnostics-audit.md` |
| `compiler/runtime/src/config` | 第16章: 解析・DSL・診断 | `docs/spec/3-7-core-config-data.md` |
| `compiler/runtime/src/data` | 第16章: 解析・DSL・診断 | `docs/spec/3-7-core-config-data.md`（要確認） |
| `compiler/runtime/src/parse` | 第16章: 解析・DSL・診断 | `docs/spec/3-16-core-dsl-paradigm-kits.md`（要確認） |
| `compiler/runtime/src/dsl` | 第16章: 解析・DSL・診断 | `docs/spec/3-16-core-dsl-paradigm-kits.md` |
| `compiler/runtime/src/ffi` | 第17章: FFI とネイティブ連携 | `docs/spec/3-9-core-async-ffi-unsafe.md` |
| `compiler/runtime/src/native` | 第17章: FFI とネイティブ連携 | `docs/spec/3-9-core-async-ffi-unsafe.md` |
| `compiler/runtime/src/lsp` | 第18章: LSP/システム補助 | `docs/spec/3-14-core-lsp.md` |
| `compiler/runtime/src/system` | 第18章: LSP/システム補助 | `docs/spec/3-18-core-system.md` |
| `compiler/runtime/src/cli` | 第15章 or 第18章（CLI 章扱い） | `docs/spec/3-12-core-cli.md` |
| `compiler/runtime/src/prelude` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-1-core-prelude-iteration.md` |
| `compiler/runtime/src/env` | 第15章: 標準ライブラリのプリミティブ | `docs/spec/3-10-core-env.md` |
| `compiler/runtime/src/test` | 第22章: テスト戦略 | `docs/spec/3-11-core-test.md` |
