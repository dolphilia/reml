# Core.Text メモリ転送計測ログ（2027-03-29）

- 実行コマンド:  
  `python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-source reports/spec-audit/ch1/core_text_grapheme_stats.json --text-mem-source reports/text-mem-metrics.json --output reports/text-mem-metrics.json --require-success`
- 目的: `Bytes::from_vec` / `Bytes::from_slice` / `Str::to_bytes` / `String::into_bytes` と `String::clone` OOM 経路で `collector.effect.transfer` と `collector.effect.mem_bytes` を検証し、`text.mem.zero_copy_ratio` / `text.mem.copy_penalty_bytes` / TA-02 アラートを自動化する。

| ケース | 入力サイズ | `transfer` | `mem_bytes` | 備考 |
| --- | --- | --- | --- | --- |
| `bytes_from_vec_zero_copy` | 10 KB | true | 0 | `Vec<u8>` → `Bytes::from_vec`。ゼロコピー率の分子。 |
| `bytes_from_slice_copy` | 2.2 KB | false | 1120 | `Bytes::from_slice`（コピー）。`mem_bytes` は入力サイズの 1/2。 |
| `str_to_bytes_copy` | 2.2 KB | false | 1120 | `Str::to_bytes` 経路。`EffectSet` 測定と連動。 |
| `string_into_bytes_zero_copy` | 10 KB | true | 0 | `String::into_bytes`。`Bytes::from_vec` と同じくゼロコピー。 |
| `bytes_clone_out_of_memory` | 128 KB | false | — | TA-02 (`String::clone`) の `UnicodeErrorKind::OutOfMemory` が `handled=true` で記録されることを確認。 |

集計結果（`reports/text-mem-metrics.json`）:

- `text.mem.zero_copy_ratio = 0.82` （ゼロコピー対象 20,480B / 24,960B）  
- `text.mem.copy_penalty_bytes = 512` （B/KB。`Bytes::from_slice` / `Str::to_bytes` の合計 `mem_bytes` を入力サイズで正規化）  
- `bytes_clone` シナリオ: `UnicodeErrorKind::OutOfMemory` ケース 1 件 / ハンドル済み 1 件。`collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json` により検証。

次アクション:

1. `effect {mem}` 計測を `Bytes::from_slice` / `Str::to_bytes` / `String::from_str` の実装へ統合する（W42 予定）。  
2. `bytes_clone` シナリオを CI へ常設し、TA-02 `String::clone` OOM の回帰検知に組み込む。  
3. `reports/text-mem-metrics.json` を `phase3-core-text` CI でアーティファクト化し、`0-3-audit-and-metrics.md` の `text.mem.*` KPI を自動更新する。
