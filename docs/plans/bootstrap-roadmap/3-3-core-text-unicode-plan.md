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
1.2. 文字列所有権モデル (コピー時の `effect {mem}`) を確認し、`Vec<u8>` の再利用方針を決める。
1.3. 内部キャッシュ (コードポイント/グラフェムインデックス) の設計とテスト戦略を定義する。

### 2. 文字列三層モデル実装（41-42週目）
**担当領域**: 基盤 API

2.1. `Bytes`/`Str`/`String` の型と基本操作 (`as_bytes`, `to_string`, `string_clone` 等) を実装し、`effect` タグと `Result` ベースのエラー処理を整える。
2.2. `Grapheme`/`GraphemeSeq` を実装し、`segment_graphemes` の性能と正確性を検証する。
2.3. `TextBuilder` の構築 API を実装し、`Iter<Grapheme>` との連携をテストする。

### 3. Unicode 正規化・ケース変換（42週目）
**担当領域**: 文字処理

3.1. NFC/NFD/NFKC/NFKD 正規化 API を実装し、ICU 互換テストベクトルで検証する。
3.2. ケース変換 (`to_upper`/`to_lower`) と幅変換 (`width_map`) を実装し、ロケール依存エラー (`UnicodeErrorKind::UnsupportedLocale`) をハンドリングする。
3.3. `prepare_identifier` を Parser 仕様 (2-3) と結合するテストを実装し、`UnicodeError` → `ParseError` 変換を確認する。

### 4. Diagnostics / IO 連携（42-43週目）
**担当領域**: 統合

4.1. `UnicodeError::to_diagnostic`・`unicode_error_to_parse_error` 等の変換を実装し、`Core.Diagnostics` のハイライト生成 (`display_width`) を統合テストする。
4.2. `decode_stream`/`encode_stream` を実装し、`Core.IO` の Reader/Writer とストリーミング decode の整合性を確認する。
4.3. `log_grapheme_stats` を実装し、監査ログ (`AuditEnvelope`) と `effect {audit}` の整合をテストする。

### 5. サンプルコード・ドキュメント更新（43週目）
**担当領域**: 情報整備

5.1. 仕様書内サンプルを Reml 実装で検証し、出力結果を `examples/` 配下にゴールデンファイルとして追加する。
5.2. `README.md`/`3-0-phase3-self-host.md` に Core.Text 完了状況とハイライトを追記し、利用者向け注意点を記載する。
5.3. `docs/guides/core-parse-streaming.md`/`docs/guides/ai-integration.md` 等、Unicode 処理に関係するガイドを更新する。

### 6. テスト・ベンチマーク統合（43-44週目）
**担当領域**: 品質保証

6.1. Unicode Conformance テスト (UAX #29/#15) を導入し、NFC 等の正確性を自動検証する。
6.2. ベンチマーク (正規化・セグメンテーション・TextBuilder) を追加し、OCaml 実装比 ±15% 以内を目指す。
6.3. CI に文字列結合の回帰テストを組み込み、大規模入力でのメモリ/性能指標を `0-3-audit-and-metrics.md` に記録する。

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
