# Phase 4 Spec Audit（ch4）

Phase 4 の `.reml` シナリオを自動実行し、`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` と突き合わせた結果を集約するディレクトリです。`tooling/examples/run_phase4_suite.py`（`run_examples.sh --suite spec_core|practical` から呼び出し）によって以下のレポートが生成されます。

| ファイル | 内容 |
| --- | --- |
| `spec-core-dashboard.md` | Chapter 1（構文・型・効果）向け `examples/spec_core/` の実行結果と Diagnostics の照合状況 |
| `practical-suite-index.md` | Chapter 3（標準ライブラリ・実務ケース）向け `examples/practical/` の実行結果と Diagnostics の照合状況 |

各レポートにはシナリオ ID、入力パス、期待される `diagnostic_keys`、実際に出力された Diagnostics のコード、CLI の終了コードが Markdown 表で記録されます。`diagnostic_keys` に差分がある場合は `❌ fail` として強調され、未実装機能や仕様差異を洗い出す指標として利用します。

> 実行手順: `tooling/examples/run_examples.sh --suite spec_core` または `--suite practical` を実行すると、`reports/spec-audit/ch4` 配下の対応レポートが更新されます。失敗シナリオが存在する場合は exit 1 で終了し、CI でも検知できるようにしています。

## 失敗ログからの自動切り分け

フェーズ E の「spec_core サンプル実行保証」では、`reports/spec-audit/ch4/logs/spec_core-*.md` に保存された失敗ログを `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` と同期させるため、`scripts/triage_spec_core_failures.py` を用意しています。

- 使い方例  
  ```
  python3 scripts/triage_spec_core_failures.py \
    --suite spec_core \
    --log reports/spec-audit/ch4/logs/spec_core-20251208T173235Z.md \
    --apply
  ```
  `--apply` を付けない場合は dry-run で更新対象のみ表示します。`--include-status pending,` のように指定すると、`resolution` がその値に一致する行のみを自動更新します。
- 判定規則  
  - `example_fix`: `diagnostic_keys` が定義されているのに CLI 実行では Diagnostics が 0 件だったケース（例: 負例サンプルの条件が弱くなった）。`.reml` や `expected/` を再整備する必要があります。
  - `impl_fix`: 期待診断が空 (`[]`) のシナリオで Diagnostics や CLI エラーが発生したケース、または JSON 出力の解析に失敗したケース。Rust Frontend/Runtime の修正が必要です。
  - `spec_fix`: 期待診断と実測診断のどちらも存在するものの、コード集合が一致しないケース。仕様または `phase4-scenario-matrix.csv` の `diagnostic_keys` を見直します。
- `resolution_notes` には `triage_spec_core_failures.py` が自動で `log=`（解析した Markdown ログ）、`CLI=`（`run_phase4_suite.py` が実行したコマンド）、期待/実測の診断コード集合を追記します。これにより、後続タスクで Example Fix / Compiler Fix / Spec Fix の根拠が追跡できます。

## 標準ライブラリ実行ログ（Phase 4）

標準ライブラリの回帰シナリオは、以下のログに実行結果を記録します。

- 保存先: `reports/spec-audit/ch4/logs/stdlib-*.md`
- 対象ログ:
  - `reports/spec-audit/ch4/logs/stdlib-test-YYYYMMDD.md`
  - `reports/spec-audit/ch4/logs/stdlib-cli-YYYYMMDD.md`
  - `reports/spec-audit/ch4/logs/stdlib-pretty-YYYYMMDD.md`
  - `reports/spec-audit/ch4/logs/stdlib-doc-YYYYMMDD.md`
  - `reports/spec-audit/ch4/logs/stdlib-lsp-YYYYMMDD.md`
- 記録項目:
  - 実行コマンド全文と実行日時
  - `diagnostics[].code` の集合（空の場合は `[]` を明記）
  - stdout の先頭 1 行と `expected/` との一致確認
  - `run_id` が含まれる場合は比較対象から除外する旨を記載
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `resolution_notes` には、該当ログファイル名と実行コマンドを併記する。

## Native Escape Hatches 実行ログ（Phase 4）

`Core.Native` の intrinsic と埋め込み API の回帰シナリオは、以下のログに実行結果を記録します。

- 保存先: `reports/spec-audit/ch4/logs/native-embed-*.md`
- 保存先: `reports/spec-audit/ch4/logs/native-intrinsic-*.md`
- 命名規則: `native-intrinsic-YYYYMMDD.md` / `native-embed-YYYYMMDD.md`（時刻を含める場合は `YYYYMMDDTHHMMSSZ`）
- 記録項目:
  - `native-intrinsic-*`: 実行コマンド、対象ターゲット、`native.intrinsic.used` / `intrinsic.name` / `intrinsic.signature` の出力有無、`native.intrinsic.polyfill` 発生有無、`expected/native/intrinsics/` との差分確認
  - 実行したホストアプリ（例: `examples/native/embedding/basic.c`）と ABI バージョン
  - `native.embed.entrypoint` / `embed.abi.version` の監査キーが出力されたこと
  - エラー時は `native.embed.abi_mismatch` / `native.embed.unsupported_target` の有無
  - `expected/native/embedding/` との差分確認結果

## Native Escape Hatches KPI（Phase 4）

- 成功率: `NATIVE-INTRINSIC-001` / `NATIVE-EMBED-001` の成功件数 ÷ 実行件数
- 監査キー欠落率: 必須キー欠落数 ÷ 必須キー総数（intrinsic は `native.intrinsic.used`/`intrinsic.name`/`intrinsic.signature`、embed は `native.embed.entrypoint`/`embed.abi.version`）
- フォールバック発生率: `native.intrinsic.polyfill` 件数 ÷ `native.intrinsic.used` 件数
- KPI は `reports/spec-audit/ch4/logs/native-*-*.md` に実測値を記録し、差分が出た場合は `phase4-scenario-matrix.csv` の `resolution_notes` に反映する。
