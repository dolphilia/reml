# 5.0 Phase 5 — Rust セルフホスト移行計画

Phase 5 は Phase 4 の実用テスト成果を基に、Rust 実装コンパイラを自分自身でビルド・検証するセルフホスト工程を定義する。ここでは `compiler/rust` の Stage 0（既存バイナリ）→ Stage 1（Rust 版でビルド）→ Stage 2（Stage 1 で再ビルド）という自己再現性チェーンを確立し、Phase 6 の正式リリース判定に必要な根拠を揃える。

## 5.0.1 目的
- Rust 実装のみで Reml コンパイラをビルド・テスト・配布できる状態を作り、OCaml 実装や手動手順への依存を排除する。
- Phase 4 の `.reml` 実用シナリオと `0-3-audit-and-metrics.md` の KPI をセルフホストチェーンへ持ち込み、自己再現性と性能回帰を定期的に測定する。
- `docs/plans/rust-migration/unified-porting-principles.md` で定義した成功指標（95% ゴールデン一致、監査キー完全一致、Windows マトリクス安定）をセルフホスト環境の検収基準に昇格させる。

## 5.0.2 スコープ境界
- **含む**: 自己ビルド手順の確立、Stage 切替の自動検証、セルフホスト専用の CI ジョブ、再現性レポート、再ビルド差分の監査。
- **含まない**: マルチターゲットリリースやパッケージ配布（Phase 6）、既存 4-x 計画のリネーム。これらは Phase 5 完了後に一括で行う。
- **前提条件**: Phase 4 のシナリオ網羅率 ≥ 85%、Rust Frontend/Backend/Runtime が Phase 3 仕様と一致、`2-7-deferred-remediation.md` に Phase 5 開始ブロッカーが無いこと。

## 5.0.3 作業ディレクトリ / 主な対象
- `compiler/rust/`（`frontend`, `driver`, `runtime`, `Cargo.*`）
- `tooling/bootstrap/`（新設: self-host runner, stage diff scripts）
- `.github/workflows/rust-self-host.yml`（新設）
- `reports/self-host/`（新設: Stage 別ログ、差分レポート）
- `docs/plans/bootstrap-roadmap/assets/phase5-self-host-checklist.md`（チェックリスト）

## 5.0.4 成果物とマイルストーン
| マイルストーン | 内容 | 検証方法 | 期限目安 |
|----------------|------|----------|----------|
| M1: Stage 0 → Stage 1 ブートストラップ | 既存 Rust バイナリで Stage 1 をビルドし、`cargo test` / Phase 4 シナリオを通過 | `reports/self-host/stage1-build-*.md`、CI での緑化 | Phase 5 開始後 4 週 |
| M2: Stage 1 → Stage 2 再帰ビルド | Stage 1 バイナリで Stage 2 を作成し、差分/診断/メトリクスを比較 | `reports/self-host/stage2-diff-*.md`、`collect-iterator-audit-metrics.py --section selfhost` | 開始後 7 週 |
| M3: 自己再現性レビュー | Stage 0/1/2 の LLVM IR・診断・性能差が許容範囲に収束 | `phase5-self-host-checklist.md`, レビュー議事録 | 開始後 9 週 |
| M4: Phase 6 ハンドオーバー | Self-host pipeline の手順・失敗時の切り分け・再実行フローを `6-0-phase6-migration.md` へ移送 | `phase5-readiness.md` | 開始後 10 週 |

## 5.0.5 ワークストリーム
1. **セルフホスト準備と依存整理**
   - `Cargo.toml` / `rust-toolchain.toml` の固定化、`rustup component` のバージョンを Phase 5 ブランチでロック。
   - Stage 0 ビルド（既存 CI 産物）を `tooling/bootstrap/download-stage0.sh` で取得し、署名/ハッシュを `reports/self-host/stage0-manifest.json` に記録。
   - Phase 4 の `phase4-scenario-matrix.csv` からセルフホスト向けの「smoke」「full」ラベルを引き継ぎ、Stage A/B 比較に使用する入力セットを定義。

2. **Stage 1/Stage 2 ビルドパイプライン**
   - `tooling/bootstrap/self_host_runner.py`（仮称）を実装し、`stage0 -> stage1 -> stage2` のビルド・テストを一括実行。各段階で `cargo test`, `run_examples.sh --suite practical`, `collect-iterator-audit-metrics.py` を呼び出す。
   - `.github/workflows/rust-self-host.yml` を追加し、Linux/Windows/macOS の 3 マトリクスで週次 self-host を実施。成果物は `reports/self-host/YYYYMMDD/` に保存。
   - `reports/self-host/stage-diff-*.md` に Stage 間の差分（バイナリサイズ、LLVM IR 行数、診断件数）を記録し、閾値を超えた場合は自動で Issue 化。

3. **診断・監査整合性チェック**
   - Stage 1/2 で生成された診断 JSON と監査 JSONL を `scripts/validate-diagnostic-json.sh` / `collect-iterator-audit-metrics.py --section selfhost` で検証し、Stage 間差分を `diagnostic_delta <= 0`（改善のみ許可）に制約。
   - Stage 2 で観測された `effects.contract.*` や `bridge.stage.*` を、Stage 0/1 のログと突き合わせて `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスクトレースを追加。
   - `reports/self-host/trace-coverage-*.md` で Stage ごとのトレース ID カバレッジを集計し、Phase 2-8 の監査要件を満たすことを確認。

4. **フォールバックとエスカレーション手順**
   - Stage 1/2 のいずれかでブロッカーが発生した場合、`tooling/bootstrap/self_host_runner.py --resume stage1` のように途中から再実行できる仕組みを実装。
   - フォールバックポリシー（Stage 0 での暫定ビルド使用可否、OCaml 実装を呼び出さないことの保証）を `phase5-self-host-checklist.md` に明文化。
   - Self-host 成果が Phase 6 の互換性検証（従来 4-x）へ流れ込むまでのデータフローを `6-0-phase6-migration.md` と共有し、`docs/plans/bootstrap-roadmap/README.md` のクリティカルパスを更新。

## 5.0.6 測定と検証
- **自己再現性スコア**: Stage 0/1/2 のバイナリハッシュ差分、LLVM IR 差分、診断差分を 3 つの指標として可視化し、目標を `stage2 == stage1`（許容差分±1%）とする。
- **ビルド時間**: `cargo build --workspace --release` の完了時間を Stage ごとに測定し、Phase 4 の性能ベースライン ±10% 以内を維持。
- **監査 KPI**: `collect-iterator-audit-metrics.py --section selfhost` で `selfhost.pass_rate == 1.0` を要求、`stage_mismatch_count == 0` を CI ゲートに加える。
- **可観測性**: `reports/self-host/stage2-observability.json` にトレース ID, AuditEnvelope, RunConfig をまとめ、Phase 6 の互換性検証チームへ提供。

## 5.0.7 リスクとフォローアップ
- **Stage 差分の長期残存**: Stage 1/2 の差分が 2 週連続で残る場合は `0-4-risk-handling.md` に `SELFHOST-###` リスクを登録し、Phase 4 シナリオや Phase 3 実装へフィードバックする。
- **ビルド時間の肥大化**: Stage 2 ビルドが 2 時間を超えた場合、`compiler/rust/` の機能フラグを見直し、`cargo build -p` ベースの分割検証を導入する。
- **環境ばらつき**: Self-host CI で OS/アーキ固有の差分が発生した場合は `reports/self-host/platform-diff-*.md` を作成し、Phase 6 のマルチターゲット計画へ共有。
- **フォールバック不備**: Stage 0 バイナリの配布や署名が失効した場合、即座に `tooling/bootstrap/download-stage0.sh` を更新し、`docs/notes/toolchain-status.md` に再取得手順を記録。

## 5.0.8 連携とハンドオーバー
- Phase 4 から受け取った `phase4-readiness.md` / `.reml` シナリオ / KPI を `phase5-self-host-checklist.md` へ転記し、差分を追跡。
- Phase 6（旧 Phase 4）へは `phase5-readiness.md`, `reports/self-host/` 一式、Self-host runner の操作手順、ブロッカーリストを引き渡す。
- `docs/plans/bootstrap-roadmap/README.md` と `SUMMARY.md` に Phase 5 の位置づけを追加し、クリティカルパスを「Phase 4 practical → Phase 5 self-host → Phase 6 release」へ更新する。
