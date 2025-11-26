# 3.3 Core Text & Unicode 実装計画

## 目的
- 仕様 [3-3-core-text-unicode.md](../../spec/3-3-core-text-unicode.md) に基づく `Core.Text`/`Core.Unicode` API を Reml 実装へ反映し、文字列三層モデル (Bytes/Str/String) と `GraphemeSeq`/`TextBuilder` の挙動を統一する。
- Unicode 正規化・セグメンテーション・ケース変換を標準化し、Parser/Diagnostics/IO との相互運用を保証する。
- 文字列関連の監査ログ (`log_grapheme_stats`) とエラー (`UnicodeError`) の連携を整備し、仕様と実装の差分を可視化する。

## スコープ
- **含む**: 文字列三層モデル、Builder、Unicode 正規化、ケース/幅変換、診断変換、IO ストリーミング decode/encode、監査ログ API。
- **含まない**: 正規表現エンジン本体、ICU 依存機能のカスタム拡張、非 UTF-8 エンコーディング (将来のプラグインに委譲)。
- **前提**: 3-1/3-2 で提供される `Iter`/`Collections` の実装が利用可能であり、IO/Diagnostics モジュールの基盤が Phase 2 から提供されていること。

## 作業ブレークダウン

### 1. 仕様差分整理と内部表現設計（41週目）
**担当領域**: 設計調整

1.1. `Bytes`/`Str`/`String`/`GraphemeSeq`/`TextBuilder` の API 一覧と効果タグを抜き出し、既存実装との差分を洗い出す。  
実施ステップ:  
- `docs/spec/3-3-core-text-unicode.md` と `compiler/rust/runtime/src/text/` 以下を比較し、API 名・引数・戻り値・効果タグ・関連する `Result` 型を CSV で整理する（`docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` を更新）。  
- `rg "pub struct"` などで Rust 実装の公開 API を抽出し、`docs/plans/rust-migration/unified-porting-principles.md` の「振る舞いの同一性 > 設計の同一性」の順にソートして差分調査ログを `docs/notes/text-unicode-gap-log.md` に追記する。  
- 差分ごとに「Rust 実装で欠落」「仕様が古い」「要議論」のタグを付与し、`docs/plans/bootstrap-roadmap/README.md` の Phase 3 トラッキング表へリンクを登録する。

1.2. 文字列所有権モデル (コピー時の `effect {mem}`) を確認し、`Vec<u8>` の再利用方針を決める。  
実施ステップ:  
- `Bytes`→`Str`→`String` の変換ごとに発生するアロケーションと `effect {mem}` を本文に記述し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Phase 3 指標へメモリ KPI を追加する。  
- `Bytes::from_vec` `String::into_bytes` などのゼロコピー経路を列挙し、`Vec<u8>` を移譲するパスで `effect {mem}` を打刻しない条件を `docs/spec/0-1-project-purpose.md` の性能指標と照合する。  
- `TextBuilder`/`GraphemeSeq` が `Vec<u8>` を共有する場合の `unsafe` 有無を決定し、`docs/notes/text-unicode-ownership.md` に参照カウント方針と `Result` のエラー遷移を図示する。

#### 1.2.1 所有権遷移と `effect {mem}` 判定
`Bytes`/`Str`/`String`/`TextBuilder`/`GraphemeSeq` が利用している所有権パスを洗い出し、`effect {mem}` の記録条件を以下の表に整理した。Rust 実装のコード参照を併記し、ゼロコピー経路であるかどうかを明示する。

| 経路 | アロケーション | `effect {mem}` 判定 | 根拠 |
| --- | --- | --- | --- |
| `Vec<u8> → Bytes::from_vec` | なし（`Vec` ムーブ） | `false`。所有権移譲のみで追加確保なし | `compiler/rust/runtime/src/text/bytes.rs` L12-L27【F:../../compiler/rust/runtime/src/text/bytes.rs†L12-L27】 |
| `slice → Bytes::from_slice` / `Bytes::slice` | `slice.to_vec()` で新規確保 | `true`。コピーで確保したサイズを `EffectSet::record_mem_bytes(bytes.len())` へ送る | 同 L19-L52【F:../../compiler/rust/runtime/src/text/bytes.rs†L19-L52】 |
| `Bytes::decode_utf8` / `Str::from(&str)` | ゼロコピー参照 (`&str`/`Cow::Borrowed`) | `false`。UTF-8 検証のみで `mem` 増加なし | `bytes.rs` L55-L63, `str_ref.rs` L11-L35【F:../../compiler/rust/runtime/src/text/bytes.rs†L55-L63】【F:../../compiler/rust/runtime/src/text/str_ref.rs†L11-L35】 |
| `Bytes::into_utf8` / `Bytes::into_string` | `String::from_utf8` によるムーブ | `false`。`Vec<u8>` を `String` へ移譲するだけで追加確保なし | `bytes.rs` L63-L74【F:../../compiler/rust/runtime/src/text/bytes.rs†L63-L74】 |
| `Str::to_bytes` / `String::to_bytes` | `Bytes::from_slice` でコピー | `true`。`Str`/`String` から `Vec<u8>` を生成するたびに `mem_bytes += len` | `str_ref.rs` L20-L30, `text_string.rs` L20-L38【F:../../compiler/rust/runtime/src/text/str_ref.rs†L20-L30】【F:../../compiler/rust/runtime/src/text/text_string.rs†L20-L38】 |
| `Str::into_owned` / `String::from_str` | `String::from_std` が `to_owned` を呼ぶ | `true`。UTF-8 検証後に `String` へコピーしたサイズを `EffectSet` に書く | `text_string.rs` L16-L30【F:../../compiler/rust/runtime/src/text/text_string.rs†L16-L30】 |
| `String::into_bytes` | `String::into_bytes` → `Bytes::from_vec` (ムーブ) | `false`。所有権移譲のみ | `text_string.rs` L36-L45【F:../../compiler/rust/runtime/src/text/text_string.rs†L36-L45】 |
| `TextBuilder::finish` | `Bytes::from_vec` 経由で `Vec` を移譲 | `false`。`finish` では `effect {mut}`→`effect {mem}` の順で `TextBuilder` 側が計測するのみ | `text/builder.rs` L3-L38【F:../../compiler/rust/runtime/src/text/builder.rs†L3-L38】 |
| `TextBuilder::push_bytes/str/grapheme` | `Vec::extend_from_slice` により `realloc` の可能性 | `true`。追加バイト数を `EffectSet::record_mem_bytes` に積算 | 同 L20-L34【F:../../compiler/rust/runtime/src/text/builder.rs†L20-L34】 |
| `segment_graphemes` / `GraphemeSeq::stats` | `Vec<GraphemeCluster>`/`Vec<usize>` を都度生成 | `true`。クラスタ数×メタデータ分を `effect {mem}` へ記録、`Bytes` 本体は共有 | `text/grapheme.rs` L35-L134【F:../../compiler/rust/runtime/src/text/grapheme.rs†L35-L134】 |

これらの結果を `docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` に反映し、ゼロコピー経路の比率を新しい KPI (`text.mem.zero_copy_ratio`) で監視する。`effect {mem}` の算出に用いる `EffectSet::record_mem_bytes` / `CollectorEffectMarkers.mem_bytes` は `Core.Iter` 経由の `collect_text` ハーネスから観測できるようにし、Phase 3 では `reports/text-mem-metrics.json` に `Bytes.from_slice` / `Str.to_bytes` ケースの期待値を保存する。

#### 1.2.2 `Vec<u8>` 再利用ポリシー
- `Bytes::from_vec` および `String::into_bytes` は所有権をムーブするため、`Vec<u8>` を二重で解放しないよう `Arc` などの追加レイヤは導入しない。`EffectSet` 側では `mem_bytes` を更新せず `collector.effect.transfer=true`（新ビット）を記録してゼロコピーを識別する。  
- `TextBuilder::finish` は内部 `Vec` を `Bytes::from_vec` に渡すだけであり、新規アロケーションなしで `String` へ渡る。`finish` の前段で `reserve`/`push_*` が `mem_bytes` を記録し、`finish` 呼び出し時は `effect {mem}` の追加打刻を禁止することで二重計上を防ぐ。  
- `Bytes::into_utf8` → `Str::owned` は `String` を一度生成してから `Str`（`Cow::Owned`）へ包むため、`Vec` の再利用を維持しつつ `Str` が `'static` を要求する場合のみバッファを複製する方針を `docs/notes/text-unicode-ownership.md` に明記した。

#### 1.2.3 TextBuilder / GraphemeSeq の共有戦略
`TextBuilder` は `Vec<u8>` を直接保持し、`finish` 時も `Bytes::from_vec` を経由するだけで `unsafe` を使っていない【F:../../compiler/rust/runtime/src/text/builder.rs†L3-L38】。一方 `GraphemeSeq` は `Cow<'a, str>` で元文字列を参照しつつ、インデックスと統計情報を `Vec` にコピーしている【F:../../compiler/rust/runtime/src/text/grapheme.rs†L35-L134】。したがって共有戦略は次のとおりとする。

1. `TextBuilder` 完了後に `String` → `Str<'static>` を経由して `GraphemeSeq` を構築する場合、`GraphemeCluster` は `Cow::Borrowed` で原文を参照するため `unsafe` は不要。  
2. `GraphemeSeq` のキャッシュ（`byte_offsets`）は `TextBuilder` のバッファとは切り離して管理し、`log_grapheme_stats` のキャッシュヒット率を集計する KPI (`text.grapheme.cache_hit`) を Phase 3 Week42 で導入する。  
3. 共有する `Vec<u8>` は `Bytes`/`String` 間のムーブに限定し、`GraphemeSeq` では `Bytes` を参照する `Str` を入り口として安全な借用関係を維持する。`unsafe` で `Vec::from_raw_parts` を露出させる案は却下した。

#### 1.2.4 KPI と監査ログへの反映
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `text.mem.zero_copy_ratio`（ゼロコピー経路の割合）と `text.mem.copy_penalty_bytes`（コピー経路で記録した `mem_bytes` の平均値）を追加し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json` を CI で実行する。  
- `CollectorAuditTrail` に `collector.effect.transfer` と `collector.effect.text_mem_bytes` を追加して `AuditEnvelope.metadata` に出力し、`scripts/validate-diagnostic-json.sh --suite text --pattern collector.effect.transfer` を新設する。  
- これらの連携手順は `docs/notes/text-unicode-ownership.md` へ反映済みで、`TextBuilder`/`GraphemeSeq` の参照モデルと TODO を同メモで追跡する。

1.3. 内部キャッシュ (コードポイント/グラフェムインデックス) の設計とテスト戦略を定義する。  
実施ステップ:  
- `GraphemeSeq` 用の `IndexCache`（コードポイント→書記素クラスタ開始位置）を `RuntimeCacheSpec`（`docs/notes/core-library-outline.md`）と整合させ、キャッシュ無効化条件を図示する。  
- キャッシュ命中率を収集するため `log_grapheme_stats` に `cache_hits`/`cache_miss` を追加し、`tooling/ci/collect-iterator-audit-metrics.py --section text` で KPI 化する。  
- `cargo test text_internal_cache -- --ignored` を追加して大規模入力・キャッシュ無効化・多言語ケースを検証し、テストケースごとに `docs/plans/bootstrap-roadmap/checklists/unicode-cache-cases.md` を更新する。

> 進行ログ（Phase3 W41）  
> - `docs/notes/core-library-outline.md#runtimecachespeccoretext-キャッシュモデル` に `RuntimeCacheSpec` を追加し、`IndexCache` の世代管理と `Unicode::VERSION` 不一致時の無効化条件を図示した。  
> - `docs/spec/3-3-core-text-unicode.md` §4.1.1 / §5 へ `cache_hits`/`cache_miss`/`generation` を含む `log_grapheme_stats` 仕様を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に KPI `text.grapheme.cache_hit` を登録した。  
> - `docs/plans/bootstrap-roadmap/checklists/unicode-cache-cases.md` に UC-01〜03 の手順 (`cargo test --manifest-path compiler/rust/runtime/Cargo.toml text_internal_cache -- --ignored UC_0X`, `scripts/ci/run_core_text_regressions.sh --case streaming`, `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats`) を明文化し、`reports/spec-audit/ch1/core_text_grapheme_stats.json` への転記要件を定義した。  
> - KPI 収集スクリプト `python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats --text-source reports/spec-audit/ch1/core_text_grapheme_stats.json --output reports/text-grapheme-metrics.json --require-success` を Phase3 `phase3-core-text` ジョブへ組み込み、ローカルでも同コマンドが成功することを確認（`docs/notes/text-unicode-known-issues.md` TUI-003 を参照）。

### 2. 文字列三層モデル実装（41-42週目）
**担当領域**: 基盤 API

2.1. `Bytes`/`Str`/`String` の型と基本操作 (`as_bytes`, `to_string`, `string_clone` 等) を実装し、`effect` タグと `Result` ベースのエラー処理を整える。  
実施ステップ:  
- `compiler/rust/runtime/src/text/bytes.rs` を基点に `Bytes` の所有権 API を確定し、`Result<Bytes, UnicodeError>` が返す代表ケースを `docs/plans/bootstrap-roadmap/checklists/text-api-error-scenarios.md` に列挙する。  
- `String`/`Str` の実装で `effect {mem}` を打刻する箇所に `EffectSet` を導入し、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone` を追加してメトリクス化する。  
- `string_clone` や `as_bytes` の `Result` 型を仕様に合わせるため、`docs/spec/3-3-core-text-unicode.md` の該当節へ脚注を追加し、挙動の差分があれば `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` にフォローアップを記載する。

#### 2.1.1 API インベントリ更新と差分記録
- `docs/plans/bootstrap-roadmap/assets/text-unicode-api-diff.csv` を再作成し、`Bytes`/`Str`/`String`/`TextBuilder` の各 API について **仕様 → Rust 実装 → ギャップ** の 3 列で最新状態を反映した。既に Rust 実装が存在するものは `PoC`（挙動整合要レビュー）あるいは `Implemented` に更新し、欠落 API のみ `Missing` とした。  
- 代表的な差分を下表に整理し、どの API が `UnicodeError`/`EffectSet` を備えているかを可視化した。

| モジュール | 代表 API | Rust 実装 | 評価 | 備考 |
| --- | --- | --- | --- | --- |
| `Bytes` | `from_vec` / `into_string` | `compiler/rust/runtime/src/text/bytes.rs`【F:../../compiler/rust/runtime/src/text/bytes.rs†L12-L74】 | PoC | 所有権ムーブ経路は揃ったが `effect {mem}` 記録と `DecodePolicy` 分岐は今後の課題。 |
| `Str<'a>` | `to_bytes` / `iter_graphemes` | `compiler/rust/runtime/src/text/str_ref.rs`【F:../../compiler/rust/runtime/src/text/str_ref.rs†L1-L52】 | PoC | `GraphemeIter` 連携と `Bytes` 変換は提供済み。`effect {unicode}` 発火位置と `Cow<'a, str>` の `mem` 計測は TODO。 |
| `String` | `from_str` / `into_bytes` / `normalize` | `compiler/rust/runtime/src/text/text_string.rs`【F:../../compiler/rust/runtime/src/text/text_string.rs†L1-L64】 / `text/normalize.rs`【F:../../compiler/rust/runtime/src/text/normalize.rs†L1-L32】 | PoC | 正規化 API まで Rust 実装あり。`EffectSet` との橋渡しと `CollectError::OutOfMemory` 変換が未着手。 |
| `TextBuilder` | `push_*` / `finish_with_effects` | `compiler/rust/runtime/src/text/builder.rs`【F:../../compiler/rust/runtime/src/text/builder.rs†L1-L92】 | Implemented（Phase3 PoC） | `EffectSet` を用いた `mem`/`mut` 計測済み。`AuditEnvelope` 連携は 2.3 で実施予定。 |

- CSV では `impl_status=PoC` に切り替えたエントリへ `Note` として参照ソースと既知ギャップ（`effect {mem}`、`UnicodeErrorKind` 等）を明記し、計画との差異をレビュアが追跡できるようにした。

#### 2.1.2 効果タグと所有権ポリシーの整理
- `docs/notes/text-unicode-ownership.md` を 1.2 節と同期し、`Bytes::from_slice`/`Str::to_bytes`/`String::from_str` などコピー経路で `EffectSet::record_mem_bytes(len)` を呼び出すべき箇所を表形式で列挙した。  
- `Bytes`・`String` のゼロコピー移譲 (`from_vec`/`into_bytes`) は `collector.effect.transfer=true` を記録し、`effect {mem}` を増やさないルールを決定。TextBuilder が積算した `mem_bytes` を `finish` で二重加算しない点も同メモへ反映した。  
- `EffectSet` の `MEM_BIT` を Core.Text でも共有するため、`EffectSet` に専用の `mark_transfer` を追加する案と既存 `collector.effect.transfer` フィールドで表現する案を比較し、Phase3 では後者（既存フィールド流用）を採用することを決定。

#### 2.1.3 `UnicodeError`／`Result` 経路の棚卸し
- `compiler/rust/runtime/src/text/error.rs`【F:../../compiler/rust/runtime/src/text/error.rs†L1-L57】の `UnicodeErrorKind` を再確認し、2.1 スコープで必要な `InvalidUtf8` / `InvalidRange` / `UnsupportedScalar` の戻り値をテーブル化。`OutOfMemory` や `DecodePolicy` は 2.3 以降で拡張する。  
- `docs/plans/bootstrap-roadmap/checklists/text-api-error-scenarios.md` に TA-01〜TA-04 をリンクし、`Bytes::from_vec` / `String::clone` / `TextBuilder::push_grapheme` / `prepare_identifier` の導線が揃っていることを確認。担当列は `@core-text` に暫定割当、`状況=Pending` のまま残し再測タイミングを Week42 に設定した。  
- Parser との連携を見据え、`UnicodeError::phase` を `unicode` 固定から `Decode`/`Encode`/`Builder` など呼び出し元で上書きできる API（`with_phase`) に統一した。`docs/notes/text-unicode-diagnostic-bridge.md` に `Span` とのマッピング案を追記済み。

#### 2.1.4 KPI・検証ルート整備
- `0-3-audit-and-metrics.md` に登録済みの `text.mem.zero_copy_ratio` / `text.mem.copy_penalty_bytes` を本タスクの出口条件に設定。`reports/text-mem-metrics.json` をサンプル入力 (`Bytes::from_slice`, `Bytes::from_vec`, `Str::to_bytes`, `String::into_bytes`) で作成し、`python3 tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone --text-mem-source reports/text-mem-metrics.json --require-success` の実行ログを `reports/spec-audit/ch1/core_text_mem-20270329.md` へ保存した。  
- `tooling/ci/collect-iterator-audit-metrics.py` に `--section text --scenario bytes_clone` を実装し、`reports/text-mem-metrics.json` 内の `expectations` を検証できるようにした。CI では `text.mem.zero_copy_ratio` が 0.70 未満の場合と `UnicodeErrorKind::OutOfMemory` ケース未検出時に失敗させる閾値を設定する。  
- `scripts/validate-diagnostic-json.sh --suite text` に `collector.effect.transfer` / `unicode.error.kind` の必須キーを追加し、Core.Text の JSON スキーマとの差分を検知できるようにした（`reports/text-mem-metrics.json` を既定対象として追加）。

#### 2.1.5 実施ログ（2027-03-29）
- `Bytes`/`Str`/`String` の API 実装位置と効果計測方針を棚卸し、`text-unicode-api-diff.csv`・`text-unicode-ownership.md` を更新して所有権フローと `EffectSet` の適用条件を同期した。  
- `UnicodeError` の戻り値と `phase` / `offset` 設計を整理し、`text-api-error-scenarios.md` のケースにリンク。Parser/Diagnostics 連携時の `Span` 変換ルールを `unicode-error-mapping.md` の TODO として登録した。  
- KPI 収集ルート（`collect-iterator-audit-metrics.py --section text --scenario bytes_clone`）をローカルで実行し、`reports/text-mem-metrics.json` / `reports/spec-audit/ch1/core_text_mem-20270329.md` に `text.mem.zero_copy_ratio = 0.82`、`text.mem.copy_penalty_bytes = 512B/KB`、`UnicodeErrorKind::OutOfMemory` ケース 1 件を記録した。  
- 今後は `EffectSet` を Text API 自体へ組み込む実装タスク（`Bytes::from_slice` 等）と、`CollectError::OutOfMemory` へ伝搬する `try_reserve` エラーの PoC を 2.3 (TextBuilder/Collector) で並走する。

2.2. `Grapheme`/`GraphemeSeq` を実装し、`segment_graphemes` の性能と正確性を検証する。  
実施ステップ:  
- `unicode-segmentation` など参照ライブラリのアルゴリズムを調査し、採用案を `docs/notes/text-unicode-segmentation-comparison.md` に記録してから実装を着手する。  
- `segment_graphemes` の双方向イテレータ・ランダムアクセス API を揃え、UAX #29 の公式テストデータを `tests/data/unicode/segment/*` に配置して `cargo test grapheme_conformance` を追加する。  
- Grapheme ごとの `display_width`/`script` 情報を `Grapheme` 型へ格納し、`log_grapheme_stats` で多言語混在ケースの割合を出力して `reports/spec-audit/ch1/core_text_grapheme_stats.json` に保存する。

#### 2.2.1 実施ログ（2027-03-30）
- `compiler/rust/runtime/src/text/grapheme.rs` に `ScriptCategory`・`TextDirection`・`ScriptStats` を導入し、`Grapheme` が `script_mix_ratio`/`rtl_ratio`/`primary_script` を計測できるようにした。`GraphemeSeq` は `IntoIterator`（`DoubleEndedIterator`）と `byte_offset_at`/`grapheme_at_byte_offset` を公開し、Diagnostics が書記素境界をランダムアクセス可能になった。
- UAX #29 rev.40 の GraphemeBreakTest データを `third_party/unicode/UAX29/GraphemeBreakTest-15.1.0.txt` として同梱し、`cargo test --manifest-path compiler/rust/runtime/Cargo.toml grapheme_conformance -- --ignored` で互換性をチェックする回帰テストを新設。投入履歴を `docs/notes/unicode-upgrade-log.md` に記録した。
- `text_internal_cache` テストを再実行し、`reports/spec-audit/ch1/core_text_grapheme_stats.json` に `primary_script`・`script_mix_ratio`・`rtl_ratio` を追記。KPI `text.grapheme.script_mix_ratio` を `0-3-audit-and-metrics.md` へ登録し、UC-02 ケースで 0.56/0.43 を達成したことをログ化した。
- `tooling/ci/collect-iterator-audit-metrics.py` に `--check script_mix` オプションを追加し、CI で UC-02 の `script_mix_ratio >= 0.55` / `rtl_ratio >= 0.4` を自動ゲートできるようにした。

2.3. `TextBuilder` の構築 API を実装し、`Iter<Grapheme>` との連携をテストする。  
実施ステップ:  
- `TextBuilder::push_bytes`/`push_grapheme`/`finish` を `Iter<Grapheme>` の stage 情報（`IterStage::Streaming`）と共有できるようにし、`Core.Iter` からの `collect_text` API を追加する。  
- `TextBuilder` のメモリ増減を `EffectSet::mark_mem` で追跡し、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario text_builder_streaming` を追加して大規模入力での `effect {mem}`/`effect {mut}` を確認する。  
- `TextBuilder` が `Result<Text, UnicodeError>` を返す際のエラーを `UnicodeErrorKind::Decode`/`Encode` 等に分類し、`docs/spec/3-3-core-text-unicode.md` の例と一致しているかを `docs/plans/bootstrap-roadmap/checklists/unicode-error-mapping.md` でクロスチェックする。

**API ドラフトと効果設計**  
- `TextBuilder`（`compiler/rust/runtime/src/text/builder.rs` を追加予定）  
  - フィールド: `buffer: Vec<u8>`, `effects: EffectSet`, `audit: CollectorAuditTrail`。  
  - API:  
    - `TextBuilder::new() -> Self` (`effect {mem}` 初期値ゼロ)  
    - `push_bytes(&mut self, Bytes) -> Result<(), UnicodeError>` (`effect {mem, mut}` + `EffectSet::record_mem_bytes`)  
    - `push_grapheme(&mut self, &str) -> Result<(), UnicodeError>` (`effect {unicode}`。`log_grapheme_stats` に渡す cluster 情報をバッファリング)  
    - `finish(self) -> Result<String, UnicodeError>` (`effect {mem}`、`String::from_utf8` 失敗時は `UnicodeErrorKind::InvalidUtf8`)  
    - `into_bytes(self) -> Bytes` (`@pure` 経路で `Vec<u8>` を譲渡)  
- `Core.Iter.collect_text(iter: Iter<Grapheme>) -> Result<CollectOutcome<String>, CollectError>`  
  - `CollectorBridge` を `TextBuilderCollector` で再利用し、`CollectorEffectMarkers` の `mem_reservation`/`finish` を `TextBuilder` に伝搬。  
  - `IterStage::Streaming` から `TextBuilder` の `EffectSet` へ `effect {unicode}` を引き継ぎ、`log_grapheme_stats` の `cache_hits` を Collector 監査へ記録。  
- 監査/Effect 連携  
  - `TextBuilder` が `EffectSet::mark_unicode()` を呼び出す箇所を定義し、`collector.effect.unicode` を `Core.Iter` の `EffectLabels` に反映する。  
  - `AuditEnvelope.metadata["text.builder"]` に `{ bytes_written, graphemes, cache_hits }` を記録し、`CollectorAuditTrail::record_change_set` と同じフォーマットで `effect.mem_bytes` を更新する。  
- 準備タスク  
  1. `TextBuilder` API の Rust ドラフトを `docs/plans/bootstrap-roadmap/checklists/textbuilder-api-draft.md` に切り出し、フェーズ 2/3 でレビュー可能にする。  
  2. `core_text_builder_effects.md`（`docs/notes/` 追加）で `EffectSet`・`CollectorEffectMarkers` の更新箇所と KPI を表形式に整理。  
  3. `Core.Iter.collect_text` のテスト計画を `compiler/rust/runtime/src/prelude/iter/tests/collect_text.rs` と `reports/spec-audit/ch1/text_builder-*.md` で管理し、`effect {mem}`/`effect {unicode}` の二重打刻が無いか `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario collect_text` を追加する。

### 3. Unicode 正規化・ケース変換（42週目）
**担当領域**: 文字処理

3.1. NFC/NFD/NFKC/NFKD 正規化 API を実装し、ICU 互換テストベクトルで検証する。  
実施ステップ:  
- Unicode コンソーシアム提供のテストデータ (`NormalizationTest.txt`) を `third_party/unicode/` に同期し、バージョン番号を `docs/notes/unicode-upgrade-log.md` に記録する。  
- 正規化 API ごとに `Result<Text, UnicodeError>` の戻り値を固定し、`cargo test normalization_conformance -- --ignored` で大規模データを検証するジョブを CI に追加する。  
- 正規化過程で `effect {mem}` が発生する箇所にメトリクスを埋め込み、`0-3-audit-and-metrics.md` へ「正規化コスト (MB/s)」を新規 KPI として追記する。

#### 3.1.1 実施ログ（2025-11-26）
- `third_party/unicode/UCD/NormalizationTest-15.1.0.txt` を追加し、`docs/notes/unicode-upgrade-log.md#履歴` に同期。`docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md` ではデータソースと実行手順（`cargo test --manifest-path compiler/rust/runtime/Cargo.toml normalization_conformance -- --ignored`）を更新し、Nightly で全ベクタを検証する体制を整えた。  
- `compiler/rust/runtime/tests/normalization_conformance.rs` を実装し、UAX #15 に記載された c1〜c5 の等式（NFC/NFD/NFKC/NFKD）をすべて検証できるようにした。テストは 1 行ごとに 20 件の変換を行い、失敗時は行番号と違反した式を報告する。  
- `compiler/rust/runtime/src/text/normalize.rs` の `normalize` API を `effects::record_mem_copy(len)` 付きで再実装し、既に正規化済みの入力はゼロコピーで即返却、未正規化の入力はフォーム別イテレータで変換した上で `effect {mem}` を打刻するようにした。  
- 新しいメトリクス `text.normalize.mb_per_s` を `reports/text-normalization-metrics.json` に記録する前提で `0-3-audit-and-metrics.md` を更新し、`cargo run --manifest-path compiler/rust/runtime/Cargo.toml --example text_normalization_metrics -- --output reports/text-normalization-metrics.json` → `tooling/ci/collect-iterator-audit-metrics.py --section text --scenario normalization_conformance --text-normalization-source reports/text-normalization-metrics.json --require-success` の流れを Phase3 `phase3-core-text` ジョブに組み込む計画を追記した。

3.2. ケース変換 (`to_upper`/`to_lower`) と幅変換 (`width_map`) を実装し、ロケール依存エラー (`UnicodeErrorKind::UnsupportedLocale`) をハンドリングする。  
実施ステップ:  
- ロケール付き API の入力 (`LocaleId`) 検証ルールを `docs/spec/3-3-core-text-unicode.md` と `docs/spec/3-5-core-io-path.md` の記述で統一し、サポートロケール表を `docs/plans/bootstrap-roadmap/assets/text-locale-support.csv` に整備する。  
- ケース変換・幅変換のアルゴリズム差分（ICU との互換度）を `docs/notes/text-case-width-gap.md` にまとめ、逸脱がある箇所は `UnicodeErrorKind::UnsupportedLocale` または `UnsupportedWidth` で確実に通知する。  
- 変換結果を Parser/Diagnostics が使用するテキストと突き合わせるため、`compiler/rust/parser/tests/unicode_identifier.rs` にケース変換→識別子検証の統合テストを追加し、`scripts/validate-diagnostic-json.sh --pattern unicode.case` で CI ゲートに組み込む。

#### 3.2.1 ロケール検証とケース変換実装（2027-03-29）
- Core.Text に `LocaleId` と `LocaleScope` を実装し、BCP47 形式のロケール入力を `LocaleId::parse` で正規化できるようにした。`LocaleSupportStatus` と fallback 情報を `compiler/rust/runtime/src/text/locale.rs` へ常駐させ、`ensure_locale_supported` が `UnicodeErrorKind::UnsupportedLocale` を返す経路を統一した。【F:../../compiler/rust/runtime/src/text/locale.rs†L1-L181】
- `to_upper`/`to_lower` を `compiler/rust/runtime/src/text/case.rs` へ追加し、`width_map` と同じ `EffectSet` で `effect {mem}` を記録。`tr-TR` 用に i/İ・ı/I の特別大小文字を実装し、`LocaleId::parse("tr-TR")` → `to_upper` のテストを `compiler/rust/runtime/tests/unicode_case_width.rs` に追加した。【F:../../compiler/rust/runtime/src/text/case.rs†L1-L96】【F:../../compiler/rust/runtime/tests/unicode_case_width.rs†L1-L20】
- `docs/plans/bootstrap-roadmap/assets/text-locale-support.csv` の `tr-TR` を `Supported` へ更新し、`az-Latn` 行を追加。`docs/notes/text-case-width-gap.md` では `tr-TR` を `Closed`、`az-Latn` を `Planned` と記録して Parser/Diagnostics 連携の TODO を明示した。

#### 3.2.2 幅変換の双方向化と統計（2027-03-29）
- `compiler/rust/runtime/src/text/width.rs` を刷新し、ASCII/半角カナ/句読点の双方向マッピングと `KANA_TABLE` を導入。`WidthMode::{Narrow,Wide,EmojiCompat}` ごとに `Cow<str>` で変換有無を判断し、変換発生時は `stats.corrections_applied` と `effect {mem}` を記録するようにした。【F:../../compiler/rust/runtime/src/text/width.rs†L1-L416】
- Emoji 補正 (`👨‍👩‍👧‍👦`/`🇯🇵`) は `EMOJI_CORRECTIONS` で追跡し、`WidthMode::EmojiCompat` で 4 カラム幅を強制。`compiler/rust/runtime/tests/unicode_case_width.rs` と `width.rs` 内のユニットテストで ASCII/KANA ラウンドトリップと統計値を検証した。【F:../../compiler/rust/runtime/tests/unicode_case_width.rs†L22-L32】【F:../../compiler/rust/runtime/src/text/width.rs†L329-L416】
- `docs/notes/text-case-width-gap.md` の `ja-JP` 行を `Closed` に更新し、Emoji/Grapheme の調整は今後も `width_corrections.csv` で追跡する旨を追記。`az-Latn` など `Planned` ロケールは `UnsupportedLocale` で警告し、`unicode.locale.requested` KPI の検証対象に追加した。

#### 3.2.3 Parser 連携テストと UnsupportedLocale 診断（2027-03-30）
- `compiler/rust/runtime/src/text/identifier.rs` を追加し、`prepare_identifier`/`prepare_identifier_with_locale` が NFC 要求・Bidi 制御拒否・`LocaleScope::Case` のサポートチェックを行うよう実装。`UnicodeErrorKind::InvalidIdentifier` を `UnicodeErrorKind` に追加し、`diagnostics.rs` のコード割り当てを更新した。【F:../../compiler/rust/runtime/src/text/identifier.rs†L1-L93】
- Lexer 側では `LexerOptions` に `identifier_locale: Option<LocaleId>` を追加し、`lex_source_with_options` が Core.Text の `prepare_identifier` を呼び出すように再設計。`unicode_error_to_frontend` ヘルパを導入して `FrontendErrorKind::UnexpectedStructure` へ橋渡しし、`lex.identifier_locale` で指定されたロケールが `UnsupportedLocale` の場合は CLI/RunConfig 両方で警告を出すようにした。【F:../../compiler/rust/frontend/src/lexer/mod.rs†L1-L210】【F:../../compiler/rust/frontend/src/bin/reml_frontend.rs†L1-L370】
- `compiler/rust/frontend/tests/lexer_unicode_identifier.rs` を新設し、(1) NFC でない識別子、(2) Bidi 制御文字を含む識別子、(3) `lex.identifier_locale = az-Latn` による `UnsupportedLocale` の 3 ケースをカバー。`docs/plans/bootstrap-roadmap/checklists/unicode-error-mapping.md`・`text-api-error-scenarios.md` の `InvalidIdentifier`/TA-04 を `Green` に更新した。

#### 3.2.4 East Asian Width 補正と検証（2027-03-30）
- Emoji/Regional 指標の幅を CSV で管理するため `compiler/rust/runtime/src/text/data/width_corrections.csv` を追加し、`once_cell::sync::Lazy` で読み込んだ値を `WidthMode::EmojiCompat` の補正に利用。`width_map` が `UnicodeWidthStr::width_cjk` ベースで `WidthCorrection` を参照するよう再設計し、`docs/notes/text-case-width-gap.md` の Emoji 行を `Closed` に更新した。【F:../../compiler/rust/runtime/src/text/width.rs†L1-L220】
- Python `unicodedata.east_asian_width` から生成した `third_party/unicode/UCD/EastAsianWidth-15.1.0.txt` をバンドルし、`compiler/rust/runtime/tests/unicode_width_mapping.rs` で W/F/A クラスをフルスキャンするテストを追加。`cargo test --manifest-path compiler/rust/runtime/Cargo.toml unicode_width_mapping` を `UCNF-Width` の検証手段に採用し、`docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md` を更新した。

3.3. `prepare_identifier` を Parser 仕様 (2-3) と結合するテストを実装し、`UnicodeError` → `ParseError` 変換を確認する。  
実施ステップ:  
- Parser の識別子前処理 (`docs/spec/2-3-lexer.md`) を読み、`prepare_identifier` が `UnicodeErrorKind::InvalidIdentifier` を `ParseErrorKind::InvalidToken` へ写像するルールを表にまとめる。  
- `compiler/rust/frontend/tests/lexer_unicode_identifier.rs` に `prepare_identifier` 経由の成功・失敗ケースを 10 件以上追加し、`reports/spec-audit/ch1/lexer_unicode_identifier-*.json` をゴールデンとして保存する。  
- `UnicodeError` から `Diagnostic` への変換で `highlight.display_width` が正しく反映されるかを `Core.Diagnostics` のスナップショットテストに加え、`docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` と KPI を同期する。

> 実施ログ（2027-03-29）  
> - `compiler/rust/frontend/tests/lexer_unicode_identifier.rs` を 12 ケース（成功 6 / 失敗 6）で更新し、`prepare_identifier` が `UnicodeErrorKind::{InvalidIdentifier,UnsupportedLocale}` を返す経路と、`TokenKind::Unknown` へのフォールバックが一致することを確認。  
> - `reports/spec-audit/ch1/lexer_unicode_identifier-20270329.json` を作成し、各ケースの `unicode.error.kind`・`unicode.error.offset`・`parse.expected` の実測値と `lex.identifier_locale` 設定を記録。`docs/plans/bootstrap-roadmap/checklists/unicode-error-mapping.md` / `text-api-error-scenarios.md` へ参照リンクを追記した。  
> - `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に KPI `unicode.diagnostic.display_span` を新設し、`scripts/validate-diagnostic-json.sh --pattern unicode.error.kind` および本レポートで `Span`/列情報を検証する手順を登録。`docs/notes/text-unicode-diagnostic-bridge.md` では `ParseError` への写像ルールを更新し、`display_width` は Diagnostics 実装タスクに残課題として引き継いだ。

### 4. Diagnostics / IO 連携（42-43週目）
**担当領域**: 統合

4.1. `UnicodeError::to_diagnostic`・`unicode_error_to_parse_error` 等の変換を実装し、`Core.Diagnostics` のハイライト生成 (`display_width`) を統合テストする。  
実施ステップ:  
- `compiler/rust/frontend/src/diagnostic/formatter.rs` に Unicode 用の `DiagnosticBuilder` 拡張を追加し、`display_width`/`grapheme_span` を `Core.Text` の API で計算する。  
- `reports/spec-audit/ch1/unicode_diagnostics-*.json` を作成し、`scripts/validate-diagnostic-json.sh --pattern unicode.display_width` を CI に組み込み `0-3-audit-and-metrics.md` の診断 KPI とリンクする。  
- `unicode_error_to_parse_error` の変換表を `docs/plans/bootstrap-roadmap/checklists/unicode-error-mapping.md` に追記し、Parser/Diagnostics の両方で差分レビューを行う。

> 実施ログ（2027-03-30）  
> - `FrontendDiagnostic` と `ParseError` に `UnicodeDetail` を追加し、`lexer` で発生した `UnicodeError` が span・locale・raw 付きで保持されるようにした。【F:../../compiler/rust/frontend/src/diagnostic/mod.rs†L244-L276】【F:../../compiler/rust/frontend/src/parser/api.rs†L104-L159】  
> - `parser::unicode_error_to_parse_error` を実装し、`FrontendErrorKind::UnexpectedStructure` から Unicode コード (`unicode.invalid_identifier` 等) を自動付与する経路を整備した。【F:../../compiler/rust/frontend/src/parser/mod.rs†L608-L633】  
> - `diagnostic/unicode.rs` を新設して `integrate_unicode_metadata` を導入し、`extensions["unicode"]` と `AuditEnvelope.metadata["unicode.*"]` に `display_width`・`grapheme_span`・`unicode.identifier.raw` を同時出力する仕組みを実装した。`reports/spec-audit/ch1/unicode_diagnostics-20270330.json` で `scripts/validate-diagnostic-json.sh --pattern unicode.display_width ...` の検証対象を追加済み。【F:../../compiler/rust/frontend/src/diagnostic/unicode.rs†L1-L223】

#### 4.1.4 Diagnostic/ParseError スキーマ統合
- `FrontendDiagnostic` が保持する `Span`/`AuditEnvelope`/`ExpectedToken` 情報【F:../../compiler/rust/frontend/src/diagnostic/mod.rs†L14-L129】と、`ParseError` 構造体が保持する `Span`/`ExpectedToken` 群【F:../../compiler/rust/frontend/src/parser/api.rs†L104-L152】の項目を 1:1 で棚卸しし、欠落項目（`context`, `notes`, `unicode_error` 等）を `diagnostic-schema.md`（今後追加予定）にまとめる。  
- `Span` は `start`/`end` の半開区間で表現されているため【F:../../compiler/rust/frontend/src/span.rs†L7-L45】、`UnicodeError::offset`（バイト位置）から `Span` へ写像する際に `len` を確定するルール（例: 単一書記素→`offset..offset+cluster_len`）を `unicode-error-mapping.md` の列として管理する。  
- `AuditEnvelope`（`metadata` と `capability` を持つラッパ）【F:../../compiler/rust/frontend/src/diagnostic/mod.rs†L22-L57】に `unicode.*` 名前空間のキー（`unicode.error.kind`, `unicode.error.offset`, `unicode.error.phase`）を予約し、Parser で `ParseError` を Diagnostic に変換する際に埋め込む。Diagnostic JSON 出力では `audit_metadata` に同じキーを複写し、`AuditEnvelope.change_set` と一貫したフォーマットを維持する。詳細は `docs/notes/text-unicode-diagnostic-bridge.md` を参照。

#### 4.1.5 データ構造の導入方針
- `ParseError` に `unicode: Option<UnicodeError>` と `span_trace: Vec<Span>` を追加し、`UnicodeError` から得た `offset`・`kind` を直接保持できるようにする。`state.record_diagnostics`（`parser/api.rs`）と `DiagnosticBuilder` の双方に `Span`/`AuditEnvelope` の参照を渡し、差分が発生した場合は `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` へフォローアップを記録する。  
- `TextBuilder`/`decode_stream` 経路で発生した `UnicodeError` は `EffectSet` の `mark_mem`/`mark_io` と同時に `AuditEnvelope.metadata["unicode.effect.mem_bytes"]` を更新し、`reports/spec-audit/ch1/unicode_diagnostics-*.json` に追加される KPI（`unicode.audit_presence_rate`）の下地を用意する。  
- スキーマレベルでは `Span` と `AuditEnvelope` を JSON Schema に取り込む必要があるため、`docs/spec/3-6-core-diagnostics-audit.md` の付録に「unicode.* キー一覧」「span_trace フィールド」「parse.expected` との依存関係」を追加し、`collect-iterator-audit-metrics.py --section text` による検証対象を拡張する。

4.2. `decode_stream`/`encode_stream` を実装し、`Core.IO` の Reader/Writer とストリーミング decode の整合性を確認する。  
実施ステップ:  
- `docs/spec/3-5-core-io-path.md` の `StreamDecoder` 仕様を参照して API を整備し、`compiler/rust/runtime/src/io/text_stream.rs` に実装を追加する。  
- `examples/io/text_stream_decode.rs` を作成し、`CI (rust-frontend-streaming)` ジョブで `cargo run --bin text_stream_decode <fixtures>` を実行して `reports/spec-audit/ch1/unicode_streaming_decode.log` を生成・検証する。  
- ストリーミング decode の backpressure と `effect {audit}` 連携が競合しないよう、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の該当 TODO にテスト結果をリンクする。

4.3. `log_grapheme_stats` を実装し、監査ログ (`AuditEnvelope`) と `effect {audit}` の整合をテストする。  
実施ステップ:  
- `Core.Diagnostics` の `AuditEnvelope.metadata["text.grapheme_stats"]` に `length`, `avg_width`, `cache_hits` を記録し、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario grapheme_stats` で監査ログを自動検証する。  
- `effect {audit}` を扱うため `core.text.audit` Capability を `docs/plans/bootstrap-roadmap/3-8-core-runtime-capability-plan.md` と同期し、`CapabilityRegistry` の登録・テスト (`cargo test capability_text_audit`) を追加する。  
- `reports/spec-audit/ch1/text_grapheme_stats.audit.jsonl` を整備し、`scripts/validate-diagnostic-json.sh --pattern text.grapheme_stats` を CI ゲートとして設定する。

### 5. サンプルコード・ドキュメント更新（43週目）
**担当領域**: 情報整備

5.1. 仕様書内サンプルを Reml 実装で検証し、出力結果を `examples/` 配下にゴールデンファイルとして追加する。  
実施ステップ:  
- `docs/spec/3-3-core-text-unicode.md` のコード例を `examples/core-text/` に移し、`cargo run --bin reml_examples --example text_unicode` で得られる出力を `examples/core-text/expected/*.golden` として保存する。  
- サンプル実行ログと差分を `reports/spec-audit/ch1/core_text_examples-YYYYMMDD.md` にまとめ、`README.md` のサンプル一覧から参照できるようリンクを追加する。  
- ゴールデン更新時には `docs-migrations.log` に記録し、`docs/spec/3-3-core-text-unicode.md` の脚注に「examples/core-text 参照」を追記する。

5.2. `README.md`/`3-0-phase3-self-host.md` に Core.Text 完了状況とハイライトを追記し、利用者向け注意点を記載する。  
実施ステップ:  
- `README.md` の Phase 3 セクションに「Core.Text 三層モデル完了」のバッジとハイレベルサマリを追加し、`3-0-phase3-self-host.md` のマイルストーン表へ完了週と成果物へのリンクを記載する。  
- Unicode 依存の注意事項（サポート Unicode バージョン、正規化/ケース変換の制約）を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` のリスク表と同期し、ユーザーが参照できる FAQ を `docs/notes/text-unicode-known-issues.md` にまとめる。  
- `docs/plans/bootstrap-roadmap/SUMMARY.md` の Phase 3 節を更新して `Core.Text` 関連ドキュメントへのクロスリンクを整理する。

5.3. `docs/guides/core-parse-streaming.md`/`docs/guides/ai-integration.md` 等、Unicode 処理に関係するガイドを更新する。  
実施ステップ:  
- ストリーミングパーサガイドに `decode_stream`/`TextBuilder` の利用例を追加し、`AI integration` ガイドでは入力正規化の注意点を脚注として追記する。  
- ガイド更新時に `README.md` と `docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md` のリンクを見直し、相互参照が切れていないか `rg "../"` で検証する。  
- 更新結果を `docs/notes/docs-update-log.md` に記載し、レビュー観点（エンコード別注意点）を `docs/plans/bootstrap-roadmap/checklists/doc-sync-text.md` で追跡する。

### 6. テスト・ベンチマーク統合（43-44週目）
**担当領域**: 品質保証

6.1. Unicode Conformance テスト (UAX #29/#15) を導入し、NFC 等の正確性を自動検証する。  
実施ステップ:  
- `tests/data/unicode/UAX29` `UAX15` を取得して `THIRD_PARTY_LICENSES.md` にライセンス表を追記し、`cargo test unicode_conformance --features unicode_full` を追加する。  
- Conformance 失敗時の再現ログを `reports/spec-audit/ch1/unicode_conformance_failures.md` にまとめ、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に暫定措置を登録する。  
- テスト実行結果を `docs/plans/bootstrap-roadmap/checklists/unicode-conformance-checklist.md` に反映し、達成率を `0-3-audit-and-metrics.md` の品質 KPI に追記する。

6.2. ベンチマーク (正規化・セグメンテーション・TextBuilder) を追加し、Rust 実装の Phase 2 ベンチマーク比 ±15% 以内を目指す。OCaml 実装は設計比較材料として参照するのみとする。  
実施ステップ:  
- `benchmarks/text/normalization.rs`・`benchmarks/text/grapheme.rs`・`benchmarks/text/builder.rs` を追加し、`criterion` による計測を `cargo bench text::*` で実行する。  
- 測定指標（MB/s、ns/char、キャッシュ命中率）を `reports/benchmarks/core_text/*.md` にまとめ、`0-3-audit-and-metrics.md` の性能表へ転載する。  
- ベンチ結果が目標を外れた場合のフォローアップ（アルゴリズム変更、SIMD 導入等）を `docs/notes/text-unicode-performance-investigation.md` に記録し、リスク登録する。

6.3. CI に文字列結合の回帰テストを組み込み、大規模入力でのメモリ/性能指標を `0-3-audit-and-metrics.md` に記録する。  
実施ステップ:  
- `scripts/ci/run_core_text_regressions.sh` を追加し、`cargo test text_builder_regression` と `cargo bench text::builder --quick` を GitHub Actions の `phase3-core-text` ジョブに組み込む。  
- 大規模入力のメモリ使用量を `log_grapheme_stats` で収集して `reports/spec-audit/ch1/text_regression_metrics.json` に保存し、`scripts/validate-diagnostic-json.sh --pattern text.mem_peak` で自動検証する。  
- KPI を `0-3-audit-and-metrics.md` と `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に反映し、未達時は TODO を起票して次スプリントの backlog に追加する。

## 成果物と検証
- `Core.Text`/`Core.Unicode` API が仕様と一致し、エラー/監査/IO 連携が正しく機能すること。
- Unicode Conformance テストとベンチマークが基準値を満たし、差分が文書化されていること。
- ドキュメントとサンプルが更新され、三層モデルの利用法が明確であること。

## リスクとフォローアップ
- ICU への依存部分でライセンス・バージョン差異が発生した場合は `docs/notes/llvm-spec-status-survey.md` に記録し、Phase 4 の運用計画で処理する。
- 文字幅計算や Grapheme 分割で性能劣化がみられた場合、キャッシュ戦略やネイティブ実装の検討をフォローアップとする。
- ストリーミング decode で大容量入力が処理できない場合、Phase 3-5 (IO & Path) でバッファリング戦略を再評価する。

## 参考資料
- [3-3-core-text-unicode.md](../../spec/3-3-core-text-unicode.md)
- [1-4-test-unicode-model.md](../../spec/1-4-test-unicode-model.md)
- [2-3-lexer.md](../../spec/2-3-lexer.md)
- [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
