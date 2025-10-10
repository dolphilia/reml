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
[TRACE] Total: 0.081s (2304 bytes allocated)
```

- 計測は `Unix.gettimeofday` と `Gc.stat` を利用し、64bit 環境を前提とした 1 word = 8 bytes の換算を行う。
- フェーズ整合性が崩れた場合（`start_phase` と異なるフェーズで `end_phase` が呼ばれた場合）は警告行を挿入し、履歴には開始フェーズ名で記録する。  
  例: `[TRACE] Warning: phase mismatch (expected Parsing, got TypeChecking)`

### サマリー出力

コンパイルが正常に完了すると `Cli.Trace.print_summary` が呼ばれ、実行順に整形したサマリーを出力する。  
失敗時でも `Parsing` フェーズのみは `end_phase` が呼び出されるため、履歴は常に一貫した状態で終了する。

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
[STATS] ====================================
```

### JSON 取得

Phase 1-6 では CLI からの JSON 出力は未実装だが、`Cli.Stats.to_json` を介してテストや CI から直接 JSON 文字列を取得できる。  
将来 `--stats-format=json` を導入する際の互換性基盤として活用する。

## 運用ガイドライン

- **CI ログ収集**: `dune exec -- remlc-ocaml sample.reml --trace --stats 2> trace.log` により標準エラーをファイルへリダイレクトし、フェーズ毎の回帰を検出する。
- **メトリクス更新**: 10MB 規模の入力に対する `--stats` 出力を [0-3-audit-and-metrics.md](../plans/bootstrap-roadmap/0-3-audit-and-metrics.md) に追記し、性能トレンドを記録する。
- **診断との整合**: `--format=json` を併用する場合でも `--trace` / `--stats` は標準エラー出力を使用するため、CI 上でのログ分離（ファイル分割やプレフィックス付与）を推奨する。
- **将来拡張**: `--verbose` レベルに応じたトレース粒度やメモリピーク値の記録は Phase 2-5 で扱う予定なため、追加要件は [1-5-to-1-6-handover.md](../plans/bootstrap-roadmap/1-5-to-1-6-handover.md) のフォローアップ欄、または後続フェーズのハンドオーバー資料に追記して共有する。

## 参考資料

- [docs/plans/bootstrap-roadmap/1-6-developer-experience.md](../plans/bootstrap-roadmap/1-6-developer-experience.md)
- [docs/guides/diagnostic-format.md](diagnostic-format.md)
- [docs/spec/3-6-core-diagnostics-audit.md](../spec/3-6-core-diagnostics-audit.md)
