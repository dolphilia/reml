# Phase4: FFI 強化実装計画

## 背景と決定事項
- `docs/plans/ffi-improvement/` で Phase 1〜4 の仕様・ガイド・サンプル方針が確定し、WS1〜WS4 は `confirmed` となった。
- 仕様側では `docs/spec/3-9-core-async-ffi-unsafe.md` と `docs/spec/3-6-core-diagnostics-audit.md` に FFI/監査の要件が追記済みであり、Phase 4 では Rust 実装と運用パイプラインへの統合が主対象となる。
- 既存の Phase 4 計画は「実用シナリオの回帰接続」に重点があるため、FFI 強化は専用の実装計画として扱い、`phase4-scenario-matrix.csv` と KPI に接続する。

## 目的
1. `reml-bindgen` / `Core.Ffi.Dsl` / `reml build` 連携を Rust 実装に落とし込み、仕様で確定した診断・監査キーと一致させる。
2. FFI 実用シナリオを Phase 4 のマトリクスへ登録し、`examples/ffi` と `expected/` の差分を回帰に接続する。
3. WASM Component Model/WIT の調査ログを整備し、将来実装へ引き継げる PoC 手順を確定する。

## スコープ
- **含む**: `reml-bindgen` 実装（CLI/設定/ログ/manifest）、`Core.Ffi.Dsl` ランタイム API、`reml build` への FFI 統合、FFI 系の監査ログ整合、Phase 4 シナリオ登録、WIT 調査ログ・PoC 手順。
- **含まない**: ABI の網羅実装、WIT 連携の本実装、`extern "C"` 互換仕様の破壊的変更。

## 成果物
- `reml-bindgen` が `reml-bindgen.toml` と CLI を解釈し、`.reml` と `bindings.manifest.json` を生成できる。
- `Core.Ffi.Dsl` の `bind_library` / `bind_fn` / `wrap` が `compiler/runtime` に実装され、`examples/ffi/dsl` が動作する。
- `reml.json` の FFI セクションと `reml build` のフローが実装され、`ffi.build.*` / `ffi.bindgen.*` の監査ログが出力される。
- `phase4-scenario-matrix.csv` に FFI シナリオが追加され、`expected/` と整合する。
- WIT 対応表と PoC 手順が `docs/notes/` / `docs/guides/` に整理される。

## 作業ステップ

### フェーズA: reml-bindgen 実装
1. `reml-bindgen` 本体を新設（`compiler/ffi_bindgen/` 新規 crate + bin `reml-bindgen`）し、`reml-bindgen.toml` と CLI オプションの優先順位を実装する。（完了）
   - ディレクトリ構成案:
     - `compiler/ffi_bindgen/Cargo.toml`（crate 名: `reml_ffi_bindgen`）
     - `compiler/ffi_bindgen/src/lib.rs`（設定読み込み・型変換・出力生成）
     - `compiler/ffi_bindgen/src/main.rs`（bin 名: `reml-bindgen`）
     - `compiler/Cargo.toml` に workspace 追加
2. `bindings.manifest.json` に `qualifiers` / 入力ハッシュ / 診断メタデータを記録し、`docs/spec/3-6-core-diagnostics-audit.md` と整合する形式を固定する。（完了）
3. `ffi.bindgen.*` 診断キーを出力し、`docs/guides/ffi/reml-bindgen-guide.md` のログ例と一致させる。（完了）
4. `examples/ffi/bindgen/minimal` を CLI 実行で再生成し、`expected/` へ出力ゴールデンを追加する。（完了）

### フェーズB: Core.Ffi.Dsl ランタイム実装
方針:
- `Core.Ffi.Dsl` の実装は `compiler/runtime/src` 直下（`ffi` モジュール配下）に配置し、Core の公開 API として整理する。
- `ffi.wrap` の監査キーは `AuditEnvelope` に統合し、`docs/spec/3-6-core-diagnostics-audit.md` の `ffi.wrapper` / `ffi.wrap.*` を Core 監査ログとして扱う。

1. `compiler/runtime/ffi` に `dsl` モジュールを追加し、`ffi.bind_library` / `ffi.bind_fn` / `ffi.wrap` の API を実装する。（完了）
   - `compiler/runtime/src/ffi/dsl/mod.rs` を追加し、`bind_library` / `bind_fn` / `wrap` の公開 API を定義する。
   - `compiler/runtime/src/ffi/mod.rs` に `dsl` の `pub mod` を追加し、`Core.Ffi.Dsl` の公開経路を確立する。
   - `bind_library` の最小実装（ライブラリ解決、`FfiLibraryHandle` 生成、失敗時の診断変換）を追加する。
   - `bind_fn` の最小実装（シンボル解決、`FfiFnSig` 検証、失敗時の診断変換）を追加する。
   - `wrap` の最小実装（引数数/型検証、戻り値の `null` 判定、`Result` 返却）を追加する。
2. `ffi.wrap` の監査メタデータ（`ffi.wrap.*`）を `AuditEnvelope` に記録し、`docs/spec/3-6-core-diagnostics-audit.md` と一致させる。（完了）
   - `AuditEnvelope.metadata["ffi.wrapper"]` を埋める処理を追加する（`name` / `null_check` / `ownership` / `error_map` / `call_mode`）。
   - `ffi.wrap.invalid_argument` / `ffi.wrap.null_return` / `ffi.wrap.ownership_violation` の診断拡張を実装する。
   - `ffi.call` 監査テンプレートへ `wrapper = "ffi.wrap"` を付与する経路を追加する。
3. `examples/ffi/dsl` を CLI 実行可能にし、`unsafe` 直呼びと `ffi.wrap` の差分を `expected/` に固定する。（完了）
   - `examples/ffi/dsl/unsafe_direct.reml` と `examples/ffi/dsl/wrapped_safe.reml` のランタイム呼び出し経路を整理する。（完了）
   - `expected/ffi/dsl/` に `unsafe` 直呼びと `ffi.wrap` の出力差分を固定する。（完了）
   - 実行ログの診断キーが `ffi.wrap.*` / `ffi.call` に揃っているかを確認する。（完了）
   - 実行ログ生成手順:
     - `reml_frontend` / `remlc` 起動時に `set_ffi_call_executor` が初期化されるため、CLI 実行に合わせて `ffi.call` の監査メタデータを生成する。（完了）
     - `expected/ffi/dsl/unsafe_direct.audit.json` と `expected/ffi/dsl/wrapped_safe.audit.json` に `ffi.call` / `ffi.wrapper` を反映し、`docs/spec/3-6-core-diagnostics-audit.md` の必須キーに合わせて更新する。（完了）

### フェーズC: reml build 統合
1. `reml.json` の `ffi` セクション（`libraries`/`headers`/`bindgen`/`linker`）を `tooling/cli` でパースし、検証エラーを `ffi.build.*` で出力する。（完了: `remlc build` で最小検証を追加）
2. `reml build` に `reml-bindgen` 呼び出しとキャッシュ層を追加し、入力ハッシュと生成物の一致を監査ログへ記録する。（進行中: `--emit-bindgen` / `--cache-dir` で起動・キャッシュ格納・復元・`ffi.bindgen` 監査ログを追加）
   - 検討: `cache_hit` 復元失敗時に `AuditEnvelope.metadata` へ `error.code` / `error.message` を追記するか整理する（要検討）。
3. Linux/macOS/Windows の差分を `docs/spec/3-10-core-env.md` と照合し、`docs/guides/ffi/ffi-build-integration-guide.md` に合わせた出力を維持する。

補足: bindgen 呼び出し/キャッシュ層のフック地点（案）
- 入口: `remlc build` サブコマンドに `--emit-bindgen` / `--cache-dir` を追加し、`reml.json` の `ffi.bindgen` を解釈して `reml-bindgen` を起動する。
- キャッシュ鍵: `ffi.headers` / `ffi.bindgen.config` / `TargetProfile` / `reml-bindgen` バージョンを正規化した `input_hash` を算出し、`expected/` と同じ構造で `cache_dir/ffi/{input_hash}` に生成物を保存する。
- 監査ログ: `ffi.bindgen` 実行ごとに `ffi.bindgen.status` と `input_hash` を `AuditEnvelope.metadata["ffi.bindgen"]` に記録し、`cache_hit` の場合は `headers` を省略可とする。
- 呼び出し制御: `ffi.bindgen.enabled = true` の場合のみ実行し、`output` が未指定なら `ffi.build.config_invalid` を返す。

### フェーズD: WIT 調査ログと PoC
1. `docs/notes/ffi/ffi-wasm-component-model-log.md` を更新し、WIT 型→Reml 型対応表の一次案を追加する。（完了）
2. Canonical ABI のメモリ境界（Shared Nothing）と FFI との差分を調査ログに追記する。（完了）
3. `docs/guides/ffi/ffi-wit-poc.md` に PoC 手順（WIT 生成→バインディング生成→呼び出し検証）を明記する。（完了）

### フェーズE: Phase 4 回帰接続
1. `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に FFI シナリオを追加（例: `FFI-BINDGEN-001` / `FFI-DSL-001` / `FFI-BUILD-001` / `FFI-WIT-001`）。（完了）
   - `FFI-BINDGEN-001` の expected パス:
     - `expected/ffi/bindgen/minimal/counter_bindings.reml`
     - `expected/ffi/bindgen/minimal/bindings.manifest.json`
2. `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` に FFI 回帰の参照先と実行コマンドを追記する。（完了）
3. `reports/spec-audit/ch5/spec-core-dashboard.md` に FFI 実行ログを登録し、Phase 5 へ引き継げるよう KPI を整理する。（完了）

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 72 週 | フェーズA: reml-bindgen 実装 |
| 73 週 | フェーズB: Core.Ffi.Dsl 実装 |
| 74 週 | フェーズC: reml build 統合 |
| 75 週 | フェーズD: WIT 調査ログ/PoC |
| 76 週 | フェーズE: Phase 4 回帰接続 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| `reml-bindgen` の生成コードが実装仕様と乖離 | FFI サンプルが回帰対象として機能しない | `examples/ffi/bindgen/minimal` を CLI で再生成し、`expected/` と差分監査を必須化 |
| 監査ログのキーがズレる | Phase 4 KPI が監査対象にならない | `ffi.bindgen.*` / `ffi.build.*` / `ffi.wrap.*` を仕様から直接参照し、ログ整形に回帰テストを追加 |
| ビルド環境差分が膨らむ | Windows/macOS で再現性が落ちる | `docs/spec/3-10-core-env.md` で差分表を維持し、`ffi-build-integration-guide.md` の手順を更新 |

## 進捗状況
- 2025-12-19: フェーズA 完了（`reml-bindgen` 実装/manifest 更新/`expected/` 追加/仕様・ガイド反映）。
- 2025-12-19: フェーズB 完了（`Core.Ffi.Dsl` ランタイム API/監査メタデータ/FFI 実行エンジン接続/`expected/ffi/dsl` 反映）。
- 2025-12-19: フェーズC 進行中（`remlc build` で `reml.json` 検証/`reml-bindgen` 起動/キャッシュ格納・復元/`ffi.bindgen` 監査ログ出力を追加）。
- 2025-12-19: フェーズD 完了（WIT 調査ログの対応表更新/Shared Nothing 差分整理/PoC 手順の明文化）。
- 2025-12-19: フェーズE 完了（FFI シナリオ追加/回帰計画反映/ダッシュボード追記）。

## 参照
- `docs/plans/ffi-improvement/0-0-overview.md`
- `docs/plans/ffi-improvement/0-1-workstream-tracking.md`
- `docs/plans/ffi-improvement/1-0-bindgen-plan.md`
- `docs/plans/ffi-improvement/1-1-ffi-dsl-plan.md`
- `docs/plans/ffi-improvement/1-2-build-integration-plan.md`
- `docs/plans/ffi-improvement/1-3-wasm-component-model-plan.md`
- `docs/spec/3-9-core-async-ffi-unsafe.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-10-core-env.md`
- `docs/guides/ffi/reml-bindgen-guide.md`
- `docs/guides/ffi/ffi-dsl-guide.md`
- `docs/guides/ffi/ffi-build-integration-guide.md`
- `docs/guides/ffi/ffi-wit-poc.md`
