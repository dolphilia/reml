# Core Collections サンプル

`Core.Collections` モジュールで定義された永続／可変コレクション API を組み合わせた動作例です。`List` → `Map` → `Vec` → `Table` → `Cell`/`Ref` の各変換を `examples/core-collections/usage.reml` に記述し、効果タグ（`effect {mem}` / `effect {mut}` / `effect {cell}` / `effect {rc}`）の伝搬と `CollectError` の取り込み方を確認できるようにしています。

## 内容

- `List.push_front` でステージ順を積んだあと `Map.from_pairs` で定数マップを生成し、`CollectError::DuplicateKey` を明示的に扱う流れを示します。
- `List.to_iter` → `Vec.collect_from` で永続構造から `Vec` への変換を行い、`Vec` を元に `Table` を構築して `Map` へ変換するまでのパイプラインを追えます。
- `Cell` / `Ref` を使ってカウンターと共有参照を管理し、`effect {cell}` / `effect {rc}` の観察ポイントをコメントで解説しています。

## 実行

```sh
cargo run --bin reml -- examples/core-collections/usage.reml
```

上記コマンドは Phase 3 の `reml` CLI が稼働している前提です。`docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §6 で定義した CI パイプライン (例: `collect-iterator-audit-metrics.py --scenario core_collections_example`) が整った段階で自動実行を追加する予定です。

このサンプルは `Table.load_csv` / `collect_table_csv` シナリオの効果タグ（`effect {io}`/`effect {mut}`）とも連携する想定で、`reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` の KPI `collector.effect.io` の検証路線と結びつけられます。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の `csv_load_latency` 目標にも整合するように更新しました。

## 合致するドキュメント

- `docs/spec/3-2-core-collections.md` §2.3 にある `Map.from_pairs` の説明
- `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` §6 で計画されたサンプル検証の成果
