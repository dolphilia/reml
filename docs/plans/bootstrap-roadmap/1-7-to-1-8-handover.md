# Phase 1-7 → Phase 1-8 引き継ぎドキュメント

**作成日**: 2025-10-11
**Phase 1-7 完了日**: 2025-10-10
**Phase 1-8 開始予定**: 2025-10-11 以降

## Phase 1-7 の成果物

### ✅ 完了した実装

**x86_64 Linux 検証インフラ構築（2025-10-10 完了）**:
- GitHub Actions ワークフロー構築（`.github/workflows/bootstrap-linux.yml`）
- ローカル CI 再現スクリプト（`scripts/ci-local.sh`）
- メトリクス記録スクリプト（`tooling/ci/record-metrics.sh`）
- LLVM IR 検証の明示化（llvm-as → opt -verify → llc）
- コンパイラバイナリの命名とバージョン対応（`remlc-ocaml --version`）
- テスト結果の JUnit XML 出力
- LLVM IR・Bitcode の統合アーティファクト化
- 依存関係キャッシュの最適化（LLVM 18, OCaml/opam）

**詳細**: [compiler/ocaml/docs/phase1-7-completion-report.md](../../compiler/ocaml/docs/phase1-7-completion-report.md)

### ⏸️ Phase 2 へ延期

以下のタスクは基礎実装が完了しましたが、完全な実装は Phase 2 へ延期：

| タスク | 理由 | Phase 2 対応内容 |
|--------|------|------------------|
| カバレッジレポート生成 | 基本的なテストは完了 | `bisect_ppx` 導入、CI 統合 |
| メトリクス可視化 | 記録スクリプトは完成 | 時系列グラフ、推移追跡 |
| 失敗時の自動 issue 作成 | 基本的なログ収集は完了 | `0-4-risk-handling.md` への自動 issue 作成 |

---

## Phase 1-8 の目標

Phase 1-8 では macOS プレビルド対応により、macOS 開発者が日常開発を円滑に行える環境を整備します：

### Phase 1-7 からの未解決課題（2025-10-12 更新）
- **Linux Lint ジョブのフォーマッタ不足**: `bootstrap-linux.yml` の Lint ステージで `opam exec -- dune build @fmt` が `ocamlformat` 未インストールにより失敗している。Phase 1-8 の macOS CI 着手前に、`ocamlformat` のバージョン固定（`dune-project` での `using fmt` または `opam install ocamlformat.0.26.2 --yes`）とキャッシュキー更新を実施して Linux CI を安定化させる。対応内容は `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` §0 に記録し、完了後のログを `compiler/ocaml/README.md` に追記する。

### 主要タスク（Week 18-22）

1. **計画キックオフと要件整理**（Week 18）
   - macOS 向けビルドの期待成果物、最小検証項目、リスク項目の整理
   - Linux CI との共通化ポイントと差分タスクの抽出
   - 開発者ヒアリングで macOS 手元検証の痛点を収集

2. **ワークフロー設計**（Week 18-19）
   - `.github/workflows/bootstrap-macos.yml` の作成
   - Linux CI と同じステージ構成（Lint → Build → Test → Artifact → LLVM Verify）
   - `actions/cache` キーのターゲット別分離（macos）

3. **ツールチェーンセットアップ**（Week 19）
   - Homebrew 経由で llvm@15, opam, pkg-config, libtool をインストール
   - Xcode Command Line Tools のバージョンチェック
   - `opam switch create 4.14.2` と依存関係インストール

4. **ビルドジョブ実装**（Week 19-20）
   - `dune build` の macOS 上での実行
   - `runtime/native` の Mach-O ターゲット向け設定追加
   - `remlc-ocaml-macos` バイナリの生成

5. **テストジョブ実装**（Week 20）
   - `dune runtest` の実行とゴールデンテスト結果の確認
   - `scripts/verify_llvm_ir.sh` の `--target x86_64-apple-darwin` 対応
   - テスト失敗時のログ収集強化

6. **LLVM/Mach-O 検証**（Week 20-21）
   - `llvm-as`, `opt -verify`, `llc -mtriple=x86_64-apple-darwin` の実行
   - 最小バイナリのリンクと実行確認
   - `otool -L` でのライブラリ依存関係検証

7. **アーティファクトとメトリクス管理**（Week 21）
   - Linux CI と揃えた命名規則でのアップロード
   - `0-3-audit-and-metrics.md` への macOS メトリクス追加
   - `tooling/ci/docker/metrics.json` への macOS セクション追加

8. **ローカル再現とドキュメント整備**（Week 21-22）
   - `scripts/ci-local.sh` への `--target macos` オプション追加
   - `compiler/ocaml/README.md` への macOS 手元検証ガイド追記
   - Phase 2 以降への TODO の記録

---

## 前提条件の確認

### Phase 1-7 から引き継ぐ実装

#### ✅ CI ワークフロー設計（既存実装）

**ファイル**: `.github/workflows/bootstrap-linux.yml`

**主要構成**:
```yaml
jobs:
  lint:      # コードフォーマットチェック
  build:     # コンパイラ・ランタイムビルド（lintに依存）
  test:      # 単体テスト・統合テスト・ランタイムテスト（buildに依存）
  llvm-verify: # LLVM IR検証（testに依存）
  record-metrics: # メトリクス記録（build, test, llvm-verifyに依存）
  artifact:  # 全アーティファクトを統合（全ジョブに依存）
```

**Phase 1-8 での再利用**:
- ステージ構成をそのまま `bootstrap-macos.yml` に転用
- `runs-on: ubuntu-latest` を `runs-on: macos-13` に変更
- トリガー設定（push, pull_request, schedule）を共通化

#### ✅ 依存関係キャッシュ戦略（既存実装）

**Linux CI の実装**:
```yaml
- name: LLVM キャッシュ
  uses: actions/cache@v4
  with:
    path: /usr/lib/llvm-18
    key: llvm-18-${{ runner.os }}

- name: OCaml セットアップ
  uses: ocaml/setup-ocaml@v3
  with:
    ocaml-compiler: 4.14.x
    dune-cache: true
```

**macOS での差分**:
- LLVM パス: `/usr/lib/llvm-18` → `/usr/local/opt/llvm@15`（Homebrewの場合）
- Homebrew キャッシュ: `~/Library/Caches/Homebrew/downloads` を追加
- キャッシュキー: `llvm-15-macos-${{ runner.os }}` に変更

#### ✅ アーティファクト管理手法（既存実装）

**Linux CI の実装**:
```yaml
- name: ビルド成果物のアップロード
  uses: actions/upload-artifact@v4
  with:
    name: linux-build
    path: |
      _build/default/compiler/ocaml/src/main.exe
      runtime/native/build/libreml_runtime.a
    retention-days: 30
```

**macOS での命名規則**:
- `linux-build` → `macos-build`
- `linux-ci-bundle` → `macos-ci-bundle`
- `llvm-ir-verified` → `llvm-ir-verified-macos`（共通化も検討）

#### ✅ メトリクス記録スクリプト（既存実装）

**ファイル**: `tooling/ci/record-metrics.sh`

**主要機能**:
- CI 実行結果（ビルド時間、テスト件数、成功/失敗）の記録
- `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` への自動追記

**macOS での拡張**:
- ターゲット引数の追加（`--target macos`）
- macOS 専用セクションへの記録
- Linux/macOS の比較レポート生成機能の追加（Phase 2）

#### ✅ LLVM IR 検証フロー（既存実装）

**スクリプト**: `compiler/ocaml/scripts/verify_llvm_ir.sh`

**現在のフロー**:
```bash
# 1. LLVM IR 生成
remlc examples/cli/add.reml --emit-ir --out-dir=_build/ir

# 2. llvm-as でアセンブル
llvm-as-18 _build/ir/add.ll -o _build/ir/add.bc

# 3. opt で検証
opt-18 -verify _build/ir/add.bc -o /dev/null

# 4. llc でコード生成
llc-18 _build/ir/add.ll -o _build/ir/add.s
```

**macOS での拡張**:
- `--target x86_64-apple-darwin` オプションの追加
- `llc -mtriple=x86_64-apple-darwin` でのコード生成
- `otool -L` でのライブラリ依存関係検証の追加

---

## Phase 1-8 開始前のチェックリスト

### 環境確認

- [x] Phase 1-7 が完了していることを確認
- [x] `.github/workflows/bootstrap-linux.yml` が正常動作
- [x] `scripts/ci-local.sh` がローカルで実行可能
- [x] `tooling/ci/record-metrics.sh` が正しく動作
- [ ] Lint ステージが `ocamlformat` インストール済みで成功する（`opam exec -- dune build @fmt` → `git diff --exit-code` が失敗しないこと）
- [ ] macOS 開発環境の準備状況を確認（Homebrew, Xcode CLT）
- [ ] GitHub Actions macOS ランナーのコスト・制限を確認

### 仕様書の理解

- [ ] [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) を読む（Phase 1-8 メイン）
- [ ] [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md) を確認（再利用可能な資産）
- [ ] [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) を確認（メトリクス定義）
- [ ] [0-4-risk-handling.md](0-4-risk-handling.md) を確認（リスク管理）

### 計画書の確認

- [ ] Phase 1-8 マイルストーンの達成条件を理解
- [ ] 作業ブレークダウンを確認
- [ ] 既存実装との統合ポイントを把握
- [ ] macOS 固有の課題を洗い出し

---

## Phase 1-7 から引き継ぐ技術的知見

### 1. CI 設計の成功パターン

**動作確認済み**:
- ステージ間の依存関係管理（`needs`）
- 失敗時の継続戦略（`if: always()`）
- アーティファクトの保持期間設定（30日 vs 7日）
- キャッシュ戦略（LLVM, OCaml/opam）

**macOS での活用**:
- 同じステージ構成を採用し、設定作業を最小化
- キャッシュキーをターゲット別に分離し、Linux/macOS で独立管理
- アーティファクト命名規則を統一し、レビュー時の比較を容易に

### 2. LLVM バージョン管理

**Linux CI の決定事項**:
- LLVM 18 系を正式採用（2025-10-10 決定）
- 理由: OCaml 実装での実績、安定性、型付き属性サポート
- 影響: Phase 1 では LLVM 15 へのダウングレードを行わない

**macOS での検討事項**:
- Homebrew の LLVM バージョン（`llvm@15` vs `llvm@18`）
- Linux との差異（型付き属性、ABI、コマンドラインオプション）
- バージョン固定戦略（`brew extract` vs prebuilt tarball）

### 3. メモリ検証の実施

**Linux CI の実装**:
- Valgrind: リリースビルドで実行
- AddressSanitizer: `DEBUG=1` ビルドで実行
- 両者の併用による衝突を回避（個別実行）

**macOS での差分**:
- Valgrind の macOS サポート状況を確認
- AddressSanitizer のみの利用も検討
- Instruments など macOS 固有ツールの活用

### 4. アーティファクトのサイズ管理

**Linux CI の経験**:
- コンパイラバイナリ: 約 10-20 MB
- ランタイムライブラリ: 約 1-2 MB
- LLVM IR: 約 1-10 KB/ファイル
- テストログ: 約 1-5 MB

**macOS での予測**:
- Mach-O バイナリは ELF より若干大きい可能性
- シンボル情報の保持による増加（デバッグ用）
- 圧縮アーティファクトのサイズ上限を設定

---

## Phase 1-7 から引き継ぐファイル

### コア実装

| ファイル | 説明 | Phase 1-8 での利用 |
|---------|------|-------------------|
| `.github/workflows/bootstrap-linux.yml` | Linux CI ワークフロー | macOS ワークフローのテンプレート |
| `scripts/ci-local.sh` | ローカル再現スクリプト | `--target macos` 対応の追加 |
| `tooling/ci/record-metrics.sh` | メトリクス記録 | macOS メトリクスの記録 |
| `compiler/ocaml/scripts/verify_llvm_ir.sh` | LLVM IR 検証 | `--target x86_64-apple-darwin` 対応 |

### ビルドシステム

| ファイル | 説明 | Phase 1-8 での利用 |
|---------|------|-------------------|
| `compiler/ocaml/dune-project` | Dune プロジェクト定義 | そのまま利用 |
| `compiler/ocaml/reml_ocaml.opam` | opam パッケージ定義 | そのまま利用 |
| `runtime/native/Makefile` | ランタイムビルド | Mach-O 向け設定追加 |

### ドキュメント

| ファイル | 説明 | Phase 1-8 での利用 |
|---------|------|-------------------|
| `docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md` | Phase 1-7 計画書 | macOS 計画との比較 |
| `docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` | メトリクス定義 | macOS セクション追加 |
| `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` | リスク管理 | macOS 固有リスク登録 |
| `compiler/ocaml/docs/phase1-7-completion-report.md` | Phase 1-7 完了報告 | 引き継ぎ情報の参照 |

---

## macOS 固有の注意点

### 1. Homebrew 依存関係

**課題**:
- Homebrew のバージョン管理が頻繁に更新される
- `brew upgrade` により LLVM のバージョンが変わる可能性
- GitHub Actions の macOS イメージには Homebrew が事前インストール

**対策**:
- `brew install llvm@15` でバージョン固定
- `brew extract` による特定バージョンの固定化
- `brew unlink`/`brew link` による明示的なパス設定
- `docs/notes/backend/llvm-spec-status-survey.md` にバージョン管理戦略を記録

### 2. Mach-O vs ELF の違い

**主な差異**:
- 実行ファイル形式: Mach-O (macOS) vs ELF (Linux)
- 動的リンカー: `dyld` (macOS) vs `ld.so` (Linux)
- ライブラリ拡張子: `.dylib` (macOS) vs `.so` (Linux)
- ABI 規約: macOS は独自の呼び出し規約を持つ

**対応**:
- `runtime/native/Makefile` に Mach-O 向けビルド規則を追加
- `AR=libtool -static` による静的ライブラリ生成
- `otool -L` でのライブラリ依存関係検証
- Phase 2 以降で ABI 差分の詳細検証

### 3. LLVM toolchain の差異

**Linux での設定**:
```bash
llvm-as-18 input.ll -o output.bc
opt-18 -verify output.bc -o /dev/null
llc-18 input.ll -o output.s
```

**macOS での設定**:
```bash
# Homebrew でインストールした場合
/usr/local/opt/llvm@15/bin/llvm-as input.ll -o output.bc
/usr/local/opt/llvm@15/bin/opt -verify output.bc -o /dev/null
/usr/local/opt/llvm@15/bin/llc -mtriple=x86_64-apple-darwin input.ll -o output.s
```

**対応**:
- パス設定スクリプト（`tooling/ci/macos/setup-env.sh`）の作成
- `llc` の `-mtriple` オプションで macOS ターゲットを明示
- `docs/notes/backend/llvm-spec-status-survey.md` に差分を記録

### 4. Xcode Command Line Tools

**必要性**:
- `clang` コンパイラの提供
- システムヘッダー（`stdio.h` など）の提供
- `libtool`, `ar` などのツールチェーン

**確認方法**:
```bash
xcode-select -p
# /Library/Developer/CommandLineTools が表示されれば OK

# バージョン確認
clang --version
```

**対応**:
- GitHub Actions ワークフローに `xcode-select --install` チェックを追加
- バージョン情報を `docs/notes/backend/llvm-spec-status-survey.md` に記録

---

## 推奨される Phase 1-8 の進め方

### Week 18: 計画キックオフとワークフロー設計

1. **要件整理とリスク登録**
   - macOS 向けビルドの期待成果物を定義
   - Linux CI との共通化ポイントを一覧化
   - `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` にリスク登録

2. **`.github/workflows/bootstrap-macos.yml` の初版作成**
   - Linux CI のワークフローを複製
   - `runs-on: macos-13` に変更
   - 基本的なステージ（Lint, Build）のみ実装

3. **開発者ヒアリング**
   - macOS 手元検証の痛点を収集
   - Homebrew 利用状況の確認
   - LLVM バージョンの確認

**成果物**:
- `.github/workflows/bootstrap-macos.yml` 初版
- リスク登録（`0-4-risk-handling.md`）
- 開発者ヒアリング結果（議事録）

### Week 19: ツールチェーンセットアップとビルドジョブ

1. **Homebrew 依存関係のインストール**
   - `brew install llvm@15 opam pkg-config libtool`
   - `brew unlink llvm && brew link llvm@15`
   - パス設定（`/usr/local/opt/llvm@15/bin`）

2. **OCaml 環境構築**
   - `opam switch create 4.14.2`
   - `opam install . --deps-only --with-test`
   - キャッシュ設定の最適化

3. **ビルドジョブの実装**
   - `dune build` の実行
   - `runtime/native` の Mach-O ビルド
   - `remlc-ocaml-macos` バイナリの生成

**成果物**:
- `tooling/ci/macos/setup-env.sh`（新規）
- `runtime/native/Makefile` への Mach-O 設定追加
- macOS ビルドの成功確認

### Week 20: テストジョブと LLVM 検証

1. **テストジョブの実装**
   - `dune runtest` の実行
   - ゴールデンテスト結果の確認
   - テスト失敗時のログ収集

2. **LLVM IR 検証の拡張**
   - `scripts/verify_llvm_ir.sh` に `--target x86_64-apple-darwin` 対応
   - `llc -mtriple=x86_64-apple-darwin` でのコード生成
   - `.s` ファイルの検証

3. **Mach-O リンクテスト**
   - `clang` でのリンク
   - 実行可能バイナリの生成
   - `otool -L` での依存関係確認

**成果物**:
- テストジョブの成功
- LLVM IR 検証フローの確立
- Mach-O バイナリの実行確認

### Week 21: アーティファクトとメトリクス管理

1. **アーティファクト管理の実装**
   - `macos-build` アーティファクトのアップロード
   - `llvm-ir-verified-macos` の保存
   - Linux CI と揃えた命名規則

2. **メトリクス記録の実装**
   - `tooling/ci/record-metrics.sh` への `--target macos` 対応
   - `0-3-audit-and-metrics.md` への macOS セクション追加
   - `tooling/ci/docker/metrics.json` への macOS データ追加

3. **CI 統合テスト**
   - GitHub Actions での全ステージ実行
   - アーティファクトのダウンロード確認
   - メトリクスの記録確認

**成果物**:
- アーティファクト管理の完成
- メトリクス記録の完成
- CI の安定動作確認

### Week 22: ローカル再現とドキュメント整備

1. **ローカル再現スクリプトの拡張**
   - `scripts/ci-local.sh` への `--target macos` オプション追加
   - macOS 固有の依存関係チェック
   - エラーメッセージの改善

2. **ドキュメント整備**
   - `compiler/ocaml/README.md` への macOS 手元検証ガイド追記
   - `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` への Mach-O TODO 追加
   - Phase 2 以降への課題登録（`0-4-risk-handling.md`）

3. **Phase 1-8 完了報告書の作成**
   - `compiler/ocaml/docs/phase1-8-completion-report.md` の作成
   - 成果物、統計、残課題の記録
   - Phase 2 への引き継ぎ情報の整理

**成果物**:
- `scripts/ci-local.sh` の macOS 対応完了
- ドキュメント整備完了
- Phase 1-8 完了報告書

---

## Phase 1-8 で注意すべき制約

### 1. GitHub Actions macOS ランナーのコスト

**課題**:
- macOS ランナーは Linux ランナーより実行時間が長い
- GitHub Actions の無料枠は macOS で消費が早い
- 並行実行数に制限がある

**対策**:
- 必要最小限のトリガー設定（push は main/develop のみ）
- キャッシュの最大活用（初回以降の実行時間短縮）
- 長時間実行テストは Phase 2 で実装（Phase 1-8 では基本テストのみ）

### 2. Homebrew のバージョン管理

**課題**:
- `brew upgrade` により LLVM のバージョンが変わる
- GitHub Actions の macOS イメージには古いバージョンの Homebrew が入っている場合がある

**対策**:
- `brew install llvm@15` でバージョン固定
- `brew link --force llvm@15` で明示的なリンク
- `docs/notes/backend/llvm-spec-status-survey.md` にバージョン管理戦略を記録

### 3. Mach-O ランタイムのリンクエラー

**リスク**:
- `runtime/native` の Makefile が Linux 向けに最適化されている
- Mach-O 向けのビルド規則が未整備
- `libtool` の使用法が Linux と異なる

**対策**:
- `Makefile` に Mach-O 向け設定を条件分岐で追加
- `AR=libtool -static -o` による静的ライブラリ生成
- Phase 2 で CMake 化を検討（Linux/macOS/Windows の統一ビルドシステム）

---

## リスク管理への登録

Phase 1-8 で想定されるリスク項目を [0-4-risk-handling.md](0-4-risk-handling.md) へ登録：

| リスク項目 | 影響 | 軽減策 |
|-----------|------|--------|
| GitHub Actions macOS ランナーの起動待ち時間 | CI 実行時間の増加 | セルフホストランナー導入を Phase 2 で検討 |
| Homebrew LLVM のバージョン変動 | ビルド失敗のリスク | `brew extract` による固定化または prebuilt tarball 配布 |
| Mach-O ランタイムのリンクエラー | ランタイム連携の失敗 | Phase 2 で CMake 化を検討 |
| ARM64 macOS 対応の遅延 | 開発者体験の低下 | Phase 3 以降で対応、Phase 1-8 では Intel macOS のみ |

---

## 連絡先とサポート

### ドキュメント

- **Phase 1-7 完了報告**: [compiler/ocaml/docs/phase1-7-completion-report.md](../../compiler/ocaml/docs/phase1-7-completion-report.md)
- **Phase 1-7 計画**: [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)
- **Phase 1-8 計画**: [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md)

### 仕様書

- **メトリクス定義**: [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- **リスク管理**: [0-4-risk-handling.md](0-4-risk-handling.md)
- **LLVM 統合**: [../../guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)

### 既存実装

- **Linux CI ワークフロー**: `.github/workflows/bootstrap-linux.yml`
- **ローカル再現スクリプト**: `scripts/ci-local.sh`
- **メトリクス記録**: `tooling/ci/record-metrics.sh`

---

**引き継ぎ完了**: 2025-10-11
**Phase 1-8 開始**: 準備完了
**次回レビュー**: Phase 1-8 Week 22（macOS CI 完成時）
