# Core.Text ベンチマークレポート

このディレクトリは Core.Text / Core.Unicode 実装の性能測定ログを保存する。`benchmarks/Cargo.toml` で定義した `criterion` ベンチから生成される統計を Markdown 形式で記録し、Phase 2 ベンチマークとの差分を追跡する。

## 実行手順
1. `cargo bench --manifest-path benchmarks/Cargo.toml text::* -- --save-baseline phase3-core-text` を実行して正規化・グラフェム・TextBuilder の 3 シナリオを測定する。
2. `target/criterion/` に出力された `raw.csv` と `report/index.html` を確認し、MB/s・ns/char・cache hit 等の代表値を `phase3-baseline.md` へ転記する。±15% を超える回帰を検知した場合は `docs/notes/text-unicode-performance-investigation.md` に原因を記録する。
3. 転記後は `git add reports/benchmarks/core_text/*.md` を実行し、当該コミットを `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` の §6.2 実施ログへリンクする。

## ファイル構成
- `phase3-baseline.md`: Phase 3 の基準値と最新測定値。フォームごとの目標値や ±15% 閾値を明示する。
- 将来的に回帰調査を行った場合は `YYYYMMDD-investigation.md` を追加し、測定条件・結果・フォローアップを記録する。
