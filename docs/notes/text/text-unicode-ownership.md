# Core.Text 所有権・参照モデルメモ

## 目的
Bytes/Str/String/GraphemeSeq/TextBuilder がどのように `Vec<u8>` / キャッシュを共有するかを整理し、`effect {mem}`・`effect {mut}`・`unsafe` の要否を判断する。

## モデル概要
- **Bytes**: `Vec<u8>` を直接所有する薄いラッパー。`Bytes::from_vec` は所有権を奪い `effect {mem}` を発火しないが、`from_slice` などコピー経路では `mem` を記録する。
- **Str**: `Cow<'a, str>` に基づく UTF-8 スライス。`Str::to_bytes` など確保を伴う API では `effect {mem}` を記録し、借用経路ではゼロコピーを維持する。
- **String**: `std::string::String` を包む。`into_bytes` はゼロコピーで `Bytes` を返し、`from_str`/`to_bytes` などコピー経路は `mem` 加算対象。
- **GraphemeSeq**: `Str` を参照する `Cow<'a, str>` + `Vec<GraphemeCluster>` + `Vec<usize>` キャッシュ。`clone` 時にメタデータを複製するが元 `Bytes` は共有する。
- **GraphemeSeq::stats**: `cache_generation`/`cache_version`/`unicode_version`/`version_mismatch_evictions` を出力し、`log_grapheme_stats` 経由で `text.grapheme_stats` メタデータに転写する。`IndexCacheGeneration` は `unicode_segmentation::UNICODE_VERSION` と `CACHE_VERSION` を結合した ID を持ち、バージョン不一致時は `version_mismatch_evictions += 1` を記録して再構築する。
- **TextBuilder**: `Vec<u8>` バッファを保持し、`push_*` のたびに `effect {mem, mut}` を更新。`finish` 後に `String` を返す際はゼロコピーで `Bytes` へ譲渡する。

## 所有権遷移マトリクス
| エントリ | 実装位置 | ゼロコピー | `effect {mem}` | 備考 |
| --- | --- | --- | --- | --- |
| `Vec<u8> -> Bytes::from_vec` | `compiler/runtime/src/text/bytes.rs` L12-L27 | ✅ | `false`。`collector.effect.transfer=true` のみ記録 | `Vec` をムーブ。UTF-8 検証は呼ばない。 |
| `slice -> Bytes::from_slice` / `Bytes::slice` | 同 L19-L52 | ❌ | `true` (`mem_bytes += len`) | `EffectSet::record_mem_bytes` を追加予定。 |
| `Bytes::decode_utf8` / `Str::from(&str)` | `bytes.rs` L55-L63, `str_ref.rs` L11-L35 | ✅ | `false` | UTF-8 検証のみ。`UnicodeError` で失敗時も `mem` 不変。 |
| `Bytes::into_utf8` / `into_string` | `bytes.rs` L63-L74 | ✅ | `false` | `String::from_utf8` で `Vec` をムーブ。`Str<'static>` を経由。 |
| `Str::to_bytes` / `String::to_bytes` | `str_ref.rs` L20-L30, `text_string.rs` L20-L38 | ❌ | `true` (`mem_bytes += len`) | `Bytes::from_slice` を内部で使用。 |
| `Str::into_owned` / `String::from_str` | `text_string.rs` L16-L30 | ❌ | `true`（`to_owned` でコピー） | 仕様上 `String` へ昇格する際に確保を伴う。 |
| `String::into_bytes` | `text_string.rs` L36-L45 | ✅ | `false` | `Bytes::from_vec` を呼び、測定値はゼロコピーにカウント。 |
| `TextBuilder::finish` | `text/builder.rs` L3-L38 | ✅ | `false` | `finish` 時は `effect {mut}` の開放扱い。push 時に `mem_bytes` を記録。 |
| `TextBuilder::push_*` | 同 L20-L34 | ❌ | `true`（再確保ぶんを積算） | `GrowthBudget` で `reserve`/`push` を KPI 化。 |
| `segment_graphemes` | `text/grapheme.rs` L35-L134 | ❌（メタデータ確保） | `true`（クラスタ数×メタ情報分） | 本体バイト列は `Str` を参照。 |

`effect {mem}` の定義は `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` §1.2.1 を参照し、ゼロコピー経路は KPI `text.mem.zero_copy_ratio` の分子として扱う。`text.mem.copy_penalty_bytes` は `copy_cases[]` の平均 `collector.effect.mem_bytes` を 1KB あたりのバイト数として算出する。

## `Vec<u8>` 再利用ルール
1. `Bytes::from_vec` / `String::into_bytes` / `TextBuilder::finish` は `Vec` をムーブするだけなので `unsafe` を使わずにゼロコピー判定 (`collector.effect.transfer=true`) を出力する。  
2. `Bytes::into_utf8` など `Str<'static>` を作る場合は `String` を一度生成し `Cow::Owned` に包む。`Str` が借用を保持できる場合は `Cow::Borrowed` を維持し、追加確保を避ける。  
3. `TextBuilder` の `reserve`/`push_*` で行った `mem_bytes` 記録を `finish` 時に再度打刻しない。`EffectSet` 側に `finalize_without_mem()`（仮名）を追加して二重計測を防ぐ。  
4. `Vec<u8>` と `IndexCache` を共有しない。`GraphemeSeq` の `byte_offsets` は別 `Vec<usize>` として構築し、キャッシュライフサイクルは `unicode-cache-cases.md` で追跡する。

## TextBuilder / GraphemeSeq の参照モデル
- `TextBuilder` は `Vec<u8>` のみで構築し、`finish` → `Bytes::from_vec` → `String` の順で所有権を受け渡しているため `unsafe` は不要。  
- `GraphemeSeq` は `Cow<'a, str>` を保持する `GraphemeCluster` と `byte_offsets: Vec<usize>` を持ち、元文字列のライフタイム `'a` と整合する。TextBuilder で生成した `String` から `Str::from(&string)` を介して `GraphemeSeq` を作る限り、`Bytes` と `GraphemeSeq` の間で `Vec` を共有しない。  
- `log_grapheme_stats` の `cache_hits`/`cache_miss` は `GraphemeSeq::stats` の結果をそのまま返すので、Phase 3 では `collector.effect.text_cache_hits` を `AuditEnvelope.metadata["text.grapheme_stats.cache_hits"]` に、`EffectSet` の新ビット `unicode_cache` に写す。`effects::record_audit_event_with_metadata` で `collector.effect.audit` と `text.grapheme_stats.*` を `CollectorAuditTrail` へ直接挿入し、Diagnostics 側での二重計測を避ける。  
- `TextBuilder` 経由の `Iter::collect_text` では `effect {unicode}`（新ビット）と `effect {mem}` を分離し、`TextBuilderCollector` が `IterStage::Streaming` を引き継ぐ。`docs/plans/bootstrap-roadmap/checklists/textbuilder-api-draft.md` で API 定義を更新済み。
- `GraphemeSeq::stats` は `ScriptCategory`/`TextDirection` を保持する `Grapheme` メタデータから `primary_script`・`script_mix_ratio`・`rtl_ratio` を算出し、`reports/spec-audit/ch1/core_text_grapheme_stats.json` に書き戻す。`tests/grapheme_conformance.rs` で UAX #29 サブセットを用いた回帰テストを実施する。

## KPI・監査ログ
- `text.mem.zero_copy_ratio`: `reports/text-mem-metrics.json` にゼロコピーケースと `mem_bytes` を書き出し、`collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json` で算出。閾値 0.65 以上を Phase3-2 の合格ラインとする。  
- `text.mem.copy_penalty_bytes`: 同 JSON のコピーケースを 1KB 入力あたりに正規化。1,024B/KB を超えた場合は `0-4-risk-handling.md#core-text-copy-penalty` に記録。  
- `text.grapheme.cache_hit`: `log_grapheme_stats` の `cache_hits/(cache_hits+cache_miss)`。`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats` で計測。

## エラー遷移
- `Bytes::decode_utf8` は `UnicodeErrorKind::InvalidUtf8` に投げ替え、`Str::from_bytes` でも同じエラーを返す。  
- `TextBuilder::finish` の失敗は `UnicodeErrorKind::InvalidUtf8` / `OutOfMemory` を返し、`collector.effect.mem_bytes` を直前までの合計で固定したまま `EffectSet::freeze()`（予定）を呼ぶ。  
- `segment_graphemes` は `UnicodeSegmentation` が panic しない想定だが、`UnicodeResult` 互換の `GraphemeError` を導入する余地を `docs/plans/bootstrap-roadmap/checklists/unicode-error-mapping.md` に記録済み。

## 決定事項
1. `Bytes`/`String` 間のゼロコピー経路では `unsafe` を使わず `Vec::into_raw_parts` 系の標準 API で所有権を移譲する。メモリ削減より安全性を優先。
2. `TextBuilder` が `GraphemeSeq` を生成する際は `IndexCache` を共有し、`cache_generation` カウンタを `log_grapheme_stats` に出力する。
3. `effect {mem}` の算出は `GrowthBudget`（別メモ）を流用し、`docs/guides/tooling/audit-metrics.md` の KPI と一致させる。

## オープン課題
- [ ] `String::try_reserve_exact` を公開するか検討（OutOfMemory エラー伝搬）。
- [ ] `TextBuilder` スレッドセーフ化（`Send`/`Sync` 実装）と Capability ガードの整合性確認。
- [ ] `BytesMut` 相当の一時バッファ導入による `effect {mut}` 最適化調査。
