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

1.3. 内部キャッシュ (コードポイント/グラフェムインデックス) の設計とテスト戦略を定義する。  
実施ステップ:  
- `GraphemeSeq` 用の `IndexCache`（コードポイント→書記素クラスタ開始位置）を `RuntimeCacheSpec`（`docs/notes/core-library-outline.md`）と整合させ、キャッシュ無効化条件を図示する。  
- キャッシュ命中率を収集するため `log_grapheme_stats` に `cache_hits`/`cache_miss` を追加し、`tooling/ci/collect-iterator-audit-metrics.py --section text` で KPI 化する。  
- `cargo test text_internal_cache -- --ignored` を追加して大規模入力・キャッシュ無効化・多言語ケースを検証し、テストケースごとに `docs/plans/bootstrap-roadmap/checklists/unicode-cache-cases.md` を更新する。

### 2. 文字列三層モデル実装（41-42週目）
**担当領域**: 基盤 API

2.1. `Bytes`/`Str`/`String` の型と基本操作 (`as_bytes`, `to_string`, `string_clone` 等) を実装し、`effect` タグと `Result` ベースのエラー処理を整える。  
実施ステップ:  
- `compiler/rust/runtime/src/text/bytes.rs` を基点に `Bytes` の所有権 API を確定し、`Result<Bytes, UnicodeError>` が返す代表ケースを `docs/plans/bootstrap-roadmap/checklists/text-api-error-scenarios.md` に列挙する。  
- `String`/`Str` の実装で `effect {mem}` を打刻する箇所に `EffectSet` を導入し、`tooling/ci/collect-iterator-audit-metrics.py --section text --scenario bytes_clone` を追加してメトリクス化する。  
- `string_clone` や `as_bytes` の `Result` 型を仕様に合わせるため、`docs/spec/3-3-core-text-unicode.md` の該当節へ脚注を追加し、挙動の差分があれば `docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md` にフォローアップを記載する。

2.2. `Grapheme`/`GraphemeSeq` を実装し、`segment_graphemes` の性能と正確性を検証する。  
実施ステップ:  
- `unicode-segmentation` など参照ライブラリのアルゴリズムを調査し、採用案を `docs/notes/text-unicode-segmentation-comparison.md` に記録してから実装を着手する。  
- `segment_graphemes` の双方向イテレータ・ランダムアクセス API を揃え、UAX #29 の公式テストデータを `tests/data/unicode/segment/*` に配置して `cargo test grapheme_conformance` を追加する。  
- Grapheme ごとの `display_width`/`script` 情報を `Grapheme` 型へ格納し、`log_grapheme_stats` で多言語混在ケースの割合を出力して `reports/spec-audit/ch1/core_text_grapheme_stats.json` に保存する。

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

3.2. ケース変換 (`to_upper`/`to_lower`) と幅変換 (`width_map`) を実装し、ロケール依存エラー (`UnicodeErrorKind::UnsupportedLocale`) をハンドリングする。  
実施ステップ:  
- ロケール付き API の入力 (`LocaleId`) 検証ルールを `docs/spec/3-3-core-text-unicode.md` と `docs/spec/3-5-core-io-path.md` の記述で統一し、サポートロケール表を `docs/plans/bootstrap-roadmap/assets/text-locale-support.csv` に整備する。  
- ケース変換・幅変換のアルゴリズム差分（ICU との互換度）を `docs/notes/text-case-width-gap.md` にまとめ、逸脱がある箇所は `UnicodeErrorKind::UnsupportedLocale` または `UnsupportedWidth` で確実に通知する。  
- 変換結果を Parser/Diagnostics が使用するテキストと突き合わせるため、`compiler/rust/parser/tests/unicode_identifier.rs` にケース変換→識別子検証の統合テストを追加し、`scripts/validate-diagnostic-json.sh --pattern unicode.case` で CI ゲートに組み込む。

3.3. `prepare_identifier` を Parser 仕様 (2-3) と結合するテストを実装し、`UnicodeError` → `ParseError` 変換を確認する。  
実施ステップ:  
- Parser の識別子前処理 (`docs/spec/2-3-lexer.md`) を読み、`prepare_identifier` が `UnicodeErrorKind::InvalidIdentifier` を `ParseErrorKind::InvalidToken` へ写像するルールを表にまとめる。  
- `compiler/rust/frontend/tests/lexer_unicode_identifier.rs` に `prepare_identifier` 経由の成功・失敗ケースを 10 件以上追加し、`reports/spec-audit/ch1/lexer_unicode_identifier-*.json` をゴールデンとして保存する。  
- `UnicodeError` から `Diagnostic` への変換で `highlight.display_width` が正しく反映されるかを `Core.Diagnostics` のスナップショットテストに加え、`docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md` と KPI を同期する。

### 4. Diagnostics / IO 連携（42-43週目）
**担当領域**: 統合

4.1. `UnicodeError::to_diagnostic`・`unicode_error_to_parse_error` 等の変換を実装し、`Core.Diagnostics` のハイライト生成 (`display_width`) を統合テストする。  
実施ステップ:  
- `compiler/rust/frontend/src/diagnostic/formatter.rs` に Unicode 用の `DiagnosticBuilder` 拡張を追加し、`display_width`/`grapheme_span` を `Core.Text` の API で計算する。  
- `reports/spec-audit/ch1/unicode_diagnostics-*.json` を作成し、`scripts/validate-diagnostic-json.sh --pattern unicode.display_width` を CI に組み込み `0-3-audit-and-metrics.md` の診断 KPI とリンクする。  
- `unicode_error_to_parse_error` の変換表を `docs/plans/bootstrap-roadmap/checklists/unicode-error-mapping.md` に追記し、Parser/Diagnostics の両方で差分レビューを行う。

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
