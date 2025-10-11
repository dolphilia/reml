# Phase 2 開始前チェックリスト

**作成日**: 2025-10-11
**Phase 1 完了日**: 2025-10-11
**Phase 2 開始予定**: 2025-10-12 以降

このチェックリストは、Phase 2 を円滑に開始するための事前確認項目をまとめたものです。すべての項目を確認してから Phase 2 作業を開始してください。

---

## 📋 必須確認項目

### 1. Phase 1 完了状態の確認

- [ ] **Phase 1-1 (Parser & Frontend)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase1-completion-report.md`
  - 確認内容: パーサー実装、AST定義、エラー回復機能

- [ ] **Phase 1-2 (Typer MVP)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase2-completion-report.md`
  - 確認内容: Hindley-Milner型推論、型制約生成、let多相

- [ ] **Phase 1-3 (Core IR & LLVM)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase3-m3-completion-report.md`
  - 確認内容: 脱糖変換、CFG構築、LLVM IR生成

- [ ] **Phase 1-5 (ランタイム連携)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase1-5-completion-report.md`
  - 確認内容: 最小ランタイムAPI、参照カウント、メモリ安全性

- [ ] **Phase 1-6 (開発者体験整備)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase1-6-completion-report.md`
  - 確認内容: CLI実装、診断システム、トレース・統計機能

- [ ] **Phase 1-7 (Linux 検証インフラ)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase1-7-completion-report.md`
  - 確認内容: GitHub Actions Linux CI、ローカルCI再現スクリプト

- [ ] **Phase 1-8 (macOS プレビルド対応)** の完了報告書を確認済み
  - 場所: `compiler/ocaml/docs/phase1-8-completion-report.md`
  - 確認内容: macOS ARM64対応、GitHub Actions macOS CI

- [ ] **Phase 1 → Phase 2 引き継ぎドキュメント** を確認済み
  - 場所: `compiler/ocaml/docs/phase1-to-phase2-handover.md`
  - 確認内容: 成果物サマリー、技術的負債、Phase 2 の推奨作業順序

---

### 2. 技術的負債の把握

- [ ] **技術的負債リスト** を確認済み
  - 場所: `compiler/ocaml/docs/technical-debt.md`
  - 確認内容: High/Medium/Low優先度の負債項目、対応計画

- [ ] **High Priority 技術的負債** (4件) を理解済み
  - H1: 型マッピングのTODO解消 (`type_mapping.ml`)
  - H2: Windows x64 ABI検証 (`abi.ml`)
  - H3: ゴールデンテストの拡充 (`tests/golden/`)
  - H4: CFG線形化の完成 (`cfg.ml`)

- [ ] **Medium Priority 技術的負債** (8件) を理解済み
  - M1: 配列リテラル型推論
  - M2: Unicode XID完全対応
  - ID 11: 統計機能の拡張
  - ID 12: CLI統合テストの完全な網羅
  - ID 14: CI実行時間の最適化
  - ID 15: メトリクス自動記録の精度向上
  - ID 18: Homebrew LLVMバージョン変動リスク
  - ID 19: GitHub Actions macOSランナーコスト

---

### 3. 開発環境の確認

#### 3.1 リポジトリの状態

- [ ] 最新の `main` ブランチを pull 済み
  ```bash
  git checkout main
  git pull origin main
  ```

- [ ] 作業ブランチがクリーンな状態
  ```bash
  git status  # "nothing to commit, working tree clean"
  ```

#### 3.2 OCaml 環境

- [ ] OCaml 5.2.1 がインストール済み
  ```bash
  ocaml -version  # "5.2.1" を確認
  ```

- [ ] opam 環境が正しく設定されている
  ```bash
  eval $(opam env)
  opam switch show  # "5.2.1" を確認
  ```

- [ ] 依存関係がすべてインストール済み
  ```bash
  cd compiler/ocaml
  opam install . --deps-only --with-test --yes
  ```

#### 3.3 LLVM 環境

- [ ] LLVM 18 がインストール済み
  ```bash
  llvm-as --version  # "18.x.x" を確認
  opt --version      # "18.x.x" を確認
  llc --version      # "18.x.x" を確認
  ```

- [ ] (macOS のみ) LLVM パスが正しく設定されている
  ```bash
  which llvm-as  # Homebrew の LLVM パスを確認
  # /usr/local/opt/llvm@18/bin/llvm-as (Intel Mac)
  # /opt/homebrew/opt/llvm@18/bin/llvm-as (Apple Silicon)
  ```

#### 3.4 ビルドツール

- [ ] Dune 3.0+ がインストール済み
  ```bash
  dune --version  # "3.x.x" を確認
  ```

- [ ] (macOS のみ) Xcode Command Line Tools がインストール済み
  ```bash
  xcode-select -p  # パスが表示されることを確認
  clang --version  # バージョンが表示されることを確認
  ```

---

### 4. ビルドとテストの動作確認

#### 4.1 コンパイラのビルド

- [ ] コンパイラがビルド成功する
  ```bash
  cd compiler/ocaml
  opam exec -- dune build
  ```

- [ ] コンパイラバイナリが生成される
  ```bash
  ls -la _build/default/src/main.exe
  ```

#### 4.2 テストの実行

- [ ] 全テストが成功する
  ```bash
  opam exec -- dune runtest
  # "All tests passed" を確認
  ```

- [ ] テスト統計を確認
  - Lexer: 15件
  - Parser: 45件
  - Type Inference: 30件
  - Type Errors: 30件
  - Core IR: 42件
  - LLVM Codegen: 20件
  - **合計: 182件** (すべて成功)

#### 4.3 ランタイムのビルドとテスト

- [ ] ランタイムがビルド成功する
  ```bash
  cd runtime/native
  make runtime
  ```

- [ ] ランタイムテストが成功する
  ```bash
  make test
  # "All 8 tests passed!" を確認
  ```

- [ ] AddressSanitizer チェックが成功する
  ```bash
  make clean
  DEBUG=1 make runtime
  DEBUG=1 make test
  # "leaked: 0" を確認
  ```

#### 4.4 LLVM IR 検証

- [ ] LLVM IR 生成と検証が成功する
  ```bash
  cd compiler/ocaml
  opam exec -- dune exec -- remlc examples/cli/add.reml --emit-ir --out-dir=/tmp/test-ir
  chmod +x scripts/verify_llvm_ir.sh
  scripts/verify_llvm_ir.sh /tmp/test-ir/add.ll
  # エラーなく完了することを確認
  ```

---

### 5. CI/CD の動作確認

#### 5.1 ローカル CI スクリプト

- [ ] (Linux) ローカル CI が成功する
  ```bash
  ./scripts/ci-local.sh --target linux
  # "すべての CI ステップが完了しました ✓" を確認
  ```

- [ ] (macOS) ローカル CI が成功する
  ```bash
  ./scripts/ci-local.sh --target macos
  # "すべての CI ステップが完了しました ✓" を確認
  ```

- [ ] (macOS Apple Silicon) ARM64 ターゲットが自動選択される
  ```bash
  ./scripts/ci-local.sh --target macos | grep "ターゲットアーキテクチャ"
  # "arm64" が表示されることを確認
  ```

#### 5.2 GitHub Actions

- [ ] Linux CI ワークフローを確認
  - URL: `.github/workflows/bootstrap-linux.yml`
  - 最終実行: 成功していることを確認

- [ ] macOS CI ワークフローを確認
  - URL: `.github/workflows/bootstrap-macos.yml`
  - 設定: `macos-14` (ARM64) であることを確認

---

### 6. ドキュメントの確認

#### 6.1 計画書

- [ ] **Phase 2 計画書** を確認（存在する場合）
  - 場所: `docs/plans/bootstrap-roadmap/2-*.md`
  - 確認内容: Phase 2 の目標、スコープ、作業ブレークダウン

- [ ] **メトリクス定義** を確認
  - 場所: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`
  - 確認内容: Phase 1 の実測値、Phase 2 の目標値

- [ ] **リスク管理** を確認
  - 場所: `docs/plans/bootstrap-roadmap/0-4-risk-handling.md`
  - 確認内容: Phase 1 から引き継ぐリスク項目

#### 6.2 仕様書

- [ ] **型システム仕様** を確認
  - 場所: `docs/spec/1-2-types-Inference.md`
  - Phase 2 で実装予定の型機能を確認

- [ ] **効果システム仕様** を確認
  - 場所: `docs/spec/1-3-effects-safety.md`
  - Effect 型の LLVM マッピングを確認

- [ ] **Capability システム仕様** を確認
  - 場所: `docs/spec/3-8-core-runtime-capability.md`
  - Capability 型の実装方針を確認

---

### 7. Phase 2 作業の準備

#### 7.1 作業環境

- [ ] Phase 2 用の作業ブランチを作成済み（または作成予定）
  ```bash
  git checkout -b phase2-dev
  ```

- [ ] エディタ/IDE が正しく設定されている
  - OCaml 拡張機能がインストール済み
  - Dune 統合が動作している
  - LLVM 構文ハイライトが有効

#### 7.2 優先タスクの確認

- [ ] **Week 17-20 の High Priority タスク** を把握
  - H1: 型マッピングの TODO 解消
  - H2: Windows x64 ABI 検証
  - H3: ゴールデンテストの拡充
  - H4: CFG 線形化の完成

- [ ] **最初に取り組むタスク** を決定
  - 推奨: H1 (型マッピング) または H3 (ゴールデンテスト)
  - 理由: 他のタスクの基盤となる重要項目

---

## ✅ チェックリスト完了確認

すべての項目にチェックを入れたら、以下を確認してください：

- [ ] Phase 1 の全成果物を理解している
- [ ] 技術的負債とその優先度を把握している
- [ ] 開発環境が正常に動作している
- [ ] ビルドとテストが全て成功している
- [ ] CI/CD が正常に動作している
- [ ] Phase 2 の計画と仕様書を確認済み
- [ ] 最初に取り組むタスクを決定済み

**Phase 2 開始準備完了！** 🚀

---

## 🔧 トラブルシューティング

### よくある問題と対処法

#### OCaml/opam の問題

**問題**: `opam install` が失敗する

**対処法**:
```bash
opam update
opam upgrade
opam install . --deps-only --with-test --yes
```

#### LLVM の問題

**問題**: LLVM が見つからない

**対処法 (macOS)**:
```bash
brew install llvm@18
brew link --force llvm@18
export PATH="/opt/homebrew/opt/llvm@18/bin:$PATH"  # Apple Silicon
# または
export PATH="/usr/local/opt/llvm@18/bin:$PATH"     # Intel Mac
```

**対処法 (Linux)**:
```bash
sudo apt-get install llvm-18 llvm-18-dev llvm-18-tools
```

#### ビルドの問題

**問題**: `dune build` が失敗する

**対処法**:
```bash
dune clean
opam exec -- dune build
```

#### テストの問題

**問題**: 一部のテストが失敗する

**対処法**:
1. `technical-debt.md` で既知の問題を確認
2. `git status` でファイルの変更がないか確認
3. `git diff` で予期しない変更がないか確認
4. 必要に応じて `git reset --hard HEAD` でリセット

---

## 📞 サポート

### ドキュメント

- **Phase 1 完了報告**: `compiler/ocaml/docs/phase1-*-completion-report.md`
- **技術的負債リスト**: `compiler/ocaml/docs/technical-debt.md`
- **引き継ぎドキュメント**: `compiler/ocaml/docs/phase1-to-phase2-handover.md`

### 計画書

- **Phase 2 計画**: `docs/plans/bootstrap-roadmap/2-*.md`
- **メトリクス定義**: `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`
- **リスク管理**: `docs/plans/bootstrap-roadmap/0-4-risk-handling.md`

### 仕様書

- **型システム**: `docs/spec/1-2-types-Inference.md`
- **効果システム**: `docs/spec/1-3-effects-safety.md`
- **Capability システム**: `docs/spec/3-8-core-runtime-capability.md`

---

**作成日**: 2025-10-11
**最終更新**: 2025-10-11
**次回更新予定**: Phase 2 Week 20（中間レビュー時）
