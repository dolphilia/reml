# 3.3 Core Text & Unicode ギャップ是正計画

## 目的
- `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` で未着手となっている実装ギャップ（ゼロコピー計測、ストリーミング decode、Grapheme キャッシュ監査）を埋め、仕様（`docs/spec/3-3-core-text-unicode.md`）と Rust 実装 (`compiler/rust/runtime/*`) の整合を確保する。
- `effect {mem}`/`effect {io}`/`effect {audit}` の記録方針と監査ログ (`AuditEnvelope`, `reports/spec-audit/*`) を統一し、Phase 3 KPI（`text.mem.zero_copy_ratio`, `text.grapheme.cache_hit` 等）を自動検証できる状態にする。
- Reml ランタイム上で Core.Text API をセルフホスト運用へ引き上げるための基盤差分を整理し、後続の Diagnostics / IO / Parser 連携タスクに引き継ぐ。

## 対象ギャップ
1. **ゼロコピー経路の EffectSet 記録不足**  
   - `Bytes::from_vec`/`String::into_bytes`/`TextBuilder::finish` が `EffectSet` へムーブ転送を記録していない。`text.mem.zero_copy_ratio` の算出根拠が欠落。
2. **`decode_stream` のバッファ一括読み出し**  
   - 現行実装は `Vec<u8>` へ全読み込みしてから `String::from_utf8` しており、ストリーミング仕様（バックプレッシャー・InvalidSequenceStrategy・IO 効果）を満たしていない。
3. **Grapheme キャッシュと監査ログの連携不足**  
   - `GraphemeSeq` の `IndexCache` が `RuntimeCacheSpec` に沿った世代管理と Unicode バージョン不一致検出を持たず、`log_grapheme_stats` の `effect {audit}` 計測が `AuditEnvelope` へ自動反映されていない。

## 実施ステップ

### A. EffectSet 強化とゼロコピー KPI 取得
1. `compiler/rust/runtime/src/prelude/iter/mod.rs`  
   - `EffectSet` に `mark_transfer`（`collector.effect.transfer=true`）を追加し、`mem_bytes` を増やさずムーブ経路を識別できるようにする。`EffectLabels` との相互変換も更新。
2. `compiler/rust/runtime/src/text/{bytes.rs,text_string.rs,builder.rs}`  
   - `Bytes::from_vec`、`String::into_bytes`、`TextBuilder::finish_with_effects` で `mark_transfer` を呼び出し、`EffectsCollector` へゼロコピー経路の累積を送る。`finish_with_effects` の戻り値に `transfer` ビットを含め、`collect_text` ハーネスで参照可能にする。
3. KPI/ドキュメント更新  
   - `docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` に `effect.transfer` 列を追加。`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone` で `text.mem.zero_copy_ratio` を算出し、`reports/text-mem-metrics.json` にサンプル値を登録。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 解説を補強。

> 実施ログ（2027-03-31）  
> - `compiler/rust/runtime/src/prelude/iter/mod.rs` に `EffectSet::mark_transfer` / `contains_transfer` を追加し、`EffectLabels` へ `transfer` フィールドを拡張。`CollectorAuditTrail`（`prelude/collectors/mod.rs`）の JSON/Audit 出力にも `collector.effect.transfer` を含めた。  
> - `Bytes::from_vec`・`String::into_bytes`・`TextBuilder::finish_with_effects` でゼロコピー時に `transfer` を計測し、`text/effects.rs` に `record_transfer` を実装。`text/builder.rs` / `text/text_string.rs` / `text/bytes.rs` のテストへ `transfer` 断言を追加し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml` で検証した。  
> - `docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` に `effect.transfer` 列を追加し、`Bytes::from_vec`・`String::into_bytes`・`TextBuilder::finish` のゼロコピー経路を明示。`text.mem.zero_copy_ratio` の収集根拠として `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の記述を参照できる状態を確認した。

### B. ストリーミング decode/encode 再設計
1. ランタイム実装  
   - `compiler/rust/runtime/src/io/text_stream.rs` をチャンク単位の逐次 decode へ改修。`UTF-8` 検証を `std::str::from_utf8` ベースのスライディングウィンドウに変更し、`InvalidSequenceStrategy::Replace` 時は `�` を随時書き込む。`effects::record_io_operation` で得たバイト数を `EffectSet::mark_io` に転写する。
2. エラーハンドリング  
   - `UnicodeError` に `source: Option<IoError>` もしくは `context` を追加し、`IoErrorKind::UnexpectedEof` を `phase="io.decode.eof"` へマッピング。`InvalidSequenceStrategy::Replace` でも `effect {unicode}` を記録するよう `effects::record_unicode_event(bytes)` を新設。
3. 検証・サンプル  
   - `compiler/rust/runtime/tests/text_stream.rs` を拡張し、(i) 巨大入力の逐次 decode、(ii) `replace` モードでの `%FF` 分割、(iii) `EffectSet` に `io`/`mem`/`transfer` が適切に記録されることを検証。  
   - `compiler/rust/runtime/examples/io/text_stream_decode.rs` の CLI に `--chunk-size`/`--replace` を追加し、`tests/data/unicode/streaming/sample_input.txt` から生成する `reports/spec-audit/ch1/unicode_streaming_decode.json` に `effect` サマリを追記。`docs/plans/bootstrap-roadmap/checklists/text-api-error-scenarios.md` の TA-05/06 ケースを更新。

> 実施ログ（2027-04-02）  
> - `compiler/rust/runtime/src/io/text_stream.rs` にスライディングウィンドウ型の UTF-8 デコーダと BOM ポリシー適用ヘルパを追加し、`InvalidSequenceStrategy::Replace` が逐次的に `�` を挿入する実装へ移行した。`UnicodeError` には `IoError` ソースを保持させ、`IoErrorKind::UnexpectedEof` を `phase="io.decode.eof"` へ強制することで TA-05 の要件を満たしている。  
> - Text/IO 効果連携として `EffectSet` に `unicode` ビットを導入し、`text::take_text_effects_snapshot()` で観測できるよう `merge_text_effects`/`record_text_unicode_event` を追加した。`compiler/rust/runtime/tests/text_stream.rs` ではチャンク境界・`Replace` 分割・`EffectLabels` の 3 ケースを追加し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml text_stream` で回帰確認済み。  
> - `compiler/rust/runtime/examples/io/text_stream_decode.rs` に `--chunk-size`/`--replace`/`effects` 出力を実装し、`tests/data/unicode/streaming/sample_input.txt` を使った JSON には `EffectSnapshot` を同梱。CLI では `take_text_effects_snapshot` で単回実行の効果を収集し、`docs/plans/bootstrap-roadmap/checklists/text-api-error-scenarios.md` TA-05/06 / `docs/notes/text-unicode-gap-log.md` に反映した。

### C. Grapheme キャッシュと監査パイプライン統合
1. キャッシュ管理  
   - `compiler/rust/runtime/src/text/grapheme.rs` に `IndexCacheGeneration` 構造を追加し、`Unicode::VERSION`（`unicode_segmentation` のバージョン値）と `CACHE_VERSION` を結合した世代 ID を持たせる。`STORE` には `version` を保存し、異なるバージョンを検出した場合に `cache_miss += len` で再構築する。
2. 監査ログ自動化  
   - `log_grapheme_stats` 実行時に `effects::record_audit_event_with_metadata(stats)` を呼び、`CollectorAuditTrail` へ `text.grapheme_stats.*` を直接埋め込む。`compiler/rust/frontend/src/diagnostic/unicode.rs` ではメタデータ挿入の重複を避けるため、ランタイムから受け取った統計を優先。
3. テスト・スクリプト  
   - `compiler/rust/runtime/tests/text_internal_cache.rs` の UC-01〜03 を `#[ignore]` から段階的に有効化し、生成する `reports/spec-audit/ch1/core_text_grapheme_stats.json` を `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --require-success` で検証。  
   - `docs/spec/3-3-core-text-unicode.md` §4.1 / `docs/notes/text-unicode-ownership.md` にキャッシュ世代と監査ログの相互関係を脚注追加。`docs/plans/bootstrap-roadmap/checklists/unicode-cache-cases.md` をアップデート。

## 成果物と受付条件
- EffectSet/Collector が `transfer` ビットと `text.mem.*` KPI を自動集計できること (`reports/text-mem-metrics.json` の閾値更新)。
- `decode_stream` が逐次処理＋IO効果を備え、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml text_stream` が改修後仕様をカバーすること。
- Grapheme キャッシュの世代管理・監査ログ自動配線が完成し、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats` が CI で緑となること。
- 変更内容と KPI 結果を `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` および関連チケットから参照できるよう、本計画書へのリンクを Phase 3 トラッキング表に追記。
