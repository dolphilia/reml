# 0.1 ベースライン資産と差分ハーネス整備

Phase P0 の中心タスクである資産棚卸しと差分検証基盤の整備方針を定義する。Rust 移植では OCaml 実装の可観測な挙動を維持することが最優先であり、既存資産をどのように再利用するかを本章で明文化する。

## 0.1.1 目的
- OCaml 実装が提供するソース資産・テスト・診断レポートを一覧化し、Rust 実装が追従すべき基準線を確定する。
- ベースライン測定（性能・安全性・診断指標）の収集手順を整理し、Phase P1 以降で再測定できる状態を作る。
- `dual-write` 構成で必要となる差分ハーネス（AST/IR 比較、診断 JSON 比較、メトリクス収集）の設計要件を定義する。

## 0.1.2 資産棚卸し

| カテゴリ | ロケーション | 主な内容 | Rust 版での利用方針 |
| --- | --- | --- | --- |
| パーサー / レキサー | `compiler/ocaml/src/parser_*.ml`, `compiler/ocaml/src/lexer.mll` | 構文解析とエラー回復実装、パーサー期待値 (`parser_expectation.ml`) | AST 生成結果を Rust 版と比較するためのゴールデンとする。パーサーハーネスで `.exp` スナップショットを共通化。 |
| 型推論 | `compiler/ocaml/src/type_inference/`, `compiler/ocaml/tests/test_type_inference.ml` | 型クラス・制約解決・辞書渡しテスト | Rust 版の `Result` ベース型推論出力と比較し、`collect-iterator-audit-metrics.py` で `typeclass.*` 指標を再測定。 |
| 診断・監査 | `compiler/ocaml/src/diagnostic*.ml`, `reports/diagnostic-format-regression.md`, `scripts/validate-diagnostic-json.sh` | CLI/LSP/Streaming 出力と JSON スキーマ検証 | 差分テストで CLI と JSON 出力を比較し、Rust 版の診断シリアライザで同一キーが揃うことを確認する。 |
| CI メトリクス | `tooling/ci/collect-iterator-audit-metrics.py`, `tooling/ci/` 各種スクリプト | Phase 2 で確立した監査ゲート指標 (`iterator.stage.audit_pass_rate` など) | Rust 版 CI で再利用し、`--require-success` を Rust/OCaml 双方に適用して差分を可視化する。 |
| ベンチマーク | `compiler/ocaml/benchmarks/`, `tooling/runtime/` | パーサー/型推論/コード生成ベンチマーク | Rust フロントエンド PoC（P1）で同スイートを実行し、性能回帰 ±10% の閾値を `collect-iterator-audit-metrics.py` 又は専用スクリプトで記録する。 |
| ランタイム連携 | `runtime/native/`, `compiler/ocaml/docs/runtime_bridge*.md` | FFI ブリッジ、Capability Stage 設定 | Rust 版 Runtime ブリッジ設計（P2）で ABI 契約を書き換える際の参照資料とする。 |

> **補足**: 各カテゴリの詳細な担当者・レビュー履歴は `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に追跡表が存在する。更新時は同表の該当行にレビュー記録を追加する。

## 0.1.3 メトリクスと基準値
- **性能**: `parse_throughput`, `memory_peak_ratio` を Phase 2 最新値で固定。`tooling/ci/collect-iterator-audit-metrics.py --section performance` の出力ログを CI にアーカイブし、Rust 版で同スクリプトを実行する。
- **安全性・診断**: `stage_mismatch_count`, `diagnostic.audit_presence_rate`, `typeclass.dictionary_pass_rate` を重点監視指標とする。`reports/diagnostic-format-regression.md` の差分表と `scripts/validate-diagnostic-json.sh` の結果をセットで保存する。
- **Windows 固有**: `collect-iterator-audit-metrics.py --require-success --platform windows-msvc`（`--platform windows-gnu` と併用）を追加し、MSVC/MinGW 双方の `stage_mismatch_count` を収集する。詳細手順は `0-2-windows-toolchain-audit.md` に記載。
- **レビュー記録**: 指標値の更新は `0-3-audit-and-metrics.md` の表にタイムスタンプ付きで追記し、Rust 版が計測した値と並べて比較できるようにする。

## 0.1.4 差分ハーネス設計
1. **AST/IR 比較**  
   - OCaml 側で `parser_expectation.ml` により出力される `.exp` ファイルをベースライン化。  
   - Rust 版は同フォーマットで AST ノード列を吐き出し、差分は `diff --strip-trailing-cr` 互換フォーマットで記録。  
   - 差分検査時は `unified-porting-principles.md` §3 のデータモデル規約に従い、数値型と文字列の正規化ルールを共有する。
2. **診断 JSON 比較**  
   - `scripts/validate-diagnostic-json.sh` を Rust 出力に適用し、欠落キーを `diagnostic-format-regression.md` の表形式で報告。  
   - `tooling/ci/collect-iterator-audit-metrics.py --section diagnostics` の結果を OCaml/Rust で並列収集し、`AuditEnvelope.metadata` の差異を自動検出する。
3. **性能・リソース計測**  
   - Linux/macOS/Windows それぞれで同ベンチマークスイートを実行し、`parse_throughput` などの指標を `.json` で保存。  
   - Rust 版導入時は `criterion` 等で詳細なプロファイルを追加しつつ、OCaml 版と同じ観測項目を残す。

## 0.1.5 作業手順
1. Phase 2 の最新ブランチで OCaml 実装をビルドし、`dune runtest` + `collect-iterator-audit-metrics.py --require-success` を実行。結果を `reports/diagnostic-format-regression.md` の「Baseline (OCaml)」欄へ反映する。  
   - ✅ 2025-11-06: `dune runtest` 成功（`reports/dual-write/20251106-ocaml-baseline.md`）。  
   - ⚠️ 2025-11-06: `scripts/validate-diagnostic-json.sh tmp/diagnostics-output/`・`collect-iterator-audit-metrics.py --require-success` を再実行したが、`effects.required_capabilities`・RunConfig `lex`・`bridge` 監査キーなどの欠落で失敗。ログは `reports/dual-write/20251106-validate-diagnostic-json.md` と `reports/dual-write/20251106-collect-iterator-metrics.json` に保存。OCaml CLI 診断シリアライザのメタデータ拡充後に再測定する。
2. `tooling/toolchains/setup-windows-toolchain.ps1` を Windows CI で実行し、`0-2` のチェックリストに従ってログを保存。Rust 版 PoC で同ログフォーマットを流用する。
3. `docs/notes/` に Rust 移植向け差分観測ノート（例: `docs/notes/rust-migration-baseline.md` 仮）を作成し、知見や TODO を記録。計画書本体には概要のみ記載する。
4. ベースライン値を `docs/plans/rust-migration/0-0-roadmap.md` §0.0.3 に反映し、P0 のマイルストーン達成を確認する。

## 0.1.6 関連資料
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`
- `docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md`
- `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md`
- `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`
- `reports/diagnostic-format-regression.md`
- `scripts/validate-diagnostic-json.sh`
- `tooling/ci/collect-iterator-audit-metrics.py`

---

> **TODO 提案**: 差分ハーネスの実装詳細は Phase P1 で Rust 側の PoC を作成するタイミングで `docs/notes/` に補助資料を追加する。P0 では手順定義のみ行い、実装は後続フェーズに引き継ぐ。
