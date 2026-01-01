# トレース出力ガイド

**対象フェーズ**: Phase 1-6 開発者体験整備  
**最終更新**: 2025-10-10

## 概要

`remlc-ocaml` は Phase 1-6 Week 15 でトレース機能と統計出力を導入し、コンパイルパイプラインの可視性を高めた。  
本ガイドでは `--trace` および `--stats` オプションの挙動、出力フォーマット、メトリクス活用の指針をまとめる。  
設計背景は [1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md) を参照。

## CLI オプション概要

| オプション | 目的 | 既定値 | 補足 |
| --- | --- | --- | --- |
| `--trace` | 各フェーズ開始・終了をリアルタイムで表示 | 無効 | フェーズ完了時に要約を標準エラーへ出力 |
| `--stats` | コンパイル統計情報を集計・表示 | 無効 | `Cli.Stats.print_stats` を通じて標準エラーへ出力 |
| `--verbose <0-3>` | ログ詳細度を制御 | `1` | `--trace` と併用することで段階的な情報開示を準備（詳細実装は Phase 2 予定） |

> 環境変数 `REMLC_LOG` を `error` / `warn` / `info` / `debug` に設定すると初期 `verbose` 値を上書きできる（Phase 1-6 時点ではトレース出力に直接は影響しない）。

## フェーズトレース

### フェーズ一覧

| フェーズ識別子 | 対応処理 | 実装箇所 |
| --- | --- | --- |
| `Parsing` | 字句解析・構文解析 (`Parser_driver.parse`) | `Cli.Trace.start_phase Parsing` |
| `TypeChecking` | HM 型推論 (`Type_inference.infer_compilation_unit`) | 同上 |
| `CoreIR` | Core IR 生成・糖衣削除 (`Core_ir.Desugar`) | 同上 |
| `Optimization` | Core IR 最適化 (`Core_ir.Pipeline.optimize_module`) | 同上 |
| `CodeGen` | LLVM IR 生成 (`Llvm_gen.Codegen.codegen_module`) | 同上 |

### 出力フォーマット

`--trace` を有効化すると各フェーズの開始・終了が標準エラーに逐次表示される。  
計測内容は実行時間（秒）とアロケーション量（バイト）で、GC 統計に基づく近似値を採用する。

```text
[TRACE] Parsing started
[TRACE] Parsing completed (0.013s, 704 bytes allocated)
[TRACE] TypeChecking started
[TRACE] TypeChecking completed (0.022s, 960 bytes allocated)
…
[TRACE] ===== Trace Summary =====
[TRACE]   Parsing: 0.013s (21.3%, 704 bytes)
[TRACE]   TypeChecking: 0.048s (78.7%, 960 bytes)
[TRACE] Total: 0.061s (1664 bytes allocated)
[TRACE] Peak memory: 65536 bytes
[TRACE] =======================
```

- 計測は `Unix.gettimeofday` と `Gc.stat` を利用し、64bit 環境を前提とした 1 word = 8 bytes の換算を行う（`Sys.word_size` に基づき自動調整）。
- フェーズ整合性が崩れた場合（`start_phase` と異なるフェーズで `end_phase` が呼ばれた場合）は警告行を挿入し、履歴には開始フェーズ名で記録する。  
  例: `[TRACE] Warning: phase mismatch (expected Parsing, got TypeChecking)`
- サマリーでは各フェーズの時間比率（%）とフェーズ単位のアロケーション量を同時に出力する。
- `Peak memory` 行は `Gc.stat ().top_heap_words` を基準にしたピークヒープサイズ（バイト換算）で、`memory_peak_ratio` を算出する際の分子として利用する。

### サマリー出力

コンパイルが正常に完了すると `Cli.Trace.print_summary` が呼ばれ、実行順に整形したサマリーを出力する。  
失敗時でも `Parsing` フェーズのみは `end_phase` が呼び出されるため、履歴は常に一貫した状態で終了する。  
`Cli.Trace.summary` を通じてサマリーデータを取得でき、`phases` 配列の各要素は `elapsed_seconds`・`time_ratio`・`allocated_bytes` を保持する。

## コンパイル統計 (`--stats`)

### カウンタ一覧

| フィールド | 内容 | 計測タイミング |
| --- | --- | --- |
| `Tokens parsed` | 解析したトークン総数 | Lexer で `Cli.Stats.incr_token_count` 呼び出し |
| `AST nodes` | 生成した AST ノード数 | Parser でノード構築時にインクリメント |
| `Unify calls` | 型推論の unify 呼び出し数 | `Type_inference.unify` の侵⽤ |
| `Optimization passes` | 適用した最適化パス数 | `Core_ir.Pipeline` |
| `LLVM instructions` | 生成した LLVM IR 命令数 | `Llvm_gen.Codegen` |

### 出力例

```text
[STATS] ===== Compilation Statistics =====
[STATS] Tokens parsed: 245
[STATS] AST nodes: 87
[STATS] Unify calls: 152
[STATS] Optimization passes: 3
[STATS] LLVM instructions: 421
[STATS] Phase timings:
[STATS]   Parsing: 0.013s (21.3%, 704 bytes)
[STATS]   TypeChecking: 0.048s (78.7%, 960 bytes)
[STATS] Total time: 0.061s
[STATS] Total allocated: 1664 bytes
[STATS] Peak memory: 65536 bytes
[STATS] memory_peak_ratio: 6.4000
[STATS] ====================================
```

### フェーズ別ランキング出力

Phase 1-6 Week 16 で、統計出力に時間比率降順のランキングが追加されました。
最も時間がかかったフェーズから順に表示されるため、パフォーマンスのボトルネック特定が容易になります。

```text
[STATS] Phase timings (ranked by time):
[STATS]   1. TypeChecking: 0.048s (78.7%, 960 bytes)
[STATS]   2. Parsing: 0.013s (21.3%, 704 bytes)
```

### メトリクスファイル出力

`--metrics <path>` オプションを使用すると、統計情報をファイルに出力できます。
出力形式は `--metrics-format` で `json`（デフォルト）または `csv` を選択できます。

```bash
# JSON形式で出力
opam exec -- dune exec -- remlc sample.reml --metrics metrics.json

# CSV形式で出力
opam exec -- dune exec -- remlc sample.reml --metrics metrics.csv --metrics-format csv
```

JSON出力はスキーマ定義 [`docs/schemas/remlc-metrics.schema.json`](../schemas/remlc-metrics.schema.json) に準拠します。

JSON には以下のフィールドが追加される。

| フィールド | 説明 |
| --- | --- |
| `phase_timings` | `Cli.Trace.summary` の `phases` 配列をシリアライズしたもの（`phase` / `elapsed_seconds` / `time_ratio` / `allocated_bytes`） |
| `total_elapsed_seconds` | 全フェーズ合計時間 |
| `total_allocated_bytes` | 全フェーズ合計アロケーション量 |
| `peak_memory_bytes` | GC 統計から算出したピークメモリ（`None` の場合は `null`） |
| `memory_peak_ratio` | `peak_memory_bytes / input_size_bytes`（分母が 0 または未設定の場合は `null`） |
| `input_size_bytes` | 処理対象ソースのバイト数 |

`Cli.Stats.set_input_size_bytes` を `--stats` 有効時に呼び出すことで、`memory_peak_ratio` の分母となる入力サイズを記録する。  
`Cli.Stats.update_trace_summary (Cli.Trace.summary ())` を呼び出すとフェーズ比率とピークメモリが統計情報へ転記される。

## 運用ガイドライン

- **CI ログ収集**: `dune exec -- remlc-ocaml sample.reml --trace --stats 2> trace.log` により標準エラーをファイルへリダイレクトし、フェーズ毎の回帰を検出する。
- **メトリクス更新**: 10MB 規模の入力に対する `--metrics` 出力を [0-3-audit-and-metrics.md](../plans/bootstrap-roadmap/0-3-audit-and-metrics.md) に追記し、性能トレンドを記録する。`scripts/benchmark-parse-throughput.sh` を使用して計測を自動化できます。
- **診断との整合**: `--format=json` を併用する場合でも `--trace` / `--stats` は標準エラー出力を使用するため、CI 上でのログ分離（ファイル分割やプレフィックス付与）を推奨する。
- **将来拡張**: `--verbose` レベルに応じたトレース粒度やメモリピーク値の記録は Phase 2-5 で扱う予定なため、追加要件は [1-5-to-1-6-handover.md](../plans/bootstrap-roadmap/1-5-to-1-6-handover.md) のフォローアップ欄、または後続フェーズのハンドオーバー資料に追記して共有する。

## 10MB入力プロファイル計測

`parse_throughput` 指標の基準値を確立するため、以下のスクリプトを使用します。

```bash
# 10MB入力ファイルを生成
./scripts/generate-large-input.sh examples/benchmark/large_input.reml

# parse_throughput を計測（3回平均）
./scripts/benchmark-parse-throughput.sh examples/benchmark/large_input.reml
```

計測結果は `0-3-audit-and-metrics.md` に手動で記録してください。

## API 連携の概要

- `Cli.Trace.summary : unit -> Cli.Trace.summary`  
  フェーズ別の時間・比率・アロケーション情報とピークメモリを取得する。`print_summary` はこの結果を利用して整形出力する。
- `Cli.Stats.set_input_size_bytes : int -> unit`  
  `memory_peak_ratio` 計算用の入力サイズ（バイト）を記録する。
- `Cli.Stats.update_trace_summary : Cli.Trace.summary -> unit`  
  `Cli.Trace.summary` で得た結果を統計情報へ統合し、`--stats` 出力や JSON シリアライズに反映する。

## 参考資料

- [docs/plans/bootstrap-roadmap/1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md)
- [docs/guides/tooling/diagnostic-format.md](docs/guides/tooling/diagnostic-format.md)
- [docs/spec/3-6-core-diagnostics-audit.md](../spec/3-6-core-diagnostics-audit.md)
