# compiler/ocaml ワークスペース

このディレクトリは Reml ブートストラップ計画 Phase 1〜3 の OCaml 実装を管理し、以降のセルフホスト化へ向けてランタイム統合を進める作業拠点です。フェーズ別の詳細計画は `docs/plans/bootstrap-roadmap/` を参照してください。

## 現在のステータス
- Phase 1 — Parser & Frontend（完了: 2025-10-06）: `compiler/ocaml/docs/phase1-completion-report.md`
- Phase 2 — Typer MVP（完了: 2025-10-07）: `compiler/ocaml/docs/phase2-completion-report.md`
- Phase 3 — Core IR & LLVM 生成（完了: 2025-10-09）: `compiler/ocaml/docs/phase3-m3-completion-report.md`
- Phase 1-5 — ランタイム連携（完了: 2025-10-10）: `compiler/ocaml/docs/phase1-5-completion-report.md`
- Phase 1-6 — 開発者体験整備（完了: 2025-10-10）: `compiler/ocaml/docs/phase1-6-completion-report.md`
- Phase 1-7 — x86_64 Linux 検証インフラ（完了: 2025-10-10）: `compiler/ocaml/docs/phase1-7-completion-report.md`
  - ✅ GitHub Actions ワークフロー構築
  - ✅ ローカル CI 再現スクリプト
  - ✅ メトリクス記録スクリプト
  - ✅ LLVM IR 検証の明示化
  - ✅ コンパイラバイナリの命名とバージョン対応
  - ✅ テスト結果の JUnit XML 出力
  - ✅ LLVM IR・Bitcode の統合アーティファクト化
  - 進捗: 100% (全タスク完了)
- **Phase 1-8 — macOS プレビルド対応（進行中: 2025-10-11〜）**
  - ⏳ GitHub Actions macOS ワークフロー設計
  - ⏳ Homebrew ツールチェーン準備
  - ⏳ Mach-O ランタイムビルド規則整備
  - ⏳ LLVM IR 検証フローの macOS 対応
  - ⏳ メトリクス記録とアーティファクト管理
  - 📄 2025-10-12: dune-project の構文エラーを修正し、ocamlformat 0.26.2 を導入
  - 📄 2025-10-12: macOS ローカル環境で ocamlformat インストール完了、全コードをフォーマット
  - 📄 2025-10-12: Bootstrap Linux CI の Lint ステージブロッカーを解消（dune-project 修正、.ocamlformat 作成）
  - 📄 2025-10-13: `scripts/ci-local.sh` を x86_64 macOS トリプル固定で更新し、GitHub Actions なしでもローカル検証できる手順を整備
  - 進捗: 15% (Linux CI ブロッカー解消、ローカル環境セットアップ完了)

過去フェーズの週次レポートや統計は `compiler/ocaml/docs/` 配下の各完了報告・引き継ぎ資料に集約しています。

## 1-5 ランタイム連携のフォーカス
### 目的
- 最小ランタイム API（`mem_alloc`, `mem_free`, `inc_ref`, `dec_ref`, `panic`, `print_i64`）を実装し、生成した LLVM IR からリンク可能にする。
- `docs/guides/llvm-integration-notes.md` §5 に準拠した参照カウント (RC) モデルを Phase 1 の OCaml 実装に接続する。
- ランタイム品質の検証（リーク・ダングリング検出、Valgrind/ASan 連携）と CI 統合を行う。

### 成果物と出口条件
- `runtime/native/` に最小ランタイムのソース・ビルドスクリプト・テストを配置し、`make runtime` で静的ライブラリ `libreml_runtime.a` を生成できる。
- `compiler/ocaml/src/llvm_gen/` からランタイム関数を宣言・呼び出し、`--link-runtime` オプションでバイナリ生成まで通す。
- ランタイム API と RC モデルのテスト結果・計測値を `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記録し、`compiler/ocaml/docs/` に検証ノートを残す。

### 進捗状況（2025-10-10 更新）

**完了タスク** ✅:
- §6.1 ランタイム関数宣言生成（`mem_alloc`, `inc_ref`, `dec_ref`, `panic`, `print_i64`, `memcpy`）
- §6.2 文字列リテラル生成時の `mem_alloc` 呼び出し実装
- §6.3 リンクヘルパー実装（`runtime_link.ml`）と CLI 統合（`--link-runtime`）
- 統合テスト作成（`tests/test_runtime_integration.sh`）

**Phase 2 へ延期** ⏳:
- タプル/レコード生成時の `mem_alloc` 呼び出し（Core IR の TupleConstruct ノード実装が必要）
- スコープ終了時の `dec_ref` 挿入（所有権解析と型情報に基づく正確な実装が必要）
- 実行可能ファイル生成 E2E テスト（文字列パラメータ処理の課題により延期）
- メモリリーク検証（実行可能バイナリ生成後に実施）

**Phase 1-5 で達成した内容** ✅:
- 文字列リテラル生成時の `mem_alloc` 呼び出し実装
- ランタイム関数宣言とリンク統合
- メモリ検証スクリプト作成（`scripts/verify_memory.sh`）
- E2E テストフレームワーク整備（`tests/test_runtime_integration.sh`）

**詳細**: [phase1-5-llvm-integration-report.md](docs/phase1-5-llvm-integration-report.md)

### 作業トラック（詳細は計画書 §1〜§8 を参照）
- ✅ **API 定義**: `runtime/native/include/reml_runtime.h` を作成し、関数シグネチャ・型タグ規約を確定（2025-10-10 完了）
  - 6 関数の最小 API 定義完了：`mem_alloc`, `mem_free`, `inc_ref`, `dec_ref`, `panic`, `print_i64`
  - 型タグ enum 定義（`REML_TAG_INT` 〜 `REML_TAG_ADT`）、9 種類の基本型を Phase 1 でサポート
  - ヒープオブジェクトヘッダ構造 `reml_object_header_t` の定義（refcount + type_tag、8 バイト）
  - コンパイラ側との整合確認：`panic` のシグネチャを FAT ポインタ形式 `(ptr, i64)` に統一
  - ディレクトリ構造整備：`runtime/native/{include,src,tests}/` を作成
  - 簡易実装例として `runtime/native/src/print_i64.c` を追加し、ヘッダのコンパイル妥当性を検証済み
- ✅ **メモリアロケータ**: `runtime/native/src/mem_alloc.c` を実装完了（2025-10-10）
  - malloc ベースの実装 + ヘッダ初期化（refcount=1, type_tag 設定可能）
  - 8 バイト境界への自動調整（`align_to_8` 関数）
  - アロケーション失敗時の `panic` 呼び出し
  - デバッグビルド時のアロケーション追跡（alloc_count / free_count）
  - 二重解放検出（DEBUG モード）
  - ユーティリティ関数：`reml_set_type_tag`, `reml_get_type_tag`, `reml_debug_print_alloc_stats`
  - テストスイート：6 件のテストケース（基本 alloc/free、アラインメント、NULL 安全性、大容量メモリ、型タグ、複数アロケーション）すべて成功
  - ビルドシステム：`runtime/native/Makefile` 整備（macOS SDK 対応、AddressSanitizer 統合）
- ✅ **パニックハンドラ**: `runtime/native/src/panic.c` を実装完了（2025-10-10）
  - エラーメッセージの stderr 出力（タイムスタンプ、PID、メッセージ）
  - `exit(1)` による異常終了
  - Phase 2 向け拡張版 `panic_at` 追加（ファイル名・行番号付き）
  - GitHub Actions の `pid_t` 未定義エラーに対応するため `<sys/types.h>` を追加し、`make runtime` ビルドを復旧
- ✅ **参照カウント**: `runtime/native/src/refcount.c` で RC 操作と型別デストラクタ呼び出しを実装完了（2025-10-10）
  - inc_ref / dec_ref の基本操作実装（単一スレッド、Phase 1）
  - 型別デストラクタディスパッチ（STRING, TUPLE, RECORD, CLOSURE, ADT, プリミティブ型）
  - 再帰的な子オブジェクト解放（クロージャ環境、ADT payload）
  - テストスイート：8 件のテストケース（基本 inc/dec、ゼロ到達解放、NULL 安全性、型別デストラクタ、リークゼロ検証）すべて成功
  - AddressSanitizer 統合：リーク・ダングリングゼロ
  - デバッグ統計機能：`reml_debug_print_refcount_stats` でカウンタ確認可能
  - Phase 2 向けTODO: アトミック操作（並行対応）、循環参照検出、型メタデータテーブル
- **ビルドシステム**: `runtime/Makefile`（`-O2`/`-Wall -Wextra`/`-g`）を用意し、プラットフォーム検出と依存関係を整理。
- ✅ **LLVM 連携**: `compiler/ocaml/src/llvm_gen/codegen.ml` と `abi.ml` でランタイムシンボル宣言・属性設定・リンクフラグを統合（`llvm_attr.ml` + C スタブで `sret` / `byval` の型付き属性を付与）完了（2025-10-10）
- ✅ **リンクヘルパー**: `runtime_link.ml` でプラットフォーム検出とリンカーコマンド生成を実装完了（2025-10-10）
- **テストと検証**: `runtime/native/tests/` と `compiler/ocaml/tests/codegen/` に単体/統合テストを追加し、Valgrind/ASan のジョブを CI に組み込む。
  - Valgrind はリリースビルド、AddressSanitizer は `DEBUG=1` ビルドで個別に実行するよう GitHub Actions を調整し、両者の併用による衝突を回避
- **ドキュメントと CI**: `docs/guides/llvm-integration-notes.md` および `compiler/ocaml/docs/` を更新し、GitHub Actions でランタイムビルドと検証を自動化。

## 直近の準備チェックリスト
- `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` を精読し、各トラックのスコープと完了条件を確認する。
- `compiler/ocaml/src/llvm_gen/` で呼び出しているランタイム関数を洗い出し、必要なシグネチャが計画書と一致しているか確認する（特に `panic` の属性と `inc_ref`/`dec_ref` の呼び出し箇所）。
- `compiler/ocaml/docs/phase3-to-phase2-handover.md`・`compiler/ocaml/docs/technical-debt.md` の High 優先度項目（型マッピング TODO, CFG 線形化など）がランタイム統合のブロッカーにならないよう対応状況を見直す。
- `runtime/native/` の既存ファイル構成と CI スクリプト (`compiler/ocaml/scripts/verify_llvm_ir.sh` など) を確認し、ランタイム検証を追加する際の差分影響を把握する。
- Docker ベースの Linux 検証フロー（`scripts/docker/build-runtime-container.sh`, `scripts/docker/run-runtime-tests.sh`, `scripts/docker/smoke-linux.sh`）を実行し、CI で利用するタグとローカル環境の整合を取る。
- クロスコンパイル成果物は `scripts/docker/run-cross-binary.sh -- artifacts/cross/hello-linux` などのコマンドで Docker コンテナ上からスモークテストし、リモート検証ノードの結果と比較する。
- 計測結果を追記するための記録先（`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md`）とリスク登録先（`docs/plans/bootstrap-roadmap/0-4-risk-handling.md`）のフォーマットを再確認する。
- macOS での Linux x86_64 クロスビルド手順（`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §10）を確認し、必要なツールチェーン・sysroot・リモート実行環境（SSH 接続できる Linux ノード、または補助エミュレータ）の準備可否を確認する。

## Phase 1-8: macOS プレビルド対応（完了）

Phase 1-8 では macOS 開発者が Linux クロスビルドに依存せずに日常開発を行える環境を整備しました。

### macOS 開発環境のセットアップ

#### 自動セットアップスクリプトの使用（推奨）

最も簡単な方法は、提供されているセットアップスクリプトを使用することです：

```bash
# リポジトリルートから実行
./tooling/ci/macos/setup-env.sh
```

このスクリプトは以下を自動的に実行します：
- Homebrew の確認
- Xcode Command Line Tools のバージョン確認
- LLVM 18 のインストールとパス設定
- OCaml 5.2.1 環境のセットアップ（opam）
- 必要なツール（pkg-config, libtool）のインストール

スクリプトのオプション：
```bash
# ヘルプを表示
./tooling/ci/macos/setup-env.sh --help

# LLVM のみをスキップ
./tooling/ci/macos/setup-env.sh --skip-llvm

# 実行せずコマンドのみ確認
./tooling/ci/macos/setup-env.sh --dry-run
```

#### 手動セットアップ

自動スクリプトを使用しない場合は、以下の手順で手動セットアップできます：

**1. Homebrew のインストール**
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

**2. Xcode Command Line Tools のインストール**
```bash
xcode-select --install
# バージョン確認
xcode-select -p
clang --version
```

**3. LLVM と opam のインストール**
```bash
brew install llvm@18 opam pkg-config libtool
# LLVM のパス設定
echo 'export PATH="/usr/local/opt/llvm@18/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# LLVM バージョン確認
llvm-as --version  # 18.x.x を確認
opt --version
llc --version
```

**4. OCaml 環境のセットアップ**
```bash
opam init
opam switch create 5.2.1
eval $(opam env)

# OCaml バージョン確認
ocaml -version  # 5.2.1 を確認
```

**5. 依存関係のインストール**
```bash
cd /path/to/kestrel/compiler/ocaml
opam install . --deps-only --with-test
```

### macOS でのビルドとテスト

**基本的なビルド**:
```bash
cd /path/to/kestrel/compiler/ocaml
opam exec -- dune build
```

**テストの実行**:
```bash
opam exec -- dune runtest
```

**ランタイムのビルド**:
```bash
cd /path/to/kestrel/runtime/native
make runtime
```

### macOS でのローカル CI 再現

GitHub Actions と同じ検証手順をローカルで実行できます：

```bash
# リポジトリルートから実行
./scripts/ci-local.sh --target macos

# 特定のステップをスキップ
./scripts/ci-local.sh --target macos --skip-lint --skip-runtime

# ヘルプを表示
./scripts/ci-local.sh --help
```

ローカル CI スクリプトは以下を実行します：
1. **Lint**: コードフォーマットチェック
2. **Build**: コンパイラ・ランタイムのビルド
3. **Test**: 単体テスト・統合テスト
4. **Memory Check**: AddressSanitizer（Valgrind は macOS でスキップ）
5. **LLVM IR Verification**: `llvm-as` → `opt -verify` → `llc -mtriple=x86_64-apple-darwin`

Apple Silicon 環境でも `--target macos`（または自動検出）で実行すると、CI と同じく `x86_64-apple-darwin` 向けに強制して検証します。ホストが `arm64` でも IR のトリプルがぶれないため、Mach-O バイナリ差分を気にせず手元で確認できます。

### macOS 固有の注意事項

#### LLVM バージョンの統一
- **macOS**: Homebrew で LLVM 18 を使用
- **Linux CI**: LLVM 18 を使用

macOS と Linux で同じ LLVM 18 を使用することで、プラットフォーム間の一貫性を確保しています。

#### Valgrind の非サポート
macOS では Valgrind が正式にサポートされていないため、代わりに AddressSanitizer を使用します：

```bash
cd /path/to/kestrel/runtime/native
make clean
DEBUG=1 make runtime
DEBUG=1 make test
```

#### Mach-O vs ELF
macOS では Mach-O 形式の実行ファイルが生成されます。LLVM IR 検証では `x86_64-apple-darwin` ターゲットを使用します：

```bash
# macOS ターゲットでの IR 検証
./compiler/ocaml/scripts/verify_llvm_ir.sh \
  --target x86_64-apple-darwin \
  path/to/output.ll
```

### トラブルシューティング

**Homebrew LLVM が見つからない**:
```bash
# LLVM のリンクを確認
brew link --force llvm@18

# パスを確認
echo $PATH | grep llvm

# 見つからない場合は明示的にパスを追加
export PATH="/usr/local/opt/llvm@18/bin:$PATH"
```

**opam 環境変数が設定されていない**:
```bash
# opam 環境変数を再設定
eval $(opam env)

# シェル起動時に自動設定
echo 'eval $(opam env)' >> ~/.zshrc
```

**Xcode Command Line Tools のエラー**:
```bash
# 再インストール
sudo rm -rf /Library/Developer/CommandLineTools
xcode-select --install
```

**ビルドエラー（SDK パス）**:
```bash
# SDK パスを確認
xcrun --show-sdk-path

# SDK が見つからない場合は Xcode を再インストール
```

### Phase 1-7 から引き継ぐ資産

Phase 1-8 では Phase 1-7 で構築した以下の資産を再利用します：

- **CI ワークフロー設計**: ステージ構成（Lint → Build → Test → LLVM Verify → Artifact）
- **依存関係キャッシュ戦略**: `actions/cache` によるツールチェーンキャッシュ
- **アーティファクト管理手法**: 命名規則、保持期間、収集対象
- **メトリクス記録スクリプト**: `tooling/ci/record-metrics.sh` の拡張
- **LLVM IR 検証フロー**: `scripts/verify_llvm_ir.sh` の macOS 対応

詳細は [docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md](../../docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md) を参照してください。

---

## 関連ドキュメント
- **計画書**: `docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md`, `docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md`, `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md`, `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md`
- **引き継ぎ**: `docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md`
- **仕様・ガイド**: `docs/spec/0-1-project-purpose.md`, `docs/spec/1-1-syntax.md`, `docs/guides/llvm-integration-notes.md`, `docs/notes/llvm-spec-status-survey.md`
- **進捗記録**: `compiler/ocaml/docs/phase1-7-completion-report.md`, `compiler/ocaml/docs/technical-debt.md`

## ワークスペース概要
- `compiler/ocaml/src/`: パーサー、型推論、Core IR、LLVM 生成、CLI
- `compiler/ocaml/tests/`: 字句解析〜LLVM 検証までのテストスイート、ゴールデンファイル
- `runtime/native/`: ランタイム実装（Phase 1-5 で拡充予定）
- `compiler/ocaml/docs/`: フェーズ完了報告、技術的負債、環境設定メモ

### 基本コマンド
```bash
opam exec -- dune build       # ビルド
opam exec -- dune test        # テスト一式
opam exec -- dune exec -- remlc --emit-ir samples/basic.reml
```
ランタイム連携後は `--link-runtime` オプションおよび `runtime/Makefile` のビルド結果を CI で検証します。

### ローカル CI 再現

GitHub Actions と同じ検証手順をローカルで実行できます：

```bash
# リポジトリルートから実行
./scripts/ci-local.sh

# 特定のステップをスキップ
./scripts/ci-local.sh --skip-lint --skip-runtime

# ヘルプを表示
./scripts/ci-local.sh --help
```

このスクリプトは以下を実行します：
1. **Lint**: コードフォーマットチェック (`dune build @fmt`)
2. **Build**: コンパイラ・ランタイムのビルド
3. **Test**: 単体テスト・統合テスト・ランタイムテスト
4. **Memory Check**: Valgrind + AddressSanitizer
5. **LLVM IR Verification**: `llvm-as` → `opt -verify` → `llc`

詳細は [docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md](../../docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md) を参照してください。

## 過去フェーズの詳細
- Phase 1/2 の仕様・テスト整備: `compiler/ocaml/docs/phase1-completion-report.md`, `compiler/ocaml/docs/phase2-completion-report.md`
- Phase 3 の Core IR・LLVM 成果: `compiler/ocaml/docs/phase3-m3-completion-report.md`, `compiler/ocaml/docs/phase3-week10-11-completion.md`
- 残課題とフォローアップ: `compiler/ocaml/docs/phase3-remaining-tasks.md`, `compiler/ocaml/docs/technical-debt.md`

詳細な進捗ログや週次の統計は各報告書を参照してください。README では次フェーズへ進むための要約と着手ポイントのみを保持します。
