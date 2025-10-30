# Lexer パフォーマンス計測メモ（Phase 2-5 LEXER-002 Step5）

## 目的
- Core.Parse.Lex 抽出後の字句処理が Phase 0 指針（`docs/spec/0-1-project-purpose.md` §1.1）の性能要件を満たすか定量的に確認する。
- `RunConfig.extensions["lex"]` を介したトリビア共有が大規模入力（10MB クラス）でオーバーヘッドを生まないか監視指標を整備する。

## 計測手順
1. 入力データ生成  
   - `scripts/generate-large-input.sh <出力先>` を用いて 10MB 以上の Reml ソースを生成する。既定では `examples/benchmark/large_input.reml` を出力。
2. ベンチマーク実行  
   - `scripts/benchmark-parse-throughput.sh` を利用し、`opam exec -- dune exec -- remlc` に `--metrics` オプションを付与して解析時間を 3 回計測する。  
     例:  
     ```bash
     ./scripts/benchmark-parse-throughput.sh \
       examples/benchmark/large_input.reml \
       /tmp/remlc-lex-metrics.json 3
     ```
3. プロファイル切替  
   - `RUNCONFIG_LEX_PROFILE` 環境変数（CLI/LSP ブートストラップで利用）を切り替えつつ、`strict_json`・`json_relaxed`・`toml_relaxed` の 3 設定で同一入力を測定する。
4. メトリクス集計  
   - `tooling/ci/collect-iterator-audit-metrics.py --summary` を実行し、`lexer.shared_profile_pass_rate` が 1.0 であることを確認する。

## 現状の計測状況（2025-11-30）
- 本リポジトリ環境では `remlc` 実行バイナリが未配置のため、`benchmark-parse-throughput.sh` による計測を実行できなかった。  
  `scripts/benchmark-parse-throughput.sh` が依存する `opam exec -- dune exec -- remlc` のビルド手順を Phase 2-6 Windows 対応と合わせて整備する必要がある。
- `lexer.shared_profile_pass_rate` は `parser-runconfig-packrat` 系ゴールデンを用いた JSON 解析で算出可能になった。CI での定常監視は `tooling/ci/collect-iterator-audit-metrics.py --summary` を追加実行することで対応予定。

## フォローアップ
- [ ] `remlc` ビルド・実行環境を構築し、上記手順で `strict_json` / `json_relaxed` / `toml_relaxed` の平均解析時間と標準偏差を計測する。  
       記録先: `docs/notes/lexer-performance-study.md` 「計測結果」セクション（次回更新で追加）。
- [ ] `Core_parse_lex.Record.consume` の収集結果を `lexer.shared_profile_pass_rate` の補助指標として `reports/metrics/lex-trivia.json`（仮称）へエクスポートする。
- [ ] CLI/LSP 経路で `RunConfig.extensions["lex"].space_id` が欠落した場合の警告を `compiler/ocaml/src/main.ml` と `tooling/lsp/run_config_loader.ml` に追加し、計測中のログで逸脱を検知できるようにする。
