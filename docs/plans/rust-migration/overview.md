# Rust 移植計画概要

Phase 2-6 の Windows 対応停滞を受け、OCaml 実装から Rust 実装へ移行するための工程と成果物を定義する。移植の原則は `unified-porting-principles.md` に統合されており、本概要は同ガイドに沿って必要ドキュメントと実務タスクを整理する。

## 背景と目的
- `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` にて移植先言語として Rust を選定済み
- Phase 2-8 仕様監査 (`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`) を阻害しないよう、Rust 版を並行準備しつつ OCaml 実装との整合を保つ
- Phase 3 のセルフホスト移行を見据え、Rust 実装の計画・リスク・CI 基盤を段階的に整備する

## 必要ドキュメント一覧（統合原則に基づく）

統合ガイドで定義したフェーズ（P0〜P4）に対応する計画書・設計書を以下に整理する。初稿は揃っており、更新順序と追加で必要になる資料をこの一覧で管理する。

| フェーズ | ドキュメント | 目的 | 主要参照元 / 連携先 |
| --- | --- | --- | --- |
| P0 ベースライン整備 | `0-0-roadmap.md` | 全体ロードマップ・マイルストーン定義・依存関係整理 | `docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md`, `docs/plans/bootstrap-roadmap/2-0-phase2-stabilization.md` |
|  | `0-1-baseline-and-diff-assets.md` | 現行 OCaml 資産棚卸し、ゴールデン/ベンチ基準、差分テストハーネス設計 | `compiler/ocaml/`, `reports/diagnostic-format-regression.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` |
|  | `0-2-windows-toolchain-audit.md` | Windows 向け環境診断・`rustup`/MSVC 設定・自動化手順 | `docs/plans/bootstrap-roadmap/2-6-windows-support.md`, `tooling/toolchains/` |
|  | `appendix/glossary-alignment.md` | 用語・略語整合（Rust 特有用語と仕様用語の対応表） | `docs/spec/0-2-glossary.md`, `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` |
| P1 フロントエンド移植 | `1-0-front-end-transition.md` | パーサ/型推論移植、テスト移行計画、dual-write 戦略 | `compiler/ocaml/src/parser_*`, `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md` |
|  | `1-1-ast-and-ir-alignment.md` | OCaml↔Rust AST/IR 対応表・検証手順 | `compiler/ocaml/docs/parser_design.md`, `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md` |
|  | `1-2-diagnostic-compatibility.md` | 診断・監査出力の互換計画、差分検証フロー | `reports/diagnostic-format-regression.md`, `tooling/ci/collect-iterator-audit-metrics.py`, `docs/spec/3-6-core-diagnostics-audit.md` |
| P2 バックエンド統合 | `2-0-llvm-backend-plan.md` | LLVM バックエンド実装、`TargetMachine`/`DataLayout` 整合、MSVC/GNU 対応 | `docs/guides/llvm-integration-notes.md`, `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` |
|  | `2-1-runtime-integration.md` | Rust 実装と既存ランタイムの橋渡し、FFI/ABI 契約、unsafe ポリシー | `docs/spec/3-8-core-runtime-capability.md`, `docs/spec/3-9-core-async-ffi-unsafe.md`, `runtime/native/` |
|  | `2-2-adapter-layer-guidelines.md` | FS/ネット/時刻/乱数などアダプタ層設計とプラットフォーム差分吸収方針 | `language-runtime` 設計メモ, `tooling/` |
|  | `2-3-p2-backend-integration-roadmap.md` | LLVM/Runtime/Adapter 計画を束ねた P2 統合ロードマップ、Go/No-Go 条件 | `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`, `docs/plans/rust-migration/overview.md` |
| P3 CI/監査統合 | `3-0-ci-and-dual-write-strategy.md` | CI マトリクス、dual-write 運用、差分ハーネス自動化 | `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md`, `.github/workflows/`, `tooling/ci/` |
|  | `3-1-observability-alignment.md` | 監査メトリクス・ログ・トレース連携、`collect-iterator-audit-metrics.py` 対応 | `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`, `reports/audit/dashboard/` |
|  | `3-2-benchmark-baseline.md` | Rust 版ベンチマーク定義、性能比較、許容回帰線 | `compiler/ocaml/benchmarks/`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` |
| P4 最適化とハンドオーバー | `4-0-risk-register.md` | 移植固有リスク・緩和策・エスカレーション経路 | `compiler/ocaml/docs/technical-debt.md`, `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` |
|  | `4-1-communication-plan.md` | チーム連携・レビュー体制・Phase 3/4 ハンドオーバー方針 | `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` |
|  | `4-2-documentation-sync.md` | 仕様・ガイド・ノート更新、脚注整合、`docs-migrations.log` 管理 | `docs/spec/`, `docs/guides/`, `docs/notes/` |


## 次のステップ
1. `unified-porting-principles.md` を参照して移植方針を確定し、その方針を `0-0-roadmap.md` へ反映する
2. Rust 実装の着手対象（フロントエンド、バックエンド、CI）の優先順位を `docs/plans/bootstrap-roadmap/2-6-windows-support-migration-options.md` の評価軸に基づいて確定する
3. 各計画書で使用する測定項目・マイルストーンを `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` と同期させ、Phase 3 へのハンドオーバー方針を `4-1-communication-plan.md` に追記する
4. P4 最適化とハンドオーバー向けのドキュメント（`4-0-risk-register.md` / `4-1-communication-plan.md` / `4-2-documentation-sync.md`）を参照し、最終調整タスクのリスク・連携・文書整合を明文化する

## 作業開始手順
1. **P0 文書セットアップのセルフレビュー**  
   - `0-0-roadmap.md`・`0-1-baseline-and-diff-assets.md`・`0-2-windows-toolchain-audit.md`・`appendix/glossary-alignment.md` の現状を確認し、欠落セクションや古い参照がないかチェックリスト化する。  
   - レビュー結果は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` の対応表へ書き戻し、更新が必要な項目に reviewer・due を割り当てる。
   - ✅ 2025-11-06: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md#phasep0review-2025-11-06` にレビュー結果を登録し、該当文書のコマンド表記を最新化済み。
2. **OCaml ベースライン測定と記録更新**  
   - Phase 2 最新ブランチで `dune runtest` と `tooling/ci/collect-iterator-audit-metrics.py --require-success` を実行し、`reports/diagnostic-format-regression.md`・`docs/plans/rust-migration/0-1-baseline-and-diff-assets.md` のメトリクス欄を埋める。  
   - 取得ログは `reports/dual-write/`（新設予定）または既存の `reports/diagnostic-format-regression.md` 付属アーカイブへ保存し、差分比較用に日付タグを付ける。  
   - ✅ 2025-11-06: `compiler/ocaml/` 直下で `dune runtest` を実行し、エラーなく完了（ログ: `reports/dual-write/20251106-ocaml-baseline.md`）。  
   - ⚠️ 2025-11-06: `scripts/validate-diagnostic-json.sh tmp/diagnostics-output/` と `python3 tooling/ci/collect-iterator-audit-metrics.py --require-success` を再実行したが、診断 JSON のメタデータ不足により失敗。詳細ログと未解決項目は `reports/dual-write/20251106-ocaml-diagnostics-refresh.md`・`reports/dual-write/20251106-collect-iterator-metrics.json`・`reports/dual-write/20251106-validate-diagnostic-json.md` を参照。`parser.core.rule.*` メタデータのネスト構造や `effects.required_capabilities`/`bridge.*` の監査伝搬が未完了であるため、OCaml 側のシリアライザと RunConfig 付与処理を追加実装して再測定するフォローアップタスクを継続する。
  - ✅ 2025-11-07: `Diagnostic.merge_audit_metadata` に `parser.core.rule` ネスト辞書と `parser` 拡張を付与し、CLI/FFI 向け監査ログの `bridge.*` 系キーを `extensions`/`metadata` 双方に出力するよう修正。`dune runtest` を再実行して全テストが緑化、`compiler/ocaml/tests/golden/` 配下の診断／監査ゴールデンを刷新済み。  
  - ✅ 2025-11-07: `with_parser_runconfig_metadata` の空配列初期化を適用し、`scripts/validate-diagnostic-json.sh compiler/ocaml/tests/golden/diagnostics/parser/{expected-summary,parser-runconfig-packrat,streaming-outcome}.json.golden` が通過することを確認（非診断ファイル `effects/syntax-constructs.json.golden` は今後も引数から除外する運用を維持）。  
  - ✅ 2025-11-07: `tooling/ci/collect-iterator-audit-metrics.py` の `ffi_bridge.audit_pass_rate` が診断/監査双方での二重カウントにより `pass_rate=0.0` となっていたため修正。現在は `python3 tooling/ci/collect-iterator-audit-metrics.py --section ffi --require-success --source compiler/ocaml/tests/golden/diagnostics/ffi/unsupported-abi.json.golden --audit-source compiler/ocaml/tests/golden/audit/cli-ffi-bridge-{linux,macos,windows}.jsonl.golden --audit-source compiler/ocaml/tests/golden/audit/ffi-bridge.jsonl.golden` がゼロ終了し、pass_rate/pass_fraction ともに `1.0` を返す。
3. **Windows ツールチェーン監査の先行着手**  
   - `tooling/toolchains/setup-windows-toolchain.ps1 -CheckOutputJson <出力パス>` と `check-windows-bootstrap-env.ps1 -OutputJson <出力パス>` を GitHub Actions の `windows-latest` かローカル VM で実行し、`0-2-windows-toolchain-audit.md` の項目順に検証結果を埋める。  
   - JSON 出力を `reports/toolchain/windows/YYYYMMDD/` に配置し、`docs/plans/bootstrap-roadmap/2-6-windows-support.md` の該当行へ監査完了ログをリンクする。
   - ⚠️ 2025-11-06: Windows ローカル環境（PowerShell 7）で `setup-windows-toolchain.ps1 -CheckOutputJson` を実行したところ、`check-windows-bootstrap-env.ps1` に `-OutputJson` 引数が伝搬されず失敗。詳細は `reports/toolchain/windows/20251106/setup-windows-toolchain-error.log` に記録。  
   - ✅ 2025-11-06: `setup-windows-toolchain.ps1 -NoCheck` で PATH/MSVC を初期化した後、`check-windows-bootstrap-env.ps1 -OutputJson` を 2 回実行して `reports/toolchain/windows/20251106/{setup-windows-toolchain.json, check-windows-bootstrap-env.json}` を生成。  
   - ✅ 2025-11-06: `setup-windows-toolchain.ps1` を更新し、PowerShell 7 でも `-CheckOutputJson` が `check-windows-bootstrap-env.ps1` へ伝搬するよう修正（ハッシュテーブル経由の引数スプラットに変更）。再実行で JSON 出力が自動生成されることを確認。  
   - ✅ 2025-11-06: `docs/plans/bootstrap-roadmap/2-6-windows-support.md` に監査ログとエラーノートのリンクを追記。  
4. **用語整合と索引用語の確定**  
   - `appendix/glossary-alignment.md` の暫定表に Rust 固有用語（所有権、Borrow Checker、unsafe block など）と Reml 仕様用語の対応案を追加し、`docs/spec/0-2-glossary.md` の脚注と突合する。  
   - 新規語彙が仕様へ波及する場合は `docs/notes/` に下書きを残し、レビュー完了後に用語集へ反映する。
   - ✅ 2025-11-08: `docs/plans/rust-migration/appendix/glossary-alignment.md` を拡張し、借用種別やムーブを含む Rust 用語の対応表を更新。参照列を `docs/spec/0-2-glossary.md#所有権とリソース管理` へ差し替え。  
   - ✅ 2025-11-08: `docs/notes/glossary-rust-alignment-draft.md` を作成し、所有権・借用・非安全ブロックの仮定義とレビュー方針を整理（履歴メモとして保管）。  
   - ✅ 2025-11-08: `docs/spec/0-2-glossary.md#所有権とリソース管理` に所有権・借用・非安全ブロック・解放責務の定義を追加し、FFI/診断章の脚注と整合させた。
5. **P1 フロントエンド作業の段取り確定**  
   - `1-0-front-end-transition.md` に沿って、Lexer/Parser・AST/IR・型推論・診断の各ワークストリームにスプリント目安を割り当てる。  
   - `1-1-ast-and-ir-alignment.md` と `1-2-diagnostic-compatibility.md` のチェックリストをスプレッドシート等へ転記し、dual-write PoC のタスク分割（Parser ハーネス整備→診断 JSON 差分検証→性能測定）の順序を明示する。
   - ✅ 2025-11-09: チェックリスト項目を取りまとめた `docs/plans/rust-migration/p1-front-end-checklists.csv` を作成し、スプレッドシートインポート用の列（カテゴリ/ワークストリーム/項目/受入基準/参照元）を整理。  
   - ✅ 2025-11-09: 各ワークストリームのスプリント割当と完了基準を以下の表に確定し、`unified-porting-principles.md` の優先順位原則に合わせて依存順序を明示。

   | スプリント | ワークストリーム | 主タスク | 完了基準 |
   | --- | --- | --- | --- |
   | S1 (W1) | Lexer / Parser | `parser_driver.ml` 相当の状態管理移植、dual-write ハーネス初期化、Packrat メトリクス収集 | `remlc --frontend {ocaml,rust}` で AST/Packrat 差分ゼロ、`collect-iterator-audit-metrics.py --section parser` 成功 |
   | S2 (W2) | AST / IR | AST/Typed AST 構造体定義、`Span`/`Ident` 正規化、`core_parse_streaming` の span_trace 連携 | `p1-front-end-checklists.csv` AST 行が全て「緑」、`1-1-ast-and-ir-alignment.md` チェックリスト 80% 消化 |
   | S3 (W3) | 型推論 | 制約生成・ソルバ移植、Impl Registry ロック方針確定、dual-write 型 JSON 比較 | `test_type_inference.ml` 対応ケースの JSON 差分ゼロ、`collect-iterator-audit-metrics.py --section effects` 成功 |
   | S4 (W4) | 診断 | Rust `Diagnostic.Builder` 実装、JSON/LSP 出力整合、dual-write 差分レポート自動化 | `scripts/validate-diagnostic-json.sh` 成功、`reports/dual-write/front-end` に差分 0 のレポート確保 |

   - ✅ 2025-11-09: dual-write PoC の実行順序を「1) Parser ハーネス整備 → 2) 診断 JSON 差分検証 → 3) パフォーマンス計測報告」と定義し、S1〜S4 の完了条件に組み込んだ。
   - ✅ 2025-11-09: dual-write 手順を `docs/plans/rust-migration/1-3-dual-write-runbook.md` に集約し、実行コマンド・切り分け手順・`reports/dual-write/` 命名規則を統一。
   - ✅ 2025-11-18: `effect_handler.reml` を Rust Frontend で受理し、`reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json` / `effect_handler-20251118-dualwrite.md` に診断 0 件と dual-write 差分ゼロを記録。Phase 1 完了条件へ「Chapter 1 効果サンプルが Rust Frontend + Streaming 経路で監査ログを残す」項目を追加し、`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の Step 2 を `Closed` に更新。
6. **CI・リスク連携の初期タスク洗い出し**  
   - `.github/workflows/` の既存ジョブを調査し、Rust ジョブ追加のために必要な環境変数・キャッシュ・アーティファクト仕様を `3-0-ci-and-dual-write-strategy.md` に列挙する。  
   - 監査・リスク関連文書（`3-1-observability-alignment.md`, `3-2-benchmark-baseline.md`, `4-0-risk-register.md`）から早期に着手すべき「事前準備」項目を抽出し、`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の未解決事項とマッピングする。
   - ✅ 2025-11-10: `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md` に §3.0.9 を追加し、Linux/macOS/Windows 各ジョブの環境変数・キャッシュ・アーティファクト仕様を Rust フロントエンド向けに整理した。
   - ✅ 2025-11-10: `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に「Rust 移植計画とのマッピング（初期タスク）」セクションを新設し、OBS-RUST-01 / BENCH-RUST-01 / RISK-RUST-01 の TODO を登録して `3-1`・`3-2`・`4-0` と紐付けた。

## W4.5 クロージングレビュー（P1→P2 ハンドオーバー）
- `1-0-front-end-transition.md#w4.5-p1-クロージングレビューp2-ハンドオーバー準備` で W4.5 の判定を実施。Parser Recover は `20280210-w4-diag-recover-else-r4` で `diag_match`/`metrics_ok` を達成し `P1_W4.5_frontend_handover/diag/recover/` に収集済み。Streaming / Type&Effect / CLI/LSP は Run ID（`20280410-*`, `20280418-*`, `20280601-*`, `20280430-*`）を Pending として P2 の TODO に移送した。
- `1-1-ast-and-ir-alignment.md#1-1-11-p2-連携メモw4.5`, `1-2-diagnostic-compatibility.md#1-2-22-w4.5-診断クロージングメモ`, `1-3-dual-write-runbook.md#1.3.6-w4.5-引き継ぎパッケージ作成手順` にハンドオーバー手順と成果物ディレクトリ（`reports/dual-write/front-end/P1_W4.5_frontend_handover/`）を明記した。
- `p1-front-end-checklists.csv`・`appendix/w4-diagnostic-case-matrix.md`・`w4-diagnostic-cases.txt` に `HandedOver` / `#handed_over` 区分を追加し、Recover ✅ / Streaming/TypeEffect/CLI Pending(W4.5) を一目で把握できるようにした。
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の `DIAG-RUST-05/06/07` セクション、ならびに `reports/dual-write/front-end/w4-diagnostics/README.md` を最新 Run ID で更新し、P2 での再測定タスク（StageAuditPayload、RunConfigBuilder、ExpectedTokenCollector など）を追跡する。

## 関連する既存タスクとの依存
- Phase 2-6 の未完了タスクは Rust 移植計画へ移管し、`docs/plans/bootstrap-roadmap/2-6-windows-support.md` の更新時に脚注で参照する。移管後は `unified-porting-principles.md` のチェックリストに沿って進捗を評価する
- Phase 2-8 の仕様監査では Rust 計画書の差分検討結果を確認対象とし、仕様更新後は即時に `glossary-alignment.md` を更新する
- 技術的負債 (`compiler/ocaml/docs/technical-debt.md`) に記載された Windows 関連項目の解消状況を追跡し、Rust 移植のリスク評価に反映する
