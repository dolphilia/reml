# 3.5 Core IO & Path 実装計画

## 目的
- 仕様 [3-5-core-io-path.md](../../spec/3-5-core-io-path.md) に準拠した `Core.IO`/`Core.Path` API を実装し、同期 IO・パス操作・セキュリティポリシーを統一する。
- Reader/Writer 抽象、ファイル操作、バッファリング、パス検証を Reml 実装へ落とし込み、Diagnostics/Audit/Runtime と安全に連携させる。
- 効果タグ (`effect {io}`, `{io.blocking}`, `{security}` 等) と Capability 検証を整備し、クロスプラットフォーム差異を管理する。

## スコープ
- **含む**: Reader/Writer/BufferedReader、File API、IO エラー、Path 抽象と正規化、セキュリティヘルパ、ファイル監視 (オプション機能) の実装、ドキュメント更新。
- **含まない**: 非同期 IO ランタイム、分散ファイルシステム統合、WASM 向け特化 API (Phase 4 以降)。
- **前提**: `Core.Text`/`Core.Numeric`/`Core.Diagnostics`/`Core.Runtime` が整備済みであり、Phase 2 で定義されたエラー型・監査モデルが利用可能であること。

## 作業ブレークダウン

### 1. API 差分整理と依存調整（47週目）
**担当領域**: 設計調整

1.1. Reader/Writer/Path/Watcher に関する公開 API を一覧化し、既存実装との差分と優先度を決定する。
1.2. 効果タグと Capability 要件 (`effect {io.blocking}`, `{security}` 等) を整理し、CI で検証するテスト計画を策定する。
1.3. OS 依存機能 (permissions, symlink) の抽象化方針を決め、Runtime Capability (3-8) との連携を確認する。

### 2. Reader/Writer 抽象実装（47-48週目）
**担当領域**: IO 基盤

2.1. `Reader`/`Writer` トレイトと共通ヘルパ (`copy`, `with_reader`) を実装し、`IoError` 体系を整備する。
2.2. バッファリング (`BufferedReader`, `read_line`) を実装し、`effect {mem}`/`{io.blocking}` を伴う動作をテストする。
2.3. `IoError` → `Diagnostic` 変換・監査メタデータ (`IoContext`) を実装し、CLI 出力と整合することを確認する。

### 3. ファイル API とメタデータ（48週目）
**担当領域**: ファイル操作

3.1. `File::open/create/remove/metadata` 等の API を実装し、プラットフォームごとのエラー挙動をテストする。
3.2. `FileOptions`/`FileMetadata` の定義を整備し、`Timestamp` (`Core.Numeric & Time`) と連携する。
3.3. `sync`/`defer` 処理の統合を確認し、リソースリーク検出テストを追加する。

### 4. Path 抽象とセキュリティ（48-49週目）
**担当領域**: パス処理

4.1. `Path`/`PathBuf` と基本操作 (`path`, `join`, `normalize`, `is_absolute`) を実装し、プラットフォーム差異に対するテストを作成する。
4.2. セキュリティヘルパ (`validate_path`, `sandbox_path`, `is_safe_symlink`) を実装し、`effect {security}` の検証を行う。
4.3. 文字列ユーティリティ (`normalize_path`, `join_paths`) を実装し、`Core.Text` と連携するテストを整備する。

### 5. Watcher / 拡張機能（49週目）
**担当領域**: オプション機能

5.1. ファイル監視 API (`watch`, `watch_with_limits`, `close`) を実装し、`effect {io.async}` のハンドリングを確認する。
5.2. 監視イベントを `AuditEnvelope` へ記録する仕組みを整備し、ログの構造化をテストする。
5.3. クロスプラットフォームでサポートが異なる機能は `Capability` 判定と `IoErrorKind::UnsupportedPlatform` で扱う。

### 6. ドキュメント・サンプル更新（49-50週目）
**担当領域**: 情報整備

6.1. 仕様書サンプル・ガイド (`docs/guides/runtime-bridges.md`) を更新し、実装差分を解消する。
6.2. `README.md`/`3-0-phase3-self-host.md` に IO/Path 実装ステータスを記載し、利用者向け注意事項を明示する。
6.3. `examples/` にファイル操作・パス検証の例を追加し、CI で自動実行する。

### 7. テスト・ベンチマーク統合（50週目）
**担当領域**: 品質保証

7.1. 単体・統合テストを追加し、エラー経路・効果タグ・Capability 検証を網羅する。
7.2. IO 性能ベンチマークを実施し、OCaml 実装比 ±15% を目標に評価する。
7.3. テスト結果とリスクを `0-3-audit-and-metrics.md`/`0-4-risk-handling.md` に記録し、追加調整が必要な項目を整理する。

## 成果物と検証
- `Core.IO`/`Core.Path` API が仕様通りに実装され、効果タグ・Capability 検証が合致していること。
- ファイル操作・パス検証がクロスプラットフォームで正しく動作し、未対応機能が明示されていること。
- ドキュメント・サンプルが更新され、安全な IO 利用方法が共有されていること。

## リスクとフォローアップ
- プラットフォーム差異でテストが不安定な場合、対象機能を実験扱いにし `docs/notes/runtime-bridges.md` に制約を記録する。
- 監視 API が OS の制限により提供できない場合、Phase 4 のマルチターゲット検証でフォローアップする。
- `security` 効果の運用が未確定な場合、Capabilities と連携したポリシー策定を Phase 3-8 に委譲する。

## 参考資料
- [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
