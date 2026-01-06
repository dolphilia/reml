# 調査メモ: 第15章 標準ライブラリのプリミティブ

## 対象モジュール

- `compiler/runtime/src/lib.rs`
- `compiler/runtime/src/prelude/mod.rs`
- `compiler/runtime/src/prelude/iter/mod.rs`
- `compiler/runtime/src/prelude/iter/generators.rs`
- `compiler/runtime/src/prelude/collectors/mod.rs`
- `compiler/runtime/src/prelude/ensure.rs`
- `compiler/runtime/src/collections/mod.rs`
- `compiler/runtime/src/collections/persistent/mod.rs`
- `compiler/runtime/src/collections/persistent/list.rs`
- `compiler/runtime/src/collections/persistent/btree.rs`
- `compiler/runtime/src/collections/mutable/mod.rs`
- `compiler/runtime/src/collections/audit_bridge.rs`
- `compiler/runtime/src/io/mod.rs`
- `compiler/runtime/src/io/error.rs`
- `compiler/runtime/src/io/context.rs`
- `compiler/runtime/src/io/text_stream.rs`
- `compiler/runtime/src/text/mod.rs`
- `compiler/runtime/src/text/normalize.rs`
- `compiler/runtime/src/text/identifier.rs`
- `compiler/runtime/src/text/grapheme.rs`
- `compiler/runtime/src/text/width.rs`
- `compiler/runtime/src/numeric/mod.rs`
- `compiler/runtime/src/time/mod.rs`
- `compiler/runtime/src/path/mod.rs`
- `compiler/runtime/src/path/security.rs`
- `compiler/runtime/src/path/glob.rs`
- `compiler/runtime/src/system/env.rs`

## 入口と全体像

- ランタイム公開モジュールの一覧は `compiler/runtime/src/lib.rs` にあり、標準ライブラリのプリミティブ領域は `prelude` / `collections` / `io` / `text` / `numeric` / `time` / `path` / `env` が中心。`numeric` と `time` は feature gate を持つ点に注意。
  - `compiler/runtime/src/lib.rs:5`
- `collections` は永続構造・可変構造・監査ブリッジの 3 層に分割される。
  - `compiler/runtime/src/collections/mod.rs:1`
- `io` は Reader/Writer とストリーミングテキスト API を一括公開し、`copy` や `with_reader` が典型的な入口関数になる。
  - `compiler/runtime/src/io/mod.rs:1`
- `text` は Core.Text/Unicode の足場として複数の submodule を再公開し、`decode_stream`/`encode_stream` の IO 連携もここで束ねている。
  - `compiler/runtime/src/text/mod.rs:1`
- `env` は `system::env` の互換エイリアスで、PlatformInfo と環境変数操作を提供する。
  - `compiler/runtime/src/env.rs:1`
  - `compiler/runtime/src/system/env.rs:8`

## データ構造

### Prelude / Iter / Collector

- 遅延列 `Iter<T>` は `Arc<IterCore<T>>` で共有し、`IterState` に Stage/Effect 情報を保持する（Iteration 仕様の土台）。
  - `compiler/runtime/src/prelude/iter/mod.rs:42`
- `Iter` には `collect_list` / `collect_vec` などの終端操作があり、Collector との連携口になる。
  - `compiler/runtime/src/prelude/iter/mod.rs:158`
- Collector の結果は `CollectOutcome` に包まれ、`audit` 情報を保持する（監査連携の最小構造）。
  - `compiler/runtime/src/prelude/collectors/mod.rs:52`
- `CollectorStageProfile` と `CollectorStageSnapshot` が Stage/Capability 情報をメタデータ化する。
  - `compiler/runtime/src/prelude/collectors/mod.rs:108`
- `ensure` / `ensure_not_null` は Prelude のガード API として定義される。
  - `compiler/runtime/src/prelude/ensure.rs:307`

### Collections

- 永続 `List<T>` は Finger tree 風ノードと `PersistentArena` で構造共有を行う。
  - `compiler/runtime/src/collections/persistent/list.rs:9`
- `List` は `push_front` / `push_back` / `concat` と `iter` / `to_vec` を提供する。
  - `compiler/runtime/src/collections/persistent/list.rs:78`
- 永続 `PersistentMap` は LLRB 風の赤黒木で、`get`/`insert`/`merge_with` を提供する。
  - `compiler/runtime/src/collections/persistent/btree.rs:21`
- `PersistentMap` は `diff_change_set` で監査用の ChangeSet を生成できる。
  - `compiler/runtime/src/collections/persistent/btree.rs:140`
- 可変コレクションは `mutable` モジュールで `Vec`/`Cell`/`Ref`/`Table` を公開する。
  - `compiler/runtime/src/collections/mutable/mod.rs:1`
- 監査ブリッジ `ChangeSet` は `collections/audit_bridge.rs` に定義され、差分を JSON 形式へ整形する。
  - `compiler/runtime/src/collections/audit_bridge.rs:13`

### Text / Unicode

- `text` モジュールは `Bytes` / `Str` / `String` と Unicode 正規化・識別子正規化・グラフェム分割・幅補正の API を再公開する。
  - `compiler/runtime/src/text/mod.rs:24`
- NFC/NFD/NFKC/NFKD の正規化は `normalize`/`is_normalized` に集約。
  - `compiler/runtime/src/text/normalize.rs:13`
- `prepare_identifier` は NFC 判定と bidi 制御文字の拒否を行う。
  - `compiler/runtime/src/text/identifier.rs:14`
- `grapheme` は Unicode セグメンテーションとキャッシュ管理を担う。
  - `compiler/runtime/src/text/grapheme.rs:7`
- `width_map` は幅補正を `WidthMode` で切り替え、補正統計を持つ。
  - `compiler/runtime/src/text/width.rs:8`

### IO

- `IoError` と `IoErrorKind` が IO エラーの中心構造。`IoError` は `IoContext` を保持できる。
  - `compiler/runtime/src/io/error.rs:15`
- `IoContext` は `operation`/`path`/`capability`/`effects` などを保持し、buffer/watch/glob のサブ統計を含む。
  - `compiler/runtime/src/io/context.rs:10`
- `TextDecodeOptions` / `TextEncodeOptions` がストリーミング文字列変換の設定を表す。
  - `compiler/runtime/src/io/text_stream.rs:32`

### Numeric / Time

- `Numeric` / `Floating` トレイトが数値演算の共通インタフェースになっている。
  - `compiler/runtime/src/numeric/mod.rs:39`
- `mean` / `variance` / `percentile` / `median` / `mode` / `range` などは `Iter<T>` から計算する設計。
  - `compiler/runtime/src/numeric/mod.rs:79`
- `Timestamp` / `Duration` / `TimeFormat` が Time のコア型。
  - `compiler/runtime/src/time/mod.rs:27`
- `now` / `monotonic_now` / `duration_between` / `sleep` が基本 API。
  - `compiler/runtime/src/time/mod.rs:188`

### Path

- `PathBuf` / `Path` / `PathError` / `PathErrorKind` が Core.Path の中心構造。
  - `compiler/runtime/src/path/mod.rs:32`
- `SecurityPolicy` と `PathSecurityError` がサンドボックス検証・シンボリックリンク検証を担う。
  - `compiler/runtime/src/path/security.rs:22`
- `glob` は `FsAdapter` の read capability を要求し、パターンにマッチする Path を列挙する。
  - `compiler/runtime/src/path/glob.rs:13`

### Env / Platform

- `PlatformInfo` / `EnvError` / `EnvErrorKind` が環境変数 API の基本構造。
  - `compiler/runtime/src/system/env.rs:8`

## コアロジック

- `Iter` は `IterState` の `next_step` を通じて `Ready`/`Pending`/`Finished` を進め、終端操作で `CollectOutcome` と監査メタデータを構築する。
  - `compiler/runtime/src/prelude/iter/mod.rs:140`
- `CollectOutcome::record_change_set` が `collections/audit_bridge` 由来の ChangeSet を監査トレイルに貼り付ける。
  - `compiler/runtime/src/prelude/collectors/mod.rs:94`
- `PersistentMap::merge_with_change_set` は `merge_with` の結果に対し差分 ChangeSet を生成する。
  - `compiler/runtime/src/collections/persistent/btree.rs:168`
- `io::copy` は `IoCopyBuffer` を利用して Reader/Writer を連結し、バッファ使用量を effects に記録する。
  - `compiler/runtime/src/io/mod.rs:54`
- `io::with_reader` は `FsAdapter` の Capability チェック後に `std::fs::File` を開き、エラー時に `IoContext` を補完する。
  - `compiler/runtime/src/io/mod.rs:79`
- `decode_stream` は BOM 判定・UTF-8 消費・無効シーケンス処理を行い、Text 側の effect 記録と統合する。
  - `compiler/runtime/src/io/text_stream.rs:103`
- `prepare_identifier` は NFC 判定と bidi 制御文字検出を行い、lex フェーズのエラーに変換する。
  - `compiler/runtime/src/text/identifier.rs:14`
- `width_map_with_stats` は grapheme 単位で幅補正を行い、補正統計を集計する。
  - `compiler/runtime/src/text/width.rs:63`
- `numeric::mean` / `variance` は Welford 法で `Iter<T>` を一回走査し、`effect` 記録のためのメモリ使用量を必要に応じてカウントする。
  - `compiler/runtime/src/numeric/mod.rs:79`
- `time::Timestamp` / `Duration` はナノ秒ベースの範囲チェック付き変換を提供する。
  - `compiler/runtime/src/time/mod.rs:34`
- `path::glob` は glob パターンの検証後、Capability チェックと IO 効果記録を行う。
  - `compiler/runtime/src/path/glob.rs:13`

## 仕様との対応メモ

- Prelude/Iter/Collector は `docs/spec/3-1-core-prelude-iteration.md` と対応。
- Collections は `docs/spec/3-2-core-collections.md` と対応。
- Text/Unicode は `docs/spec/3-3-core-text-unicode.md` と対応。
- Numeric/Time は `docs/spec/3-4-core-numeric-time.md` と対応（実装は feature gate 付き）。
- IO/Path は `docs/spec/3-5-core-io-path.md` と対応。
- Env は `docs/spec/3-10-core-env.md` と対応。

## TODO / 不明点

- `text` モジュールの冒頭コメントでは「プレースホルダー」とあり、仕様の広範な API に対する実装の充足率が未確認。
  - `compiler/runtime/src/text/mod.rs:1`
- `numeric` と `time` は feature flag があり、ビルド構成によってモジュールが欠落する可能性がある。
  - `compiler/runtime/src/lib.rs:21`
- `Env` は `system::env` の薄いラッパで、監査ログ連携（spec 3.10）とのギャップがある可能性。
  - `compiler/runtime/src/env.rs:1`
