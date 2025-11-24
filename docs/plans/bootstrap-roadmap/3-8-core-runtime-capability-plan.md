# 3.8 Core Runtime & Capability 実装計画

## 目的
- 仕様 [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md) に準拠した Capability Registry / Runtime API を実装し、Stage 判定・Capability 検証・監査統合を Reml 実装へ提供する。
- GC/IO/Async/Audit 等の Capability ハンドル登録・検証・記述 (`CapabilityDescriptor`) を整備し、Chapter 3 の各モジュールが安全に利用できる基盤を構築する。
- `verify_capability_stage` や `verify_conductor_contract` を通じたステージ管理を整備し、Manifest (3-7) や Diagnostics (3-6) と連携する。
- 全ステップは Rust 版 Reml コンパイラ（`compiler/rust/`）を唯一の実装対象とし、OCaml 実装は歴史資料として参照する。

## スコープ
- **含む**: CapabilityRegistry 構造、CapabilityHandle バリアント、登録・取得・検証 API、Stage 要件検証、Descriptor 表示、監査連携、ドキュメント更新。
- **含まない**: 各 Capability の個別実装詳細 (Async runtime 等)。それらは Phase 3 の別タスクや Chapter 4 プラグインで扱う。
- **前提**: `Core.Diagnostics`/`Core.Config`/`Core.Runtime` 基盤が整備済みであり、Phase 2 の効果システムタスクが完了していること。

## 作業ブレークダウン

### 1. 仕様差分整理とデータモデル設計（56週目）
**担当領域**: 設計調整

1.1. CapabilityRegistry 構造と CapabilityHandle バリアントの一覧を作成し、既存実装との差分を洗い出す。
1.2. Stage/Effect 情報の保持形式と `CapabilityDescriptor` のフィールドを設計し、Diagnostics/Audit との連携要件を整理する。
1.3. 登録 API (`register`, `describe`) の初期化順序と競合処理を決定する。

### 2. Registry とハンドル実装（56-57週目）
**担当領域**: 基盤 API

2.1. `CapabilityRegistry` 構造体と `registry()` シングルトン取得 API を実装する。
2.2. `CapabilityHandle` バリアントと具象 Capability (Gc/Io/Async/Audit 等) のメタデータ構造を定義する。
2.3. `register`/`get`/`describe` API を実装し、重複登録・未登録エラー (`CapabilityError`) をテストする。

### 3. Stage 検証 API 実装（57週目）
**担当領域**: ステージ管理

3.1. `StageId`/`StageRequirement`/`CapabilityError::StageViolation` を実装し、比較ロジックをテストする。
3.2. `verify_capability`/`verify_capability_stage` を実装し、Stage 条件と効果スコープの検証を行う。
3.3. `verify_conductor_contract` を実装し、Manifest (3-7) と連携した契約チェックをテストする。

### 4. Descriptor と監査統合（57-58週目）
**担当領域**: 観測と可視化

4.1. `CapabilityDescriptor`/`CapabilityMetadata` を実装し、CLI/Diagnostics で利用できる説明文を整備する。
4.2. Capability 検証結果を `AuditEnvelope` へ書き込む API を実装し、`AuditEvent::CapabilityMismatch` の発火をテストする。
4.3. `describe_all` 等の補助関数を実装し、ドキュメント生成や CLI (`reml capability list`) で再利用する。

### 5. 依存モジュールとの統合（58週目）
**担当領域**: Chapter 3 連携

5.1. `Core.Runtime` API から Capability チェックを呼び出し、IO/Time/Async 操作前に検証が行われることを確認する。
5.2. Diagnostics (3-6) の Stage 診断と連携し、Capability 情報が診断出力に含まれることをテストする。
5.3. Config Manifest (3-7) との連携を確認し、マニフェストに記載された Capability 要件が契約検証へ渡ることを確かめる。
5.4. `core.collections.ref` capability を `CapabilityRegistry` に登録し、`RefHandle` を介した `collector.effect.rc`/`collector.effect.mut` の監査を `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` の 3.2.3 セクションと `docs/guides/runtime-bridges.md` の橋渡し解説に記録することで、Core.Collections と RuntimeBridge の契約整合性を担保する。

### 6. ドキュメント・サンプル更新（58-59週目）
**担当領域**: 情報整備

6.1. 仕様書内の Capability テーブル・シーケンス図を実装に合わせて更新する。
6.2. `3-0-phase3-self-host.md`/`README.md` に Capability Registry 実装ステータスと利用ガイドを追記する。
6.3. `docs/notes/dsl-plugin-roadmap.md` に Stage/Capability の適用例を追加し、プラグイン開発者向けに共有する。

### 7. テスト・CI 統合（59週目）
**担当領域**: 品質保証

7.1. 単体テストで登録/検証/記述 API の動作とエラー経路を網羅する。
7.2. 統合テストで TypeChecker/Runtime/Config からの Capability チェックが期待通り動くことを確認する。
7.3. CI に Capability 検証を組み込み、違反が発生した場合に `0-4-risk-handling.md` へ記録する仕組みを追加する。

## 成果物と検証
- Capability Registry/API が仕様通りに実装され、Stage 判定と監査ログが正しく機能すること。
- Diagnostics/Config/Runtime との連携が成立し、Capability 情報が全体で共有されていること。
- ドキュメント・サンプルが更新され、利用者が Capability を確認・登録・検証できる状態になっていること。

## リスクとフォローアップ
- Capability の登録順序が依存関係を満たさない場合、初期化フェーズを再設計し `docs/notes/runtime-bridges.md` に指針を記録する。
- Stage ポリシーが未確定な Capability は `Experimental` 扱いとし、Phase 4 で正式化する。
- 大規模プラグインで Capability 数が増えた場合、登録手順の自動化 (コード生成) をフォローアップに追加する。

## 参考資料
- [3-8-core-runtime-capability.md](../../spec/3-8-core-runtime-capability.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [3-7-core-config-data.md](../../spec/3-7-core-config-data.md)
- [3-5-core-io-path.md](../../spec/3-5-core-io-path.md)
- [3-4-core-numeric-time.md](../../spec/3-4-core-numeric-time.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)
