# Core.Iter Numeric チェックリスト

`docs/plans/bootstrap-roadmap/3-4-core-numeric-time-plan.md` §3.3 に対応するテストケースを集約する。

| Case ID | 項目 | 入力/条件 | 期待結果 | ステータス | コマンド |
| --- | --- | --- | --- | --- | --- |
| CIN-01 | `rolling_average` が手計算結果と一致 | 固定乱数シーケンス × `window = 1..5` | `iter_numeric_props::rolling_average_matches_manual_samples` が全域通過 | ✅ | `cargo test --manifest-path compiler/runtime/Cargo.toml --features core-numeric iter_numeric_props::rolling_average_matches_manual_samples` |
| CIN-02 | `z_score` の数値安定性 | 代表サンプル 5 ケース | `iter_numeric_props::z_score_matches_reference_samples` が成功 | ✅ | `cargo test --manifest-path compiler/runtime/Cargo.toml --features core-numeric iter_numeric_props::z_score_matches_reference_samples` |
| CIN-03 | `effect {mem}` 記録確認 | `window = 3`, `values = [1,2,3,4]` | `take_numeric_effects_snapshot().mem = true` かつ `mem_bytes >= 3*sizeof(f64)` | ✅ | `cargo test --manifest-path compiler/runtime/Cargo.toml --features core-numeric iter_numeric_props::rolling_average_records_mem_effect` |
| CIN-04 | `NumericCollector` Stage 整合 | `Iter::collect_numeric` 実行 | `CollectorStageSnapshot.kind = "numeric"` で Stage mismatch なし（`iter_numeric_props` 内で暗黙検証） | ✅ | `cargo test --manifest-path compiler/runtime/Cargo.toml --features core-numeric iter_numeric_props` |

> Responsible: Runtime/Core Numeric ワークストリーム（当面: @bootstrap-rust-team）  
> 更新履歴: 2025-12-07 作成。新規ケース追加時は `docs/notes/runtime/core-numeric-time-gap-log.md` と同期すること。
