# 3.7 Core Config & Data 実装計画

## 目的
- 仕様 [3-7-core-config-data.md](../../3-7-core-config-data.md) に準拠した `Core.Config`/`Core.Data` API を Reml 実装へ統合し、マニフェスト・スキーマ・差分管理の標準モデルを確立する。
- `reml.toml` マニフェスト、DSL エクスポート情報、互換性プロファイル、データモデリング (`Schema`, `ChangeSet`) を実装し、監査・診断と連携する。
- Config 互換性ポリシーとステージ管理を提供し、Phase 4 の移行計画へ滑らかに接続する。

## スコープ
- **含む**: Manifest ロード/検証、DSL エクスポート連携、Schema/Manifest API、ConfigCompatibility 設定、差分・監査 API、ドキュメント更新。
- **含まない**: 外部レジストリ連携のネットワークコード、マイグレーションツール自動生成 (Phase 4 で扱う)。
- **前提**: Core.Collections/Diagnostics/IO/Numeric が整備済みであり、Phase 2 の仕様差分解決タスクが完了していること。

## 作業ブレークダウン

### 1. API 差分整理と構造設計（53週目）
**担当領域**: 設計調整

1.1. Manifest/Schema/Data API の公開リストを作成し、既存実装との差分・未実装項目を洗い出す。
1.2. 効果タグ (`effect {config}`, `{audit}`, `{io}`, `{migration}`) と `Diagnostic` との連携ポイントを整理する。
1.3. Manifest/Schema のシリアライズ形式 (TOML/JSON) とバリデーション順序を仕様と照合する。

### 2. Manifest モジュール実装（53-54週目）
**担当領域**: `reml.toml`

2.1. `Manifest`/`ProjectSection`/`DslEntry`/`BuildSection` 等のデータ構造を実装する。
2.2. `load_manifest`/`validate_manifest`/`declared_effects`/`update_dsl_signature` 等の API を実装し、エラー時に `Diagnostic` を返す仕組みを整備する。
2.3. DSL エクスポートシグネチャとの同期 (`@dsl_export`) を確認し、Capability/Stage 情報が正しく投影されることをテストする。

### 3. Schema & ConfigCompatibility 実装（54週目）
**担当領域**: データモデリング

3.1. `Schema`/`Field`/`ValidationRule` など Core.Data の主要構造を実装し、差分 (`SchemaDiff`) の出力を確認する。
3.2. `ConfigCompatibility` 設定 (`trailing_comma`, `unquoted_key`, `duplicate_key` 等) を実装し、フォーマット別既定値をテストする。
3.3. `compatibility_profile`/`resolve_compat` 等の API を実装し、Manifest/RunConfig からの利用を確認する。

### 4. 差分・監査・診断連携（54-55週目）
**担当領域**: Quality & Audit

4.1. `ChangeSet` や `AuditEvent::ConfigCompatChanged` の発火条件を実装し、監査ログと連携する。
4.2. Config 解析エラー (`Diagnostic.code = "config.*"`) のテンプレートとメタデータを実装し、LSP/CLI 出力を確認する。
4.3. `RunConfig` との連携 API を整備し、`Core.Env`/`Core.Runtime` との接合をテストする。

### 5. データ互換性・マイグレーション支援（55週目）
**担当領域**: 将来互換

5.1. `MigrationPlan`/`MigrationStep` (仕様に記載された実験的 API) のドラフトを実装し、`effect {migration}` の扱いを定義する。
5.2. Manifest/Schema のバージョン互換チェックを追加し、移行シナリオを `notes/dsl-plugin-roadmap.md` に記録する。
5.3. CLI 連携 (`reml config lint`, `reml config diff`) の出力仕様を整備し、サンプルを作成する。

### 6. ドキュメント・サンプル更新（55-56週目）
**担当領域**: 情報整備

6.1. 仕様書内の表・サンプルを実装に合わせて更新し、`samples/` に Manifest/Schema 例を追加する。
6.2. `README.md`/`3-0-phase3-self-host.md` に Config/Data 実装状況を記載し、Phase 4 への連携点をまとめる。
6.3. `guides/runtime-bridges.md`/`guides/plugin-authoring.md` 等で設定連携の記述を更新する。

### 7. テスト・CI 統合（56週目）
**担当領域**: 品質保証

7.1. Manifest/Schema の単体・統合テストを追加し、バリデーションエラーや互換性チェックのケースを網羅する。
7.2. 差分出力のスナップショットテストと監査ログ検証を行う。
7.3. CI へ Config Lint を組み込み、回帰時に `0-4-risk-handling.md` へ自動記録する。

## 成果物と検証
- Manifest/Schema/ConfigCompatibility API が仕様通りに実装され、効果タグ・診断・監査が整合すること。
- DSL エクスポート・Capability 情報がマニフェストから取得でき、Phase 4 の移行処理に再利用できること。
- ドキュメント・サンプルが更新され、設定ファイルの互換性ポリシーが明確であること。

## リスクとフォローアップ
- TOML/JSON パーサの差異で互換性チェックが不安定な場合、フォーマット別に冪等テストを追加し、必要なら構文制限を仕様側へ提案する。
- Migration API が未成熟な場合、Phase 4 で段階的導入する前提で `notes/` に TODO を残す。
- レジストリ連携で追加機能が必要になった場合、`notes/dsl-plugin-roadmap.md` に記録し、エコシステム計画 (5-x) と調整する。

## 参考資料
- [3-7-core-config-data.md](../../3-7-core-config-data.md)
- [3-6-core-diagnostics-audit.md](../../3-6-core-diagnostics-audit.md)
- [3-4-core-numeric-time.md](../../3-4-core-numeric-time.md)
- [3-5-core-io-path.md](../../3-5-core-io-path.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)
