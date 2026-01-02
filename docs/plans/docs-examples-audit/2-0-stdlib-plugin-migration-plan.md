# 2.0 標準ライブラリ移行に伴う仕様改訂計画（公式プラグイン整理）

`docs/spec/5-0-official-plugins-overview.md` の移行方針に基づき、公式プラグインから標準ライブラリへ移行する領域を整理し、仕様書の改訂タスクを定義する計画書。

## 背景
- 公式プラグイン章は標準ライブラリ拡張の再整理に伴いドラフト再検討中（`docs/spec/5-0-official-plugins-overview.md`）。
- `docs/notes/stdlib/stdlib-expansion-research.md` と `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md` が `Core.System` への昇格を明示。
- 仕様と実装の同期は `docs/plans/rust-migration/4-2-documentation-sync.md` のフローに従う。

## 目的
- 公式プラグインと標準ライブラリの責務境界を明確化し、移行対象の仕様を Chapter 3 に再配置する。
- 仕様書内の参照関係（Chapter 3/4/5/README）を更新し、誤った導線を解消する。
- ドキュメント内の Reml コード例・監査ログが移行後の API を参照するように更新する。

## 調査結果（要約）
- 移行対象: `5-2 Process`、`5-4 Signal`、`Core.Env`（3-10）を `Core.System` に統合。
- 残留対象: `5-1 System`（低レベル syscall）、`5-3 Memory`、`5-5 Hardware`、`5-6 Realtime` は公式プラグインとして維持。
- 依存更新: `5-3/5-5/5-6` は `core.process` 依存を `Core.System.Process` 参照へ置換が必要。
- ランタイム能力定義: `3-8 Core Runtime & Capability Registry` は低レベル Capability 定義として維持し、標準ライブラリ API とのブリッジ説明を追加する必要がある。

## 決定事項（採用）
- **章番号**: `Core.System` の新設章は `docs/spec/3-18-core-system.md` を正式採用する。
- **互換方針**: `Core.Env` は `Core.System.Env` の互換エイリアスとして維持し、`Core.Env` の単独仕様は `3-10` に残しつつ「移行先: `Core.System.Env`」を明記する。段階移行期間は Phase 4 〜 Phase 5 を想定し、以降は `Core.System.Env` を正準とする。
- **Signal 型整合**: `Core.System.Signal` は `Core.Runtime.Signal` の型エイリアス（`Int`）として定義し、標準ライブラリ側は `Signal`/`SignalInfo` の再エクスポートで統一する。追加情報が必要な場合は `Core.System.SignalDetail` 等の拡張型で提供し、`Core.Runtime` の型は変更しない。
- **SignalDetail 方針**: `Core.System.SignalDetail` を採用する。`SignalInfo`（再エクスポート）に不足する情報を補完するため、`timestamp: Option<Timestamp>`, `payload: Option<SignalPayload>`, `source_pid: Option<ProcessId>`, `raw_code: Option<Int>` を持つ拡張型として定義し、`SignalInfo` からの昇格関数（`from_runtime_info`）を標準ライブラリ側で提供する。
- **Timestamp 参照元**: `SignalDetail.timestamp` は `Core.Numeric & Time` の `Timestamp` を正準として参照し、`Core.System` では必要に応じて `use Core.Numeric.Time.Timestamp` の再エクスポートで提供する（別名型は作らない）。
- **再エクスポート表記**: `Core.System` では `Timestamp` を再エクスポートせず、仕様本文中は常に `Core.Numeric.Time.Timestamp` を明示参照する。
- **`raw_code` 表記ガイド**: `SignalDetail.raw_code` は OS 依存のシグナル番号を格納し、`Signal` の列挙値と一致しない場合がある。仕様本文では「`raw_code` は `Signal` の変換前コードであり、Windows では `CTRL_C_EVENT` 等の値を返す場合がある」と明記し、`raw_code = None` は「OS が未提供・取得不能・安全ポリシーで隠蔽」のいずれかを意味する、と記述する。
- **`raw_code` 監査マスク方針**: 監査ログでは `raw_code` を既定で `masked` とし、`Core.Diagnostics` の監査ポリシーに `signal.raw_code = "allow"` が明示された場合のみ数値を出力する。`masked` でも `Signal` の高レベル種別は必ず記録する。
- **`from_runtime_info` 変換方針**: `fn from_runtime_info(info: Core.Runtime.SignalInfo) -> Core.System.SignalDetail` を提供し、欠落したメタデータは `None` で補完する。`raw_code` が取得不能/秘匿の場合は `None` とする（失敗は返さない）。
- **`SignalPayload` 出典**: `Core.System.SignalPayload` は `Core.Runtime` の低レベル型に依存せず、標準ライブラリ側で定義する。`Core.Runtime.SignalInfo` とのブリッジで該当情報がある場合のみ `SignalDetail.payload` に反映する。

## 変更対象（ファイル）
### 追加候補
- `docs/spec/3-18-core-system.md`: `Core.System` 配下の `Process`/`Signal`/`Env`/`Daemon`/安全ラッパ方針を整理。

### 更新対象（必須）
- `docs/spec/3-0-core-library-overview.md`: `Core.System` 追加と章ガイド更新。
- `docs/spec/3-8-core-runtime-capability.md`: `ProcessCapability`/`SignalCapability` と標準 API の橋渡し指針を追記。
- `docs/spec/3-10-core-env.md`: `Core.System.Env` への統合方針と互換エイリアスの記述更新。
- `docs/spec/5-0-official-plugins-overview.md`: 移行済み/移行対象の明確化、章構成の調整。
- `docs/spec/5-2-process-plugin.md`: 「標準ライブラリへ移行済み/移行中」へ明記し、低レベル Capability の残留範囲を明文化。
- `docs/spec/5-4-signal-plugin.md`: 同上。安全 API の標準ライブラリ側移行を明示。
- `docs/spec/5-1-system-plugin.md`: `Core.System` との橋渡し方針を更新。
- `docs/spec/5-3-memory-plugin.md`: 依存先を `Core.System.Process` に更新。
- `docs/spec/5-5-hardware-plugin.md`: 依存先を `Core.System.Process` に更新。
- `docs/spec/5-6-realtime-plugin.md`: 依存先を `Core.System.Process` に更新。
- `docs/spec/README.md`: Chapter 4 の一覧・説明を更新。
- `docs/spec/4-4-community-content.md`: 参照章の更新（Process/Signal の移行先を反映）。
- `docs/spec/4-5-roadmap-metrics.md`: 参照章の更新。
- `docs/spec/4-6-risk-governance.md`: 参照章の更新。
- `docs/guides/runtime/portability.md`: `Core.Process` 記述を `Core.System.Process` へ修正。

### 影響調査対象（必要に応じて）
- `docs/notes/stdlib/stdlib-expansion-research.md`: 「移行済み」反映の注記追加。
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md`: Phase 4 の優先順位表に「移行済み」注記を追加。

## 作業ステップ
### フェーズA: 章構成の確定
1. `Core.System` の章名・構成（Process/Signal/Env/Daemon の範囲）を確定する。
2. `Core.Env` の扱い（`Core.System.Env` への統合 or 互換エイリアス）を決定する。
3. 標準 API と Capability API の境界（安全ラッパ/低レベル）を明文化する。

### フェーズB: Chapter 3 追加・更新
1. `docs/spec/3-18-core-system.md`（仮）を新設し、API と効果タグ、診断/監査連携を定義する。
2. `docs/spec/3-0-core-library-overview.md` に `Core.System` を追記する。
3. `docs/spec/3-10-core-env.md` を `Core.System.Env` の説明に更新する。
4. `docs/spec/3-8-core-runtime-capability.md` に標準 API のブリッジ説明を追記する。

### フェーズC: Chapter 5 の整理
1. `docs/spec/5-0-official-plugins-overview.md` を更新し、移行済み領域を明示する。
2. `docs/spec/5-2-process-plugin.md` / `docs/spec/5-4-signal-plugin.md` を「低レベル Capability のみ」へ位置付け直す。
3. `docs/spec/5-1-system-plugin.md` と `5-3/5-5/5-6` の相互参照を更新する。

### フェーズD: 参照整合と README 更新
1. `docs/spec/README.md` と Chapter 4 の参照章を更新する。
2. `docs/guides/runtime/portability.md` の `Core.Process` 表記を `Core.System.Process` に改める。
3. 参照リンク切れ・命名揺れを `docs/plans/rust-migration/4-2-documentation-sync.md` のチェックリストに従って確認する。

### フェーズE: サンプル/監査ログ更新（docs-examples-audit）
1. `docs/spec` 内の `Core.Process`/`Signal` を参照する Reml コード例を棚卸しする。
2. `examples/docs-examples/spec/` の `.reml` を更新し、在庫表と対応付けを調整する。
3. `reports/spec-audit/` へ再検証ログを追加し、`docs-migrations.log` に変更履歴を記録する。

## 成果物
- 追加: `docs/spec/3-18-core-system.md`（仮）
- 改訂: Chapter 3/4/5 と README、ガイドの参照更新
- 監査: `docs-migrations.log` と `reports/spec-audit/summary.md` の更新

## チェックリスト（進捗）
- [x] `Core.System` 章番号を `3-18-core-system.md` に確定
- [x] `Core.Env` を `Core.System.Env` の互換エイリアスとする方針を確定
- [x] `Core.System.Signal` を `Core.Runtime.Signal` の型エイリアスとする方針を確定
- [x] `SignalDetail` の採用とフィールド設計を確定
- [x] `SignalDetail.timestamp` の参照元を `Core.Numeric.Time.Timestamp` とする方針を確定
- [x] `SignalDetail.raw_code` の表記ガイドを確定
- [x] `raw_code` の監査マスク方針を確定
- [x] `from_runtime_info` の変換方針を確定
- [x] `SignalPayload` の出典を `Core.System` 側とする方針を確定
- [x] `docs/spec/3-18-core-system.md` に Signal/SignalDetail 方針を反映
- [x] `docs/spec/3-8-core-runtime-capability.md` に SignalDetail との橋渡し記述を追記
- [x] `docs/spec/3-0-core-library-overview.md` に `Core.System` 章を追記
- [x] `docs/spec/3-10-core-env.md` の `Core.System.Env` 統合方針を反映
- [x] `docs/spec/5-0-official-plugins-overview.md` の移行済み領域を明記
- [x] `docs/spec/5-2-process-plugin.md` / `docs/spec/5-4-signal-plugin.md` の位置付け更新
- [x] `docs/spec/5-1-system-plugin.md` / `5-3/5-5/5-6` の参照更新
- [x] `docs/spec/README.md` / `docs/spec/4-4-community-content.md` / `docs/spec/4-5-roadmap-metrics.md` / `docs/spec/4-6-risk-governance.md` の参照更新
- [x] `docs/guides/runtime/portability.md` の `Core.Process` 表記を `Core.System.Process` へ更新
- [x] `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の移行対象タグ付け
- [ ] `examples/docs-examples/spec/` の `.reml` 更新と監査ログ再採取

## リスクと対応
- **仕様重複**: `Core.Env` と `Core.System.Env` が二重記載になるリスク。統合方針を早期確定し、旧名は互換エイリアスとして明記。
- **参照破綻**: Chapter 5 依存リンクの不整合。更新対象リストに基づく一括確認を必須化。
- **サンプル不整合**: 旧 API のサンプルが残留するリスク。`docs-examples-audit` の棚卸しで優先修正。

## TODO
- `docs/spec/3-18-core-system.md` で `Signal`/`SignalInfo`/`SignalDetail` の相互参照表記を確定する。
- `docs/spec/3-8-core-runtime-capability.md` に `SignalDetail` との関係（低レベル Capability と標準 API の橋渡し）を追記する。
- `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の `Signal`/`Process` 参照サンプルを移行対象としてタグ付けする。
