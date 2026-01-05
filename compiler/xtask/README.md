# xtask

開発支援用の xtask クレートです。`cargo xtask` 経由で監査や補助タスクを実行します。

## 実行例
```
cargo xtask prelude-audit --wbs 2.1b --strict --baseline docs/spec/3-1-core-prelude-iteration.md
```

`--wbs` や `--section` を変更すると、特定範囲のみを対象にできます。未実装が残っている状態で `--strict` を指定すると非ゼロ終了になります。

## 関連
- `docs/plans/bootstrap-roadmap/assets/prelude_api_inventory.toml`
