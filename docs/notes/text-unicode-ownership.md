# Core.Text 所有権・参照モデルメモ

## 目的
Bytes/Str/String/GraphemeSeq/TextBuilder がどのように `Vec<u8>` / キャッシュを共有するかを整理し、`effect {mem}`・`effect {mut}`・`unsafe` の要否を判断する。

## モデル概要
- **Bytes**: `Arc<[u8]>` + slice。`Bytes::from_vec` は所有権を奪い `effect {mem}` を発火しない。コピー時は `Arc` の参照カウントのみ増加。
- **Str**: `Bytes` への参照で UTF-8 を保証。`Str::to_string` は `effect {mem}` を記録し `String` を返す。
- **String**: `Vec<u8>` を所有。`String::into_bytes` はゼロコピーで `Bytes` を返すが、`effect {mem}` の打刻ルールは `text-api-error-scenarios.md` を参照。
- **GraphemeSeq**: `Bytes` 共有 + `IndexCache`。`clone` 時は cache を共有するが invalidation ルールは `unicode-cache-cases.md` に従う。
- **TextBuilder**: `Vec<u8>` バッファを保持。`finish` 後に `String` を返し、`effect {mut}` → `effect {mem}` を順番に記録する。

## 決定事項
1. `Bytes`/`String` 間のゼロコピー経路では `unsafe` を使わず `Arc`/`Vec` の `into_raw_parts` を使用する。メモリ削減より安全性を優先。
2. `TextBuilder` が `GraphemeSeq` を生成する際は `IndexCache` を共有し、`cache_generation` カウンタを `log_grapheme_stats` に出力する。
3. `effect {mem}` の算出は `GrowthBudget`（別メモ）を流用し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI と一致させる。

## オープン課題
- [ ] `String::try_reserve_exact` を公開するか検討（OutOfMemory エラー伝搬）。
- [ ] `TextBuilder` スレッドセーフ化（`Send`/`Sync` 実装）と Capability ガードの整合性確認。
- [ ] `BytesMut` 相当の一時バッファ導入による `effect {mut}` 最適化調査。
