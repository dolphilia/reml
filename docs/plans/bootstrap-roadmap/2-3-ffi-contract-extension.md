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

## 進捗トラッキング（2025-10 時点）

| 作業ブロック | ステータス | 完了済み項目 | 次のステップ |
| --- | --- | --- | --- |
| 前提確認・計画調整 | **進行中** | `scripts/validate-runtime-capabilities.sh` を再実行し、`reports/runtime-capabilities-validation.json` を更新。macOS override 草案と `reports/ffi-macos-summary.md` のテンプレート整備、CI ローカルフロー（Lint/Build/Test/LLVM/Runtime 完走）を確認済み。 | override 変更の PR 化とレビュー共有。Linux/Windows 向け計測テンプレート整備。 |
| 1. ABI モデル設計 | **進行中** | Darwin 計測計画を `docs/notes/llvm-spec-status-survey.md` に追記し、`ffi_contract` モジュール（所有権・ABI 判定スケルトン）を追加。`normalize_contract` でターゲット別 `expected_abi`・所有権正規化を実装。 | Linux/Windows/macOS 向け ABI 差分ノート（`reports/ffi-bridge-summary.md` 仮）作成と、型ホワイトリスト方針の明文化。 |
| 2. Parser / AST 拡張 | **進行中** | `extern_metadata` PoC を維持しつつ、`extern_block_target` への改名と `test_parser` ゴールデン更新を完了。 | Typer 連携で得たメタデータ要求をフィードバックし、属性バリデーションを Parser レイヤへ逆移譲するか検討。 |
| 3. Typer 統合と ABI 検証 | **完了** | `check_extern_bridge_contract` を `type_inference.ml` に実装し、`ffi_contract` の所有権/ABI 正規化を参照。`ffi.contract.symbol_missing` / `ownership_mismatch` / `unsupported_abi` 診断を生成し、`AuditEnvelope.metadata.bridge.*` を Typer で構築。 | ランタイム stub 連携時に追加される型ホワイトリストとの整合チェックを継続。 |
| 4. ブリッジコード生成 | **進行中** | `codegen/ffi_stub_builder.ml` を新設し、ターゲット別 `BridgeStubPlan` の正規化・監査タグ抽出を実装。`llvm_gen/ffi_value_lowering.ml` で `reml.bridge.version` モジュールフラグと `reml.bridge.stubs` メタデータを生成し、`tests/test_ffi_stub_builder.ml` / `tests/test_ffi_lowering.ml` で Linux/Windows/macOS の初期ケースをカバー。 | Stub/Thunk の LLVM lowering 実装と、出力メタデータを CI で検証するパイプライン（`sync-iterator-audit.sh` / `collect-iterator-audit-metrics.py` 連携）を整備。 |
| 5. 監査ログ統合 | **進行中** | `tooling/runtime/audit-schema.json` に bridge オブジェクトを追加し、`tooling/ci/collect-iterator-audit-metrics.py` を拡張して `ffi_bridge.audit_pass_rate` を集計。`reports/ffi-bridge-summary.md` を更新し、メタデータ確認項目とターゲット別進捗を記録。 | Typer 実装後に `AuditEnvelope` ゴールデンを追加し、CI ゲート（`sync-iterator-audit.sh`）へ FFI ブリッジ検証を統合。Linux/Windows 監査ログのゴールデン化と pass rate 自動チェックを実装。 |
| 6. プラットフォーム別テスト | **進行中** | Apple Silicon で `scripts/ci-local.sh --target macos --arch arm64 --stage beta` をフル実行し、`reports/ffi-macos-summary.md` にログと比較観点を追記。Linux/Windows 版テンプレート（`reports/ffi-linux-summary.md`, `reports/ffi-windows-summary.md`）を追加。 | FFI サンプル（借用/転送/構造体戻り）を各ターゲットで実行し、テンプレートへ結果を反映。Windows CI (`windows-latest`) への `ffi_bridge.audit_pass_rate` 収集を常設。 |
| 7. ランタイム連携とテスト | **進行中** | `runtime/native/include/reml_ffi_bridge.h` に加え `src/ffi_bridge.c` を実装し、借用/移譲ヘルパと `reml_ffi_bridge_*` 計測 API を提供。`runtime/native/tests/test_ffi_bridge.c` を追加し、`make test` で計測値・Span 変換を検証。 | ランタイム計測値を CI アウトプットへ連携し、失敗ケースが `ffi_bridge.audit_pass_rate` に反映されるよう CLI/監査パイプラインを拡充。 |
| 8. ドキュメント更新と引き継ぎ | **進行中** | `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に FFI ブリッジ指標を追加し、`reports/ffi-macos-summary.md` を再構成。`reports/ffi-bridge-summary.md` を更新し、Linux/Windows/macOS 向けの記入テンプレートを整備。`docs/notes/licensing-todo.md` で外部ツール導入時の確認項目を下書き。 | 実装進捗に合わせて仕様 (`docs/spec/3-9`, `docs/guides/runtime-bridges.md`) と引継ぎ資料を更新。ライセンス調査の TODO を精査し、Phase 3 移行時のチェックリストとして確定。 |

### 最新進捗サマリー（2025-10-19）

- `llvm_gen/ffi_value_lowering.ml` を追加し、`Codegen.codegen_module` が `BridgeStubPlan` から `reml.bridge.version` モジュールフラグと `reml.bridge.stubs` メタデータを生成するよう更新。`tests/test_ffi_lowering.ml` で Windows プランの smoke テストを追加。
- ランタイムに `runtime/native/src/ffi_bridge.c` と `runtime/native/tests/test_ffi_bridge.c` を追加し、借用/移譲ヘルパと `reml_ffi_bridge_*` 計測 API を検証 (`make test`)。
- `reports/ffi-bridge-summary.md` を更新して Windows メタデータの確認項目を追記し、Linux/Windows 用計測テンプレート（`reports/ffi-linux-summary.md`, `reports/ffi-windows-summary.md`）を整備。
- `dune build tests/test_ffi_lowering.exe` を実行し、LLVM メタデータ出力の回帰テストを導入。

### 完了済みタスク

- `llvm_gen/ffi_value_lowering.ml` と `compiler/ocaml/tests/test_ffi_lowering.ml` を追加し、`BridgeStubPlan` メタデータ出力の smoke テストを固定化。
- `runtime/native/src/ffi_bridge.c` / `runtime/native/tests/test_ffi_bridge.c` を実装し、借用/移譲ヘルパと `reml_ffi_bridge_*` 計測 API を `make test` で検証。
- `reports/ffi-linux-summary.md` と `reports/ffi-windows-summary.md` を追加し、ターゲット別計測テンプレートを整備。
- `compiler/ocaml/src/codegen/ffi_stub_builder.ml` と `compiler/ocaml/tests/test_ffi_stub_builder.ml` を追加し、`dune runtest` で Linux/Windows/macOS の監査タグ正規化を検証。
- `compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を更新し、`scripts/ci-local.sh --target macos --arch arm64 --stage beta` をフルパスで完走。
- `tooling/runtime/audit-schema.json` に bridge オブジェクトを追加し、`tooling/ci/collect-iterator-audit-metrics.py` へ `ffi_bridge.audit_pass_rate` を実装。
- `reports/ffi-macos-summary.md` を刷新し、AddressSanitizer ログとクロスプラットフォーム比較観点を追記。
- `runtime/native/include/reml_ffi_bridge.h` を追加し、借用/移譲ヘルパおよび `reml_span_t` を定義。
- `reports/ffi-bridge-summary.md` と `docs/notes/licensing-todo.md` を雛形化し、監査ログ集約とライセンス整理の TODO を整理。
- `compiler/ocaml/src/type_inference.ml` に `check_extern_bridge_contract` を実装し、`ffi_contract` の正規化ロジックと連携した `ffi.contract.*` 診断・`AuditEnvelope.metadata.bridge.*` 出力を確立。`type_error.ml`・`main.ml` も同期し、CLI/Audit の整合を確認。
- `compiler/ocaml/tests/test_ffi_contract.ml` とゴールデン（`diagnostics/ffi/unsupported-abi.json.golden`, `audit/ffi-bridge.jsonl.golden`）を追加し、`dune runtest` で `ffi_bridge.audit_pass_rate` を検証。仕様書 `docs/spec/3-6`, `docs/spec/3-9` に診断・ABI テーブルを追記。

### 残タスクと次のステップ

1. **ブリッジコード生成パイプラインの仕上げ**
   - `BridgeStubPlan` から実際の stub/thunk 関数を生成し、マーシャリング処理 (`FfiValueLowering`) と runtime API (`reml_ffi_bridge_*`) を接続する。
   - LLVM IR に埋め込んだメタデータを活用し、`call` 命令への属性付与や `!dbg` 連携を実装する。
2. **ランタイム計測と CI 連携**
   - ランタイム計測値 (`reml_ffi_bridge_get_metrics`, `reml_ffi_bridge_pass_rate`) を CLI/CI から取得できるよう `tooling/ci/sync-iterator-audit.sh` を拡張し、`ffi_bridge.audit_pass_rate` をゲート条件に追加する。
   - Windows / Linux ランナーでの `make test`（runtime）実行と計測ログ収集を自動化する。
3. **プラットフォーム別サンプルとゴールデン**
   - Linux/macOS/Windows 各ターゲットで FFI サンプル（借用・転送・構造体戻り値）を実行し、`reports/ffi-*-summary.md` に結果とログを反映する。
   - 監査ログのゴールデン (`compiler/ocaml/tests/golden/audit/ffi-bridge-*.jsonl.golden`) を追加し、ターゲット別の `bridge.*` キーを固定化する。
4. **仕様・ドキュメントとライセンス整理**
   - 仕様書（3-9/3-6）とガイド（runtime-bridges.md）に stub メタデータ出力・計測 API の利用方法を追記し、Phase 3 に向けた手順を明文化する。
   - `docs/notes/licensing-todo.md` の TODO を精査し、生成ヘッダの SPDX/生成情報の取り扱い方針を決定する。

### 2025-10-18 ログ・測定サマリー

- **`scripts/validate-runtime-capabilities.sh`**: `tooling/runtime/capabilities/default.json` を対象に再実行し、`reports/runtime-capabilities-validation.json` の timestamp を `2025-10-18T03:23:33.958135+00:00` へ更新。`arm64-apple-darwin` override が `runtime_candidates` に出力されること、および `validation.status = ok` を確認済み。
- **`scripts/ci-local.sh --target macos --arch arm64 --stage beta`**: Lint → Build → Test → LLVM IR → Runtime（AddressSanitizer 含む）まで完走。生成された LLVM IR は `/tmp/reml-ci-local-llvm-ir-5983`、詳細ログは `reports/ffi-macos-summary.md` §2 に追記。
- **`compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll`**: 追加で単体検証を実行し、LLVM 18.1.8 で `.ll → .bc → .o` パイプラインの成功を確認。生成物パスは `reports/ffi-macos-summary.md` §2 に追記。
- **フォローアップ**: (1) `effects-residual.jsonl.golden` を含む監査ゴールデンを Stage Trace 仕様に合わせて更新。（2）`ci-local` のテストステップ常時実行に備えて、`dune fmt` 差分の解消タスクを Phase 2-3 backlog に登録。

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

- `BridgeStubPlan` は `ffi_contract` 正規化結果（ターゲットトリプル・Calling Convention・Ownership・監査キー）から生成し、`compiler/ocaml/src/codegen/ffi_stub_builder.ml` で `emit_stub_module` / `emit_thunk` の両方に供給する。`reports/ffi-bridge-summary.md` にサンプルプランを追記してレビュー材料とする。
- ランタイム側では `runtime/native/include/reml_ffi_bridge.h` を新設し、`reml_ffi_acquire_borrowed(void*)` / `reml_ffi_acquire_transferred(void*)` / `reml_ffi_release_transferred(void*)` / `reml_ffi_box_string(const reml_string_t*)` / `reml_ffi_unbox_span(const reml_span_t*)`（`reml_span_t` も新設予定）などのヘルパ API を提供する。実装は `src/ffi_bridge.c` にまとめ、既存の `mem_alloc`・`inc_ref`・`dec_ref` と連携させる。
- 生成する LLVM IR には `llvm::Metadata` で `bridge.platform` / `bridge.abi` / `bridge.ownership` / `bridge.stub_id` を埋め込み、`AuditEnvelope` が利用するキーと揃える。IR 生成時に `!llvm.module.flags` へ `reml.bridge.version = 1` を追加し、将来の互換性チェックを容易にする。
- C ヘッダの生成は Phase 2 ではリポジトリ内スクリプト（`scripts/gen-ffi-headers.reml`）で行い、生成物に `SPDX-License-Identifier`・コミットハッシュ・生成日時を付記する。外部ツール（`cbindgen` 等）を導入する場合はライセンス互換性と再現性を `docs/notes/licensing-todo.md`（新規予定）へ記録し、Phase 3 での自動化移行を前提にレビューする。

### JSON 監査スキーマ更新案（FFI Bridge 拡張）

- `AuditEnvelope` スキーマに `bridge` オブジェクトを追加し、必須プロパティとして `bridge.target` / `bridge.arch` / `bridge.abi` / `bridge.ownership` / `bridge.extern_symbol` を定義。オプションで `bridge.alias`, `bridge.library`, `bridge.callconv`, `bridge.audit_stage` を許容する。
- スキーマ改訂は `tooling/runtime/audit-schema.json`（ドラフト）で管理し、検証スクリプトに `./scripts/validate-runtime-capabilities.sh --schema audit` を追加して自動チェックする案を提案。2025-10-18 時点でドラフト v0 を追加し、`diagnostics[]` 配列や `bridge.*` 必須キーを定義済み。
- ゴールデンテスト: `compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` を新設し、`bridge.target = arm64-apple-darwin`／`bridge.target = x86_64-pc-windows-msvc` のサンプルを記録。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `ffi.bridge.audit_pass_rate` 指標を追記する。
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

- LLVM lowering で生成した `reml.bridge.stubs` メタデータを検証するゴールデンを追加し、`tests/test_ffi_lowering.ml` をプラットフォーム別ケースへ拡張する。
- ランタイム計測 API を CI に統合する方式を決定し、`tooling/ci/sync-iterator-audit.sh` で `ffi_bridge.audit_pass_rate` を収集・評価できる状態にする。
- Linux/Windows 向けの FFI サンプル実行とログ収集を行い、`reports/ffi-linux-summary.md`・`reports/ffi-windows-summary.md` を初回更新する。
- 仕様書・ガイド更新案（3-9, 3-6, runtime-bridges.md）をレビューに回し、メタデータ出力・計測 API の使い方を文書化する。

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
- `BridgeStubPlan`（ターゲットトリプル、Calling Convention、所有権、監査キー）を `ffi_contract` 正規化後に生成し、`compiler/ocaml/src/codegen/ffi_stub_builder.ml` で `emit_stub_module` へ受け渡す（初期版完了）。
- Linux System V / Windows MSVC (`win64`) / macOS arm64 (`aapcs64`) 用のテンプレートをテーブル化し、シンボル名・`dso_local`・可視性・`linkage` を自動決定する（テンプレート確立済み、stub/thunk 実生成が次段階）。
- Reml ↔ C のマーシャリングは `FfiValueLowering` へ段階的に移行中で、現在はメタデータ埋め込みを提供。今後、引数/戻り値の変換と RC 操作を実装する。
- 監査ログで利用する `bridge.platform` / `bridge.abi` / `bridge.ownership` は stub 生成段階で `llvm::Metadata` として埋め込む（`reml.bridge.version` フラグと `reml.bridge.stubs` Named Metadata を出力済み）。

4.2. **LLVM IR への lowering**
- 呼出規約を `ccc` / `win64` / `aarch64_aapcscc` 等の LLVM 属性で明示し、ターゲット固有の `signext`・`zeroext`・`sret` 等補助属性を付与する。
- `DataLayout` 由来のサイズ・アライメントを用いて構造体・配列・スライスを `llvm::StructType` へ写像し、Reml `Span` と一致するようパディングを調整する。
- 借用引数に `nonnull` / `dereferenceable` / `align` 属性を付与し、移譲パスでは `llvm.lifetime.end` を挿入して RC 解放シーケンスと同期させる。
- `!dbg` 情報へ `ffi_contract` のシグネチャ・所有権注釈を転写し、監査ログ `call_site` と突合できるようにする。

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

6.2. **Windows x64 テスト**
- MSVC ABI のサンプル FFI 呼び出し
- Windows API の呼び出しテスト（`MessageBoxW` 等）
- ABI 差分の動作検証
- Phase 2 Windows タスクとの連携

6.3. **macOS arm64 テスト**
- Apple Silicon (arm64-apple-darwin) 上での FFI 呼び出し検証（`libSystem` / `dispatch` API など）
- Mach-O 向けスタブ生成と `codesign --verify` の簡易チェック
- Darwin ABI 固有のシグネチャ（構造体戻り値、可変長引数）の検証
- Phase 1-8 macOS 計測値と比較し、差分を `reports/ffi-macos-summary.md`（新規）へ記録

6.4. **CI/CD 統合**
- GitHub Actions に FFI テストジョブ追加
- Linux/Windows/macOS の 3 ターゲットでのテスト実行
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
- `docs/guides/runtime-bridges.md` の FFI セクション更新
- プラットフォーム別の注意事項を追記
- cbindgen 等のツール使用例
- トラブルシューティング情報

8.3. **Phase 3 準備**
- FFI のセルフホスト移植計画
- 残存課題の `docs/notes/` への記録
- 非同期 FFI の将来設計検討
- メトリクスの CI レポート化

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
