# 2.1 標準ライブラリ移行に伴う Rust 実装追従計画（Core.System 対応）

`docs/plans/docs-examples-audit/2-0-stdlib-plugin-migration-plan.md` により `Core.System` 仕様が確定したため、Rust 実装が新仕様に追従するための実装計画を整理する。

## 背景
- `docs/spec/3-18-core-system.md` に `Core.System`（Process/Signal/Env/Daemon）が追加され、`Core.Env` は互換エイリアスとして扱う方針になった。
- `docs/spec/3-8-core-runtime-capability.md` では `Core.Runtime.Signal` / `SignalInfo` の再エクスポートと `SignalDetail` の橋渡しが明記された。
- 公式プラグイン（Process/Signal）は低レベル Capability に残るため、Rust 実装では標準 API と Capability の境界を明確化する必要がある。

## 調査結果（Rust 実装の現状）
1. Capability のハンドル型は存在するが、Registry で `core.process` / `core.signal` / `core.system` が既定登録されていない。
   - `compiler/rust/runtime/src/capability/process.rs`
   - `compiler/rust/runtime/src/capability/signal.rs`
   - `compiler/rust/runtime/src/capability/system.rs`
   - `compiler/rust/runtime/src/capability/registry.rs`
2. `Core.System` 相当の標準 API 実装が存在せず、`runtime` 側の公開モジュールにも `system` がない。
   - `compiler/rust/runtime/src/lib.rs`
3. `Core.Runtime.Signal` / `SignalInfo` / `SignalDetail` に対応する型が Rust 実装に未定義。
   - `compiler/rust/runtime/src/runtime/`
4. `SignalDetail.raw_code` の監査マスク方針や `process.*`/`signal.*` の監査メタデータ生成が未実装。
   - `compiler/rust/runtime/src/audit/mod.rs`
5. 環境変数のスナップショット取得は `Core.IO` 内部の補助 (`TimeEnvSnapshot`) に留まり、`Core.System.Env` と `Core.Env` の標準 API が存在しない。
   - `compiler/rust/runtime/src/io/env.rs`

## 目的
- `Core.System` の標準 API を Rust 実装に追加し、仕様と実装の差分を解消する。
- Capability レイヤと標準 API の責務境界を明確化し、監査ログ・診断が仕様通りに動作する状態へ整備する。
- `Core.Env` 互換エイリアスを保持しつつ `Core.System.Env` を正準 API として提供する。

## 対象範囲
- Rust Runtime (`compiler/rust/runtime`) における標準ライブラリ実装と Capability 登録。
- `Core.Runtime` / `Core.System` に関する型と監査出力の追加。

## 対象外
- OCaml 実装の更新。
- 公式プラグインの実装拡張（低レベル Capability は保持するが追加実装は別計画）。

## 実装影響（ギャップ整理）
- **Capability 登録ギャップ**: `core.process` / `core.signal` / `core.system` が `CapabilityRegistry` の既定登録に含まれず、標準 API 側からの Stage 検証が成立しない。
- **標準 API ギャップ**: `Core.System` のモジュール・型・関数が Rust Runtime に存在しない。
- **Signal 型ギャップ**: `Core.Runtime.Signal` / `SignalInfo` / `Core.System.SignalDetail` の型が未整備で、`from_runtime_info` の変換規約を実装できない。
- **監査ギャップ**: `process.spawn` / `process.wait` / `process.kill` などの監査メタデータ、`signal.raw_code` のマスク規約が未実装。
- **Env 互換ギャップ**: `Core.Env` エイリアスの公開経路が未定義。

## 実装計画

### フェーズA: Capability Registry の拡充
1. `compiler/rust/runtime/src/capability/registry.rs` に `core.process` / `core.signal` / `core.system` を既定登録する。
2. `ProcessCapabilityMetadata` / `SignalCapabilityMetadata` / `SystemCapabilityMetadata` の既定値と effect スコープを仕様（`docs/spec/3-8-core-runtime-capability.md`）に合わせて調整する。
3. `CapabilityDescriptor` の Stage/効果タグが `Core.System` API の effect 要件を満たすよう整理する。

### フェーズB: Core.Runtime Signal 型の追加
1. `compiler/rust/runtime/src/runtime/` に `Signal` / `SignalInfo` / `SignalError` 相当の型定義を追加し、`Core.Runtime` API として公開する。
2. `SignalInfo` の最小フィールド（`signal`, `sender`）を規定し、Capability 側のインターフェースに渡せる構造を作る。
3. `Core.System.Signal` が `Core.Runtime.Signal` のエイリアスになるよう、公開 API の再エクスポート方法を整理する。

### フェーズC: Core.System モジュール実装
1. `compiler/rust/runtime/src/system/` を新設し、`process.rs` / `signal.rs` / `env.rs` / `daemon.rs` を配置する。
2. `Core.System.Process` の型 (`Command`, `SpawnOptions`, `ProcessHandle` など) と API（`spawn`/`wait`/`kill`）を実装し、Capability 未登録時は `Unsupported` を返す。
3. `Core.System.Signal` の型 (`SignalPayload`, `SignalDetail`, `SignalError`) と API（`send`/`wait`/`raise`）を実装し、`from_runtime_info` で `SignalDetail` を構成する。
4. `Core.System.Env` の環境変数 API を実装し、`Core.Env` からの互換エイリアスを提供する。
5. `Core.System.Daemon` は Phase 4 の最小 API としてスタブを提供し、Phase 5 拡張に備えた TODO を記載する。

### フェーズD: 監査・診断の整備
1. `process.*` / `signal.*` の監査イベント（`AuditEnvelope.metadata`）を生成するヘルパを追加する。
2. `SignalDetail.raw_code` のマスク方針を `Core.Diagnostics` 監査ポリシーに従って実装し、`signal.raw_code = "allow"` 時のみ数値を出力する。
3. Capability 未登録時の診断コード（`system.capability.missing`）の発生経路を追加し、エラー情報が CLI へ届くことを確認する。

### フェーズE: テストと回帰接続
1. `Core.System` API のユニットテスト（Capability 未登録時の `Unsupported`、Signal 変換、Env 互換）を追加する。
2. `examples/docs-examples/spec/` の該当サンプルが実行できる最小挙動を確認し、必要なら `reports/spec-audit/` に実行ログを追加する。
3. `docs-migrations.log` に実装追従の記録を追記する。

## 成果物
- `Core.System` 標準 API の Rust 実装と公開モジュール追加。
- `Core.Runtime.Signal` / `SignalInfo` / `Core.System.SignalDetail` の型整備。
- `core.process` / `core.signal` / `core.system` の Capability 登録と監査ログ整備。
- 追従状況の記録（`docs-migrations.log` / `reports/spec-audit/`）。

## リスクと対応
- **OS 依存差分**: Signal/Process 操作は OS により差分があるため、`Unsupported` の返却と監査ログで明示する。
- **監査ポリシー逸脱**: `raw_code` のマスク実装が不足すると監査規約違反になるため、診断ポリシーの参照点を明確にする。
- **Capability 未登録**: 標準 API が Capability 検証で失敗する可能性があるため、Registry の既定登録を最優先で実施する。

## 進捗チェック
- [x] フェーズA: Capability Registry の拡充
- [ ] フェーズB: Core.Runtime Signal 型の追加
- [ ] フェーズC: Core.System モジュール実装
- [ ] フェーズD: 監査・診断の整備
- [ ] フェーズE: テストと回帰接続

## 参照
- `docs/spec/3-18-core-system.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/spec/3-10-core-env.md`
- `docs/spec/4-0-official-plugins-overview.md`
- `docs/plans/docs-examples-audit/2-0-stdlib-plugin-migration-plan.md`
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md`
