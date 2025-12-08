# 1.3 Dual-write 実行ランブック

本書は P1 フロントエンド移植で実施する dual-write（OCaml 版と Rust 版の並行実行）を再現性高く運用するための手順とログ管理ルールをまとめる。`1-0-front-end-transition.md`・`1-1-ast-and-ir-alignment.md`・`1-2-diagnostic-compatibility.md` で定義した検証項目を一括で遂行できるよう、実行コマンド・失敗時の切り分け手順・`reports/dual-write/` 配下の命名規則を明示する。

## 1.3.1 前提条件
- OCaml フロントエンドと Rust フロントエンドを `remlc --frontend {ocaml|rust}` で切り替えられる状態になっていること。
- P0 ベースライン（`0-1-baseline-and-diff-assets.md`）のゴールデンデータが最新であり、`scripts/validate-diagnostic-json.sh` が通過する。
- `tooling/ci/collect-iterator-audit-metrics.py` がローカル環境で実行できる（Python 3.10 以上を推奨）。
- 出力先ディレクトリ `reports/dual-write/` に書き込み権限がある。
- `reml_runtime_ffi` の capability shim（`compiler/rust/runtime/ffi/src/capability.rs`）が `core_prelude` 機能付きでビルド可能であること。Phase4 `FFI-CORE-PRELUDE-001` の前提として `cd compiler/rust/runtime/ffi && cargo check --features core_prelude` を通し、`core_iter_*` テストと同じ Stage/Capability メタデータが得られるか確認する。

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

### 手順 2b: 型推論ログ（typeck モード）の取得
```bash
scripts/poc_dualwrite_compare.sh \
  --mode typeck \
  --run-id 20270115-w3-typeck \
  --cases docs/plans/rust-migration/appendix/w3-dualwrite-cases.txt
```

- `--mode typeck` を指定すると `reports/dual-write/front-end/<run>/<case>/typeck/` に `typed-ast.{ocaml,rust}.json`, `constraints.{ocaml,rust}.json`, `impl-registry.{ocaml,rust}.json`, `effects-metrics.{ocaml,rust}.json`, `typeck-debug.{ocaml,rust}.json` が保存される（スキーマ定義: `appendix/w3-typeck-dualwrite-plan.md`、成果物例・命名規約: `reports/dual-write/front-end/w3-type-inference/README.md`）。
- OCaml CLI には `--emit-constraints-json`, `--emit-typeck-debug` を、Rust CLI には `--emit typed-ast --emit constraints --emit typeck-debug <dir>` を追加し、同一ケースを dual-write 実行する。
- 失敗ケースは `typeck/stderr.log` と `typeck/command.json` に再現手順を残す。`summary.json` の `typeck_metrics.match` が `false` の場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に TODO を登録する。
- CI (`.github/workflows/bootstrap-linux.yml` の `dual-write-typeck` ジョブ) では `scripts/poc_dualwrite_compare.sh --mode typeck` 実行後に `scripts/dualwrite_summary_report.py --update-typeck-readme` を呼び出し、README のサマリ表と `typeck/impl-registry.{ocaml,rust}.json` を含む成果物をアーティファクト化する。

### 手順 2c: 診断互換（diag モード）
```bash
scripts/poc_dualwrite_compare.sh \
  --mode diag \
  --run-id 20271107-w4-diagnostics \
  --cases docs/plans/rust-migration/appendix/w4-diagnostic-cases.txt

scripts/dualwrite_summary_report.py \
  reports/dual-write/front-end/w4-diagnostics/20271107-w4-diagnostics \
  --diag-table tmp/w4-diagnostics-table.md \
  --update-diag-readme reports/dual-write/front-end/w4-diagnostics/README.md
```

- TPM-TYPE-03 用には `docs/plans/bootstrap-roadmap/p1-test-migration-ffi-cases.txt` に `cli-callconv` / `ffi-contract` を `#metrics-case: effects-contract` で追加し、`scripts/poc_dualwrite_compare.sh --mode diag --cases docs/plans/bootstrap-roadmap/p1-test-migration-ffi-cases.txt` を `FORCE_TYPE_EFFECT_FLAGS=true` で実行する。これにより `--runtime-capabilities windows.ffi`/`--experimental-effects --effect-stage beta` を両フロントエンドに注入し、`reports/dual-write/front-end/w4-diagnostics/effects-contract/<run>/<case>/` に `diagnostics.{ocaml,rust}.json`、`audit.{ocaml,rust}.jsonl`、`diag-metrics.{frontend}.json`（`collect-iterator-audit-metrics.py --section diag --metrics-case effects-contract` の出力）を残す運用を確立した。

- `appendix/w4-diagnostic-cases.txt` に登録されたケースを順番に実行し、`reports/dual-write/front-end/w4-diagnostics/<run>/<case>/` に以下を保存する:
  - `diagnostics.{ocaml,rust}.json`, `diagnostics.diff.json`, `schema-validate.log`
  - `parser-metrics.{ocaml,rust}.json`, `effects-metrics.{ocaml,rust}.json`, `streaming-metrics.{ocaml,rust}.json`
  - `summary.json`（case ごとのゲート判定・Diag/metrics 状態を記録）
- Recover ケースでは `--emit-expected-tokens <dir>` を追加し、`expected_tokens/ocaml.json`・`expected_tokens/rust.json`・`expected_tokens.diff.json` を保存する。`diff` は `jq -S '.diagnostics[].extensions.recover.expected_tokens // []'` の結果を比較して生成し、`summary.json.expected_tokens_match` に反映する。`expected_tokens_match=false` の場合は `gating=false` とし、Run 全体を再試行する。
- `scripts/validate-diagnostic-json.sh` は `reports/diagnostic-format-regression.md#1-ローカル検証手順` のスキーマ要件を満たすよう拡張済みなので、diag モードでも出力ペアを必ず同スクリプトへ渡し、失敗したケースは `summary.json` の `schema_ok=false` / `gating=false` で識別できるようにする。
- `scripts/dualwrite_summary_report.py --diag-table` で Markdown テーブルを生成し、`reports/dual-write/front-end/w4-diagnostics/README.md` の `<!-- DIAG_TABLE_START/END -->` ブロックへ自動埋め込みする。CI で運用する場合は `README.md` をソースオブトゥルースとして扱い、Run ID ごとに表を更新する。
- CI (`bootstrap-linux` / `diag-dualwrite` ジョブ想定) でもローカルと同じ Run ID を指定し、`scripts/dualwrite_summary_report.py <run_dir> --diag-table <tmp.md> --update-diag-readme reports/dual-write/front-end/w4-diagnostics/README.md` を実行してからレポートを保存する。これにより `reports/dual-write/front-end/w4-diagnostics/README.md` が常に最新の `gating/schema_ok/metrics_ok` サマリになり、後続フェーズは README の表だけを参照すればよい。
- ケース行に `gating=false` が記録された場合は `summary.json` / `.err.log` を参照し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の該当 TODO へリンクを貼って再実行手順を残す。
- LSP 連携や CLI RunConfig ケースは `appendix/w4-diagnostic-case-matrix.md` 側で管理し、Ready になり次第 `cases.txt` と README のサマリ表を更新する。
- Type/Effect/FFI ケース（`type_*` / `effect_*` / `ffi_*`）は `force_type_effect_flags` により `--experimental-effects --effect-stage beta --type-row-mode dual-write --emit-typeck-debug <case>/typeck` を両フロントエンドへ強制し、Rust 側のみ `--emit-effects-metrics <case>/effects` を追加する。生成された `typeck/typeck-debug.{ocaml,rust}.json`／`effects/effects-metrics.{ocaml,rust}.json`／`typeck/command.{ocaml,rust}.json` は `summary.json` の `typeck_logs.present`・`metrics_ok` 判定に組み込む。`collect-iterator-audit-metrics.py --section effects --section parser` を必ず実行し、`effect_scope.audit_presence` / `parser.expected_summary_presence` が 1.0 に達しない場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-diag-rust-06` へ Run ID を添えて記録する。
- diag モードでは OCaml CLI に `--left-recursion off` を自動付与しつつ `--packrat` を有効化し、PARSER-003 未実装警告を抑止した状態で Packrat メトリクスを取得する（Rust CLI は未対応なので空配列）。Streaming メトリクスを持たないケースでも `collect-iterator-audit-metrics.py --section streaming` を実行し、`parser.stream_extension_field_coverage < 1.0` が出た場合は `DIAG-RUST-05` へ転記して原因をトリアージする。
- 🆕 2028-02-26: Streaming ケース（`stream_*`）は diag ハーネス側で自動的に `parser.expected_summary_presence` をゲートへ追加するよう更新済み。`collect-iterator-audit-metrics.py` の結果を解析し、`parser_expected_summary.json`（`parser-metrics.{ocaml,rust}.json` 内 `parser.expected_summary_presence` と `related_metrics`、および `parser.stream_extension_field_coverage`）を同ディレクトリへ保存する。いずれかの `pass_rate` が 1.0 未満の場合は `summary.json` の `metrics_ok`/`gating` を自動で `false` に戻し、次回 Run で基準を満たした時点で `README.md` と `p1-front-end-checklists.csv` のステータスをそのまま更新できる。  
- 🆕 2028-03-01: `poc_dualwrite_compare.sh` が `dune exec remlc` からの stderr を監視し、JSON 診断が stdout へ出力されない場合でも `diagnostics.ocaml.json` へ復元するようにした。また、`stream_*` ケースでは `expected`/`alternatives` が空の診断に `parse.expected.empty` プレースホルダを挿入し、`diag_counts` を記録したうえで `parser_expected_summary.json` のゲートを判定する。`diag_counts.<frontend>=0` のケース（例: `stream_backpressure_hint`）は当該フロントエンドのメトリクス収集をスキップし、存在する側のみ `pass_rate=1.0` をチェックする。

### 手順 3: メトリクス比較
```bash
python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section parser \
  --source "$OUT_OCAML" \
  --require-success \
  > "${OUT_OCAML%.json}-parser-metrics.json"

python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section effects \
  --source "$OUT_OCAML" \
  --require-success \
  > "${OUT_OCAML%.json}-effects-metrics.json"

python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section parser \
  --source "$OUT_RUST" \
  --require-success \
  > "${OUT_RUST%.json}-parser-metrics.json"

python3 tooling/ci/collect-iterator-audit-metrics.py \
  --section effects \
  --source "$OUT_RUST" \
  --require-success \
  > "${OUT_RUST%.json}-effects-metrics.json"
```

- スクリプトが失敗した場合はログ末尾の `missing_keys`・`mismatch` を確認し、`1-2-diagnostic-compatibility.md` の重点監視フィールドへ差分を登録する。
- Streaming ケースでは `--section streaming` も追加で実行し、`parser.stream_extension_field_coverage` / `parser.stream.backpressure_sync` / `parser.stream.demandhint_coverage` が 1.0 であることを確認する。`scripts/poc_dualwrite_compare.sh --mode diag` が自動生成する `parser_expected_summary.json`（`stream_*` ディレクトリ内）で `pass_rate=1.0` を外した項目がないかを確認し、未達の場合は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md#TODO:-DIAG-RUST-05` へ Run ID とともにリンクする。OCaml 側で診断が発生しないケースは `diag_counts.ocaml=0` を記録してゲート対象から外し、Rust 側のみで pass 条件を満たすかを確認する。
- メトリクス差の許容範囲は `1-1-ast-and-ir-alignment.md#1-1-7-検証パイプライン` で規定された 0.5pt 以内。typeck モードでは追加で `collect-iterator-audit-metrics.py --section effects` を実行し、`effects.impl_resolve.delta` / `effects.stage_mismatch.delta` が ±0.5pt 以内であることを確認する（具体的な保存手順: `reports/dual-write/front-end/w3-type-inference/README.md#メトリクス可視化`）。

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
| 型推論ログ | `typeck/<artifact>.<frontend>.json` | `typeck/typed-ast.rust.json`, `typeck/effects-metrics.ocaml.json` | `--mode typeck` 実行時のみ。詳細は `appendix/w3-typeck-dualwrite-plan.md` |

- CI から生成される成果物は同じ命名規則を用いる。`3-0-ci-and-dual-write-strategy.md` で定義するジョブは `UPLOAD_PATH=reports/dual-write/<date>-<workflow>` を用いてアーティファクト化する。
- ベンチマークや追加ログを保存する場合はサブディレクトリ（`benchmarks/`, `lsp/` 等）を作成し、この命名規則を基に管理する。

## 1.3.5 フォローアップ
- 新しいケースを追加した場合は `p1-front-end-checklists.csv` に該当項目を追加し、完了可否を管理する。
- トラブルシュートの知見は `docs/notes/` に TODO 付きで転記し、次回実行時の参考とする。
- CI 連携を実施した際は `3-0-ci-and-dual-write-strategy.md` に反映し、命名規則の差異がないか確認する。

## 1.3.6 W4.5 引き継ぎパッケージ作成手順

`1-0-front-end-transition.md#w4.5-p1-クロージングレビューp2-ハンドオーバー準備` で定義した通り、P1 W4.5 の成果物は `reports/dual-write/front-end/` に散在する Run をまとめて P2 へ渡す。以下の手順で `P1_W4.5_frontend_handover/` を構築する。

1. **ディレクトリ構成**  
   ```
   reports/dual-write/front-end/
     P1_W4.5_frontend_handover/
       ast-ir/            # w3-typeck / AST スナップショット
       diag/recover/      # 20280210-w4-diag-recover-else-r4
       diag/streaming/    # 20280410-w4-diag-streaming-r21
       diag/effects/      # 20280418-w4-diag-effects-r3, 20280601-*
       diag/cli-lsp/      # 20280430-w4-diag-cli-lsp
       README.md          # この節の要約
   ```
   - `ast-ir/` には `w3-type-inference/2027-01-15-w3-typeck/{typed-ast,constraints,impl-registry}.{ocaml,rust}.json` と `summary.md` を保存する。
   - `diag/*` サブディレクトリは `summary.{md,json}` / `diagnostics.{ocaml,rust}.json` / `audit_metadata.*` / `parser-metrics.*` / `effects-metrics.*` / `expected_tokens.*` / `typeck-debug.*` をフロントエンド別に格納する。

2. **Run ID の収集**  
   - Recover: `20280210-w4-diag-recover-else-r4`
   - Streaming: `20280410-w4-diag-streaming-r21`
   - Type/Effect/FFI: `20280418-w4-diag-effects-r3` + `20280601-w4-diag-type-effect-rust-typeck-r7`
   - CLI/LSP: `20280430-w4-diag-cli-lsp`
   これらの Run ID を `P1_W4.5_frontend_handover/README.md` 内で表にまとめ、`docs/plans/rust-migration/overview.md` や P2 計画書から参照できるようにする。

3. **コマンド記録 (`command.json`)**  
   各ケースディレクトリに `command.json`（`scripts/poc_dualwrite_compare.sh` / `collect-iterator-audit-metrics.py` / `scripts/dualwrite_summary_report.py` の引数と環境変数）を保存する。`--emit-expected-tokens` や `--force-type-effect-flags` の値も記録し、P2 側で Run を再現できるようにする。

4. **チェックリスト更新**  
   - `p1-front-end-checklists.csv` に `HandedOver` 列を追加し、Recover は `Pass(W4.5)`、Streaming/TypeEffect/CLI は `Pending(W4.5)` と Run ID を記録する。
   - `appendix/w4-diagnostic-case-matrix.md` と `w4-diagnostic-cases.txt` へ `HandedOver` 列を追加し、`P1_W4.5_frontend_handover/diag/<category>/` のパスをリンクさせる。

5. **ログと README の作成**  
   - `P1_W4.5_frontend_handover/README.md` に以下を含める:  
     - 各カテゴリの Run ID / 成果物パス / ステータス（✅ or Pending）  
     - `docs/plans/rust-migration/1-0`, `1-1`, `1-2`, `1-3` の該当節へのリンク  
     - `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の DIAG-RUST-05/06/07 番号  
   - `docs/migrations.log` へ「2029-05 Rust Migration P1 W4.5 handover」を追記する（別タスク）。

6. **P2 ドキュメントへの導線**  
   - `2-0-llvm-backend-plan.md` §2.0.10、`2-1-runtime-integration.md` §2.1.7、`2-2-adapter-layer-guidelines.md` §2.2.8 へ本ディレクトリをリンクする。
   - `3-0-ci-and-dual-write-strategy.md` では `P1_W4.5_frontend_handover/diag/*/summary.json` を CI ゲート入力として参照する旨を記載する。

これにより、P2 着手時に W4.5 の成果物／未解決課題を即座に再現・検証できる。
