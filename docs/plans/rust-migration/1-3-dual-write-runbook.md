# 1.3 Dual-write 実行ランブック

本書は P1 フロントエンド移植で実施する dual-write（OCaml 版と Rust 版の並行実行）を再現性高く運用するための手順とログ管理ルールをまとめる。`1-0-front-end-transition.md`・`1-1-ast-and-ir-alignment.md`・`1-2-diagnostic-compatibility.md` で定義した検証項目を一括で遂行できるよう、実行コマンド・失敗時の切り分け手順・`reports/dual-write/` 配下の命名規則を明示する。

## 1.3.1 前提条件
- OCaml フロントエンドと Rust フロントエンドを `remlc --frontend {ocaml|rust}` で切り替えられる状態になっていること。
- P0 ベースライン（`0-1-baseline-and-diff-assets.md`）のゴールデンデータが最新であり、`scripts/validate-diagnostic-json.sh` が通過する。
- `tooling/ci/collect-iterator-audit-metrics.py` がローカル環境で実行できる（Python 3.10 以上を推奨）。
- 出力先ディレクトリ `reports/dual-write/` に書き込み権限がある。

## 1.3.2 実行手順

### 手順 0: 設定の確認
```bash
# フロントエンド切替オプションが機能するか確認
remlc --frontend ocaml --version
remlc --frontend rust --version
```

エラーが出る場合は `compiler/rust/` のビルドまたは CLI ブリッジ設定を確認する。

### 手順 1: AST ダンプの取得と比較
```bash
CASE=examples/cli/sample.reml
OUT_OCAML=reports/dual-write/front-end/$(date +%Y%m%d)-sample/ast-ocaml.json
OUT_RUST=reports/dual-write/front-end/$(date +%Y%m%d)-sample/ast-rust.json

mkdir -p "$(dirname "$OUT_OCAML")"

remlc --frontend ocaml --emit-ast --format json "$CASE" | jq --sort-keys > "$OUT_OCAML"
remlc --frontend rust  --emit-ast --format json "$CASE" | jq --sort-keys > "$OUT_RUST"
diff -u "$OUT_OCAML" "$OUT_RUST" > "${OUT_OCAML%.json}-ast.diff" || true
```

- 差分が空であれば AST 構造は一致。差分がある場合は `1-1-ast-and-ir-alignment.md` のチェックリストを参照し、該当ノードの実装を確認する。

### 手順 2: 診断 JSON の検証
```bash
reports/diagnostic-format-regression.md#schema-validation に従い、JSON スキーマ検証を実施
scripts/validate-diagnostic-json.sh "$OUT_OCAML" "$OUT_RUST"
```

- スキーマエラーが出た場合は `1-2-diagnostic-compatibility.md` の重点監視フィールドを参照し、欠落フィールドや型違いを調査する。
- 差分比較は `diff -u` または `jq --sort-keys` で再度差分を出力し、`reports/dual-write/front-end/$(date)-sample/diagnostic.diff` に保存する。

### 手順 3: メトリクス比較
```bash
python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section parser \
  --baseline "$OUT_OCAML" \
  --candidate "$OUT_RUST" \
  --require-success \
  > "${OUT_OCAML%.json}-parser-metrics.json"

python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section effects \
  --baseline "$OUT_OCAML" \
  --candidate "$OUT_RUST" \
  --require-success \
  > "${OUT_OCAML%.json}-effects-metrics.json"
```

- スクリプトが失敗した場合はログ末尾の `missing_keys`・`mismatch` を確認し、`1-2-diagnostic-compatibility.md` の重点監視フィールドへ差分を登録する。
- メトリクス差の許容範囲は `1-1-ast-and-ir-alignment.md#1-1-6-検証パイプライン` で規定された 0.5pt 以内。

### 手順 4: 自動判定レポートの生成
```bash
RUN_DIR=reports/dual-write/front-end/poc/2025-11-07-w2-ast-inventory
scripts/dualwrite_summary_report.py \
  "$RUN_DIR" \
  --out-json "$RUN_DIR/summary_report.json" \
  --out-md "$RUN_DIR/summary_report.md"
```
- `*.summary.json` を集計し、AST/診断/Packrat の一致状況を Markdown・JSON にまとめる。CI では Markdown をアーティファクト化し、JSON をゲート判定に利用する。
- 診断件数差分など追加の考察は `reports/dual-write/front-end/<run>/diagnostic_diff.md` のような派生レポートに記録し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の TODO と連携させる。

### 手順 5: 付随テスト（任意）
- `compiler/ocaml/tests/parser_*` や `test_type_inference.ml` に対応する dual-write テストハーネスがある場合は、`cargo test --package remlc_frontend --features dual-write` のような形で同時実行する。
- ベンチマークを取得する場合は `scripts/benchmark.sh --suite <name> --frontend {ocaml|rust}` を用い、結果を `reports/dual-write/benchmarks/` へ保存する（詳細は `3-2-benchmark-baseline.md` を参照）。

## 1.3.3 失敗時の切り分け

| 兆候 | 想定原因 | 切り分け手順 | 対応先 |
| --- | --- | --- | --- |
| `remlc --frontend rust` が失敗 | Rust CLI バイナリ未ビルド、機能フラグ不一致 | `cargo build -p remlc_cli` を再実行し、`--help` でフラグを確認 | CLI 実装チーム |
| AST diff に構造差が多数 | AST/Span 正規化が未実装 | `1-1-ast-and-ir-alignment.md` の対応表で該当ノードを特定、Rust 側実装を調査 | Parser/AST チーム |
| `validate-diagnostic-json.sh` でスキーマエラー | フィールド欠落・型不一致 | エラーログの JSON パスを `1-2-diagnostic-compatibility.md` の重点監視フィールドと照合、OCaml 版と Rust 版のエミッタを比較 | 診断チーム |
| `collect-iterator-audit-metrics.py` の `missing_keys` | 拡張メトリクス未出力 | Rust 実装で `extensions.*` を生成しているか確認、必要なら `Diagnostic.Builder` へ追加 | 診断/効果チーム |
| メトリクス誤差が閾値超過 | Packrat/効果の挙動差 | `reports/dual-write/front-end/*-parser-metrics.json` を精査し、関連テストを個別実行 (`streaming_runner_tests.ml` など) | Parser/効果チーム |
| レポート出力が上書きされる | 命名規則未遵守 | 手順 1 の `CASE` 名と日付を見直し、`reports/dual-write/<日付>-<ケース>/` を作り直す | 実行担当 |

## 1.3.4 `reports/dual-write/` 命名規則

| レイヤー | 規則 | 例 | 備考 |
| --- | --- | --- | --- |
| ルートディレクトリ | `reports/dual-write/` | `reports/dual-write/` | Dual-write の全成果物を集約 |
| 日付フォルダ | `YYYYMMDD-<scope>` 形式。`scope` は `sample`, `cli-tests`, `parser-batch` など入力集合を表す | `reports/dual-write/20251109-sample/` | 同一日に複数ケースがある場合は `-a`, `-b` を付与 (`20251109-parser-a`) |
| AST/診断 JSON | `<artifact>-<frontend>.json` (`ast-ocaml.json`, `diagnostic-rust.json` など) | `ast-ocaml.json` | `jq --sort-keys` で整形済みの JSON を格納 |
| Diff ファイル | `<artifact>.diff` | `diagnostic.diff`, `ast.diff` | `diff -u` の結果を保存 |
| メトリクス | `<artifact>-<section>-metrics.json` | `diagnostic-parser-metrics.json` | `collect-iterator-audit-metrics.py` の標準出力を保存 |
| 補足ノート | `README.md` または `notes.md` | `reports/dual-write/20251109-sample/README.md` | 手動調査の要点を Markdown で記録 |

- CI から生成される成果物は同じ命名規則を用いる。`3-0-ci-and-dual-write-strategy.md` で定義するジョブは `UPLOAD_PATH=reports/dual-write/<date>-<workflow>` を用いてアーティファクト化する。
- ベンチマークや追加ログを保存する場合はサブディレクトリ（`benchmarks/`, `lsp/` 等）を作成し、この命名規則を基に管理する。

## 1.3.5 フォローアップ
- 新しいケースを追加した場合は `p1-front-end-checklists.csv` に該当項目を追加し、完了可否を管理する。
- トラブルシュートの知見は `docs/notes/` に TODO 付きで転記し、次回実行時の参考とする。
- CI 連携を実施した際は `3-0-ci-and-dual-write-strategy.md` に反映し、命名規則の差異がないか確認する。
