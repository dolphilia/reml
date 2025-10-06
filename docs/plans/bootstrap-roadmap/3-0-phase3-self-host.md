# 3.0 Phase 3 — Core Library 完成

Phase 3 では、Reml 標準ライブラリ Chapter 3 の正式仕様を Reml 実装へ揃えます。Prelude から Runtime Capability までの各モジュールを仕様と照合し、効果タグ・監査・Capability 契約が一貫して動作する状態を構築します。

## 3.0.1 目的
- `Core.Prelude`/`Core.Collections`/`Core.Text`/`Core.Numeric`/`Core.IO`/`Core.Diagnostics`/`Core.Config`/`Core.Runtime` の API を Reml で実装し、仕様書と相互参照が成立した状態で提供する。
- 効果タグと Capability Stage の境界を検証し、Chapter 3 全体の診断・監査連携が [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) と一致するように統合する。
- 標準ライブラリのコード例・サンプル・メトリクスを最新化し、Phase 4 の移行とエコシステム展開に備えたベースラインを整備する。

## 3.0.2 スコープ境界
- **含む**: Core Prelude/Collections/Text/Numeric/IO/Diagnostics/Config/Runtime Capability の実装・テスト・ドキュメント更新、効果タグ・Capability 検証、監査／メトリクスの記録。
- **含まない**: 非同期ランタイム (`3-9`)、プラグイン／DSL 拡張 (`4-x`)、エコシステム仕様 (`5-x`) の本格対応（Phase 4 以降に委譲）。
- **前提条件**: Phase 2 で確定した型クラス・効果システム・診断仕様が利用可能であり、`0-3-audit-and-metrics.md` と `0-4-risk-handling.md` に基準値・リスク管理手順が登録済みであること。

## 3.0.2a 作業ディレクトリ
- `compiler/ocaml/src` : 標準ライブラリ各モジュールの実装
- `compiler/ocaml/tests` : API ゴールデンテスト、性能ベンチマーク
- `examples/` : API 使用例の整理（`examples/algebraic-effects/`, `examples/language-impl-comparison/` 等）
- `docs/spec/3-x` : 仕様本文の更新とリンク整備
- `docs/guides/` : 運用ガイドの同期 (`docs/guides/runtime-bridges.md` など)
- `docs/notes/` : 設計判断・メトリクスの記録 (`docs/notes/core-library-outline.md` ほか)

## 3.0.3 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Prelude & Iteration | `Option`/`Result`/`Iter` と Collector を実装し効果タグを整合 | API テスト、効果タグ静的検証、サンプル実行 | Phase 3 開始後 8 週 |
| M2: Collections | 永続・可変コレクションと差分 API を実装 | 構造共有プロパティテスト、`CollectError` シナリオ CI | 開始後 16 週 |
| M3: Text & Unicode | 文字列三層モデル・Unicode 正規化・Builder を実装 | UAX コンフォーマンス、Decode/Encode ストリーミングテスト | 開始後 20 週 |
| M4: Numeric / IO & Path | 統計・時間 API と IO 抽象／Path セキュリティを実装 | ベンチマーク ±15% 以内、IO/Path 統合テスト | 開始後 26 週 |
| M5: Diagnostics & Config | Diagnostic/Audit と Manifest/Schema を統合 | 診断スナップショット、Config Lint、監査ログ比較 | 開始後 30 週 |
| M6: Runtime Capability | Capability Registry と Stage 検証を完成 | Capability テストマトリクス、Manifest 契約検証 | 開始後 34 週 |

## 3.0.4 主要タスク

1. **Core Prelude & Iteration** (`3-1`)
   - `Option`/`Result`/`Never` と `Iter` 本体・アダプタ・終端操作を Reml で実装。
   - 効果タグと `@must_use` 属性を静的解析し、Chapter 1 の構文・効果仕様と一致させる。
   - `Collector` 契約を定義し、`Core.Collections`／`Core.Text` から再利用できるよう拡張。
2. **Core Collections** (`3-2`)
   - 永続構造（List/Map/Set）と可変構造（Vec/Cell/Ref/Table）を実装し、構造共有・順序保持・効果タグを検証。
   - `Iter` との相互運用（`collect_*`, `Map.from_iter`）と監査差分 (`ChangeSet`) を整備。
3. **Core Text & Unicode** (`3-3`)
   - 文字列三層モデル（Bytes/Str/String）と `GraphemeSeq`/`TextBuilder` を実装。
   - Unicode 正規化・ケース変換・幅変換 API を `UnicodeError` と診断変換で統合。
   - IO/Diagnostics と連携したストリーミング decode・監査ログ API (`log_grapheme_stats`) を検証。
4. **Core Numeric & Time** (`3-4`)
   - 数値トレイト／統計ヘルパ／Histogram／回帰 API を実装し、`Iter` ベースでテスト。
   - `Timestamp`/`Duration`/`Timezone` とフォーマット／パースを整備し、`Core.IO` と統合。
   - `MetricPoint` と監査メトリクス送出を整備し、`AuditEnvelope` メタデータを共通化。
5. **Core IO & Path** (`3-5`)
   - `Reader`/`Writer` 抽象、ファイル API、バッファリング、IO エラー体系を実装。
   - Path 抽象・セキュリティヘルパ・ファイル監視 (オプション) を整備し、クロスプラットフォーム差異を `TargetCapability` で吸収。
6. **Core Diagnostics & Audit** (`3-6`)
   - `Diagnostic` 構造・`AuditEnvelope`・`TraitResolutionTelemetry` 等を実装。
   - CLI/LSP 出力フォーマット、ステージ別フィルタ・抑制ポリシー、監査ログ記録を統合。
7. **Core Config & Data** (`3-7`)
   - `Manifest`/`Schema`/`ConfigCompatibility` API を実装し、DSL エクスポート情報・Capability Stage を同期。
   - Config Diff・ChangeSet を Diagnostics/Audit に連携し、CLI (`reml config lint/diff`) フローを整備。
8. **Core Runtime & Capability** (`3-8`)
   - `CapabilityRegistry`・`CapabilityHandle`・`verify_capability_stage`・`verify_conductor_contract` を実装。
   - Stage/Capability 情報を Diagnostics/Config/Runtime 各層で共有し、監査イベント (`CapabilityMismatch`) を記録。
9. **横断タスク**
   - API ドキュメント・サンプル・ガイド（`docs/guides/runtime-bridges.md`, `docs/notes/dsl-plugin-roadmap.md` 等）を更新。
   - `0-3-audit-and-metrics.md` へベンチマーク・監査指標を継続記録し、差分理由を明示。

## 3.0.5 測定と検証
- **API 完全性**: 仕様書に列挙された公開 API が Reml 実装に存在し、効果タグ・属性が一致することを静的チェックと API テストで確認。
- **効果タグ／Capability 整合**: `effect` タグと `CapabilityStage` の組み合わせを検証し、違反時は Diagnostics で再現できることを確認。
- **性能ベンチマーク**: Prelude/Collections/Numeric/IO の代表関数で OCaml 実装比 ±15% 以内を目標に測定し、結果を `0-3-audit-and-metrics.md` に記録。
- **ドキュメント同調**: 仕様書・ガイド・サンプルが更新され、リンク切れ・用語揺れがないことをレビュー。
- **監査／診断スナップショット**: `Diagnostic` と `AuditEnvelope` の出力をゴールデンテスト化し、CI で差分を検出。

## 3.0.6 リスクとフォローアップ
- **効果タグの逸脱**: 実装と仕様で効果タグが不一致の場合、クロスレビューとツール支援を追加し、`0-4-risk-handling.md` に改善タスクを登録。
- **Unicode/IO の性能劣化**: UAX コンフォーマンスを優先した結果として性能が不足する場合、キャッシュ・バッファリング戦略の改善や Phase 4 の最適化項目として記録。
- **Config/Capability のルール変更**: Manifest と Capability の整合が難航した場合、Phase 4 の移行計画と連携し、`docs/notes/dsl-plugin-roadmap.md` に暫定運用を明記。
- **テストボリューム増加**: Chapter 3 全体の CI 実行時間が長くなる恐れがあるため、テスト分割・キャッシュ・nightly ジョブを検討。
- **Phase 2 実装との差分**: 型クラス方式や効果システムが Phase 2 結果と異なる場合、差分を `docs/notes/llvm-spec-status-survey.md` にまとめ、Phase 4 の移行判断に備える。

---

Phase 3 完了時点で Reml 標準ライブラリの基盤が整い、Phase 4 ではマルチターゲット互換性検証とエコシステム移行に集中できる状態になる。
