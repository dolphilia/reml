# 2.0 LLVM バックエンド統合計画

本章は Phase P2（バックエンド統合）における Rust 実装側 LLVM バックエンドの設計・マイルストーン・検証手順を定義する。`unified-porting-principles.md` の優先順位原則（振る舞いの同一性最優先）と P0/P1 で確立したベースラインに従い、OCaml 実装 (`compiler/ocaml/src/llvm_gen/`) と等価な IR を Rust 実装で再現しつつ、Windows x64 を含む主要プラットフォームのツールチェーン統合を完了させることを目的とする。

## 2.0.1 目的
- Reml Rust 実装の LLVM IR 生成器を構築し、OCaml 実装が出力する IR と観測可能な挙動（`TargetMachine` 設定、`DataLayout`、最適化パス、診断ログ）を等価に再現する。
- Windows x64 (`x86_64-pc-windows-gnu` / `x86_64-pc-windows-msvc`) を含む 3 ターゲット（Linux GNU、macOS Darwin、Windows GNU/MSVC）で LLVM バックエンドが動作することを `opt -verify` と `llc` 生成物比較で保証する。
- バックエンドの成果物を P2 以降のランタイム統合 (`2-1-runtime-integration.md`) と CI 統合 (`3-0-ci-and-dual-write-strategy.md`) に引き渡せる状態に整理し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` が示す環境制約を Rust 実装側の運用に落とし込む。

## 2.0.2 スコープと前提
- **対象範囲**
  - MIR から LLVM IR までのコード生成（型レイアウト、呼出規約、アトリビュート、例外ハンドリング経路、代数的データ型の表現）。
  - `TargetMachine`/`DataLayout` の構築と Triple 切替、`llvm::PassManager` 相当の最適化パイプライン定義。
  - IR 検証 (`opt -verify`)、`llc`/`llvm-dis` を用いた差分検証とゴールデン比較。
  - Windows 編成（MSYS2 LLVM 16 継続利用、公式 ZIP 19.1.1 への切替条件）および macOS/Linux の LLVM 配布物連携。
- **除外**
  - ランタイム呼び出し FFI 層（`2-1-runtime-integration.md` で扱う）。
  - CI 差分ハーネス更新・監査メトリクス集約（P3 スコープ）。
  - Self-host Rust コンパイラへの完全移行判断（P4 スコープ）。
- **前提**
  - P1 フロントエンド移植文書の完了条件（AST/診断ゴールデン整備）が満たされており、MIR 生成まで Rust 実装が進んでいる。
  - P0 で規定したゴールデン比較基盤 (`0-1-baseline-and-diff-assets.md`) が動作し、LLVM IR 用の比較テーブルが用意されている。
  - `docs/guides/llvm-integration-notes.md` の設計原則と `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` の調査結果が最新である。

## 2.0.3 完了条件
- Rust バックエンドが生成した LLVM IR が、P0/P1 ゴールデン比較で差分ゼロ（コメント・メタデータ差分は既知の許容範囲内）を維持し、`opt -verify` と `llc` による検証を全ターゲットで通過する。
- `TargetMachine` 設定（Triple、`CPU`、`features`、`relocation_model`、`code_model`、`optimization_level`）が OCaml 実装と一致し、`reports/diagnostic-format-regression.md` に記録されるバックエンド診断 (`target.config.*`, `effects.contract.stage_mismatch` 等) がゼロ件である。
- Windows CI (`windows-latest` × `msvc`/`gnu`) の 5 連続スイートグリーンを達成し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` で定義された fallback 手順（MSYS2 LLVM 16 継続利用、公式 ZIP 19.1.1 切替）が Rust 実装ドキュメントに反映される。
- LLVM バックエンドの API／CLI インタフェースが整理され、P3/P4 へのハンドオーバー資料（サブセットで `docs/notes/` に補足）を添付できる。

## 2.0.4 主成果物

| 成果物 | 内容 | 依存資料 |
| --- | --- | --- |
| `compiler/rust/backend/llvm/` | Rust 版 LLVM コードジェン crate（MIR→LLVM IR、`Builder`／`Module` ラッパ、`TargetMachine` 初期化、最適化パス定義） | `compiler/ocaml/src/llvm_gen/`, `docs/guides/llvm-integration-notes.md` |
| バックエンド差分ハーネス | OCaml/Rust バックエンド出力を比較する CLI (`--emit-llvm --backend={ocaml,rust}`)、`opt -verify` と `llc` 実行の自動化 | `0-1-baseline-and-diff-assets.md`, `tooling/ci/collect-iterator-audit-metrics.py` |
| Windows ツールチェーン手順更新 | MSYS2 LLVM 16 継続利用と公式 ZIP 19.1.1 切替条件、`rustup target add`、`clang-cl`/`link.exe` 連携の手順書 | `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`, `docs/plans/rust-migration/0-2-windows-toolchain-audit.md` |
| 監査メタデータ整備 | `Diagnostic.extensions["backend"]`, `audit.log("llvm.verify", ...)` 等の JSON フィールド定義と CI 保存先 | `docs/spec/3-6-core-diagnostics-audit.md`, `reports/diagnostic-format-regression.md` |

## 2.0.5 マイルストーン（目安）

| 週 | マイルストーン | 主タスク | 検証方法 |
| --- | --- | --- | --- |
| W1 | OCaml バックエンド資産棚卸し | `llvm_gen/*.ml` のモジュール依存関係解析、`DataLayout`／`CallingConvention` 対応表整理 | `compiler/ocaml/src/llvm_gen/` grep と `1-1-ast-and-ir-alignment.md` の参照更新 |
| W2 | Rust LLVM ラッパ層構築 | `llvm-sys`/`inkwell` 採用判断、`Module`/`Context`/`Builder` ラッパ、`TargetMachine` 初期化 | `opt -verify` 付き単体テスト、`llvm-config --host-target` 比較 |
| W3 | コード生成本体移植 | 型レイアウト、呼出規約、エントリポイント (`@k__main`) 生成、GC フック呼び出し実装 | OCaml 版との IR text diff（`diff --color=always`）、`llc` 実行 |
| W4 | Windows / macOS 向け統合 | `x86_64-pc-windows-gnu` / `x86_64-pc-windows-msvc` / `x86_64-apple-darwin` のビルドライン検証、`opt -verify` 自動化 | GitHub Actions matrix 成果、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` 更新 |
| W4.5 | P2 バックエンドレビュー | 差分レポート提出、監査ログ整備、P3 へのハンドオーバー資料ドラフト | `docs/plans/rust-migration/README.md` 更新、`docs-migrations.log` 記録 |

## 2.0.6 作業ストリーム

- **LLVM ラッパ層設計**  
  - Rust 側で `inkwell` ベース（安全ラッパ重視）と `llvm-sys` ベース（バージョン追従柔軟性重視）を比較し、Phase P2 では `llvm-sys` + 独自安全ラッパの採用を前提とする。`llvm-config --version` で 16.x / 19.x 双方を扱える設計にし、`builder.set_fastmath` 等の細部 API 差異を吸収する。  
  - `TargetOptions`、`RelocMode`、`CodeModel`、`OptLevel` は OCaml 実装 (`codegen_config.ml`) を参照してマッピングし、Rust 側で Triple ごとの差分（MSVC = `CodeModel::Large`, GNU = `CodeModel::Default` など）を明示したテーブルを `appendix/glossary-alignment.md` に追記する。

- **型レイアウト・ABI 整合**  
  - `docs/guides/llvm-integration-notes.md` §5 のレイアウト仕様に沿って、`StructType`/`ArrayType` の配置とアライメントを Rust 側で計算する。  
  - 呼出規約は `Abi::SystemV`（Linux/macOS）/`Abi::Win64`（Windows）で分岐し、`compiler/ocaml/src/llvm_gen/abi.ml` のロジックを `enum CallingConvention` で再現する。  
  - GC とランタイム呼び出し（`mem_alloc`/`inc_ref`/`dec_ref`/`panic`）は `2-1-runtime-integration.md` の FFI 層と連携し、`llvm::Attribute::NoUnwind` 等の付与規則を共有する。

- **最適化パイプライン**  
  - P1 で生成された MIR に対して `OCaml` 実装と同一のパス列（`mem2reg`, `instcombine`, `simplifycfg`, `tailcallelim` 等）を Rust 側 `PassManager` に設定する。  
  - `opt -verify` を各パス間で挿入し、失敗時に `Diagnostic.domain = "Backend"`、`code = "llvm.verify.failed"` を発行する。ログは `audit.log("llvm.verify", { target, pass, module })` へ記録し、CI から収集する。

- **Windows & クロスコンパイル対応**  
  - `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` が推奨する MSYS2 LLVM 16 継続運用を Rust バックエンドでも採用し、`llvm-config` が提供する `--libdir` `--includedir` へのパス解決を `tooling/toolchains/` スクリプトで自動化。  
  - LLVM 19 公式 ZIP への切替条件（`conf-llvm-static.19` が Pass、`reports/windows-env-check.json` 記録済み）を検知し、`REML_LLVM_DISTRIBUTION={msys2,official-zip}` フラグで切替。  
  - `clang-cl`/`link.exe` を使う MSVC パスでは `llvm-config --system-libs` の結果をそのまま使用できないため、`libcmt.lib` などの追加ライブラリを `docs/plans/rust-migration/0-2-windows-toolchain-audit.md` のチェックリストに追記する。

## 2.0.7 検証とメトリクス
- **IR 差分検証**: `remlc --backend={ocaml,rust} --emit-llvm` を dual-run し、`diff` 結果と `opt -verify` ログを `reports/backend-ir-diff/*.json` に保存。許容差分はコメントや LLVM バージョン固有のメタデータのみ。  
- **性能メトリクス**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` のバックエンド項目（IR 生成時間、`opt` 実行時間、`llc` 所要時間）を Rust バックエンドで採取し、OCaml 値との変動が ±10% 以内であることを確認。  
- **Windows 監査**: `audit.log("windows.toolchain", ...)` に LLVM パス情報・`TargetMachine` 設定・`llvm-config` 出力を記録し、`reports/windows-env-check.json` へ追記。  
- **CI サマリ**: GitHub Actions で `llvm-backend-verify` ジョブを追加し、`opt -verify` と `llc` のログを成果物化。5 連続成功後に P3 へ引き継ぐ。

## 2.0.8 リスクと対応
- **LLVM バージョン差異による API 変更**  
  - *リスク*: LLVM 16 と 19 の API 差異で Rust 側ビルドが不安定化。  
  - *対応*: `llvm-sys` を features 付きで 16/19 両対応にし、バージョン固有コードは `cfg(feature = "llvm19")` 等で切替。`docs/guides/llvm-integration-notes.md` へ差異一覧を追記する。

- **Windows MSVC ライブラリ欠落**  
  - *リスク*: `conf-llvm-static.19` の静的ライブラリ検出が再発し、Rust バックエンドがリンク失敗。  
  - *対応*: 公式 ZIP の `.lib` 確保手順を `tooling/toolchains/setup-windows-toolchain.ps1` に組み込み、`llvm-config --shared-mode` が `static` を返すことを CI で確認。

- **MIR→LLVM の所有権モデル崩れ**  
  - *リスク*: Rust MIR からの RC 操作挿入位置が OCaml とズレ、`inc_ref`/`dec_ref` 呼び出し数が変動。  
  - *対応*: `collect-iterator-audit-metrics.py` に `runtime.refcount.*` メトリクスを追加し、差異が閾値（±2%）以内であるか監視。大きな差分は `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に登録。

- **最適化パイプライン差異による IR 不一致**  
  - *リスク*: LLVM パス順序の違いで IR 差分が拡大し、ゴールデン比較が不安定。  
  - *対応*: OCaml 実装と同じ順序をデフォルトとし、Rust 側固有のパス追加は Feature flag (`-Zrust-backend-extra-pass`) で opt-in。デフォルト差分は `docs/notes/llvm-spec-status-survey.md` に記録。

## 2.0.9 関連ドキュメント更新
- 本章の進捗に応じて `docs/plans/rust-migration/README.md` と `docs/plans/README.md` に P2 セクションを追記すること。  
- Windows ツールチェーンの変更が発生した場合は `0-2-windows-toolchain-audit.md` と `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` を更新し、`docs-migrations.log` に記録する。  
- バックエンド仕様に影響が出る場合は `docs/spec/3-8-core-runtime-capability.md` / `3-9-core-async-ffi-unsafe.md` の該当節を参照し、必要であれば脚注を提案する。

## 2.0.10 ハンドオーバーと次フェーズ
- Rust バックエンドの API と差分検証ログを P3 CI 統合チームへ渡し、`3-0-ci-and-dual-write-strategy.md` での自動検証に利用する。  
- ランタイム連携タスクは `2-1-runtime-integration.md` へ移譲し、`ffi`, `panic`, `audit` 関連の呼び出し箇所を共有。  
- P4 最適化フェーズで性能調整を行えるよう、バックエンド内の計測ポイント（カウンタ、トレースログ）を残しておく。

---
**参照**: `docs/plans/rust-migration/overview.md`, `docs/plans/rust-migration/unified-porting-principles.md`, `docs/guides/llvm-integration-notes.md`, `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`, `reports/diagnostic-format-regression.md`
