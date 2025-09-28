# CLI コマンド例

```bash
# Experimental stage を有効化
reml run -Zalgebraic-effects test async::collect_logs --effects-debug

# Capability opt-in
reml capability enable console --stage experimental
reml capability enable foreign-call --stage experimental

# Stage 昇格
reml capability stage promote console --to beta
reml capability stage promote foreign-call --to beta

# Stable 化
reml capability stage promote console --to stable
reml capability stage promote foreign-call --to stable
```

各コマンドは `Diagnostic.extensions["effects"].stage` の値を確認しながら実行することを推奨します。
