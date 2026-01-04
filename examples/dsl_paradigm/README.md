# DSL パラダイム参照 DSL

`Core.Dsl.*` パラダイムキットの最小利用例をまとめた参照 DSL 群です。Phase 4 の回帰・監査ログの足掛かりとして、最小構成で「どのキットを使うか」を明示します。

## 収録内容

- `mini_ruby/`: `Core.Dsl.Object` + `Core.Dsl.Gc` を使う OOP DSL の最小例。
- `mini_erlang/`: `Core.Dsl.Actor` + `Core.Dsl.Gc` を使うアクター DSL の最小例。
- `mini_vm/`: `Core.Dsl.Vm` + `Core.Dsl.Object` を使う VM DSL の最小例。

## 運用メモ

- `.reml` は最小記述を優先し、監査イベントや Stage 条件は `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に集約します。
- 出力スナップショットは `expected/dsl_paradigm/` に置き、対応表は `expected/dsl_paradigm/README.md` を参照してください。
