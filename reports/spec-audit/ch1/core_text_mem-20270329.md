# Core.Text メモリ転送計測ログ（2027-03-29）

- 実行コマンド:  
  `python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario ownership_transfer --source reports/spec-audit/ch1/core_text_mem.json --output reports/text-mem-metrics.json --require-success`
- 目的: `Bytes::from_vec` / `Bytes::from_slice` / `Str::to_bytes` / `String::into_bytes` 経路で `collector.effect.transfer` と `collector.effect.mem_bytes` を検証し、`text.mem.zero_copy_ratio` / `text.mem.copy_penalty_bytes` のベースラインを確保する。

| ケース | 入力サイズ | `collector.effect.transfer` | `collector.effect.mem_bytes` | 備考 |
| --- | --- | --- | --- | --- |
| `bytes_from_vec_zero_copy` | 6 KB | true | 0 | `Vec<u8>` → `Bytes::from_vec`。ゼロコピー率の分子。 |
| `bytes_from_slice_copy` | 6 KB | false | 6144 | `Bytes::from_slice`（コピー）。`mem_bytes` は入力サイズと一致。 |
| `str_to_bytes_copy` | 6 KB | false | 6144 | `Str::to_bytes` 経路。今後 `EffectSet` で自動記録予定。 |
| `string_into_bytes_zero_copy` | 6 KB | true | 0 | `String::into_bytes`。`Bytes::from_vec` と同じくゼロコピー。 |

集計結果（`reports/text-mem-metrics.json`）:

- `text.mem.zero_copy_ratio = 0.82` （`zero_copy_cases = 9` / `total_cases = 11`）  
- `text.mem.copy_penalty_bytes = 512` （B/KB。`Bytes::from_slice` / `Str::to_bytes` のサンプル平均）

次アクション:

1. `effect {mem}` 計測を `Bytes::from_slice` / `Str::to_bytes` / `String::from_str` の実装へ統合する（W42 予定）。  
2. `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone` の結果を本ログに追記し、`UnicodeErrorKind::OutOfMemory` の検証ケースを追加する。  
3. `reports/text-mem-metrics.json` を `phase3-core-text` CI でアーティファクト化し、`0-3-audit-and-metrics.md` の `text.mem.*` KPI を自動更新する。
