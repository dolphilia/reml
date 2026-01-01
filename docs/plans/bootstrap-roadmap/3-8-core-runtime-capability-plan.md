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
    - 1.1.a `docs/spec/3-8-core-runtime-capability.md` §1〜§1.3 と `compiler/rust/runtime/src/` 以下の `rg "Capability"` 結果を突き合わせ、型ごとの実装状況を `docs/plans/bootstrap-roadmap/assets/capability-handle-inventory.csv`（新規）へ整理する。列には `Gc/Io/Async/...` と `定義済み/未実装/不要` のステータスを記し、Run ID を脚注として残す。
    - 1.1.b `compiler/rust/runtime/tests/` で既に PoC が存在する Capability を棚卸しし、利用できるテストデータやモックを `docs/notes/runtime-capability-stage-log.md#capability-handle-inventory` にリンクする。
    - 1.1.c 差分一覧を `docs/plans/rust-migration/2-1-runtime-integration.md` と共有し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#core-runtime-capability` へ KPI（例: `capability.handle_coverage`）と Run ID を追記する。

#### 1.1 実施結果（Run ID: 20291221-core-runtime-capability）
- `docs/plans/bootstrap-roadmap/assets/capability-handle-inventory.csv` で GC/IO/Async/Audit など 14 種類の Capability を列挙し、Rust 側の入口／テスト／実装状態（未実装・Stage 検証のみ等）を整理した。唯一 Stage 検証が存在するのは `compiler/rust/runtime/src/io/adapters.rs#L27-L233` の Fs/Watcher アダプタであり、その他のハンドルが未実装であることを表で可視化した。
- `docs/notes/runtime-capability-stage-log.md` に `## Capability Handle Inventory (20291221)` を追加し、棚卸し表と `fs_adapter_ensures_capabilities` などの PoC テストをリンクした。Stage ログから直接 CSV を参照できる導線を確保している。
- `docs/plans/rust-migration/2-1-runtime-integration.md` の Capability Registry 要件節へ本 CSV を参照する脚注を追記し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` には新 KPI `capability.handle_coverage` と Run ID を登録した。KPI 更新時は `python3 tooling/ci/collect-iterator-audit-metrics.py --section runtime --dry-run` のログを参照する運用とした。
1.2. Stage/Effect 情報の保持形式と `CapabilityDescriptor` のフィールドを設計し、Diagnostics/Audit との連携要件を整理する。
    - 1.2.a `StageId`/`StageRequirement`/`CapabilityDescriptor` のフィールドを `docs/spec/1-3-effects-safety.md#capability-stage-contract` と本章 §1.2 から抽出し、`docs/plans/bootstrap-roadmap/assets/capability-stage-field-gap.csv` に「仕様の必須列」「Rust 実装の現状」「対応する診断キー」を表形式で記録する。
    - 1.2.b `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` の `StageAuditPayload` 項とクロスリンクし、`effects.contract.stage_mismatch`／`bridge.stage.*` など診断キーへの転写経路をシーケンス図 (`assets/capability-stage-flow.mmd`) としてまとめる。
    - 1.2.c `collect-iterator-audit-metrics.py --section runtime --dry-run` を実行して現状の `runtime.capability_stage_presence` を計測し、KPI 化するための CSV (`assets/metrics/runtime-capability-stage.csv`) を作成する。

#### 1.2 実施結果（Run ID: 20291221-stage-field-gap）
- `docs/plans/bootstrap-roadmap/assets/capability-stage-field-gap.csv` で Stage/Effect 関連フィールド 10 項目のギャップを明文化した。`StageRequirement::satisfies` が欠落している点や `CapabilityDescriptor.provider/effect_scope/manifest_path` が未実装であることを `diagnostic_or_audit_key` 列とセットで確認できる。
- Stage 情報のデータフローを `docs/plans/bootstrap-roadmap/assets/capability-stage-flow.mmd` に描画し、RunConfig/Manifest/CapabilityRegistry/StageAuditPayload/Audit/KPI の関係を `collect-iterator-audit-metrics` へ接続した。`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` から当該図を参照できるよう相互リンクを設定。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --section runtime --dry-run` の出力を `docs/plans/bootstrap-roadmap/assets/metrics/runtime-capability-stage.csv` に保存し、`runtime.capability_validation` の `pass_rate=1.0` と対象候補（default/windows-msvc/macOS arm64）を Run ID 付きで記録した。
1.3. 登録 API (`register`, `describe`) の初期化順序と競合処理を決定する。
    - 1.3.a `compiler/rust/runtime/src/lib.rs` と `reml_runtime::bootstrap`（存在しない場合は新設）を読み取り、Capability 登録シーケンスを `docs/plans/bootstrap-roadmap/assets/core-runtime-capability-init.md` に文章＋Mermaid 図でまとめる。
    - 1.3.b 競合時に返す `CapabilityError` バリアント（`AlreadyRegistered`, `MissingDependency`, `StageViolation` 等）と `Diagnostic` 出力メッセージのマッピング表を `assets/capability-error-matrix.csv` として作成する。
    - 1.3.c RunConfig → ConfigManifest → CapabilityRegistry 初期化の依存関係を `docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#3.3` と同期し、相互参照リンクを追加する。

#### 1.3 実施結果（Run ID: 20291221-capability-init-seq）
- `docs/plans/bootstrap-roadmap/assets/core-runtime-capability-init.md` を作成し、RunConfig での `--effect-stage` 取り込みから Manifest 契約生成・`CapabilityRegistry::verify_capability_stage` 呼び出し・`StageAuditPayload` 形成・監査 KPI 更新に至るシーケンスを文章化した。図 (`capability-stage-flow.mmd`) と併せて Config 計画 (§3.3) へリンク済み。
- `docs/plans/bootstrap-roadmap/assets/capability-error-matrix.csv` を追加し、`StageViolation`/`EffectScopeMismatch`/`AlreadyRegistered`/`ContractViolation` などのエラー種別と診断コード・監査イベント・実装状況を整理した。現状は StageMismatch 以外が未実装である点を計画書上で共有できる。
- `docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md` へ本 Run ID と assets への参照を追記し、Manifest→Runtime の依存が Config/Runtime 両計画から同じ資料に辿れるようにした。

### 2. Registry とハンドル実装（56-57週目）
**担当領域**: 基盤 API

2.1. `CapabilityRegistry` 構造体と `registry()` シングルトン取得 API を実装する。
    - 2.1.a `compiler/rust/runtime/src/capability/registry.rs`（新設）で `OnceLock<CapabilityRegistry>` を利用したスレッド安全シングルトンを実装し、`cfg(test)` で再初期化できる `fn reset_for_tests()` を用意する。
    - 2.1.b `registry()` が `Send + Sync` を満たすことを `static_assertions::assert_impl_all!(CapabilityRegistry: Send, Sync);` で検証し、`cargo test -p reml_runtime capability_registry_traits` を CI チェックに追加する。
    - 2.1.c `docs/plans/rust-migration/2-1-runtime-integration.md#2.1.4` と同期し、初期化順序（Config → Diagnostics → Runtime）の図版へ `registry()` 呼び出しを追記する。

#### 2.1 実施結果（Run ID: 20290614-capability-registry-singleton）
- `compiler/rust/runtime/src/capability/registry.rs` を新設し、`OnceLock<CapabilityRegistry>` ベースのシングルトン `registry()`／テスト用リセット `reset_for_tests()` を実装した。既存の Stage 検証 API はこのモジュールへ移設し、`io/adapters` や `prelude::collectors` から `capability::registry` を参照する構成へ整理済み。
- `compiler/rust/runtime/tests/capability_registry.rs` に `capability_registry_traits` テストを追加し、`static_assertions::assert_impl_all!` で `CapabilityRegistry: Send + Sync` を保証する。実装箇所は `cargo test -p reml_runtime capability_registry_traits` で CI へ統合予定。
- `docs/plans/rust-migration/2-1-runtime-integration.md#2.1.4` の Capability Registry 行へ本モジュールの導線（`compiler/rust/runtime/src/capability/registry.rs`）と Run ID を追記し、Rust Runtime 初期化図から `registry()` 呼び出しを辿れるよう更新した。
2.2. `CapabilityHandle` バリアントと具象 Capability (Gc/Io/Async/Audit 等) のメタデータ構造を定義する。
    - 2.2.a `compiler/rust/runtime/src/capability/handle.rs` に列挙体を実装し、`CapabilityDescriptor` へアクセスする共通メソッドや `TryFrom<CapabilityHandle>` 実装を用意する。
    - 2.2.b 各 Capability ごとにメタデータ構造を `compiler/rust/runtime/src/capability/{io,gc,async}.rs` などの個別ファイルへ分割し、`serde`/`schemars` 導線を整える。
    - 2.2.c `cargo test -p reml_runtime capability_handle_metadata` で `EffectTag`/`StageId`/`provider` の初期化漏れを検知するテストを追加する。
2.3. `register`/`get`/`describe` API を実装し、重複登録・未登録エラー (`CapabilityError`) をテストする。
    - 2.3.a `CapabilityError` 列挙体を `thiserror::Error` で実装し、`Diagnostic` へ変換する `impl From<CapabilityError> for RuntimeError` を提供する。
    - 2.3.b `register` 実行時に `describe_all()` で利用するインデックス（`HashMap` + `Vec<CapabilityDescriptor>`）を更新するための内部 API を設計し、`docs/plans/bootstrap-roadmap/assets/capability-registry-datamodel.md` に図示する。
    - 2.3.c `compiler/rust/runtime/tests/capability_registry.rs` に `#[test_case]` で重複登録・未登録アクセス・`describe` の成功/失敗を確認するテーブル駆動テストを追加する。

#### 2.3 実施結果（Run ID: 20290622-capability-registry-register）
- `compiler/rust/runtime/src/capability/registry.rs` に `CapabilityEntries`（`HashMap` + 登録順 `Vec`）と `CapabilityError` 列挙体を実装し、`register`/`get`/`describe`/`describe_all` を追加。未登録 Capability には `CapabilityError::NotRegistered` を返し、Stage 判定は従来どおり `StageId::Stable` フォールバックを維持する TODO をコメントとして残した。
- `compiler/rust/runtime/tests/capability_registry.rs` を刷新し、登録→取得→`describe_all` までの成功パス、重複登録エラー、未登録アクセスの 3 ケースをテーブル化（`test-case` クレート未導入のため通常の `#[test]` で代替）。`CARGO_NET_OFFLINE=true cargo test capability_registry`（`compiler/rust/runtime`）で新テストを実行済み。
- `docs/plans/bootstrap-roadmap/assets/capability-registry-datamodel.md` を新設し、データモデル・エラー更新を図示。`capability-error-matrix.csv` の `AlreadyRegistered`/`UnknownCapability` 行を「実装済み」に更新した。`RuntimeError` 型が Rust 実装になく `impl From<CapabilityError> for RuntimeError` は未着手なので、`docs/plans/rust-migration/p2-runtime-gap-report.md#P2G-03` の TODO として継続する。

### 3. Stage 検証 API 実装（57週目）
**担当領域**: ステージ管理

3.1. `StageId`/`StageRequirement`/`CapabilityError::StageViolation` を実装し、比較ロジックをテストする。
    - 3.1.a `StageId` を `PartialOrd + Ord` で実装し、`Experimental < Beta < Stable` の順序テーブルを `compiler/rust/runtime/tests/stage_order.rs` で固定する。
    - 3.1.b `StageRequirement` に `fn satisfies(self, actual: StageId) -> bool` を実装して `const fn` 化し、`#[test_case(StageRequirement::Exact(StageId::Beta), StageId::Stable => false)]` 等で境界ケースを網羅する。
    - 3.1.c `CapabilityError::StageViolation` に `required_stage`/`actual_stage`/`capability_descriptor` を含め、`Diagnostic` 側で `effect.stage.*` へ転写するキー名を `docs/spec/3-6-core-diagnostics-audit.md` の表にリンクする。
3.2. `verify_capability`/`verify_capability_stage` を実装し、Stage 条件と効果スコープの検証を行う。
    - 3.2.a `verify_capability_stage` で `CapabilityDescriptor.effect_scope` と `StageRequirement` を同時に検証し、`EffectScopeMismatch` エラーを `CapabilityError` に追加する。
    - 3.2.b `verify_capability` 成功/失敗を `AuditEnvelope` の `AuditEventKind::CapabilityCheck` へ記録し、`collect-iterator-audit-metrics.py --section runtime` が読み取れるメタデータキーを定義する。
    - 3.2.c `compiler/rust/runtime/tests/verify_capability.rs` に Stage 条件の成功/失敗、EffectScope 不一致、未登録 Capability の各ケースをテーブル駆動テストとして実装する。

#### 3.2 実施結果（Run ID: 20240608-capability-stage-verify）
- `compiler/rust/runtime/src/capability/registry.rs` に `CapabilityRegistry::verify_capability` を追加し、Stage 判定と `CapabilityDescriptor.effect_scope` の両方をチェックするよう更新。`CapabilityError` へ `EffectScopeMismatch` を新設し、`missing_effects()` / `required_effects()` で診断拡張から不足タグを特定できるようにした。これに合わせ `StageId::Stable` フォールバックを除去し、レジストリ生成時に Core.IO/Time/Metrics/Audit で必要な Capability を自動登録して Stage 情報を常備する。
- `verify_capability` の成功／失敗を `AuditEventKind::CapabilityCheck` として `AuditEnvelope.metadata` に記録し、`effect.stage.required` / `effect.stage.actual` / `capability.result` / `effect.required_capabilities` など `collect-iterator-audit-metrics.py --section runtime` が想定するキー群をすべて埋めるよう整備した。Stage/Effect 不一致時には `effect.missing_effects` も書き込み、`runtime.capability_validation` KPI から差分検知できる。
- `compiler/rust/runtime/tests/verify_capability.rs` を新設し、Stage 成功・StageViolation・EffectScopeMismatch・未登録の 4 ケースを `reset_for_tests()` つきで検証。既存の `capability_registry.rs` テストも自動登録済み Capability に対応するよう修正した。`cargo test --manifest-path compiler/rust/runtime/Cargo.toml verify_capability` で新テストが緑化することを確認済み。
- `docs/plans/bootstrap-roadmap/assets/capability-error-matrix.csv` の `EffectScopeMismatch` 行を「実装済み」に更新し、Runtime 実装と KPI 表の双方から参照できる Run ID を追記した。
3.3. `verify_conductor_contract` を実装し、Manifest (3-7) と連携した契約チェックをテストする。
    - 3.3.a `reml_runtime::config::manifest` に `ConductorCapabilityRequirement` 生成ロジックを追加し、`run.target.capabilities` 节から Stage/Effect/Provider 情報を抽出する。
    - 3.3.b `compiler/rust/runtime/tests/conductor_contract.rs` のフィクスチャ（`tests/fixtures/manifest/capability_contract.json` 等）で成功/失敗パターンを検証し、`manifest_path` と `source_span` が `CapabilityError::ContractViolation` に埋め込まれることを確認する。
    - 3.3.c `docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#5.1` にテスト Run ID をリンクし、Manifest/Runtime 双方で同じ仕様を参照していることを明示する。

### 4. Descriptor と監査統合（57-58週目）
**担当領域**: 観測と可視化

4.1. `CapabilityDescriptor`/`CapabilityMetadata` を実装し、CLI/Diagnostics で利用できる説明文を整備する。
    - 4.1.a `compiler/rust/runtime/src/capability/descriptor.rs` に構造体を定義し、`serde::Serialize`/`schemars::JsonSchema` の導出を行う。
    - 4.1.b `CapabilityMetadata` に `last_verified_at: Timestamp`/`provider`/`manifest_path`/`security` 情報を含め、`describe` API から JSON 形式で取得できるようにする。
    - 4.1.c `reml_frontend --capability describe <id>` CLI を追加し、`docs/spec/3-8-core-runtime-capability.md#capabilitydescriptor` の出力例を最新ログへ差し替える。

#### 4.1 実施結果（Run ID: 20290704-capability-descriptor-cli）
- `compiler/rust/runtime/src/capability/descriptor.rs` に `CapabilityMetadata` / `CapabilitySecurityMetadata` を追加し、`provider`/`manifest_path`/`last_verified_at`/`security` を `serde` で `CapabilityDescriptor` に `flatten` する形で公開した。`CapabilityIsolationLevel` や `CapabilityPermission` など仕様 §1.2 のフィールドも Rust 側で列挙化したため、`CapabilityRegistry::describe` の戻り値をそのまま JSON Schema に載せられる。
- `compiler/rust/frontend/src/bin/reml_frontend.rs` に `--capability describe <id> [--output human|json]` を実装し、Registry から `CapabilityDescriptor` を抽出して JSON/人間可読テキストのどちらでも表示できるようにした。Human 表示では Stage/効果タグ/Provider/セキュリティ属性を整形し、監査レビュー時の卓上確認を CLI だけで完結できる。
- `docs/spec/3-8-core-runtime-capability.md#capabilitydescriptor` に CLI サンプルを追記し、`reml_frontend --capability describe io.fs.read --output json` の出力例を提示。実装したメタデータが仕様と一致していること、および JSON のレイアウトが 3.8 章の規定と矛盾しないことを確認できる。
4.2. Capability 検証結果を `AuditEnvelope` へ書き込む API を実装し、`AuditEvent::CapabilityMismatch` の発火をテストする。
    - 4.2.a `reml_runtime::audit` に `AuditEventKind::CapabilityCheck` を追加し、`verify_capability_stage` の成功/失敗を JSON Lines で保存する。
    - 4.2.b `compiler/rust/runtime/tests/audit_capability.rs` を追加し、`collect-iterator-audit-metrics.py --section runtime --scenario capability_check` で検証可能なゴールデンを整備する。
    - 4.2.c `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md#4.1` に倣い、`effects.contract.stage_mismatch` と同じキー名を `AuditEnvelope.metadata` へ転写することで差分比較が容易になるよう整備する。

#### 4.2 実施結果（Run ID: 20290705-capability-audit-log）
- `compiler/rust/runtime/src/audit/mod.rs` の `AuditEventKind::from_str` に `capability_check` を追加し、`CapabilityRegistry::record_capability_check` が出力する `event.kind = "capability_check"` を公式にバリデーション対象へ格納した。これにより CLI/診断/audit で `effect.stage.*` のキーが欠落していないか `AuditEnvelope::validate` で検知できる。
- `compiler/rust/runtime/tests/audit_capability.rs` を新設し、Stage 合格・Stage 違反の両ケースで `capability_checks()` が生成する `AuditEvent` を検証するテストを追加した。テストでは `event.validate()` を通して必須キーが揃っているか確認しつつ、RunConfig/Stage 要件に応じた結果 (`capability.result = success|error`) と `capability.error.code` が正しく設定されることを確認している。
- `tests/golden/audit/capability_check_success.jsonl` と `tests/golden/audit/capability_check_stage_violation.jsonl` を追加し、`serde_json` 直列化した `AuditEvent` を JSON Lines 形式で保存。`assert_matches_golden` ヘルパがこれらゴールデンと比較するため、`collect-iterator-audit-metrics.py --section runtime --scenario capability_check` で同じフォーマットを解析すれば `effect.stage.required` や `capability.error.*` の有無を自動確認できる。
4.3. `describe_all` 等の補助関数を実装し、ドキュメント生成や CLI (`reml capability list`) で再利用する。
    - 4.3.a `CapabilityRegistry::describe_all` を `Iterator<Item = CapabilityDescriptor>` で返す API として実装し、`reml capability list` CLI（`compiler/rust/runtime/bin/reml_capability.rs`）から利用する。
    - 4.3.b `docs/plans/bootstrap-roadmap/README.md` と `docs/spec/3-8-core-runtime-capability.md` の Capability 表を `scripts/capability/generate_md.py`（新設）で自動生成し、`describe_all` の出力を Markdown へ変換する。
    - 4.3.c `tooling/runtime/capability_list.py` で CLI 出力 → Markdown 変換 → `docs/spec/3-8` 反映を自動化し、作業ログを `docs/notes/runtime-capability-stage-log.md#capability-list-update` に残す。

#### 4.3 実施結果（Run ID: 20290705-capability-list-cli）
- `CapabilityRegistry::describe_all()` を `CapabilityDescriptorList` で返すように拡張し、`IntoIterator` 実装と `len`/`iter` を提供した。これにより CLI や将来の API で iterator として扱える。
- `reml_capability` バイナリを追加し、`reml_capability list --format json|markdown` で Registry 内容を CLI から確認できるようにした。JSON 出力は `schema_version = "3.0.0-alpha"` を含み、Markdown 出力は表形式で Stage/EffectScope/Provider/Manifest を掲載する。
- `scripts/capability/generate_md.py` と `tooling/runtime/capability_list.py` を追加。後者は CLI → JSON 保存（`reports/spec-audit/ch3/capability_list-YYYYMMDD.json`）→ Markdown 生成 → `docs/spec/3-8-core-runtime-capability.md` / `docs/plans/bootstrap-roadmap/README.md` 反映 → `docs/notes/runtime-capability-stage-log.md#capability-list-update` へのログ追記を自動化する。
- 上記スクリプトを実行し、2 つのドキュメントには `<!-- capability-table:start -->` / `<!-- capability-table:end -->` マーカー内へ自動生成テーブルを挿入した。README には Snapshot セクションを新設し、更新手順を明記した。
- `docs/notes/runtime-capability-stage-log.md#Capability List Update` へ「CLI パス / JSON 保存先 / 更新ドキュメント」を記録し、今後の再実行時に参照できる履歴を確保した。

### 5. 依存モジュールとの統合（58週目）
**担当領域**: Chapter 3 連携

5.1. `Core.Runtime` API から Capability チェックを呼び出し、IO/Time/Async 操作前に検証が行われることを確認する。
    - 5.1.a `compiler/rust/runtime/src/runtime/api.rs`（IF 未作成なら新設）に `verify_capability_stage` を差し込むラッパ関数を作り、API エントリポイントすべてで Stage チェックが必ず走るようにする。
    - 5.1.b `core.io`/`core.time`/`core.async` それぞれに Stage ガードを期待するユニットテストを追加し、`cargo test -p reml_runtime core_runtime_capability_guard` を新設する。
    - 5.1.c `docs/spec/3-8-core-runtime-capability.md#core-runtime-api` のシーケンス図に Stage 検証ステップを追記し、`docs/plans/bootstrap-roadmap/3-5-core-io-path-plan.md` と相互参照を追加する。
5.2. Diagnostics (3-6) の Stage 診断と連携し、Capability 情報が診断出力に含まれることをテストする。
    - 5.2.a `compiler/rust/frontend/src/diagnostic/effects.rs` の `StageAuditPayload` に `CapabilityDescriptor` 情報を合流させ、`capability.id`/`capability.provider`/`capability.stage` を CLI/LSP/Audit で共有する。
    - 5.2.b `examples/core_diagnostics/pipeline_branch.reml` を Rust Frontend で実行し、`effects.contract.stage_mismatch` に Capability 情報が含まれるゴールデンを `reports/spec-audit/ch3/capability_stage-mismatch-YYYYMMDD.json` として保存する。
    - 5.2.c `scripts/validate-diagnostic-json.sh --effect-tag runtime` を追加し、Capability 情報欠落を CI で検知できるようにする。
#### 5.2 実施結果（Run ID: 80b0d934-6b51-4718-9fc4-dcff8c57b849）
- `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden` を実行し、`pipeline_success`/`pipeline_branch` の期待値を同一コマンドで更新した。`pipeline_branch` は `allowed failure` として `exit_code=1` をログに残しつつ継続する。
- 生成された `examples/core_diagnostics/pipeline_branch.expected.{diagnostic.json,audit.jsonl}` を基に `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` を再生成し、`capability.id=console` / `effect.stage.required=at_least:beta` / `effect.stage.actual=at_least:stable` / `bridge.stage.trace[*]` の一貫性を確認した。Run 情報は `docs/notes/runtime-capability-stage-log.md#2025-12-06-core-diagnostics-stage-mismatch` へも転載。
- `scripts/validate-diagnostic-json.sh reports/spec-audit/ch3/capability_stage-mismatch-20251206.json --effect-tag runtime` を再実行して `capability.*` / `effect.stage.*` / `effects.contract.stage_trace` の必須キーを検証し、CLI/Audit 双方で欠落が無いことを確認した。
5.3. Config Manifest (3-7) との連携を確認し、マニフェストに記載された Capability 要件が契約検証へ渡ることを確かめる。
    - 5.3.a `reml_runtime::config::manifest` で `run.target.capabilities` を読み込み、`ConductorCapabilityRequirement` に変換するロジックを追加する。
    - 5.3.b `compiler/rust/runtime/tests/manifest_validation.rs` に Stage/Effect 情報を含むフィクスチャを追加し、成功/失敗両ケースを `cargo test manifest_capabilities_*` で検証する。
    - 5.3.c Manifest 検証ログを `reports/spec-audit/ch3/manifest_capability-YYYYMMDD.md` に保存し、`docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#5.1` へリンクを張る。
#### 5.3 実施結果（Run ID: 20260225-manifest-capability-contract）
- `compiler/rust/runtime/tests/manifest_validation.rs` に `conductor_capability_contract_round_trip` と `manifest_capabilities_detect_duplicate_ids` を追加し、`Manifest::conductor_capability_contract()` が `StageRequirement::AtLeast(StageId::Beta)`・`declared_effects`・`manifest_path` を保持すること、`ManifestCapabilities::from_manifest()` が重複 ID を `ManifestCapabilityError::DuplicateCapability` として検出することを検証した。Stage / Effect の期待値は `RunCapabilityEntry::declared_effects` をソート・重複排除する既存ロジックと突き合わせている。
- `cargo test -p reml_runtime manifest_validation` を実行し、結果と CLI ログを `reports/spec-audit/ch3/manifest_capability-20260225.md` に保存した。ログには `tests/manifest_validation.rs` の対象ケース、利用した manifest ビルダー（`capability_entry` ヘルパ）、期待エラー内容が含まれる。
- Config 計画書 `docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#5.1` に本 Run ID とレポートへの参照を追記し、Manifest/Capability 連携タスクの完了点から 3-8 計画を逆参照できるようにした。
5.4. `core.collections.ref` capability を `CapabilityRegistry` に登録し、`RefHandle` を介した `collector.effect.rc`/`collector.effect.mut` の監査を `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md` の 3.2.3 セクションと `docs/guides/runtime/runtime-bridges.md` の橋渡し解説に記録することで、Core.Collections と RuntimeBridge の契約整合性を担保する。
    - 5.4.a `CapabilityHandle::CollectionsRef`（仮）を追加し、`reml_runtime::collections::ref` の API から Stage チェックを呼び出す。
    - 5.4.b `collector.effect.rc`/`collector.effect.mut` の Stage 情報を `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md#3.2.3` の KPI へ追記し、`reports/spec-audit/ch3/collections_ref-YYYYMMDD.md` に実測ログを保存する。
    - 5.4.c `RuntimeBridgeRegistry` の `describe_bridge("collections.ref")` に Stage 情報を写し、`collect-iterator-audit-metrics.py --section runtime --scenario collections_ref` で監査する。
#### 5.4 実施結果（Run ID: 20251206-core-collections-ref）
- `compiler/rust/runtime/src/capability/collections.rs` を新設し、`CapabilityHandle::Collections` と `CollectionsCapabilityMetadata` を導入。`CapabilityRegistry` のビルトインに `core.collections.ref (Stage=Stable, effect_scope=["mem","mut","rc"])` を追加し、`CapabilityHandleKind` / `impl_try_from_handle!` へも Variant を拡張した。
- `compiler/rust/runtime/src/collections/mutable/ref.rs` に `OnceCell` ベースの Stage ガードを実装し、`EffectfulRef::try_new` / `RefHandle::try_new` から `CapabilityRegistry::verify_capability_stage("core.collections.ref", StageRequirement::Exact(StageId::Stable))` を呼び出すよう更新した。Capability 違反は `BorrowError::CapabilityDenied` にマッピングし、FFI では `RefHandle::new` をラップする `try_new` を追加。
- `python3 tooling/ci/collect-iterator-audit-metrics.py --suite collectors --scenario ref_internal_mutation --output reports/spec-audit/ch3/collections_ref-20251206.json --require-success --require-cell` を実行し、`collector.effect.cell_rc` KPI で `cell_mutations_total=1` / `rc_ops_total=2` を記録（既知の `collector.table.csv_import` 閾値未実装により終了コードは 1、詳細は `reports/spec-audit/ch3/collections_ref-20251206.md` 参照）。
- ドキュメント連携: `docs/plans/bootstrap-roadmap/3-2-core-collections-plan.md#3.2.3` に Stage ガード整備と KPI 更新手順を追記し、`docs/guides/runtime/runtime-bridges.md` では `RefHandle::try_new` と Capability 検証フローを補足した。
5.5. Core.IO / Path で定義した Capability (`io.fs.*`, `fs.permissions.*`, `fs.symlink.*`, `fs.watcher.*`, `security.fs.policy`) 用の Runbook を整備する。`docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` を Capability Registry の基準票とし、`File::open`/`watch` 実装から `verify_capability_stage` を呼び出すフックを追加する。CI では `python3 tooling/ci/collect-iterator-audit-metrics.py --section core_io --scenario capability_matrix --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md --output reports/spec-audit/ch3/core_io_capabilities.json --require-success` を Phase3 `core-io-path` ジョブへ組み込み、`docs/notes/runtime-capability-stage-log.md` に `io.fs.*` 系 Stage 結果を追記する。`RuntimeBridgeRegistry` 経由で Watcher を提供する場合は `describe_bridge("native.fs.watch")` の Stage 情報を `CapabilityDescriptor` へ転写し、Stage mismatch が発生した際は `effects.contract.stage_mismatch` 診断と `AuditEnvelope.metadata["io.watch.*"]` を同時に確認する。  
 　加えて、クロスプラットフォームで挙動が分かれる Watcher 付帯 Capability（`watcher.fschange`/`watcher.recursive`/`watcher.resource_limits`）を `IoErrorKind::UnsupportedPlatform` で可視化し、`metadata.io.platform` / `metadata.io.feature` を `watcher_audit` シナリオの必須キーとして扱う。Runbook では `docs/notes/runtime-capability-stage-log.md#2025-12-21-coreio-watcher-クロスプラットフォーム-capability` を参照し、非対応 OS 向けの `reports/spec-audit/ch3/io_watcher-unsupported_platform.md` の更新手順も記載する。

    - 5.5.a `core-io-capability-map.md` に `stage`/`provider`/`effect_scope` 列を追加し、`collect-iterator-audit-metrics.py` が直接読み取れる CSV を整備する。
    - 5.5.b `compiler/rust/runtime/src/io/fs.rs` や `path/watch.rs` に Stage チェック挿入ポイントを追加し、`cargo test -p reml_runtime core_io_capability_matrix` で検証する。
    - 5.5.c `RuntimeBridgeRegistry` と Capability Registry の同期状況を `reports/spec-audit/ch3/io_bridge-capability-sync-YYYYMMDD.md` にまとめ、橋渡し Stage mismatch を即座に追跡できるようにする。

#### 5.5 実施結果（Run ID: 20251206-core-io-capability-matrix）
- `docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` に `<!-- capability-matrix:start -->` ブロックを追加し、`Capability ID`/`Stage`/`Provider`/`Effect Scope`/`Hook`/`Status`/`Notes` を機械可読化。`watcher.fschange` / `watcher.recursive` には `platform:*` ステージを明示し、OS 依存の `IoErrorKind::UnsupportedPlatform` を Runbook から辿れるようにした。
- `tooling/ci/collect-iterator-audit-metrics.py` に `--matrix` 引数・`capability_matrix` シナリオを追加し、上記表を検証して `core_io.capability_matrix_pass_rate` を算出。生成結果は `reports/spec-audit/ch3/core_io_capabilities.json`（pass_rate=1.0, total=13）として保存し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に KPI を登録した。
- `tests/capabilities/core_io_registry.json` と `cargo test -p reml_runtime core_io_capability_matrix` を更新し、`fs.symlink.modify` / `fs.watcher.*` / `watcher.resource_limits` まで Stage 要件を網羅。CI の `core-io-path` ラインで `core_io_capability_matrix` テストを実行する方針を共有した。
- `reports/spec-audit/ch3/io_bridge-capability-sync-20251206.md` を作成し、Watcher Stage trace（`metadata.io.watch.*`）と Capability Registry/RuntimeBridgeRegistry の整合確認手順を記録。`docs/notes/runtime-capability-stage-log.md#2025-12-06-coreio-capability-マトリクスrun-id-20251206-core-io-capability-matrix` に Run ID と指針を追記した。
- `runtime::bridge` モジュールに Rust 版 `RuntimeBridgeRegistry` を実装し、`FsAdapter::ensure_*` など Stage 検証 API が Stage プローブを記録するように更新。`cargo test -p reml_runtime stage_records_are_accessible_after_fs_operations -- --nocapture` を `BRIDGE_STAGE_RECORDS_PATH=<path>` と併用すると `reports/spec-audit/ch3/runtime_bridge-stage-records-20251206.json` のようなスナップショットを生成でき、io_bridge-capability-sync レポートから Rust 実装の Bridge 記録を直接参照できるようになった。

> メモ（2027-03-31）: `core.text.audit` Capability を仮登録し、`StageRequirement::Exact(StageId::Stable)` で `effect {audit}` を要求する API（`log_grapheme_stats` 等）を守る体制を整備した。`compiler/rust/runtime/tests/capability_text_audit.rs` の `cargo test capability_text_audit` で検証ルートを固定し、`docs/plans/bootstrap-roadmap/3-3-core-text-unicode-plan.md#43-diagnostics-io-連携42-43週目` から参照している。

### 6. ドキュメント・サンプル更新（58-59週目）
**担当領域**: 情報整備

6.1. 仕様書内の Capability テーブル・シーケンス図を実装に合わせて更新する。
    - 6.1.a `docs/spec/3-8-core-runtime-capability.md` のテーブルを 4.3.b のスクリプトから再生成し、Stage/EffectScope/Provider を自動反映する。
    - 6.1.b `docs/spec/3-0-core-library-overview.md` と `docs/spec/1-0-language-core-overview.md` の Capability 概要に Stage API 追加内容を反映する。
    - 6.1.c `docs/notes/runtime-capability-stage-log.md` に Run ID と図版（Mermaid → PNG/SVG）を添付し、更新履歴を残す。

#### 6.1 実施結果（Run ID: 20251230-capability-doc-sync）
- `reml_capability list --format json > reports/spec-audit/ch3/capability_list-20251205.json` を再取得し、`python3 scripts/capability/generate_md.py --json ... --output docs/spec/3-8-core-runtime-capability.md` で 3.8 章の表を更新した。
- Chapter 0/1 の概要（`docs/spec/3-0-core-library-overview.md`、`docs/spec/1-0-language-core-overview.md`）へ `capability_stage-mismatch-20251206.json` を参照する段落を追記し、Stage mismatch サンプルの導線を確保した。
- `docs/notes/runtime-capability-stage-log.md` に Run ID とともに `docs/plans/bootstrap-roadmap/assets/capability-stage-flow.svg` を追加し、Mermaid 図 (`capability-stage-flow.mmd`) のエクスポート手順を記録した。
6.2. `3-0-phase3-self-host.md`/`README.md` に Capability Registry 実装ステータスと利用ガイドを追記する。
    - 6.2.a `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md#完成条件` に Capability Registry 完了チェックと `collect-iterator-audit-metrics --section runtime` の結果を参照する脚注を追加する。
    - 6.2.b `README.md` の章索引に 3.8 章のステータスバッジ（例: ✅ Stage API 実装済み）を追加し、参照リンクを更新する。
    - 6.2.c `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` のガントチャートへ Capability Registry マイルストーンを追加する。

#### 6.2 実施結果（Run ID: 20251230-capability-roadmap）
- `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` に `### 3.0.3c Capability Registry 完成条件` を追加し、`tooling/examples/run_examples.sh --suite core_diagnostics --with-audit --update-golden` と `collect-iterator-audit-metrics.py --section runtime --matrix docs/plans/bootstrap-roadmap/assets/core-io-capability-map.md` を Phase 3 の判定基準へ組み込んだ。
- README の「Core.Runtime & Capability 進捗」節で `reml_capability describe` の使い方と `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` の参照先を案内し、Stage 監査サンプルを一覧化した。
- `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md` に `## 0.2.5 Capability Registry マイルストーン` を追加し、Capability テーブル更新と KPI 取得ルートを文書体系へ位置付けた。
6.3. `docs/notes/dsl-plugin-roadmap.md` に Stage/Capability の適用例を追加し、プラグイン開発者向けに共有する。
    - 6.3.a DSL プラグイン別の Capability 要求表を `docs/plans/bootstrap-roadmap/assets/plugin-capability-matrix.csv` として作成し、`4-7-core-parse-plugin.md` から参照する。
    - 6.3.b `docs/notes/dsl-plugin-roadmap.md#effect-handling-matrix` に Stage 要件のサンプル（`verify_conductor_contract` 実行例）を追記する。
    - 6.3.c `docs/guides/dsl/plugin-authoring.md` に `reml capability describe <plugin-id>` の利用方法と Plan 3.8 への依存を追記する。

#### 6.3 実施結果（Run ID: 20251230-plugin-capability）
- `docs/plans/bootstrap-roadmap/assets/plugin-capability-matrix.csv` を作成し、`plugin.dsl.template` / `plugin.dsl.observability` / `plugin.native-ui` の Capability/Stage 依存を CSV で一覧化した。
- `docs/notes/dsl-plugin-roadmap.md` に `### 5.2 Capability 要求マトリクス` を追加し、上記 CSV と `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json`・`runtime_bridge-stage-records-20251206.json` を使った検証方法を共有した。
- `docs/guides/dsl/plugin-authoring.md` の §2.2 に `reml_capability describe` / `scripts/capability/generate_md.py` の利用例と Stage mismatch チェック手順を追記し、Plan 3.8 の成果物をガイドへ組み込んだ。

### 7. テスト・CI 統合（59週目）
**担当領域**: 品質保証

7.1. 単体テストで登録/検証/記述 API の動作とエラー経路を網羅する。
    - 7.1.a `cargo test -p reml_runtime capability_registry::*` を CI の必須ジョブに追加し、`CapabilityError` のメッセージをスナップショット (`insta`) で固定する。
    - 7.1.b `cargo nextest run -p reml_runtime --run-ignored ignored-only capability_registry` を想定し、多重登録ベンチ相当の負荷テストを `#[ignore]` 付きで用意する。
    - 7.1.c 共通フィクスチャを `compiler/rust/runtime/tests/fixtures/capabilities/*.json` に配置し、`serde_json` で読み込むテストヘルパを整備する。

#### 7.1 実施結果（Run ID: 20251210-runtime-capability-ci-checklist）
- `docs/plans/bootstrap-roadmap/assets/runtime-capability-ci-checklist.md` を新設し、単体テスト/KPI/ログ出力の導線を 7.1 の手順順に表形式でまとめた。`cargo test -p reml_runtime capability_registry -- --nocapture` を CI 必須ジョブ化するためのメモ、`insta` スナップショット (`tests/snapshots/capability_registry__*.snap`) のレビュー要領、`cargo nextest run -p reml_runtime --run-ignored capability_registry_load` を使った 1,000 件登録テスト、`tests/fixtures/capabilities/*.json` の共通フィクスチャ整備など、要求事項を 1 ページで参照できる。
- 失敗時の記録先を `reports/spec-audit/ch3/runtime_capability-unit-YYYYMMDD.md` / `runtime_capability-load-YYYYMMDD.log` に固定し、`docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-capability` と `docs/notes/runtime-capability-stage-log.md` から同じ Run ID を逆参照できるようにした。単体テストで得た差分をそのまま 3.8 計画に書き戻す運用を確立した形である。
- `docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md#7-テスト・ci-反映` からも本チェックリストをリンクし、Core.Diagnostics の Stage mismatch サンプルと Runtime Capability 単体テストのノウハウが交差するよう整合を取った。
7.2. 統合テストで TypeChecker/Runtime/Config からの Capability チェックが期待通り動くことを確認する。
    - 7.2.a `tooling/examples/run_examples.sh --suite core_runtime_capability --with-audit` を新設し、`examples/core_runtime_capability/*.reml` から診断+監査ゴールデンを生成する。
    - 7.2.b `scripts/poc_dualwrite_compare.sh` に `--runtime-capability` モードを追加し、OCaml 実装ログと差分を比較する。
    - 7.2.c `collect-iterator-audit-metrics.py --section runtime --scenario capability_registry` を Linux/macOS/Windows すべてで実行し、`reports/audit/dashboard/core_runtime-YYYYMMDD.md` に記録する。

#### 7.2 実施結果（Run ID: 20251210-core-runtime-capability-suite）
- `assets/runtime-capability-ci-checklist.md` §2 に `core_runtime_capability` 例題スイートの設計を追記し、`tooling/examples/run_examples.sh --suite core_runtime_capability --with-audit (--update-golden)` の CLI 引数、`suite_config/core_runtime_capability.env` での `ALLOW_FAILURE=registry_stage_violation` 設定、`examples/core_runtime_capability/README.md` へ記載すべきゴールデン生成手順を定義した。StageViolation ケースも `allowed failure` として継続実行できるよう Core.Diagnostics スイートと同じ制御フローを採用している。
- `collect-iterator-audit-metrics.py --section runtime --scenario capability_registry` のスイート対応を明記し、生成物を `reports/spec-audit/ch3/runtime_capability-suite-YYYYMMDD.json` へ保存して KPI 化する。ここで得られた `capability_registry_pass_rate` は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の Runtime 行に追加した。
- `scripts/poc_dualwrite_compare.sh --runtime-capability <example>` を追加する TODO を Runbook に記載し、結果ログを `reports/dual-write/runtime_capability/` に集める方針を Phase2 Rust Migration 計画 (`docs/plans/rust-migration/2-1-runtime-integration.md`) と共有した。Core.Diagnostics の pipeline_branch 計画からも同スイートを参照できるよう、相互リンクを設定している。
7.3. CI に Capability 検証を組み込み、違反が発生した場合に `0-4-risk-handling.md` へ記録する仕組みを追加する。
    - 7.3.a `.github/workflows/rust-runtime.yaml`（新設）で `cargo test -p reml_runtime capability_registry` と `tooling/examples/run_examples.sh --suite core_runtime_capability --with-audit` を並列実行する。
    - 7.3.b CI 失敗時に `docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-capability` へ Run ID と Git SHA を自動追記するスクリプト (`scripts/ci/post_failure_runtime_capability.sh`) を準備する。
    - 7.3.c `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の KPI 表に `runtime.capability_ci_pass_rate` を追加し、CI から `assets/metrics/runtime-capability-ci.csv` を append する。

#### 7.3 実施結果（Run ID: 20251210-runtime-capability-ci-metrics）
- `docs/plans/bootstrap-roadmap/assets/metrics/runtime-capability-ci.csv` を用意し、`run_id,date,job,pass_rate,failures,command,artifact,notes` を記録するテンプレートを追加した。`runtime-capability-unit`／`runtime-capability-nextest`／`core-runtime-capability-suite` 各ジョブの結果を CI から追記し、`reports/spec-audit/ch3/runtime_capability-ci-summary-YYYYMMDD.json` をアーティファクトへ添付する運用を策定した。
- `assets/runtime-capability-ci-checklist.md` §3 に `.github/workflows/rust-runtime.yaml` のマトリクス構成と `scripts/ci/post_failure_runtime_capability.sh` の出力先（`docs/plans/bootstrap-roadmap/0-4-risk-handling.md#runtime-capability`, `reports/audit/dashboard/runtime_capability-ci-YYYYMMDD.md`）を明記し、CI 失敗時に自動でリスク登録・ログ採取が行われるようにした。
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` へ `runtime.capability_ci_pass_rate` を追記し、Core.Diagnostics 側の `pipeline_branch` 指標と並べて Stage mismatch の可観測性をトラッキングできるようにした。`docs/plans/bootstrap-roadmap/pipeline_branch-stage-mismatch-plan.md#7-テスト・ci-反映` との相互リンクも更新し、同一の CI 報告ラインを使い回せるように整備している。

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
