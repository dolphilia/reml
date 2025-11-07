# W3 型推論 Dual-write ログ

W3（型推論コア移植）における dual-write 実行結果と派生メトリクスのまとまった置き場。`1-0-front-end-transition.md#W3` および `appendix/w3-typeck-dualwrite-plan.md` で定義した成果物を、再現性のある形で保管・可視化する。

## ディレクトリ構造

- `YYYY-MM-DD-w3-typeck/` — `scripts/poc_dualwrite_compare.sh --mode typeck` の 1 回分の実行ディレクトリ  
  - `<case>/summary.json` — AST/診断/typeck の一致状況 (`typeck_metrics.match` など)  
  - `<case>/typeck/typed-ast.{ocaml,rust}.json` — Typed AST のスナップショット  
  - `<case>/typeck/constraints.{ocaml,rust}.json` — 制約セット/スキームの比較用ログ  
  - `<case>/typeck/impl-registry.{ocaml,rust}.json` — Impl Registry の登録順序確認ログ（今後追加予定）  
  - `<case>/typeck/effects-metrics.{ocaml,rust}.json` — `collect-iterator-audit-metrics.py --section effects` 実行結果（Step4 で生成）  
  - `<case>/typeck/typeck-debug.{ocaml,rust}.json` — `Type_inference_effect` / `Constraint_solver` の詳細トレース  
  - `<case>/typeck/metrics.json` — Rust 版 typeck が出力した集計値（`typed_functions`, `constraints_total` など）

## 実行手順メモ

```bash
# ケース定義（name::label::path）を列挙したファイルを用意
CASES=docs/plans/rust-migration/appendix/w3-dualwrite-cases.txt
RUN_ID=$(date +%Y-%m-%d)-w3-typeck

scripts/poc_dualwrite_compare.sh \
  --mode typeck \
  --run-id "$RUN_ID" \
  --cases "$CASES" \
  --dualwrite-root reports/dual-write/front-end/w3-type-inference
```

- OCaml CLI: `remlc --frontend ocaml --emit-constraints-json <path> --emit-typeck-debug <dir>`  
- Rust CLI: `remlc --frontend rust --emit typed-ast --emit constraints --emit typeck-debug <dir>`  
- `typeck-debug` には `effect_scope`, `residual_effects`, `recoverable`, `ocaml_exception` を含める。フィールドの正規化規則は `appendix/w3-typeck-dualwrite-plan.md` を参照。

## メトリクス可視化

### effects セクション

型推論フェーズでは `collect-iterator-audit-metrics.py --section effects` をケースごとに実行し、`effects.impl_resolve.delta` / `effects.unify.delta` が ±0.5pt 以内であることを受入基準とする。

```bash
CASE_DIR=reports/dual-write/front-end/w3-type-inference/2027-01-15-w3-typeck/pattern_tuple

python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section effects \
  --baseline "$CASE_DIR/ocaml.diagnostics.json" \
  --candidate "$CASE_DIR/rust.json" \
  --require-success \
  > "$CASE_DIR/typeck/effects-metrics.rust.json"

python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section effects \
  --baseline "$CASE_DIR/rust.json" \
  --candidate "$CASE_DIR/ocaml.diagnostics.json" \
  > "$CASE_DIR/typeck/effects-metrics.ocaml.json"
```

- 結果ファイルは `typeck/effects-metrics.{ocaml,rust}.json` に格納し、`1-0-front-end-transition.md#W3` の Step4 で参照する。
- 失敗したキーは `missing_keys` `mismatch` に出力されるため、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` へ TODO として転記する。

### 診断 JSON の再検証

`scripts/validate-diagnostic-json.sh` を同一ケースで再実行し、Schema を逸脱する診断が無いことを確認する。ログは `<case>/typeck/diagnostic-validate.log` として保存する。

```bash
scripts/validate-diagnostic-json.sh \
  "$CASE_DIR/ocaml.diagnostics.json" \
  "$CASE_DIR/rust.json" \
  2>&1 | tee "$CASE_DIR/typeck/diagnostic-validate.log"
```

## 2027-01-15 ランのサマリ

| case | typeck_match | typed_functions (ocaml/rust) | constraints_total (ocaml/rust) | diagnostics (ocaml/rust) |
| --- | --- | --- | --- | --- |
| callconv_windows_messagebox | True | 5 / 5 | 0 / 0 | 1 / 64 |
| diagnostic_effect_stage | True | 1 / 1 | 0 / 0 | 1 / 1 |
| ffi_dispatch_async | False | 1 / 3 | 1 / 0 | 0 / 42 |
| pattern_tuple | True | 2 / 2 | 0 / 0 | 1 / 1 |
| residual_effect | True | 2 / 2 | 0 / 0 | 1 / 5 |

- `ffi_dispatch_async` は OCaml 側が型推論エラーを返す一方、Rust 側は fallback 集計のみとなっているため `typed_functions.delta=-2` で `typeck_metrics.match=false`。`W3-TYPECK-ffi-dispatch-async`（`2-7-deferred-remediation.md`）で追跡。  
- `callconv_windows_messagebox` / `residual_effect` では AST/診断 diff が残っているため、W4 診断互換タスクで再検証する。  
- すべての `typeck-debug.{ocaml,rust}.json` は `effect_scope` / `residual_effects` の配列長が一致しており、ログ整形規約が機能している。

## フォローアップ

- `impl-registry.{ocaml,rust}.json` と `effects-metrics.{ocaml,rust}.json` の生成を自動化し、`summary.json` にリンクを追加する。  
- `scripts/poc_dualwrite_compare.sh --mode typeck` 実行時に `diagnostic-validate.log` を常に保存するオプションを追加（Issue: `W3-TYPECK-log-retention`）。  
- 今後のランでは `summary.md` だけでなく `summary.json` からメトリクスを抽出し、`README.md` のサマリ表を更新する運用とする。

