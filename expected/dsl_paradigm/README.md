# DSL パラダイム参照 DSL 出力スナップショット

`examples/dsl_paradigm/` の最小 DSL に対応する出力・監査ログのスナップショット一覧です。監査イベントの有無は Phase 4 の回帰観点として扱い、`phase4-scenario-matrix.csv` と 1:1 対応させます。

| DSL | 入力 | stdout | 監査ログ | 備考 |
| --- | --- | --- | --- | --- |
| Mini-Ruby | `examples/dsl_paradigm/mini_ruby/mini_ruby_basic.reml` | `expected/dsl_paradigm/mini_ruby/mini_ruby_basic.stdout` | `expected/dsl_paradigm/mini_ruby/mini_ruby_basic.audit.jsonl` | `dsl.object.dispatch` / `dsl.gc.root` を確認対象とする。 |
| Mini-Erlang | `examples/dsl_paradigm/mini_erlang/mini_erlang_basic.reml` | `expected/dsl_paradigm/mini_erlang/mini_erlang_basic.stdout` | `expected/dsl_paradigm/mini_erlang/mini_erlang_basic.audit.jsonl` | `dsl.actor.mailbox` / `dsl.gc.root` を確認対象とする。 |
| Mini-VM | `examples/dsl_paradigm/mini_vm/mini_vm_basic.reml` | `expected/dsl_paradigm/mini_vm/mini_vm_basic.stdout` | `expected/dsl_paradigm/mini_vm/mini_vm_basic.audit.jsonl` | `dsl.vm.execute` / `dsl.object.dispatch` を確認対象とする。 |

> 監査ログの JSON Lines 形式は `docs/spec/3-6-core-diagnostics-audit.md` の `AuditEvent` に準拠します。内容は Phase 4 実行ログの確定後に更新してください。
