# 2.2 標準ライブラリ移行に伴うサンプル/テスト追従計画（Core.System 対応）

`docs/plans/docs-examples-audit/2-0-stdlib-plugin-migration-plan.md` と `2-1-stdlib-plugin-migration-impl-plan.md` により `Core.System` 仕様と Rust 実装が更新されたため、ドキュメント内サンプル・`.reml`・Rust テストの追従計画を整理する。

## 背景
- `docs/spec/3-18-core-system.md` が追加され、`Core.System.Process/Signal/Env/Daemon` を正準 API として定義した。
- `docs/spec/3-10-core-env.md` は `Core.System.Env` を正準にし、`Core.Env` は互換エイリアスとして残す方針。
- 公式プラグイン（Process/Signal）は低レベル Capability として残留し、標準 API は Chapter 3 へ移行済み。
- `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` には `Process/Signal` 移行対象のタグが付与済みだが、新章 3-18 のサンプル抽出は未実施。

## 目的
- `Core.System` への移行に合わせてサンプルと `.reml` を更新し、正準 API 参照へ統一する。
- `docs/plans/docs-examples-audit/` の棚卸し表と監査ログを更新し、差分履歴を残す。
- Rust テストの参照先・命名が新仕様と一致していることを確認する。

## 対象範囲
- ドキュメント: `docs/spec/3-18-core-system.md`, `docs/spec/3-10-core-env.md`, `docs/spec/4-2-process-plugin.md`, `docs/spec/4-4-signal-plugin.md`
- 在庫表: `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md`
- サンプル: `examples/docs-examples/spec/`, `examples/practical/`, `examples/language-impl-comparison/`
- Rust テスト: `compiler/rust/runtime/tests/core_system_api.rs`
- 監査ログ: `reports/spec-audit/`, `docs-migrations.log`

## 対象外
- OCaml 実装の更新。
- 公式プラグインの実装拡張（Capability の機能追加は別計画）。

## 調査結果（現状）
### 1. `.reml` とサンプルの影響
- `examples/practical/core_env/envcfg/env_merge_by_profile.reml` は `Core.System.Env` へ更新済み。
- `examples/language-impl-comparison/reml/config_manifest_lifecycle.reml` は `Core.System.Env` へ更新済み。
- `docs/spec/3-18-core-system.md` の `reml` コードブロックは棚卸し表に登録し、`examples/docs-examples/spec/3-18-core-system/` を作成済み。
- `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` には `docs/spec/4-2-process-plugin.md` / `4-4-signal-plugin.md` の `migration:core.system.*` タグが付与済み。

### 2. Rust テストの影響
- `compiler/rust/runtime/tests/core_system_api.rs` は `Core.System.Env` と `Core.Env` の互換性確認を含む。
- `core.process` / `core.signal` の Capability ID は仕様上維持対象のため、現状テストは継続可能。

## 実施計画

### フェーズA: サンプル棚卸しと抽出
1. `docs/spec/3-18-core-system.md` の `reml` ブロックを抽出し、`examples/docs-examples/spec/3-18-core-system/` に配置する。
2. 在庫表へ 3-18 の各節（Process/Signal/Daemon）を登録し、`migration:core.system.process` / `migration:core.system.signal` / `migration:core.system.daemon` を付与する。
3. `docs/spec/3-10-core-env.md` の既存 `.reml` に `migration:core.system.env` を付与し、正準 API 追従の対象として明示する。

### フェーズB: 既存 `.reml` の更新
1. `examples/practical/core_env/envcfg/env_merge_by_profile.reml` を `Core.System.Env` 基準へ更新する（互換 API を使う場合は注記を添える）。
2. `examples/language-impl-comparison/reml/config_manifest_lifecycle.reml` の `use Core.Env` を `Core.System.Env` へ更新し、文中の説明も正準 API に合わせる。
3. `examples/docs-examples/spec/4-2-process-plugin/*.reml` / `4-4-signal-plugin/*.reml` を確認し、型参照が `Core.System.Signal` / `Core.System.Process` と整合するよう更新する（必要に応じて `use` を追加）。

### フェーズC: Rust テストの確認
1. `compiler/rust/runtime/tests/core_system_api.rs` の命名・コメントが `Core.System` 正準に沿っているか確認し、必要ならリネームまたは注記を追加する。
2. `core.process` / `core.signal` / `core.system` の Capability ID が仕様通りに維持されているかを再確認する。

### フェーズD: 監査と記録
1. 更新した `.reml` の検証を `reml_frontend` で実行し、`reports/spec-audit/` にログを追加する。
2. `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の状態・備考を更新する。
3. `docs-migrations.log` に移行履歴を記録する。

## 進捗状況
### フェーズA: サンプル棚卸しと抽出
- [x] `docs/spec/3-18-core-system.md` の `reml` ブロックを抽出し、`examples/docs-examples/spec/3-18-core-system/` に配置する。
- [x] 在庫表へ 3-18 の各節（Process/Signal/Daemon）を登録し、`migration:core.system.process` / `migration:core.system.signal` / `migration:core.system.daemon` を付与する。
- [x] `docs/spec/3-10-core-env.md` の既存 `.reml` に `migration:core.system.env` を付与し、正準 API 追従の対象として明示する。

### フェーズB: 既存 `.reml` の更新
- [x] `examples/practical/core_env/envcfg/env_merge_by_profile.reml` を `Core.System.Env` 基準へ更新する（互換 API を使う場合は注記を添える）。
- [x] `examples/language-impl-comparison/reml/config_manifest_lifecycle.reml` の `use Core.Env` を `Core.System.Env` へ更新し、文中の説明も正準 API に合わせる。
- [x] `examples/docs-examples/spec/4-2-process-plugin/*.reml` / `4-4-signal-plugin/*.reml` を確認し、型参照が `Core.System.Signal` / `Core.System.Process` と整合するよう更新する（必要に応じて `use` を追加）。

### フェーズC: Rust テストの確認
- [x] `compiler/rust/runtime/tests/core_system_api.rs` の命名・コメントが `Core.System` 正準に沿っているか確認し、必要ならリネームまたは注記を追加する。
- [x] `core.process` / `core.signal` / `core.system` の Capability ID が仕様通りに維持されているかを再確認する。

### フェーズD: 監査と記録
- [x] 更新した `.reml` の検証を `reml_frontend` で実行し、`reports/spec-audit/` にログを追加する。
- [x] `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の状態・備考を更新する。
- [x] `docs-migrations.log` に移行履歴を記録する。
  - 2026-01-01 初回実行時は `reml_frontend` のリンクエラーで失敗。`reports/spec-audit/summary.md` に記録済み。
  - `RUSTFLAGS="-C link-arg=-fuse-ld=lld"` で再実行し diagnostics を採取済み。

## 成果物
- `examples/docs-examples/spec/3-18-core-system/` の追加と在庫表登録。
- `Core.System` 正準 API に合わせた `.reml` 更新。
- 監査ログと `docs-migrations.log` の更新。

## リスクと対応
- **互換エイリアスの混在**: `Core.Env` と `Core.System.Env` の記述が混在し、読者が正準 API を誤認する可能性がある。正準表記を優先し、互換用途のみ注記で残す。
- **棚卸し漏れ**: 3-18 の新規章が在庫表に未登録のまま残るリスク。フェーズA での登録を必須化する。
- **監査ログ不足**: 変更後の実行ログが不足すると追跡性が下がるため、フェーズDで必ず更新する。

## TODO
- `Core.System.Env` の記述を残すサンプルと、互換性検証のサンプルの役割分担を確定する。
- `docs/spec/3-18-core-system.md` の `Daemon` サンプルに対する実行検証方法を決める。
