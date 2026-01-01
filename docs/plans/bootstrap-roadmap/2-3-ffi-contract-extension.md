# 2.3 FFI 契約拡張計画

## 目的
- Phase 2 で [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) に定義された ABI/所有権契約を OCaml 実装へ反映し、x86_64 Linux (System V)、Windows x64 (MSVC)、Apple Silicon macOS (arm64-apple-darwin) の 3 ターゲットでブリッジコードを検証する。
- `AuditEnvelope` に FFI 呼び出しのメタデータを記録し、診断と監査の一貫性を確保する。

## スコープ
- **含む**: FFI 宣言構文の Parser 拡張、Typer による ABI/所有権チェック、ブリッジコード生成、ターゲット別（Linux x86_64 / Windows x64 / macOS arm64）ビルド、監査ログ拡張。
- **含まない**: 非同期ランタイム実装の刷新、プラグイン経由の FFI 自動生成。これらは Phase 3 以降。
- **前提**: Phase 1 のランタイム連携が完成し、Phase 2 の効果システム統合と衝突しない設計であること。Apple Silicon 対応については [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) および [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) に整備済みの計測・CI 手順を踏襲する。

## 作業ディレクトリ
- `compiler/ocaml/src/parser`, `compiler/ocaml/src/typer` : FFI 宣言解析と型検証
- `compiler/ocaml/src/codegen` : ブリッジコード生成、ABI 設定
- `runtime/native` : 所有権ヘルパ・FFI スタブ
- `tooling/ci`, `tooling/ci/macos`, `tooling/runtime/capabilities` : Linux/Windows/macOS 向けブリッジ検証と Capability ステージ管理
- `docs/spec/3-9-core-async-ffi-unsafe.md`, `docs/notes/llvm-spec-status-survey.md`, `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` : 契約・測定・macOS 支援資料

## 作業ブレークダウン

## 進捗サマリー（2025-10-24 時点）

- **完了**: Typer 統合、ブリッジ生成、監査スキーマとドキュメント更新、3 ターゲットでの CLI 追試とゴールデン整備。
- **引き継ぎ対象**: Windows Stage override 自動検証（技術的負債 ID 22）、macOS 固有サンプル (`ffi_dispatch_async`) の自動テスト化（ID 23）、`--verify-ir` の再有効化、GitHub Actions への `ffi_bridge.audit_pass_rate` ゲート導入。
- Phase 2-3 の主タスクは完了したため、残作業は Phase 3 へ移管する。

### 引き継ぎメモ（2025-10-24）

- 完了報告書: `docs/plans/bootstrap-roadmap/2-3-completion-report.md`
- 技術的負債: Windows Stage 自動検証（ID 22）、macOS FFI サンプル自動化（ID 23）
- 参照レポート: `reports/ffi-bridge-summary.md`, `reports/ffi-macos-summary.md`
- 次フェーズ計画: `docs/plans/bootstrap-roadmap/2-4-diagnostics-audit-pipeline.md`

> **備考**: 以下の表や「最新進捗サマリー」節は実行中の履歴記録を保持しています。最新の完了状況は上記引き継ぎメモおよび完了報告書を参照してください。

## 進捗トラッキング（2025-10 時点）

| 作業ブロック | ステータス | 完了済み項目 | 次のステップ |
| --- | --- | --- | --- |
| 前提確認・計画調整 | **進行中（成果物ゴールデン化待ち）** | `scripts/validate-runtime-capabilities.sh` を再実行し、`reports/runtime-capabilities-validation.json` を更新。macOS override 草案と `reports/ffi-macos-summary.md` のテンプレートを整備。`dune build @fmt --auto-promote` を実行してフォーマット差分を解消し、`scripts/ci-local.sh --target macos --arch arm64 --stage beta` の SEGV を `test_ffi_lowering` 修正で解消。さらに Linux/Windows/macOS で `tmp/cli-callconv-sample.reml`（＋ `cli-callconv-macos.reml`）を再実行し、`tmp/cli-callconv-out/<platform>/` へ IR/Audit を収集。2025-10-24 に stub 無終端ブロックを修正し、3 ターゲットすべてで `--verify-ir` が通過。 | 収集した監査ログのゴールデン化（Linux/Windows/macOS）と `reports/ffi-*-summary.md` の更新、macOS override の更新 PR・pass rate ログ共有を完了させる。 |
| 1. ABI モデル設計 | **進行中（差分整理中）** | Darwin 計測計画を `docs/notes/llvm-spec-status-survey.md` に追記し、`ffi_contract` モジュール（所有権・ABI 判定スケルトン）を追加。`normalize_contract` でターゲット別 `expected_abi`・所有権正規化を実装。 | Linux/Windows/macOS 向け ABI 差分ノート（`reports/ffi-bridge-summary.md` 仮）作成と、型ホワイトリスト方針の明文化。 |
| 2. Parser / AST 拡張 | **進行中（Typer フィードバック反映待ち）** | `extern_metadata` PoC を維持しつつ、`extern_block_target` への改名と `test_parser` ゴールデン更新を完了。 | Typer 連携で得たメタデータ要求をフィードバックし、属性バリデーションを Parser レイヤへ逆移譲するか検討。 |
| 3. Typer 統合と ABI 検証 | **完了** | `check_extern_bridge_contract` を `type_inference.ml` に実装し、`ffi_contract` の所有権/ABI 正規化を参照。`ffi.contract.symbol_missing` / `ownership_mismatch` / `unsupported_abi` 診断を生成し、`AuditEnvelope.metadata.bridge.*` を Typer で構築。 | ランタイム stub 連携時に追加される型ホワイトリストとの整合チェックを継続。 |
| 4. ブリッジコード生成 | **進行中（CI 連携前）** | `codegen/ffi_stub_builder.ml` を新設し、ターゲット別 `BridgeStubPlan` の正規化・監査タグ抽出を実装。`llvm_gen/ffi_value_lowering.ml` では `reml.bridge.version` フラグと `reml.bridge.stubs` メタデータを出力し、`tests/test_ffi_stub_builder.ml` / `tests/test_ffi_lowering.ml` で Linux/Windows/macOS をカバー。`compiler/ocaml/src/main.ml` から `Codegen.codegen_module` への `stub_plans` 伝播も確立済み。さらに `llvm_gen/codegen.ml` では Borrowed/Transferred を考慮した stub/thunk を生成し、`reml_ffi_bridge_record_status` を呼び出す経路を固定。LLVM CallConv (`win64`=79 / `aapcs64`=67) を反映したゴールデン (`compiler/ocaml/tests/golden/llvm/*.ll`) と CLI 追試用 `tmp/cli-callconv-sample.reml` を整備した。 Darwin Register Save Area のスタック確保を `emit_stub_function` に実装し、Darwin varargs/sret プリセット (`compiler/ocaml/tests/llvm-ir/presets/darwin-arm64/*.ll`) と `tests/test_ffi_lowering.ml` で回帰検証（2025-10-20）。 | Borrowed/Transferred 返り値の所有権を [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) §2.6 と一致させるため、(1) `llvm_gen/codegen.ml` で `Ownership::Borrowed` → `wrap_foreign_ptr`、`Ownership::Transferred` → `dec_ref` / `reml_ffi_release_transferred` を呼び分けつつ失敗パスで `reml_ffi_bridge_record_status` を記録、(2) `runtime/native/src/ffi_bridge.c` に返り値向け `reml_ffi_acquire_*` API を追加して監査メトリクスを更新、(3) `tests/test_ffi_lowering.ml`・LLVM ゴールデン・`compiler/ocaml/tests/golden/audit/ffi-bridge-*.jsonl.golden` で `bridge.return.ownership` を検証する。CLI (`src/main.exe --emit-ir`) で 3 ターゲットの IR を再取得し、`sync-iterator-audit.sh` / `collect-iterator-audit-metrics.py` を拡張して `ffi_bridge.audit_pass_rate` を集計・可視化する。 |
| 5. 監査ログ統合 | **進行中** | `tooling/runtime/audit-schema.json` に bridge オブジェクトを追加し、`tooling/ci/collect-iterator-audit-metrics.py` を拡張して `ffi_bridge.audit_pass_rate` を集計。`reports/ffi-bridge-summary.md` を更新し、メタデータ確認項目とターゲット別進捗を記録。 | Typer 実装後に `AuditEnvelope` ゴールデンを追加し、CI ゲート（`sync-iterator-audit.sh`）へ FFI ブリッジ検証を統合。Linux/Windows 監査ログのゴールデン化と pass rate 自動チェックを実装。 |
| 6. プラットフォーム別テスト | **進行中** | Apple Silicon で `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を再実行し、Lint/Build 完了後に `compiler/ocaml/tests/test_ffi_lowering` の SEGV で停止する事象を記録。`reports/ffi-macos-summary.md` を更新し、Linux/Windows 版テンプレート（`reports/ffi-linux-summary.md`, `reports/ffi-windows-summary.md`）を追加。 | `test_ffi_lowering` のクラッシュ原因を解消したうえで macOS で再実行し、Build/Test/Runtime のログを取得。FFI サンプル（借用/転送/構造体戻り）を各ターゲットで実行し、テンプレートへ結果を反映。Windows CI (`windows-latest`) への `ffi_bridge.audit_pass_rate` 収集を常設。 |
| 7. ランタイム連携とテスト | **進行中** | `runtime/native/include/reml_ffi_bridge.h` に加え `src/ffi_bridge.c` を実装し、借用/移譲ヘルパと `reml_ffi_bridge_*` 計測 API を提供。`runtime/native/tests/test_ffi_bridge.c` を追加し、`make test` で計測値・Span 変換を検証。 | ランタイム計測値を CI アウトプットへ連携し、失敗ケースが `ffi_bridge.audit_pass_rate` に反映されるよう CLI/監査パイプラインを拡充。 |
| 8. ドキュメント更新と引き継ぎ | **進行中** | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に FFI ブリッジ指標を追加し、`reports/ffi-macos-summary.md` を再構成。`reports/ffi-bridge-summary.md` を更新し、Linux/Windows/macOS 向けの記入テンプレートを整備。`docs/notes/licensing-todo.md` で外部ツール導入時の確認項目を下書き。 | 実装進捗に合わせて仕様 (`docs/spec/3-9`, `docs/guides/runtime/runtime-bridges.md`) と引継ぎ資料を更新。ライセンス調査の TODO を精査し、Phase 3 移行時のチェックリストとして確定。 |

### 最新進捗サマリー（2025-10-21）

- CLI `--emit-ir` / `--emit-audit` を Linux・Windows・macOS で再実行し、`tmp/cli-callconv-out/<platform>/` に成果物を集約。`reports/ffi-bridge-summary.md` / `reports/ffi-macos-summary.md` を更新し、ステージ情報・呼出規約・所有権メタデータを記録。`--verify-ir` 併用時の LLVM 検証失敗（stub エントリ無終端）を技術的負債として登録。

- `llvm_gen/ffi_value_lowering.ml` を追加し、`Codegen.codegen_module` が `BridgeStubPlan` から `reml.bridge.version` モジュールフラグと `reml.bridge.stubs` メタデータを生成するよう更新。`tests/test_ffi_lowering.ml` で Windows プランの smoke テストを追加。
- ランタイムに `runtime/native/src/ffi_bridge.c` と `runtime/native/tests/test_ffi_bridge.c` を追加し、借用/移譲ヘルパと `reml_ffi_bridge_*` 計測 API を検証 (`make test`)。
- `reports/ffi-bridge-summary.md` を更新して Windows メタデータの確認項目を追記し、Linux/Windows 用計測テンプレート（`reports/ffi-linux-summary.md`, `reports/ffi-windows-summary.md`）を整備。
- `dune build tests/test_ffi_lowering.exe` を実行し、LLVM メタデータ出力の回帰テストを導入。

### 完了済みタスク

- `llvm_gen/ffi_value_lowering.ml` と `compiler/ocaml/tests/test_ffi_lowering.ml` を追加し、`BridgeStubPlan` メタデータ出力の smoke テストを固定化。
- `runtime/native/src/ffi_bridge.c` / `runtime/native/tests/test_ffi_bridge.c` を実装し、借用/移譲ヘルパと `reml_ffi_bridge_*` 計測 API を `make test` で検証。
- `reports/ffi-linux-summary.md` と `reports/ffi-windows-summary.md` を追加し、ターゲット別計測テンプレートを整備。
- `compiler/ocaml/src/codegen/ffi_stub_builder.ml` と `compiler/ocaml/tests/test_ffi_stub_builder.ml` を追加し、`dune runtest` で Linux/Windows/macOS の監査タグ正規化を検証。
- `compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を更新。`scripts/ci-local.sh --target macos --arch arm64 --stage beta` は 2025-10-21 の再実行で Lint/Build/Test/Runtime/LLVM 検証の全ステップを完了。
- `tooling/runtime/audit-schema.json` に bridge オブジェクトを追加し、`tooling/ci/collect-iterator-audit-metrics.py` へ `ffi_bridge.audit_pass_rate` を実装。
- `reports/ffi-macos-summary.md` を刷新し、AddressSanitizer ログとクロスプラットフォーム比較観点を追記。
- `runtime/native/include/reml_ffi_bridge.h` を追加し、借用/移譲ヘルパおよび `reml_span_t` を定義。
- `reports/ffi-bridge-summary.md` と `docs/notes/licensing-todo.md` を雛形化し、監査ログ集約とライセンス整理の TODO を整理。
- `compiler/ocaml/src/type_inference.ml` に `check_extern_bridge_contract` を実装し、`ffi_contract` の正規化ロジックと連携した `ffi.contract.*` 診断・`AuditEnvelope.metadata.bridge.*` 出力を確立。`type_error.ml`・`main.ml` も同期し、CLI/Audit の整合を確認。
- `compiler/ocaml/tests/test_ffi_contract.ml` とゴールデン（`diagnostics/ffi/unsupported-abi.json.golden`, `audit/ffi-bridge.jsonl.golden`）を追加し、`dune runtest` で `ffi_bridge.audit_pass_rate` を検証。仕様書 `docs/spec/3-6`, `docs/spec/3-9` に診断・ABI テーブルを追記。

### 残タスクと次のステップ

1. **ブリッジコード生成パイプラインの仕上げ**
   - `Ffi_stub_builder.stub_plan` を入力に stub/thunk 関数本体を生成し、`llvm_gen/ffi_value_lowering.ml` と `llvm_gen/codegen.ml` での引数マーシャリングと `runtime/native/src/ffi_bridge.c` のヘルパ呼び出しを一体化する。既存の `codegen/ffi_stub_builder.ml`・`llvm_gen/ffi_value_lowering.ml` インターフェイス確定が前提。
      - stub 生成ロジックを専用モジュール（仮称 `llvm_gen/ffi_stub_lowering.ml`）へ切り出し、シンボル名・可視性・`dso_local` をテンプレート化する。
      - `llvm_gen/codegen.ml` 側で `Ffi_stub_builder.stub_plan` が持つ所有権フックと監査キーを、引数マーシャリングと同一の AST/IR ノードで扱えるよう拡張する。
      - `runtime/native/src/ffi_bridge.c` では `reml_ffi_bridge_record_status` など取得済み API を利用し、stub から成否が必ず報告されるように `cleanup` ブロックを共有する。
   - LLVM `call` 命令へ `callconv`・`sret`・`signext` 等の属性を付与し、ターゲット別テンプレート（System V / MSVC / AAPCS64）を `tests/test_ffi_lowering.ml` の追加ケースで検証する。
      - Linux: `callconv ccc` + 可変長引数 (`varargs`) ケースを追加し、`signext` / `zeroext` 属性が整数型サイズに応じて付与されることを Golden で固定化。
      - Windows: `callconv win64` + `sret` が必要な構造体戻り値ケースを実装し、`byval` / `inreg` など MSVC 固有属性の付与を検証。
      - macOS: `callconv aarch64_aapcscc` + `align` 属性の確認に加え、`nonnull` / `dereferenceable` を借用ポインタに付与するテストを追加。
   - `reml.bridge.stubs` Named Metadata に `stub_index`・`extern_symbol`・`bridge.platform`・`bridge.ownership` をシリアライズし、`compiler/ocaml/tests/golden/llvm/ffi-stub-*.ll` を生成して回帰検証する。
      - `Llvm_metadata.Builder` に FFI 専用ヘルパを追加し、Named Metadata 生成を `codegen/ffi_stub_builder.ml` から一元化。
      - Golden 生成用スクリプト（`scripts/gen-ffi-stub-golden.sh` 仮称）を作成し、ターゲット別に `llvm-dis` 出力を取得して `tests/golden/llvm/ffi-stub-*.ll` へ保存。
      - Metadata と `AuditEnvelope.metadata.bridge.*` のキー整合を `compiler/ocaml/tests/test_ffi_contract.ml` の追補テストで検証する。
   - 検証指標: `dune build tests/test_ffi_stub_builder.exe tests/test_ffi_lowering.exe`、`opt -verify` / `llc` が成功し、`reports/ffi-bridge-summary.md` の Linux/Windows/macOS 行が `監査タグ確認 = yes` へ更新されること。
2. **ランタイム計測と CI 連携**
   - `tooling/ci/sync-iterator-audit.sh` に FFI メトリクス取得処理を追加し、`reml_ffi_bridge_pass_rate` と `AuditEnvelope.metadata.bridge.*` の欠落を検知した場合に CI を失敗させる。Windows では PowerShell ラッパを用意し、`ffi_bridge.audit_pass_rate` の収集を共通化する。
   - `tooling/ci/collect-iterator-audit-metrics.py` を拡張して `ffi_bridge.audit_pass_rate` を JSON 集計へ組み込み、`reports/iterator-stage-summary.md` と同様のテンプレートを `reports/ffi-bridge-summary.md` §1 に反映する。
   - ランタイム単体テスト (`runtime/native/tests/test_ffi_bridge.c`) を Linux/Windows 両ターゲットで実行するワークフローを `.github/workflows/bootstrap-linux.yml` / `bootstrap-windows.yml` へ追加し、メトリクス取得コマンドと合わせてアーティファクト化する。
   - 検証指標: `make -C runtime/native test` 成功、CI ログで `ffi_bridge.audit_pass_rate = 1.0` が確認でき、`tooling/runtime/audit-schema.json` の `ffi_bridge` セクションがスキーマ検証を通過する。
3. **プラットフォーム別サンプルとゴールデン**
   - Linux/macOS/Windows 向けに借用 (`Ownership::Borrowed`)、移譲 (`Transferred`)、構造体戻り値の各サンプル (`examples/ffi/borrowed/*.reml` 等) を整備し、`reports/ffi-linux-summary.md`・`reports/ffi-macos-summary.md`・`reports/ffi-windows-summary.md` に実行ログと検証結果を記録する。
   - `compiler/ocaml/tests/golden/audit/ffi-bridge-{linux,windows,macos}.jsonl.golden` を作成し、ターゲット別 `bridge.platform`・`bridge.callconv`・`bridge.ownership` が Typer 出力と一致することを固定化する。合わせて `diagnostics/ffi/*.json.golden` に失敗ケース（所有権違反・未解決シンボル）を追加。
   - CI 用スクリプトに `reports/ffi-*-summary.md` へログを追記するステップを追加し、`reports/ffi-bridge-summary.md` §3 のチェックボックスを実測値で更新する。
   - 検証指標: `scripts/ci-local.sh --target <linux|windows|macos> --arch <x86_64|arm64> --stage beta` の成功、監査ゴールデン差分が `git status` 上でゼロ、`reports/ffi-*-summary.md` の TODO が消化済みになること。
4. **仕様・ドキュメントとライセンス整理**
   - 仕様書 `docs/spec/3-9-core-async-ffi-unsafe.md` に stub メタデータ出力と計測 API (`reml_ffi_bridge_get_metrics`, `reml_ffi_bridge_pass_rate`) の参照フローを追記し、`docs/spec/3-6-core-diagnostics-audit.md` に `ffi_bridge.*` 診断キーと `AuditEnvelope.metadata.bridge.*` の定義を統合する。
   - ガイド `docs/guides/runtime/runtime-bridges.md` を更新し、CI へのログ収集手順・`reports/ffi-*-summary.md` テンプレートの利用方法・プラットフォーム別注意点（Win32 API サンプル、Darwin codesign チェック等）を盛り込む。
- `docs/notes/licensing-todo.md` のチェックリストを精査し、生成ヘッダの SPDX 表記・コミットハッシュ埋め込み手順・外部ツール採用時のライセンス対応を Phase 2-3 中に決定。決定事項は `reports/ffi-bridge-summary.md` §5 に転記する。
- 検証指標: ドキュメント更新後に `docs/spec/README.md` と `README.md` の該当リンクを追記済みであること、レビューコメントに `reports/ffi-bridge-summary.md` と `docs/notes/licensing-todo.md` の更新差分を添付できる状態になっていること。

### Borrowed/Transferred 返り値処理の詳細計画（2025-10-20 更新）

- **コード生成**: `llvm_gen/codegen.ml` で `ffi_stub_builder` から渡される `Ownership` を参照し、Borrowed → `wrap_foreign_ptr`、Transferred → `dec_ref` / `reml_ffi_release_transferred` を呼び分ける。戻り値が構造体の場合は `Abi.classify_struct_return` の結果に従い sret/bysret を適用しつつ、`bridge.return.abi` メタデータを `reml.bridge.stubs` に追加する。`docs/spec/3-9-core-async-ffi-unsafe.md` §2.6 の契約を逸脱した場合は `reml_ffi_bridge_record_status` へ `status = "ownership_mismatch"` を記録する。
- **ランタイムと監査**: `runtime/native/src/ffi_bridge.c` に `reml_ffi_acquire_borrowed_result` / `reml_ffi_acquire_transferred_result` を追加し、RC カウンタ操作 (`inc_ref`/`dec_ref`) と `wrap_foreign_ptr` のダミー実装を橋渡しする。`AuditEnvelope.metadata.bridge.return_ownership` / `bridge.return.rc_adjustment` / `bridge.return.release_handler` を `ffi_bridge_status` へ追加し、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) §5.1 のテンプレートと突き合わせる。
- **テストとゴールデン**: `compiler/ocaml/tests/test_ffi_lowering.ml` に Borrowed/Transferred 返り値ケース（`ffi_return_borrowed.reml`, `ffi_return_transferred.reml`）を追加し、`compiler/ocaml/tests/golden/audit/ffi-bridge-{linux,windows,macos}.jsonl.golden` で `bridge.return.ownership`・`bridge.return.status` を固定化する。ランタイム側は `runtime/native/tests/test_ffi_bridge.c` で `wrap_foreign_ptr` / `dec_ref` 呼び出し回数をカウントし、`reports/ffi-bridge-summary.md` に実測値を記録する。
- **CI とメトリクス**: `tooling/ci/collect-iterator-audit-metrics.py` に `ffi_bridge.audit_pass_rate` と `bridge.return.leak_detected` を集計する項目を追加し、`.github/workflows/bootstrap-*.yml` で Linux/Windows/macOS の 3 ターゲットに対して `reports/ffi-*-summary.md` を自動更新する。CI で検出した `bridge.return` の欠落は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` へ記録し、Phase 2-3 のリスクレビューに回す。

### 2025-10-19 ログ・測定サマリー

- **`scripts/validate-runtime-capabilities.sh`**: `tooling/runtime/capabilities/default.json` を対象に再実行し、`reports/runtime-capabilities-validation.json` の timestamp を `2025-10-18T03:23:33.958135+00:00` へ更新。`arm64-apple-darwin` override が `runtime_candidates` に出力されること、および `validation.status = ok` を確認済み。
- **`dune build @fmt --auto-promote`**: `compiler/ocaml` 直下で 2025-10-19 に実行し、`src/llvm_gen/`・`tests/test_ffi_contract.ml` などのフォーマット差分を自動反映。
- **`scripts/ci-local.sh --target macos --arch arm64 --stage beta`**: 2025-10-19 の再実行では Lint/Build を完走後、テスト ステップで `compiler/ocaml/tests/test_ffi_lowering` が `Command got signal SEGV` を報告して中断。LLVM/Runtime ステップは未実行。詳細ログは `reports/ffi-macos-summary.md` §2 に追記。
- **`compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll`**: LLVM 18.1.8 で `.ll → .bc → .o` パイプラインが成功。`/opt/homebrew/Library/Homebrew/cmd/shellenv.sh` で `ps` 警告が出るが処理には影響せず、生成物パスは `reports/ffi-macos-summary.md` §2 に記録。
- **フォローアップ**: (1) `test_ffi_lowering` の SEGV 原因を調査・修正し、`ci-local` を Runtime まで再実行する。（2）`effects-residual.jsonl.golden` を含む監査ゴールデンの Stage Trace 更新済み状態を維持しつつ、`ffi_bridge.audit_pass_rate` の導入後に再検証する。

### Typer `extern_metadata` 設計メモ（ドラフト）

- Parser で抽出済みの `extern_metadata` を Typer へ伝搬し、以下のキーを `AuditEnvelope.metadata.bridge.*` に写像する：`bridge.target`（例: `arm64-apple-darwin`）、`bridge.arch`（`arm64` / `x86_64`）、`bridge.abi`（`system_v` / `msvc` / `darwin_aapcs64`）、`bridge.ownership`（`borrowed` / `transferred` / `reference`）、`bridge.extern_symbol`（リンク先シンボル名）、必要に応じて `bridge.alias`・`bridge.library` を追加。`extern_decl` 側の集約フィールドは `extern_block_target` へ改名済みで、Typer ではブロック既定値とアイテム個別値の整合を照合する。
- Typer では `extern_metadata` の欠落・矛盾を `ffi.contract.ownership_mismatch` / `ffi.contract.unsupported_abi` / `ffi.contract.symbol_missing` 診断として報告し、CLI JSON と監査ログで同一内容を表示する。`RuntimeCapabilityResolver` が提供する `stage_trace` に FFI 情報を追記し、効果診断と整合したメタデータを構築する。
- Runtime 側が追記する `bridge.callsite`（モジュール/関数）と整合させるため、Typer で `bridge.symbol_path` を計算したうえで `AuditEnvelope` へ渡す。

#### Issue 下書き案（Typer: extern_metadata パイプライン）

1. **AST → Typer のデータ受け渡し**  
   `typed_ast.ml` に extern 解析結果を格納するレコードを追加し、`extern_metadata` を必須フィールドとして保持。`bridge.target` 未指定時は Capability JSON のデフォルトターゲットを補完する。
2. **所有権と ABI の検証ロジック実装（完了済み 2025-10-18）**  
   `type_inference.ml` に `check_extern_bridge_contract` を実装し、許可されていない所有権/ABI 組合せを検出。失敗時は `ffi.contract.ownership_mismatch` / `ffi.contract.unsupported_abi` / `ffi.contract.symbol_missing` 診断を発火し、違反箇所の `bridge.extern_symbol`・`bridge.target` を添付する。
3. **`AuditEnvelope` 拡張（完了済み 2025-10-18）**  
   `type_error.ml`・`main.ml` を更新し、Typer で構築した `bridge` メタデータを CLI (`--emit-json`)・監査 (`--emit-audit`) 双方へ転送。`AuditEnvelope.metadata.bridge.*` に `status`・`source_span` を含める。
4. **ゴールデンテスト更新（完了済み 2025-10-18）**  
   `compiler/ocaml/tests/test_ffi_contract.ml` を追加し、`compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden` と `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` を固定化。`dune runtest` で `ffi_bridge.audit_pass_rate` を自動検証。

### ブリッジコード生成設計メモ（ドラフト）

- `Ffi_stub_builder.stub_plan` は `ffi_contract` 正規化結果（ターゲットトリプル・Calling Convention・Ownership・監査キー）から生成し、`compiler/ocaml/src/codegen/ffi_stub_builder.ml` でテンプレート化した後に LLVM lowering 側（`llvm_gen/ffi_value_lowering.ml` と今後追加する `llvm_gen/ffi_stub_lowering.ml` 仮）へ供給する。`reports/ffi-bridge-summary.md` にサンプルプランを追記してレビュー材料とする。
- ランタイム側では `runtime/native/include/reml_ffi_bridge.h` を新設し、`reml_ffi_acquire_borrowed(void*)` / `reml_ffi_acquire_transferred(void*)` / `reml_ffi_release_transferred(void*)` / `reml_ffi_box_string(const reml_string_t*)` / `reml_ffi_unbox_span(const reml_span_t*)`（`reml_span_t` も新設予定）などのヘルパ API を提供する。実装は `src/ffi_bridge.c` にまとめ、既存の `mem_alloc`・`inc_ref`・`dec_ref` と連携させる。
- 生成する LLVM IR には `llvm::Metadata` で `bridge.platform` / `bridge.abi` / `bridge.ownership` / `bridge.stub_id` を埋め込み、`AuditEnvelope` が利用するキーと揃える。IR 生成時に `!llvm.module.flags` へ `reml.bridge.version = 1` を追加し、将来の互換性チェックを容易にする。
- C ヘッダの生成は Phase 2 ではリポジトリ内スクリプト（`scripts/gen-ffi-headers.reml`）で行い、生成物に `SPDX-License-Identifier`・コミットハッシュ・生成日時を付記する。外部ツール（`cbindgen` 等）を導入する場合はライセンス互換性と再現性を `docs/notes/licensing-todo.md`（新規予定）へ記録し、Phase 3 での自動化移行を前提にレビューする。

### JSON 監査スキーマ更新案（FFI Bridge 拡張）

- `AuditEnvelope` スキーマに `bridge` オブジェクトを追加し、必須プロパティとして `bridge.status` / `bridge.target` / `bridge.arch` / `bridge.platform` / `bridge.abi` / `bridge.ownership` / `bridge.extern_symbol` を定義。`bridge.return` は `ownership`・`status`・`wrap`・`release_handler`・`rc_adjustment` を持つネストオブジェクトとして記録し、`bridge.alias`, `bridge.library`, `bridge.callconv`, `bridge.audit_stage` は任意項目とする。
- スキーマ改訂は `tooling/runtime/audit-schema.json` v1.1 で管理し、`bridge.return.*` と `bridge.platform` を追加済み。`tooling/ci/collect-iterator-audit-metrics.py` が新フィールドを必須キーとして検証する。
- ゴールデンテスト: `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` および `compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden` を更新し、CLI の `--emit-audit` 実行結果を dune テストで固定化。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `ffi.bridge.audit_pass_rate` 指標を追記する。
- レビュー体制: 一次レビューは Diagnostics チーム、二次レビューは FFI チーム、最終承認は `tooling/ci` チーム（CI ゲート整合確認）。週次スタンドアップで進捗共有し、採択前に `reports/ffi-macos-summary.md` のサンプルを提示する。
- スクリプト更新: `tooling/ci/collect-iterator-audit-metrics.py` に FFI ブリッジ指標 `ffi_bridge.audit_pass_rate` を追加済み。出力 JSON は後方互換性のため iterator 指標をトップレベルに残したまま、`metrics[]` 配列と `ffi_bridge` サマリーを併記する。

### Capability override 提案（arm64-apple-darwin）

- ステージ案: `beta`（Phase 2-3 で FFI 契約と診断が安定するまで安定版から分離）
- 追加 Capability 候補: `ffi.bridge`, `process.spawn`（Windows x64 と同一セットで開始し、macOS 固有 Capability は Phase 2-3 後半で再評価）
- 検証手順案:
  - `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を実行し、`runtime_candidates` に `arm64-apple-darwin` を追加。
  - Apple Silicon ランナーで `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を実行し、`iterator.stage.audit_pass_rate` が 1.0 であることを確認。
  - 監査ログ: `reports/ffi-macos-summary.md` に呼出規約検証結果と Capability stage 差分を記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記載するメトリクス更新案と合わせてレビュー依頼を出す。
- レビューコメント草案（ドラフト）: 「`tooling/runtime/capabilities/default.json` に `arm64-apple-darwin` override を追加し、ステージは `beta` で開始。Capability は既存 Windows beta と同じ `ffi.bridge` / `process.spawn` を割り当て、Phase 2-3 期間中に macOS 固有 Capability を精査する。追加後に `scripts/validate-runtime-capabilities.sh` と `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を実行してレポートを共有する。」

## 直近アクション（次の 2 週間）

- **実装仕上げ**: `llvm_gen/codegen.ml` と `llvm_gen/ffi_value_lowering.ml` に Borrowed/Transferred 返り値処理を実装し、`runtime/native/src/ffi_bridge.c` のヘルパと連動させる。ローカル環境で `_build/default/src/main.exe --emit-ir --out-dir <out>` を実行し、`tmp/cli-callconv-sample.reml` を用いた Linux/Windows/macOS の IR・監査ログを取得して `reports/ffi-bridge-summary.md`・`reports/ffi-macos-summary.md` に反映する。
- **監査・CI 統合**: `tooling/ci/sync-iterator-audit.sh` / `collect-iterator-audit-metrics.py` に `ffi_bridge.audit_pass_rate` と Darwin プリセット成功条件を追加し、`--emit-audit` ゴールデン（`compiler/ocaml/tests/golden/audit/ffi-bridge-*.jsonl.golden`）を更新する。`AuditEnvelope` スキーマへ `bridge.*` フィールドを正式追加し、CI の JSON 検証を必須化する。
- **ドキュメント整合と引き継ぎ**: 仕様書（`docs/spec/3-9`, `docs/spec/3-6`）とガイド（`docs/guides/runtime/runtime-bridges.md`）へ stub メタデータと監査指標の整合結果を追記し、Phase 3 に引き継ぐ TODO リストと Plan 2-3 完了報告の下書きを本計画書に追加してレビューする。

### 1. ABI モデル設計と仕様整理（29-30週目）
**担当領域**: FFI 基盤設計

1.1. **ABI 仕様の抽出**
- [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) の ABI テーブルを OCaml データ型に写像
- System V ABI (x86_64 Linux)、MSVC ABI (x86_64 Windows)、AAPCS64/Darwin ABI (arm64 macOS) の差分整理
- 呼出規約（calling convention）の形式化
- 構造体レイアウト・アライメントルールの定義（Darwin 固有のレイアウト差分は [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) と突合）

1.2. **所有権契約の設計**
- `docs/notes/llvm-spec-status-survey.md` §2.4 の RC 契約を OCaml データ構造化（`Ownership::Transferred`/`Borrowed` 等）
- FFI 境界での所有権移転ルール
- メモリ安全性の検証ポリシー
- `effect {ffi}`/`effect {unsafe}` 境界との連携

1.3. **ターゲット設定システム**
- ターゲット別の ABI 設定テーブル（`x86_64-unknown-linux-gnu` / `x86_64-pc-windows-msvc` / `arm64-apple-darwin`）
- コンパイル時のターゲット切替ロジック
- `--target` フラグと `--arch` の整合処理（`scripts/ci-local.sh` の引数設計を参照）
- Phase 2 型クラス・効果との統合方針

**成果物**: ABI データモデル、所有権設計、ターゲット設定

### 2. Parser/AST 拡張（30週目）
**担当領域**: FFI 構文解析

2.1. **FFI 宣言構文の実装**
- `extern "C"` ヘッダおよび複数宣言ブロックの構文（1-1 §B.4）
- ライブラリ指定や名前マングリングのオプションを既存仕様の拡張ポイント（コメント/属性）として扱い、新構文を導入しない
- 所有権契約はシグネチャ付随メタデータとして格納（構文レイヤでは属性追加を行わない）

2.2. **AST ノード拡張**
- `Decl::Extern` ノードの追加
- ターゲットトリプル/呼出規約/所有権メタデータを保持するフィールド
- Span 情報の保持
- デバッグ用の AST pretty printer 更新

2.3. **パーサテスト整備**
- FFI 宣言の正常系テスト
- ABI/所有権注釈のエラーケース
- ゴールデンテスト（AST 出力）
- Phase 1 パーサとの統合検証

**成果物**: 拡張 Parser、FFI AST、パーサテスト

### 3. Typer 統合と ABI 検証（30-31週目）
**担当領域**: 型検証と整合性チェック

3.1. **FFI 型の検証**
- FFI 境界で許可される型のホワイトリスト
- ポインタ型・参照型の検証
- 構造体レイアウトの互換性チェック
- 型サイズ・アライメントの計算

3.2. **所有権注釈の検証**
- 所有権の整合性チェック
- Unsafe ブロックの必要性判定
- 所有権違反の検出とエラー報告
- 借用規則の FFI への適用

3.3. **ABI 整合性チェック**
- ターゲット別の ABI ルール適用
- 呼出規約の検証
- 名前マングリングの生成
- Phase 2 効果システムとの連携（`effect {ffi}`, `effect {unsafe}` による契約確認）

**成果物**: FFI 型検証、所有権チェック、ABI 検証

### 4. ブリッジコード生成（31-32週目）
**担当領域**: コード生成

4.1. **Stub 生成ロジック**
- `Ffi_stub_builder.stub_plan` に型シグネチャ（引数型・戻り値型）とサニタイズ済み `stub_symbol` / `thunk_symbol` を保持し、Typer が収集した `ffi_bridge_snapshots` を CodeGen まで伝搬。
- `llvm_gen/codegen.ml` で ABI 判定 (`Abi.classify_struct_return` / `Abi.classify_struct_argument`) に基づき stub/thunk 関数を定義し、Borrowed 所有権の引数には `inc_ref` を自動挿入。呼び出し後は `reml_ffi_bridge_record_status` で成功メトリクスを記録する。
- `llvm_gen/ffi_value_lowering.ml` で `reml.bridge.stubs` Named Metadata に `bridge.stub_symbol` / `bridge.thunk_symbol` / `bridge.arch` を追加し、監査ログと IR の突合せを容易化（Windows ケースはテストゴールデンで固定化済み）。
- 呼出規約は暫定的に `Llvm.CallConv.c` へフォールバックしており、`win64` / `aarch64_aapcscc` 専用の CallConv は LLVM OCaml バインディング調査後に追加予定。

4.2. **LLVM IR への lowering**
- `Abi` 判定に基づき sret/byval 属性を stub/thunk/external 宣言へ適用し、構造体戻り値・構造体引数でも ABI が崩れないことを確認。
- 借用引数への `inc_ref` 挿入は完了。Transferred 所有権向けの `dec_ref`・返り値のラップ処理、`nonnull` / `dereferenceable` 属性、`llvm.lifetime.*` 命令は未実装。
- `!dbg` 情報の転写は未着手。監査ログ `call_site` と自動突合できるよう、Phase 2-3 中に DWARF/Debug metadata の設計を固める。

4.3. **C ヘッダ生成の検討**
- `runtime/native/include/reml_ffi_bridge.h` を共通入口とし、ターゲット固有の補助ヘッダは `include/generated/<triple>/` へ自動出力する案を比較する。
- OCaml 製のヘッダ生成スクリプト（仮称 `scripts/gen-ffi-headers.reml`）と `cbindgen` 等外部ツール利用案の再現性・レビュー容易性・依存ライセンスを整理する。
- 生成物へ `SPDX-License-Identifier`・生成日時・ソースコミットハッシュを埋め込み、`reports/ffi-bridge-summary.md` で差分監査する手順を確立する。
- Phase 2 では手動管理 fallback のレビューチェックリストを `docs/notes/` に記録し、Phase 3 の自動化移行に備える。

**成果物**: Stub 生成、LLVM lowering、ヘッダ生成調査

### 5. 監査ログ統合（32週目）
**担当領域**: 診断と監査

5.1. **FFI メタデータの記録**
- `AuditEnvelope.metadata` に `bridge.stage.*`・`bridge.platform`・`bridge.abi` を追加
- ABI 種別・所有権注釈の記録
- FFI 呼び出しのトレース情報（ターゲットトリプルとアーキテクチャを含める）
- Phase 2 診断タスクとの連携

5.2. **診断メッセージの実装**
- FFI 型エラーの詳細メッセージ
- 所有権違反の説明と修正提案
- ABI ミスマッチの検出とレポート
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) との整合

5.3. **監査ログの出力**
- `--emit-audit` での FFI 情報出力
- JSON スキーマの定義
- CI でのスキーマ検証
- `0-3-audit-and-metrics.md` への記録

**成果物**: FFI 監査ログ、診断メッセージ、スキーマ

### 6. プラットフォーム別テスト（32-33週目）
**担当領域**: クロスプラットフォーム検証

6.1. **Linux x86_64 テスト**
- System V ABI のサンプル FFI 呼び出し
- libc 関数の呼び出しテスト（`printf`, `malloc` 等）
- 構造体渡し・戻りのテスト
- 所有権注釈の検証テスト
- `tmp/cli-callconv-sample.reml` を `--emit-ir` で再実行し、生成 IR・監査ログをレポートへ反映

6.2. **Windows x64 テスト**
- MSVC ABI のサンプル FFI 呼び出し
- Windows API の呼び出しテスト（`MessageBoxW` 等）
- ABI 差分の動作検証
- Phase 2 Windows タスクとの連携
- CLI 追試で `--emit-ir --target x86_64-windows` を確認し、`bridge.callconv=79` の監査ログを取得

6.3. **macOS arm64 テスト**
- Apple Silicon (arm64-apple-darwin) 上での FFI 呼び出し検証（`libSystem` / `dispatch` API など）
- Mach-O 向けスタブ生成と `codesign --verify` の簡易チェック
- Darwin ABI 固有のシグネチャ（構造体戻り値、可変長引数）の検証
- Register Save Area (RSA) をスタブ側で確保し、macOS varargs/sret プリセットで検証（2025-10-20）
- Phase 1-8 macOS 計測値と比較し、差分を `reports/ffi-macos-summary.md`（新規）へ記録
- CLI 追試で `--emit-ir --target arm64-apple-darwin` を実行し、RSA 測定結果と `bridge.callconv=67` を監査ログへ記録

6.4. **CI/CD 統合**
- GitHub Actions に FFI テストジョブ追加
- Linux/Windows/macOS の 3 ターゲットでのテスト実行
- macOS ジョブで `compiler/ocaml/scripts/verify_llvm_ir.sh --preset darwin-arm64 --target arm64-apple-darwin` を実行し、varargs/sret リグレッションを検知（2025-10-20 追加）
- `ffi_bridge.audit_pass_rate` を CI 成功条件に組み込み、Darwin プリセットの成功を pass/fail 判定に含める。
- テストカバレッジの計測（>75%）
- ビルド時間の監視

**成果物**: プラットフォーム別テスト、CI 設定

### 7. ランタイム連携とテスト（33週目）
**担当領域**: ランタイム統合

7.1. **ランタイム C コードの拡張**
- `runtime/native/include/reml_ffi_bridge.h` と `src/ffi_bridge.c` を追加し、`reml_ffi_acquire_borrowed` / `reml_ffi_acquire_transferred` / `reml_ffi_release_transferred` / `reml_ffi_box_string` / `reml_ffi_unbox_span`（`reml_span_t` も併せて新設予定）などのヘルパを定義する。
- RC と連携するマーシャリング API（`reml_ffi_wrap_result`, `reml_ffi_unwrap_error` 等）を整備し、`mem_alloc`・`inc_ref`・`dec_ref` を内部で利用する。
- ランタイム内で `ffi_bridge.audit_pass_rate` を更新するフックを設置し、失敗時には `panic` ではなく `ffi_bridge_status` を返してコンパイラ側診断へエスカレートする。
- 既存 Phase 1 ランタイムとの ABI を維持するため、ヘッダは `#ifdef REML_RUNTIME_ENABLE_FFI` でガードし、既存ビルドに影響を与えないようにする。

7.2. **統合テスト**
- Reml → FFI → C → Reml のラウンドトリップ
- 複雑な構造体の受け渡し
- コールバック関数の検証
- メモリリークの検出（valgrind）

7.3. **性能計測**
- FFI 呼び出しのオーバーヘッド測定
- マーシャリングコストの評価
- `0-3-audit-and-metrics.md` への記録
- 最適化機会の特定

**成果物**: ランタイム拡張、統合テスト、性能計測

### 8. ドキュメント更新と引き継ぎ（33-34週目）
**担当領域**: 仕様整合と引き継ぎ

8.1. **仕様書フィードバック**
- [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) への実装差分の反映
- ABI 差分の詳細化
- 所有権契約の擬似コードを追加
- 新規サンプルコードの追加

8.2. **ガイド更新**
- `docs/guides/runtime/runtime-bridges.md` の FFI セクション更新
- プラットフォーム別の注意事項を追記
- cbindgen 等のツール使用例
- トラブルシューティング情報

8.3. **Phase 3 準備**
- FFI のセルフホスト移植計画
- 残存課題の `docs/notes/` への記録
- 非同期 FFI の将来設計検討
- メトリクスの CI レポート化
- Phase 2-3 完了報告と Phase 3 TODO リストを整理し、後続チームが参照できる計画補遺（仮: `docs/plans/bootstrap-roadmap/2-3-completion-report.md`）を準備する。

**成果物**: 更新仕様書、ガイド、引き継ぎ文書

## 成果物と検証
- 3 ターゲットすべてで FFI サンプルが成功し、所有権違反時に診断が出力される。
- `AuditEnvelope` に FFI 呼び出しのトレースが追加され、`0-3-audit-and-metrics.md` で確認できる。
- 仕様ドキュメントの更新がレビュー済みで、記録が残る。

## リスクとフォローアップ
- Windows (MSVC) / macOS (Darwin) の呼出規約差異によりバグが潜む恐れがあるため、`2-6-windows-support.md` と [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) と連携してテストケースを共有。
- 所有権注釈の表現力が不足している場合、Phase 3 で DSL 拡張を検討する。
- FFI ブリッジ生成に外部ツールを使う場合はライセンス・再現性を `0-3-audit-and-metrics.md` に記録。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
