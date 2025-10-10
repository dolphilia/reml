# 1.8 macOS プレビルド対応計画

## 目的
- Phase 1 の完了時点で x86_64 macOS 向けの最小ビルド・検証フローを確立し、macOS 開発者が Linux クロスビルドに依存せずに日常開発を行える状態を用意する。
- Linux CI（[1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)）で整備したパイプライン資産を再利用しつつ、Mach-O ランタイム差分および LLVM Toolchain の整合性を確認する。
- Phase 2 で開始予定の Windows 対応（[2-6-windows-support.md](2-6-windows-support.md)）と競合しないタイミングで macOS の課題洗い出しを前倒しし、後続フェーズでのマルチターゲット化を円滑にする。

## スコープ
- **含む**: GitHub Actions macOS ランナーのワークフロー定義、Homebrew ベースのツールチェーン準備、`dune build` / `dune runtest` の自動化、Mach-O での LLVM IR 検証、ランタイムビルド手順の整理、macOS 計測指標の追加。
- **含まない**: ARM64 ネイティブ最適化、Notarization/署名処理、GUI 向けバイナリ配布、Xcode プロジェクト生成。本計画では Intel macOS（x86_64）での CLI コンパイラ動作検証に限定する。
- **前提**: Phase 1-7 の Linux CI が運用開始済みであり、`compiler/ocaml` のビルドとテストが安定していること。macOS 開発者用の Homebrew と Xcode Command Line Tools が各自の環境に導入されていること。

## 作業ディレクトリ
- `.github/workflows/` : `bootstrap-macos.yml`（新規）を配置し、ワークフローを Linux 版と並行管理する。
- `tooling/ci/macos/` : macOS 向けのセットアップスクリプト、依存キャッシュ管理、ローカル再現スクリプトを配置。
- `runtime/native` : Mach-O ビルド規則とライブラリ出力先を管理。
- `compiler/ocaml/` : `dune` でビルドされるコンパイラ本体とテスト資産。
- `docs/notes/llvm-spec-status-survey.md`, `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` : ツールチェーンバージョンと測定値の記録先。

## 作業ブレークダウン

### 1. 計画キックオフと要件整理（18週目）
- macOS 向けビルドの期待成果物、最小検証項目、リスク項目を `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に登録。
- Linux CI との共通化ポイント（環境セットアップ、依存キャッシュ、アーティファクト構成）を一覧化し、差分タスクを抽出。
- 開発者ヒアリングで macOS 手元検証の痛点（Homebrew 版 LLVM、`brew unlink clang` 等）を収集し、作業優先度を決定。

### 2. ワークフロー設計（18-19週目）
- `.github/workflows/bootstrap-macos.yml` を作成し、`on` トリガー（push, pull_request, schedule）と `runs-on: macos-13` を設定。
- Linux CI と同じステージ構成（Lint → Build → Test → Artifact → LLVM Verify）を採用し、`needs` 依存を調整。
- `actions/cache` キーをターゲット別（`macos`）に分離し、Homebrew のキャッシュ対象（`~/Library/Caches/Homebrew/downloads`）を明示。

### 3. ツールチェーンセットアップ（19週目）
- Homebrew 経由で `llvm@18`, `opam`, `pkg-config`, `libtool` をインストールし、パス設定を `tooling/ci/macos/setup-env.sh` に記述。
- Xcode Command Line Tools のバージョンをチェックし `xcode-select --install` の要否を確認、GitHub Actions での差分を `docs/notes/llvm-spec-status-survey.md` に記録。
- `opam switch create 4.14.2` と `opam install . --deps-only --with-test` をワークフローに組み込み、インストール時間を測定して `metrics.json` に反映。

### 4. ビルドジョブ実装（19-20週目）
- `dune build` を macOS 上で実行し、ビルドログと所要時間を `actions/upload-artifact` に保存。
- `runtime/native` の `Makefile` に Mach-O ターゲット向け `CC=clang` `AR=libtool -static` などの設定を追加し、`libreml_runtime.a` を生成。
- `compiler/ocaml/src/main.exe` の出力を `remlc-ocaml-macos` として命名し、シンボル情報を保持したままアーティファクト化。

### 5. テストジョブ実装（20週目）
- `dune runtest` を実行し、ゴールデンテスト（AST/TAST/Core IR/LLVM IR）結果に macOS 固有差分がないか確認。
- `scripts/verify_llvm_ir.sh` を `--target x86_64-apple-darwin` で実行可能に拡張し、macOS 用 IR 検証パスを確立。
- テスト失敗時のログ収集を強化し、`_build/default/**/*.log` を `test-results-macos` としてアップロード。

### 6. LLVM/Mach-O 検証（20-21週目）
- `llvm-as`, `opt -verify`, `llc -mtriple=x86_64-apple-darwin` を実行し、Mach-O オブジェクト生成までを CI に組み込む。
- `clang` でリンクした最小バイナリを実行し、`DYLD_LIBRARY_PATH` の設定が不要であることを確認。
- `otool -L` でリンク先ライブラリを検証し、不要な依存が混入していないかチェックして結果を `docs/notes/llvm-spec-status-survey.md` に追記。

### 7. アーティファクトとメトリクス管理（21週目）
- Linux CI と揃えた命名規則で `compiler-macos`, `runtime-macos`, `llvm-ir/macos` をアップロードし、レビュー時の比較手順を `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` に追補。
- `0-3-audit-and-metrics.md` に macOS 用メトリクス（ビルド時間、テスト時間、IR 検証結果）を追加し、週次で更新する運用を定義。
- `tooling/ci/docker/metrics.json` に macOS セクションを追加し、CI 実行時間のトレンドを可視化。

### 8. ローカル再現とドキュメント整備（21-22週目）
- `scripts/ci-local.sh` に `--target macos` オプションを追加し、開発者が GitHub Actions と同等の手順をローカルで再現できるようにする。
- `compiler/ocaml/README.md` に macOS 手元検証ガイド（Homebrew セットアップ、`opam env` の読み込み、LLVM パス設定）を追記。
- `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` へ Mach-O 向け TODO を脚注で追加し、Phase 2 以降に検討すべき項目（Notarization、ARM64 対応）を記録。

## 成果物と検証
- GitHub Actions で `bootstrap-macos` ワークフローが push/pr/schedule の各トリガーで成功し、`dune build` と `dune runtest` が macOS 上で安定実行される。
- `remlc-ocaml-macos` と `libreml_runtime.a`（Mach-O）がアーティファクトとして 30 日保持され、レビューでダウンロード・実行できる。
- `0-3-audit-and-metrics.md` に macOS 指標が追加され、Linux 指標と比較できる状態が整う。
- `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に macOS 固有リスク（Homebrew ミラー障害、Xcode CLT の更新ズレ等）が登録され、対応方針が明示される。

## リスクとフォローアップ
- GitHub Actions macOS ランナーの起動待ち時間が長い場合、セルフホストランナー導入を検討しタスクを Phase 2 に引き継ぐ。
- Homebrew の LLVM バージョンが頻繁に更新される場合、`brew extract` によるバージョン固定または prebuilt tarball 配布を `docs/notes/llvm-spec-status-survey.md` に追記。
- Mach-O ランタイムのリンクエラーが発生した場合、Phase 2 で `runtime/native` を CMake 化する選択肢を検討し、`0-4-risk-handling.md` に改善案を追加。
- ARM64 macOS 対応は Phase 3 のクロスコンパイル計画（[3-3-core-text-unicode-plan.md](3-3-core-text-unicode-plan.md) 以降）と連動するため、成果物とテストの差分を記録して将来の拡張に耐える構成を維持する。

## 参考資料
- [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)
- [1-5-runtime-integration.md](1-5-runtime-integration.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [docs/notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [docs/guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
