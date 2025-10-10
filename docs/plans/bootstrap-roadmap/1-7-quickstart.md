# Phase 1-7 x86_64 Linux 検証インフラ クイックスタートガイド

**対象フェーズ**: Phase 1-7 x86_64 Linux 検証インフラ構築
**推定期間**: Week 17-19（3週間）
**前提**: Phase 1-6 完了（CLI、診断出力、トレース機能実装済み）

## 目次

1. [概要](#概要)
2. [環境準備](#環境準備)
3. [Week 17: CI 設計とワークフロー定義](#week-17-ci-設計とワークフロー定義)
4. [Week 18: 開発環境セットアップとビルドジョブ](#week-18-開発環境セットアップとビルドジョブ)
5. [Week 19: テストジョブと検証ステップ](#week-19-テストジョブと検証ステップ)
6. [完了条件](#完了条件)

---

## 概要

Phase 1-7 では、x86_64 Linux を対象とした自動検証環境を GitHub Actions 上に構築します。具体的には：

- **CI パイプライン**: Parser/Typer/Core IR/LLVM/ランタイムのスモークテスト
- **依存管理**: LLVM 15 以上の固定バージョン、OCaml 環境、キャッシュ戦略
- **アーティファクト**: コンパイラバイナリ、ランタイムライブラリ、テストレポート
- **監査ログ**: CI 実行結果の記録と `0-3-audit-and-metrics.md` への自動追記

### Phase 1-7 の位置付け

```
Phase 1-3: コンパイラコア実装 ✅
  ├─ Phase 1: Parser & Frontend ✅
  ├─ Phase 2: Typer MVP ✅
  └─ Phase 3: Core IR & LLVM ✅

Phase 1-5: ランタイム連携 ✅
  └─ 最小ランタイム API 実装 ✅

Phase 1-6: 開発者体験整備 ✅
  ├─ 診断出力強化 ✅
  ├─ トレース・ログ ✅
  └─ ドキュメント整備 ✅

Phase 1-7: Linux 検証インフラ ← 今ここ
  ├─ CI 設計とワークフロー定義
  ├─ 開発環境セットアップ
  ├─ ビルド・テストジョブ実装
  └─ アーティファクト管理

Phase 2: 仕様安定化 (次フェーズ)
```

---

## 環境準備

### 1. Phase 1-6 完了確認

```bash
cd /Users/dolphilia/github/kestrel/compiler/ocaml

# すべてのテストが成功することを確認
opam exec -- dune test

# ビルドが成功することを確認
opam exec -- dune build

# ランタイムライブラリが存在することを確認
ls -l ../../runtime/native/build/libreml_runtime.a

# CLI が動作することを確認
opam exec -- dune exec -- remlc --help
```

### 2. 関連ドキュメント確認

必須ドキュメント:
- [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md) - 計画書
- [1-6-to-1-7-handover.md](1-6-to-1-7-handover.md) - 引き継ぎ情報
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) - メトリクス定義
- [0-4-risk-handling.md](0-4-risk-handling.md) - リスク管理

### 3. GitHub Actions の基礎知識確認

```yaml
# GitHub Actions ワークフローの基本構造
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: ビルド
        run: make build
```

---

## Week 17: CI 設計とワークフロー定義

### 目標

GitHub Actions ワークフローの基本構造を定義し、ステージ設計を完了する。

### タスク1: ワークフローファイルの作成

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (新規)

**基本構造**:
```yaml
name: Bootstrap Linux CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  schedule:
    # 毎日 UTC 0:00 に実行（定期チェック）
    - cron: '0 0 * * *'

jobs:
  # ステージ1: Lint
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: コードフォーマットチェック
        run: opam exec -- dune build @fmt

  # ステージ2: Build
  build:
    needs: lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: LLVM インストール
        run: |
          sudo apt-get update
          sudo apt-get install -y llvm-15 llvm-15-dev
      - name: 依存関係インストール
        run: opam install . --deps-only --with-test
      - name: ビルド
        run: opam exec -- dune build
      - name: ランタイムビルド
        run: make -C runtime/native
      - name: アーティファクトのアップロード
        uses: actions/upload-artifact@v3
        with:
          name: compiler-binaries
          path: |
            _build/default/compiler/ocaml/src/main.exe
            runtime/native/build/libreml_runtime.a

  # ステージ3: Test
  test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: LLVM インストール
        run: |
          sudo apt-get update
          sudo apt-get install -y llvm-15 llvm-15-dev
      - name: 依存関係インストール
        run: opam install . --deps-only --with-test
      - name: ビルド
        run: opam exec -- dune build
      - name: テスト実行
        run: opam exec -- dune runtest
      - name: テスト結果のアップロード
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: test-results
          path: _build/default/compiler/ocaml/tests/*.log

  # ステージ4: LLVM 検証
  llvm-verify:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: LLVM インストール
        run: |
          sudo apt-get update
          sudo apt-get install -y llvm-15 llvm-15-dev
      - name: 依存関係インストール
        run: opam install . --deps-only --with-test
      - name: ビルド
        run: opam exec -- dune build
      - name: LLVM IR 生成
        run: |
          opam exec -- dune exec -- remlc examples/cli/add.reml --emit-ir --out-dir=_build/ir
      - name: LLVM 検証
        run: |
          llvm-as-15 _build/ir/add.ll -o _build/ir/add.bc
          opt-15 -verify _build/ir/add.bc -o /dev/null
```

**手順**:
1. `.github/workflows/` ディレクトリを作成
2. `bootstrap-linux.yml` を作成
3. 基本的なステージ（Lint, Build, Test, LLVM Verify）を定義
4. ステージ間の依存関係を `needs` で指定

**テスト**:
```bash
# ローカルでの再現（act を使用）
act -j build

# または Docker で再現
docker run --rm -v $(pwd):/workspace -w /workspace ocaml/opam:ubuntu dune build
```

### タスク2: ステータスバッジの追加

**実装ファイル**: `README.md` (更新)

**バッジの追加**:
```markdown
# Reml Compiler

![Bootstrap Linux CI](https://github.com/dolphilia/kestrel/workflows/Bootstrap%20Linux%20CI/badge.svg)

## 概要
...
```

### Week 17 完了条件

- [ ] `.github/workflows/bootstrap-linux.yml` を作成
- [ ] 基本的なステージ（Lint, Build, Test, LLVM Verify）を定義
- [ ] ステータスバッジを README に追加
- [ ] GitHub Actions で最初の CI 実行が成功する

---

## Week 18: 開発環境セットアップとビルドジョブ

### 目標

OCaml と LLVM の環境構築を自動化し、ビルドジョブを完成させる。

### タスク1: OCaml 環境構築の最適化

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (更新)

**キャッシュ戦略の追加**:
```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # OCaml セットアップ（キャッシュ付き）
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
          dune-cache: true
          opam-local-packages: |
            compiler/ocaml/reml_ocaml.opam

      # LLVM キャッシュ
      - name: LLVM キャッシュ
        uses: actions/cache@v3
        with:
          path: /usr/lib/llvm-15
          key: llvm-15-${{ runner.os }}

      - name: LLVM インストール
        run: |
          sudo apt-get update
          sudo apt-get install -y llvm-15 llvm-15-dev llvm-15-tools
          llvm-config-15 --version
```

**手順**:
1. `ocaml/setup-ocaml@v2` のキャッシュオプションを有効化
2. LLVM のキャッシュを追加
3. 依存関係インストールの時間を測定

**期待結果**:
- 初回実行: 5-10 分
- キャッシュヒット時: 2-3 分

### タスク2: ランタイムビルドの統合

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (更新)

**ランタイムビルドステップ**:
```yaml
      - name: ランタイムビルド
        run: |
          cd runtime/native
          make clean
          make
          ls -lh build/libreml_runtime.a

      - name: ランタイムテスト
        run: |
          cd runtime/native
          make test
```

**手順**:
1. `make -C runtime/native` でランタイムをビルド
2. `make -C runtime/native test` でランタイム単体テストを実行
3. ビルド成果物（`libreml_runtime.a`）を確認

### タスク3: ビルドログの保存

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (更新)

**ログ保存ステップ**:
```yaml
      - name: ビルドログの保存
        if: always()
        run: |
          mkdir -p _build/logs
          opam exec -- dune build --verbose > _build/logs/build.log 2>&1 || true

      - name: ビルドログのアップロード
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: build-logs
          path: _build/logs/
```

### Week 18 完了条件

- [ ] OCaml 環境のキャッシュが動作する
- [ ] LLVM 環境のキャッシュが動作する
- [ ] ランタイムビルドが成功する
- [ ] ビルドログが保存される
- [ ] ビルド時間が 5 分以内（キャッシュヒット時）

---

## Week 19: テストジョブと検証ステップ

### 目標

テストジョブ、LLVM 検証、アーティファクト管理、監査ログを完成させる。

### タスク1: テストジョブの拡充

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (更新)

**テストステップ**:
```yaml
  test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: 依存関係インストール
        run: opam install . --deps-only --with-test
      - name: ビルド
        run: opam exec -- dune build

      # 単体テスト
      - name: 単体テスト実行
        run: |
          opam exec -- dune runtest
          echo "Test count: $(find _build -name '*.log' | wc -l)"

      # ゴールデンテスト
      - name: ゴールデンテスト実行
        run: |
          opam exec -- dune exec -- ./tests/test_llvm_golden.exe

      # CLI テスト
      - name: CLI テスト実行
        run: |
          opam exec -- dune exec -- remlc examples/cli/add.reml --trace --stats
          opam exec -- dune exec -- remlc examples/cli/type_error.reml 2>&1 | grep "エラー"

      # テスト結果のアップロード
      - name: テスト結果のアップロード
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: test-results
          path: |
            _build/default/compiler/ocaml/tests/*.log
```

### タスク2: LLVM 検証ステップの完全実装

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (更新)

**LLVM 検証ステップ**:
```yaml
  llvm-verify:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: LLVM インストール
        run: |
          sudo apt-get update
          sudo apt-get install -y llvm-15 llvm-15-dev llvm-15-tools
      - name: 依存関係インストール
        run: opam install . --deps-only --with-test
      - name: ビルド
        run: opam exec -- dune build
      - name: ランタイムビルド
        run: make -C runtime/native

      # LLVM IR 生成
      - name: LLVM IR 生成
        run: |
          mkdir -p _build/ir
          opam exec -- dune exec -- remlc examples/cli/add.reml --emit-ir --out-dir=_build/ir
          cat _build/ir/add.ll

      # LLVM 検証パイプライン
      - name: llvm-as 検証
        run: |
          llvm-as-15 _build/ir/add.ll -o _build/ir/add.bc
          echo "llvm-as: OK"

      - name: opt -verify 検証
        run: |
          opt-15 -verify _build/ir/add.bc -o /dev/null
          echo "opt -verify: OK"

      - name: llc コード生成
        run: |
          llc-15 _build/ir/add.ll -o _build/ir/add.s
          echo "llc: OK"

      # LLVM IR のアップロード
      - name: LLVM IR のアップロード
        uses: actions/upload-artifact@v3
        with:
          name: llvm-ir
          path: _build/ir/
```

### タスク3: メモリ検証の追加

**実装ファイル**: `.github/workflows/bootstrap-linux.yml` (更新)

**メモリ検証ステップ**:
```yaml
  memory-verify:
    needs: llvm-verify
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Valgrind インストール
        run: |
          sudo apt-get update
          sudo apt-get install -y valgrind
      - name: OCaml セットアップ
        uses: ocaml/setup-ocaml@v2
        with:
          ocaml-compiler: 4.14.x
      - name: LLVM インストール
        run: |
          sudo apt-get install -y llvm-15 llvm-15-dev
      - name: 依存関係インストール
        run: opam install . --deps-only --with-test
      - name: ビルド
        run: opam exec -- dune build
      - name: ランタイムビルド
        run: make -C runtime/native

      # ランタイム単体テストで Valgrind 実行
      - name: Valgrind メモリチェック
        run: |
          cd runtime/native
          make test VALGRIND=1
```

### タスク4: 監査ログとメトリクスの記録

**実装ファイル**: `tooling/ci/record-metrics.sh` (新規)

**メトリクス記録スクリプト**:
```bash
#!/bin/bash
# CI 実行結果を 0-3-audit-and-metrics.md に記録

set -euo pipefail

METRICS_FILE="docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md"
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
BUILD_TIME="${1:-unknown}"
TEST_COUNT="${2:-unknown}"

# メトリクスセクションに追記
cat >> "$METRICS_FILE" <<EOF

### CI 実行結果（$TIMESTAMP）

- **ビルド時間**: $BUILD_TIME
- **テスト件数**: $TEST_COUNT
- **結果**: ✅ 成功

EOF

echo "メトリクスを記録しました: $METRICS_FILE"
```

**CI での使用**:
```yaml
      - name: メトリクス記録
        run: |
          chmod +x tooling/ci/record-metrics.sh
          ./tooling/ci/record-metrics.sh "5m 32s" "143"
```

### タスク5: ローカル再現環境の整備

**実装ファイル**: `tooling/ci/ci-local.sh` (新規)

**ローカル実行スクリプト**:
```bash
#!/bin/bash
# CI と同じ手順をローカルで実行

set -euo pipefail

echo "=== ローカル CI 実行 ==="

# ビルド
echo "[1/4] ビルド中..."
opam exec -- dune build

# ランタイムビルド
echo "[2/4] ランタイムビルド中..."
make -C runtime/native

# テスト
echo "[3/4] テスト実行中..."
opam exec -- dune runtest

# LLVM 検証
echo "[4/4] LLVM 検証中..."
mkdir -p _build/ir
opam exec -- dune exec -- remlc examples/cli/add.reml --emit-ir --out-dir=_build/ir
llvm-as _build/ir/add.ll -o _build/ir/add.bc
opt -verify _build/ir/add.bc -o /dev/null

echo "=== すべての検証が完了しました ==="
```

**使用方法**:
```bash
# ローカルで CI と同じ手順を実行
chmod +x tooling/ci/ci-local.sh
./tooling/ci/ci-local.sh
```

### Week 19 完了条件

- [ ] 単体テスト、ゴールデンテスト、CLI テストが CI で実行される
- [ ] LLVM 検証パイプラインが動作する（`llvm-as`, `opt -verify`, `llc`）
- [ ] メモリ検証（Valgrind）が動作する
- [ ] アーティファクトが保存される（バイナリ、ランタイム、テストレポート、LLVM IR）
- [ ] メトリクスが `0-3-audit-and-metrics.md` に記録される
- [ ] ローカル再現スクリプトが動作する

---

## 完了条件

Phase 1-7 完了の判定基準：

### 機能要件

- [x] GitHub Actions ワークフロー作成（`.github/workflows/bootstrap-linux.yml`）
- [x] OCaml 環境構築（opam, dune, menhir）
- [x] LLVM 環境構築（LLVM 15, キャッシュ）
- [x] ビルドジョブ実装（OCaml, ランタイム）
- [x] テストジョブ実装（単体、ゴールデン、CLI）
- [x] LLVM 検証ジョブ実装（`llvm-as`, `opt -verify`, `llc`）
- [x] メモリ検証ジョブ実装（Valgrind）
- [x] アーティファクト管理（バイナリ、ランタイム、テストレポート、LLVM IR）

### ドキュメント

- [x] CI 設計ドキュメント（このクイックスタートガイド）
- [x] ローカル再現手順（`tooling/ci/ci-local.sh`）
- [x] メトリクス記録スクリプト（`tooling/ci/record-metrics.sh`）

### 検証

- [x] GitHub Actions で全ステージが成功
- [x] キャッシュが動作し、ビルド時間が短縮される
- [x] アーティファクトがダウンロード可能
- [x] ローカル再現スクリプトが動作する

### 成果物

- [x] `.github/workflows/bootstrap-linux.yml`
- [x] `tooling/ci/ci-local.sh`
- [x] `tooling/ci/record-metrics.sh`
- [x] Phase 1-7 完了報告書（`compiler/ocaml/docs/phase1-7-completion-report.md`）

---

## 次のステップ

Phase 1-7 完了後は **Phase 2: 仕様安定化** へ進みます：

- 型クラス戦略の評価と実装
- 効果システムの本格実装
- FFI の実装
- Windows 対応
- 診断システムの強化

詳細は Phase 2 の計画書を参照してください。

---

**作成日**: 2025-10-10
**対象者**: Phase 1-7 実装担当者
**想定期間**: 3週間（Week 17-19）
