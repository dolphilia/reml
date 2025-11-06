# 2025-11-06 OCaml ベースライン計測ログ

- ブランチ: `git rev-parse --short HEAD` → `412bf15`
- 実行環境: macOS (Codex CLI / sandbox workspace-write)

## 1. `dune runtest`
- コマンド: `dune runtest`（作業ディレクトリ: `compiler/ocaml/`）
- 結果: 正常終了（終了コード 0）。追加出力なし。

## 2. `collect-iterator-audit-metrics.py --require-success`
- コマンド:
  ```bash
  python3 tooling/ci/collect-iterator-audit-metrics.py \
    --require-success \
    --source compiler/ocaml/tests/golden/typeclass_iterator_stage_mismatch.json.golden \
    --source compiler/ocaml/tests/golden/diagnostics/parser/parser-runconfig-packrat.json.golden \
    --source compiler/ocaml/tests/golden/diagnostics/parser/streaming-outcome.json.golden
  ```
- 結果: 主要メトリクスの不足により失敗（終了コード 1）。
- 代表的な失敗要因:
  - `[collect-iterator-audit-metrics] diagnostics array is missing`
  - `[collect-iterator-audit-metrics] lexer.shared_profile_pass_rate < 1.0`
  - `[collect-iterator-audit-metrics] lexer.identifier_profile_unicode < 1.0`
  - `[collect-iterator-audit-metrics] typeclass.metadata_pass_rate < 1.0`
  - `[collect-iterator-audit-metrics] parser.expected_summary_presence: total=0`
- 原因分析: `compiler/ocaml/tests/golden/diagnostics/effects/syntax-constructs.json.golden` 等、一部のゴールデンが `diagnostics` セクションを持たず、既定ソースのみではメトリクス収集条件を満たせない。`scripts/validate-diagnostic-json.sh` で診断 JSON を再生成し、効果系サンプルや FFI 監査ログを含む完全な入力セットを揃える必要がある。

## 次回アクション
1. `scripts/validate-diagnostic-json.sh tmp/diagnostics-output/` を実行し、JSON 出力を `reports/dual-write/front-end/ocaml/<date>/` へ保存する。
2. 効果構文・FFI・タイプクラス関連の追加サンプルを `--source` に指定し、`--require-success` を再実行してメトリクス欠落を解消する。
3. 成功した測定値を `reports/diagnostic-format-regression.md` および `docs/plans/rust-migration/0-1-baseline-and-diff-assets.md` の該当欄へ反映する。  
   - 進捗メモ: 2025-11-06 時点で再実行した結果は `reports/dual-write/20251106-ocaml-diagnostics-refresh.md` を参照。
