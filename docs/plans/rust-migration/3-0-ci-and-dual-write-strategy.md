# 3.0 CI と dual-write 運用戦略

本書は Phase P3 において Rust 実装を CI へ本格統合し、OCaml 実装との dual-write 比較を継続的に実施するための方針とタスクを定義する。`unified-porting-principles.md` の「振る舞いの同一性を最優先する」原則と、`docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` で示された実務フローに従い、CI マトリクス・差分ハーネス・監査メトリクスを Rust 移植計画へ移管する。

## 3.0.1 目的
- GitHub Actions とローカル CI の両方で OCaml/Rust 出力を同一パイプライン上で比較し、診断・IR・監査メトリクスの差分を検出できる状態を確立する。
- Windows (`windows-latest`) を含む主要 3 プラットフォームで dual-write を常時実行し、`collect-iterator-audit-metrics.py --require-success` を Rust フロントエンドへ拡張する。
- P0/P1 で整備したゴールデン資産・診断比較フロー（[0-1-baseline-and-diff-assets.md](0-1-baseline-and-diff-assets.md)、[1-2-diagnostic-compatibility.md](1-2-diagnostic-compatibility.md)）を CI 上で自動化し、フェーズ完了時には Rust 実装を default パスへ昇格できる証跡を蓄積する。

## 3.0.2 スコープと前提
- **対象**: GitHub Actions `.github/workflows/bootstrap-*.yml`、`tooling/ci/`・`scripts/` 配下の差分ハーネス、`reports/` 配下の CI 成果物。Rust 側 CLI/ランタイムが dual-write 実行可能であることを前提とする。
- **除外**: Rust バックエンド固有の最適化（`2-0-llvm-backend-plan.md`）、ランタイム FFI 連携（`2-1-runtime-integration.md`）、監査ダッシュボードの最終整備（`3-1-observability-alignment.md`）。
- **前提条件**:
  - P0/P1 の成果物（ゴールデン比較・診断互換性計画・Windows ツールチェーン監査）が最新である。
  - `remlc --frontend {ocaml,rust}` による dual-write 比較 CLI が利用可能で、`collect-iterator-audit-metrics.py` の JSON スキーマ (2.0.0-draft) に沿って出力される。
  - 現行 OCaml CI が安定運用されており、Rust ジョブを追加しても `bootstrap-linux.yml` 等の SLA を満たせる。

## 3.0.3 成果物
| 成果物 | 内容 | 参照資料 |
| --- | --- | --- |
| Dual-write CI 設定案 | `bootstrap-{linux,macos,windows}.yml` に Rust Frontend ジョブを追加し、OCaml / Rust / Dual の 3 系統を matrix で管理 | `.github/workflows/bootstrap-*.yml`, `0-0-roadmap.md` |
| 差分ハーネス統合手順 | `scripts/run-dual-write.sh`（新規）で diagnostic / IR / audit の三系統を比較し、CI から `reports/dual-write/` に成果物を保存 | `0-1-baseline-and-diff-assets.md`, `1-2-diagnostic-compatibility.md` |
| CI チェックリスト | dual-write での失敗分類・エスカレーションのガイド (`reports/dual-write/triage-template.md`) | `docs/spec/3-6-core-diagnostics-audit.md`, `reports/diagnostic-format-regression.md` |
| マイルストーン表 | M1 (Linux dual-write)、M2 (macOS / Windows 追加入)、M3 (Rust default 化レビュー) のチェックリスト | `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md` |

## 3.0.4 CI マトリクス更新計画
現行ワークフローを次のように拡張し、全プラットフォームで dual-write を常設する。`frontend` パラメータにより OCaml/Rust/dual を切り替え、`strategy.fail-fast: false` で並列実行時の中断を防ぐ。

| ワークフロー | 新規 matrix パラメータ | 追加ジョブ | 成果物 |
| --- | --- | --- | --- |
| `bootstrap-linux.yml` | `frontend: ["ocaml", "rust", "dual"]` | `dual-diff`（OCaml → Rust 比較、`scripts/run-dual-write.sh`） | `reports/dual-write/linux/*.diff`, `tooling/ci/iterator-audit-metrics.json` |
| `bootstrap-macos.yml` | `frontend: ["ocaml", "rust"]` + `dual` ジョブを個別追加（macOS runner のコスト最適化） | `dual-audit`（監査メトリクス差分） | `reports/audit/macOS/*.json`, `reports/dual-write/macos/*.md` |
| `bootstrap-windows.yml` | `frontend: ["ocaml", "rust", "dual"]` を `diagnostic-json` と `audit` に導入 | `dual-streaming`（`--section streaming` 比較）、`dual-env`（環境診断 diff） | `reports/iterator-stage-summary-windows.md`, `reports/audit/streaming-windows-rust.json` |

### 実装手順
1. `strategy.matrix.frontend` を追加し、OCaml ジョブを既存デフォルトとする。Rust ジョブは `cargo` キャッシュと `rustup component add llvm-tools-preview` をセットアップ。
2. Dual-write ジョブでは `remlc --frontend ocaml` / `remlc --frontend rust` を順に実行し、`jq --sort-keys` を通した後に `diff -u` を取得。差分は `reports/dual-write/<platform>/diagnostics-<case>.diff` に保存する。
3. `collect-iterator-audit-metrics.py --baseline path/to/ocaml.json --candidate path/to/rust.json --require-success` を dual-write ジョブに追加し、pass_rate < 1.0 の場合は CI を失敗とする。
4. `.github/workflows/bootstrap-*.yml` の `Upload ...` ステップを拡張し、Rust / dual-write の成果物を `iterator-audit-metrics-rust.json`、`dual-write-diff.tar.gz` としてアップロード。
5. `tooling/ci/record-metrics.sh` を改修し、`--frontend` 引数を受け取って `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` のテーブルに「rust」「dual」列を追記する（P3 完了時に記録）。

## 3.0.5 Dual-write 実行フロー
1. **入力準備**: `compiler/tests/**` のゴールデンを `git ls-files` で列挙し、差分対象リストを `tmp/dual-write-targets.txt` に生成。P0 で定義したフィルタ（`*.json.golden`, `*.ir`, `*.log`）を再利用する。
2. **OCaml 実行**: `remlc --frontend ocaml --emit {diagnostics,llvm,ir}` を実行し、出力を `tmp/ocaml/<case>.json` などへ保存。
3. **Rust 実行**: `remlc --frontend rust` で同一オプションを実行。エラー終了時は即座に CI を失敗扱いとし、`reports/dual-write/<platform>/failure-<timestamp>.md` にスタックトレースを記録。
4. **差分比較**: `scripts/run-dual-write.sh` で JSON / IR / CLI を比較。許容差分（フィールド順序等）は `1-2-diagnostic-compatibility.md` 記載の正規化ルールで整形し、実装差分は `reports/dual-write/triage-template.md` へ分類。
5. **監査メトリクス**: `collect-iterator-audit-metrics.py` を baseline/candidate モードで実行し、`iterator.stage.audit_pass_rate`、`diagnostics.effect_stage_consistency` 等の pass_rate を算出。閾値は [0-3-audit-and-metrics.md](../bootstrap-roadmap/0-3-audit-and-metrics.md) の値を継承する。
6. **結果アップロード**: 差分とメトリクスを Artifact 化し、`reports/audit/index.json` に新規ログのメタデータ (`ci:linux:dual-write` 等) を追記。更新は `tooling/ci/create-audit-index.py` による。

## 3.0.6 エスカレーションとレビュー体制
- **失敗分類**:
  - `CI_FAILURE_TYPE=rust-runtime`：Rust 実装のクラッシュ/パニック。`compiler/rust/` のオーナーへ即通知。
  - `CI_FAILURE_TYPE=diagnostic-diff`：JSON/LSP 差分。`docs/plans/rust-migration/1-2-diagnostic-compatibility.md` の手順に従い再現ログを取得。
  - `CI_FAILURE_TYPE=audit-gap`：監査メトリクス不足。`docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` の KPI を参照し、欠落キーを特定。
- **レビュー頻度**: 週次で `dual-write` 成果物を確認し、`reports/dual-write/status-<week>.md` にサマリを残す。マイルストーン到達時は `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` の関連リスクをクローズ。
- **連携先**: 監査整合は `3-1-observability-alignment.md`、ベンチマーク整合は `3-2-benchmark-baseline.md` に引き渡す。

## 3.0.7 スケジュール（目安）
| マイルストーン | 期間 | 完了条件 |
| --- | --- | --- |
| M1: Linux dual-write 可視化 | Sprint 1 | Linux dual-write ジョブが安定して成功し、`iterator.stage.audit_pass_rate = 1.0` を維持 |
| M2: macOS / Windows 拡張 | Sprint 2 | 各プラットフォームで dual-write 成果物が `reports/dual-write/<platform>/` に出力され、監査メトリクスが 100% 揃う |
| M3: Rust default 評価 | Sprint 3 | dual-write の差分が 4 週間連続でゼロ。Rust frontend を `bootstrap-*` のデフォルトに昇格するレビューを実施 |

## 3.0.8 リスクとフォールバック
- **CI 実行時間の増大**: dual-write によりジョブ時間が 1.8〜2.2 倍となる見込み。`strategy.max-parallel` とキャッシュ (`cargo`, `opam`, LLVM) を調整し、30 分以内を維持。閾値超過時は Rust ジョブを nightly に分離して検証を継続。
- **監査メトリクス欠落**: Rust 側で `AuditEnvelope.metadata` を生成する前に dual-write を有効化すると失敗が増加する。P2 での `collect-iterator-audit-metrics.py` 対応を完了してから P3 に着手。
- **Windows ツールチェーン差異**: `check-windows-bootstrap-env.ps1` の結果が一致しない場合は `0-2-windows-toolchain-audit.md` の fallback 手順（MSYS2 LLVM 16 継続）を適用し、Rust ジョブを optional に切り替える。
- **差分の長期化**: Rust 実装の差分が多い状態で dual-write を常時稼働させるとノイズが増える。M1 達成前に既知差分を `reports/dual-write/allowlist.txt` へ登録し、毎週見直す。

## 3.0.9 Rust ジョブ追加時に再利用する CI 設定
既存の OCaml 向けワークフローで確立した環境変数・キャッシュ・アーティファクト仕様は、Rust フロントエンドのジョブでも整合性維持のため再利用する。以下にプラットフォーム別の主要設定を整理し、Rust ジョブ追加時のチェックリストとする。

### Linux (`.github/workflows/bootstrap-linux.yml`)
- **環境変数**: `build` ジョブの `LLVM_CONFIG=/usr/bin/llvm-config-19` に合わせ、Rust 版も LLVM 19 系を固定する。`audit-matrix` ジョブでは `AUDIT_TARGET` / `AUDIT_PLATFORM` / `AUDIT_SAMPLE` / `PYTHONUTF8` を設定し、dual-write ジョブから出力する監査メトリクスのキーを揃える。
- **キャッシュ**: `actions/cache@v4` で `/usr/lib/llvm-18` を共有し、Rust 側も同キー（`llvm-18-${{ runner.os }}`）を利用する。`ocaml/setup-ocaml` の `dune-cache: true` で有効化したディレクトリを Rust ジョブでも `dune cache` 互換の作業領域として扱う。
- **アーティファクト**: `linux-build`（OCaml バイナリ）、`audit-ci-linux`、`dual-write-front-end`、`test-results-junit`、`test-output-log` を既存命名のまま保持し、Rust フロントエンド成果物は新規名前空間（例: `remlc-rust-linux`）を追加して差異を明示する。dual-write スモーク (`tooling/ci/run-dual-write-smoke.sh`) が書き出す `reports/dual-write/front-end/` 配下の構造は Rust 版でも共有する。

### macOS (`.github/workflows/bootstrap-macos.yml`)
- **環境変数**: Homebrew 由来の `LDFLAGS` / `CPPFLAGS` / `CMAKE_PREFIX_PATH` を `build`・`audit-matrix` 両ジョブで `GITHUB_ENV` に書き込んでいる。Rust ジョブも LLVM 19（`brew install llvm@19`）を同一プレフィックスで参照し、リンク時のフラグを共有する。
- **キャッシュ**: Homebrew ダウンロードキャッシュ (`~/Library/Caches/Homebrew/downloads`) と `/opt/homebrew/opt/llvm@19` を共有。Rust ジョブは同じ `key` / `restore-keys`（`homebrew-${{ runner.os }}-arm64-*`, `llvm-19-macos-arm64-*`）を流用し、LLVM ビルド時間を抑制する。
- **アーティファクト**: `macos-build`、`audit-ci-macos` を既存命名で維持し、Rust 版成果物は `artifacts/build` 内に OCaml 版と並べて格納する。監査ジョブの出力先 (`reports/audit/macos/<run-id>/`) を Rust でも共通化し、`collect-iterator-audit-metrics.py --append-from` によるマージ処理をそのまま適用できるようにする。

### Windows (`.github/workflows/bootstrap-windows.yml`)
- **環境変数**: `diagnostic-json` / `audit` 両ジョブで `CI_WINDOWS_ENV_JSON_NAME` と `PYTHONUTF8` を利用し、PowerShell スクリプトが `CI_WINDOWS_ENV_JSON_PATH` を `GITHUB_ENV` へ書き込む運用になっている。Rust ジョブでも同じ環境変数名を継承し、ツールチェーン診断 JSON を単一パスで扱う。
- **キャッシュ**: Windows Runner では LLVM をローカルにインストールせず、Ubuntu 上の `audit-matrix` ジョブで `/usr/lib/llvm-18` をキャッシュしている。Rust 版のクロスビルドも同キー (`llvm-18-${{ runner.os }}`) を使い、OCaml 版とキャッシュを共有する。
- **アーティファクト**: `windows-env-check-diagnostic` / `windows-env-check-audit`（ツールチェーン診断）、`windows-iterator-audit-summary`、`windows-iterator-audit-metrics` を既存のまま公開し、Rust 版の追加成果物はサフィックス（例: `*-rust`）で区別する。`reports/iterator-stage-summary-windows.md` と `tooling/ci/iterator-audit-metrics.json` のフォーマットは共通であるため、Rust ジョブでは出力を追記する形で管理する。

---

本計画は P3 完了時に Rust CI をデフォルトへ昇格させるための前提条件を整理したものである。dual-write 成果物の取扱いと監査メトリクスの詳細は [3-1-observability-alignment.md](3-1-observability-alignment.md) へ引き渡す。
