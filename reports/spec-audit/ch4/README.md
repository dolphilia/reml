# Phase 4 Spec Audit（ch4）

Phase 4 の `.reml` シナリオを自動実行し、`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` と突き合わせた結果を集約するディレクトリです。`tooling/examples/run_phase4_suite.py`（`run_examples.sh --suite spec_core|practical` から呼び出し）によって以下のレポートが生成されます。

| ファイル | 内容 |
| --- | --- |
| `spec-core-dashboard.md` | Chapter 1（構文・型・効果）向け `examples/spec_core/` の実行結果と Diagnostics の照合状況 |
| `practical-suite-index.md` | Chapter 3（標準ライブラリ・実務ケース）向け `examples/practical/` の実行結果と Diagnostics の照合状況 |

各レポートにはシナリオ ID、入力パス、期待される `diagnostic_keys`、実際に出力された Diagnostics のコード、CLI の終了コードが Markdown 表で記録されます。`diagnostic_keys` に差分がある場合は `❌ fail` として強調され、未実装機能や仕様差異を洗い出す指標として利用します。

> 実行手順: `tooling/examples/run_examples.sh --suite spec_core` または `--suite practical` を実行すると、`reports/spec-audit/ch4` 配下の対応レポートが更新されます。失敗シナリオが存在する場合は exit 1 で終了し、CI でも検知できるようにしています。
