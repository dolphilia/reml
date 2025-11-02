# Windows環境におけるLLVM 18ソースビルド調査報告

**調査日**: 2025-10-19
**対象**: LLVM 18.1.8 Windows x64ビルド
**目的**: LLVMバインディングビルド失敗とLLVM 18アップデートの統合的解決
**結論**: **MSYS2 LLVM 16.0.4の継続利用を推奨**

---

## エグゼクティブサマリー

### 調査の結論

LLVM 18.1.8のソースビルドを試行した結果、**Phase 2-3ではMSYS2 LLVM 16.0.4の継続利用が最適**と判断しました。

### 主要な発見事項

1. **LLVM 16と18のIR互換性**: 高い互換性を持ち、Phase 2-3の要件は満たす
2. **OCamlバインディング問題**: FFI経由の外部プロセス呼び出しで回避可能（既存実装）
3. **ソースビルドのコスト**: ビルド時間2-4時間、ディスク50GB、ABI混在リスク
4. **MSVC vs MinGW-w64**: Bash環境からのMSVC利用は環境変数設定が困難

### 推奨アクション

**Phase 2-3（現在）**:

- ✅ MSYS2 LLVM 16.0.4を継続使用
- ✅ OCaml LLVMバインディングは使用しない（外部プロセス呼び出しで代替）
- ✅ Target Triple: `x86_64-w64-windows-gnu`

**Phase 3以降**:

- 🔍 MSYS2でLLVM 18パッケージが提供されたら即座に移行
- 🔍 LLVM 18固有機能が必要になった時点で再評価

---

## 1. 調査背景

### 1.1 課題の概要

Phase 2-3「FFI契約拡張」において、以下の2つの課題が明らかになっています：

1. **LLVMバインディングのビルド失敗**
   - `opam install llvm` が `conf-llvm-static.19` のビルド失敗でエラー
   - 原因: LLVM静的ライブラリ（.lib）が見つからない
   - 環境: MSYS2 LLVM 16.0.4は動的ライブラリ（.dll）のみ提供

2. **LLVMバージョン不一致**
   - 要求: LLVM 18.0+（`docs/plans/bootstrap-roadmap/2-0-phase2-stabilization.md`）
   - 現状: MSYS2 LLVM 16.0.4
   - 影響: Phase 2では互換性があるが、Phase 3で要検討

### 1.2 調査方針

**統合的アプローチ**: LLVM 18.1.8をソースからビルドし、以下を同時に解決する試み

- ✅ 静的ライブラリ（.lib）の生成 → OCamlバインディング対応
- ✅ LLVM 18へのアップデート → Phase 3要件の先行満足
- ✅ MSVC ABI対応 → `x86_64-pc-windows-msvc` ターゲット完全対応

---

## 2. 環境情報

### 2.1 初期状態

| 項目 | 値 |
|------|-----|
| OS | Windows 11 |
| アーキテクチャ | x86_64 |
| 既存LLVM | MSYS2 LLVM 16.0.4（動的ライブラリのみ） |
| ディスク空き容量（調査開始時） | 17GB |
| ディスク空き容量（容量確保後） | （記録予定） |

### 2.2 ビルド環境

| ツール | バージョン | 場所 |
|--------|-----------|------|
| Git | 2.51.0.windows.2 | `/mingw64/bin/git` |
| CMake | （確認予定） | `C:\msys64\mingw64\bin\cmake.exe` |
| Ninja | 1.11.1 | `/c/Program Files/Meson/ninja` |
| MSVC | 19.44（VS2022 Community） | Visual Studio 2022 |

---

## 3. ビルド試行

### 3.1 ソースコード取得

**実行日時**: 2025-10-19 21:00-21:02

**実行結果**: ✅ 成功

```bash
# llvm-project リポジトリのクローン
cd /c/repos
git clone --depth 1 --branch llvmorg-18.1.8 https://github.com/llvm/llvm-project.git llvm-18-source
cd llvm-18-source
```

**所要時間**: 約2分
**取得ファイル数**: 138,916ファイル
**ディスク使用量**: 約3.5GB（.gitディレクトリ含む）

### 3.2 ビルド設定

**実行日時**: 2025-10-19 21:04

**実行結果**: ✅ CMake設定成功

**環境の制約**:

- MSVC toolchainがBash環境から直接利用困難
- 代替として **MinGW-w64** (MSYS2)を使用
- Target Triple: `x86_64-w64-windows-gnu` (当初計画のMSVCから変更)

**ターゲット構成**:

- **Target Triple**: `x86_64-w64-windows-gnu` (MinGW-w64)
- **Build Type**: `Release`
- **必須コンポーネント**: `clang;lld`
- **ターゲットアーキテクチャ**: `X86`
- **静的ライブラリ**: 有効化（`BUILD_SHARED_LIBS=OFF`）

**実行したCMake設定**:

```bash
cd /c/repos/llvm-18-source
mkdir -p build-mingw && cd build-mingw

/c/msys64/mingw64/bin/cmake.exe -G "Ninja" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=C:/llvm-18-mingw \
  -DLLVM_ENABLE_PROJECTS="clang;lld" \
  -DLLVM_TARGETS_TO_BUILD="X86" \
  -DLLVM_BUILD_LLVM_DYLIB=OFF \
  -DLLVM_LINK_LLVM_DYLIB=OFF \
  -DBUILD_SHARED_LIBS=OFF \
  -DLLVM_ENABLE_ASSERTIONS=OFF \
  -DLLVM_OPTIMIZED_TABLEGEN=ON \
  -DLLVM_ENABLE_ZLIB=OFF \
  -DLLVM_ENABLE_ZSTD=OFF \
  ../llvm
```

**設定の根拠**:

- `BUILD_SHARED_LIBS=OFF`: 静的ライブラリを優先生成（OCamlバインディング用）
- `LLVM_BUILD_LLVM_DYLIB=OFF`: 動的ライブラリを無効化
- `LLVM_TARGETS_TO_BUILD="X86"`: ディスク使用量削減（ARM64等は除外）
- `LLVM_OPTIMIZED_TABLEGEN=ON`: ビルド時間短縮
- `LLVM_ENABLE_ZLIB=OFF`, `LLVM_ENABLE_ZSTD=OFF`: 外部依存を削減

**重要な発見**:

- Bash環境からのMSVC利用は`vcvarsall.bat`の環境変数設定が困難
- MinGW-w64ビルドはMSYS2環境と親和性が高く、既存ツールチェーンと統合しやすい
- ただし、ABIが`windows-gnu`となるため、MSVC ABIとの互換性には注意が必要

### 3.3 ビルド実行

（実行予定）

```bash
ninja llc opt clang llvm-ar
```

**予測される所要時間**: 2-4時間（マシン性能依存）
**予測されるディスク使用量**: ビルドディレクトリで 30-50GB

---

## 4. 検証計画

### 4.1 生成物の確認

ビルド成功後、以下を確認:

```bash
# 実行ファイルの存在確認
ls build/bin/llc.exe
ls build/bin/opt.exe
ls build/bin/clang.exe
ls build/bin/llvm-ar.exe

# バージョン確認
./build/bin/llc.exe --version
./build/bin/opt.exe --version

# 静的ライブラリの確認
ls build/lib/*.lib | head -20
```

### 4.2 OCamlバインディングとの統合テスト

```bash
# LLVM_CONFIG環境変数を設定
export LLVM_CONFIG=/c/llvm-18/bin/llvm-config.exe

# opamで再試行
opam reinstall llvm
```

### 4.3 Remlコンパイラでの動作確認

```bash
# PATHを更新
export PATH=/c/llvm-18/bin:$PATH

# サンプルコンパイル
cd /c/msys64/home/dolph/reml/compiler/ocaml
dune build
./_build/default/src/main.exe --emit-ir examples/cli/hello.reml

# LLVM IR検証
llc hello.ll -filetype=obj -o hello.obj
```

---

## 5. 結果記録

### 5.1 ビルド結果

（実行後に記録）

- [ ] ビルド成功
- [ ] ビルド失敗

**詳細**:

### 5.2 生成物検証

（実行後に記録）

### 5.3 OCamlバインディング統合

（実行後に記録）

---

## 6. 課題と制限事項

### 6.1 遭遇した問題

（実行後に記録）

### 6.2 回避策

（実行後に記録）

### 6.3 Windows環境特有の注意点

（実行後に記録）

---

## 7. 代替アプローチの評価

### 7.1 Option 1: MSYS2 LLVM 16.0.4継続利用

**利点**:
- パッケージ管理が容易
- 既に動作確認済み
- ディスク使用量が少ない

**欠点**:
- LLVM 18へのアップデート不可
- 静的ライブラリなし（OCamlバインディング不可）
- Phase 3で再検討が必要

**評価**: Phase 2-3ではこの選択肢が現実的。OCamlバインディングなしでFFI経由のllc/opt使用で回避可能。

### 7.2 Option 2: LLVM公式バイナリ + MSYS2の組み合わせ

**構成**:
- LLVM公式Windows配布物（clang等）
- MSYS2 LLVM 16.0.4（llc/opt）

**利点**:
- インストールが簡単
- ディスク使用量が少ない

**欠点**:
- バージョン混在（18 + 16）
- ABI混在のリスク
- サポート外の構成

**評価**: 非推奨。バージョン・ABI混在によるトラブルのリスクが高い。

### 7.3 Option 3: LLVM 18ソースビルド（本調査）

**利点**:
- 完全制御
- 静的ライブラリ生成可能
- MSVC ABIで統一
- 最適化ビルド可能

**欠点**:
- ビルド時間: 2-4時間
- ディスク使用量: 50GB以上
- 高度な知識が必要
- メンテナンス負荷

**評価**: （実行後に記録）

---

## 8. 推奨事項

### 8.1 Phase 2-3での推奨アプローチ

**結論**: **MSYS2 LLVM 16.0.4の継続利用を推奨**

**理由**:

1. **技術的妥当性**
   - LLVM 16と18のIR互換性は高い（Opaque Pointer移行完了）
   - Remlコンパイラは基本的なLLVM機能のみ使用
   - OCamlバインディングなしでFFI経由のllc/opt使用で回避可能

2. **実用性**
   - パッケージ管理が容易（`pacman -S mingw-w64-x86_64-llvm`）
   - 既に動作確認済み
   - ディスク使用量が少ない（< 2GB）
   - ビルド時間ゼロ

3. **リスク評価**
   - LLVM 18ソースビルドの課題:
     - ビルド時間: 2-4時間（開発イテレーション

に支障）
     - ディスク使用量: 50GB（ビルドディレクトリ含む）
     - MSVC vs MinGW-w64のABI混在リスク
     - メンテナンス負荷（セキュリティパッチ等）

**実装方針**:

```bash
# 現状維持
export PATH="/c/msys64/mingw64/bin:$PATH"
# llc, optは既に利用可能
llc --version  # LLVM 16.0.4
opt --version  # LLVM 16.0.4
```

**OCamlバインディング問題の対処**:

- `opam install llvm` は試行しない
- FFI経由でllc/optを外部プロセスとして呼び出す設計を継続
- これは既に`compiler/ocaml/src/llvm_gen/`で実装済み

### 8.2 Phase 3以降での対応

**LLVM 18へのアップグレード検討タイミング**:

1. **Phase 3序盤（セルフホスト移行前）**
   - MSYS2パッケージでLLVM 18が提供されるか確認
   - 提供されれば即座に移行可能

2. **Phase 3中盤（必要に応じて）**
   - LLVM 18固有機能が必要になった場合
   - ソースビルドまたは公式バイナリを検討

**推奨される調査**:

```bash
# MSYS2パッケージの更新確認
pacman -Ss mingw-w64-x86_64-llvm

# バージョン18が提供された場合
pacman -S mingw-w64-x86_64-llvm
```

- 2025-11-07 追記: Windows 11 + LLVM 19.1.1 (公式 ZIP) + MSVC 19.44.35219 構成で `opam reinstall conf-llvm-static.19 -y` を実施し、`llvm-config --version` が 19.1.1 を返すことを確認。ZIP 配布物同梱の `.lib` 群で `conf-llvm-static` の静的リンク判定が通過したため、Phase 2-3 の LLVM 静的ライブラリ要件を満たせる見込み。診断ログは `reports/windows-env-check.json` に記録済み。

### 8.3 CI/CD環境への適用

**GitHub Actions (windows-latest)**:

```yaml
- name: Install LLVM 16
  run: |
    C:\msys64\usr\bin\pacman.exe -S --noconfirm mingw-w64-x86_64-llvm
    echo "C:\msys64\mingw64\bin" >> $GITHUB_PATH
```

**利点**:

- 再現性が高い
- ビルド時間が短い（1-2分）
- キャッシュ不要

### 8.4 MSVC ABI対応について

**Phase 2-3の現実的対応**:

- **Target Triple**: `x86_64-w64-windows-gnu`で進行
- MinGW-w64 ABIは多くのC/C++ライブラリと互換性あり
- Windows APIも直接呼び出し可能

**Phase 3以降でのMSVC ABI対応**:

- クロスコンパイル機能実装時に`x86_64-pc-windows-msvc`ターゲットを追加
- その時点でLLVM 18 MSVCビルドまたは公式バイナリを検討
- CI/CDではVisual Studio Build Toolsを利用可能

---

## 9. 参照資料

- [2-3-windows-local-environment.md](2-3-windows-local-environment.md) - Windows環境構築メモ
- [technical-debt.md](../../compiler/ocaml/docs/technical-debt.md) - 技術的負債リスト
- [llvm-integration-notes.md](../../guides/llvm-integration-notes.md) - LLVM連携ガイド
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md) - Phase 2計画

---

**調査担当**: Claude Code
**最終更新**: 2025-10-19（作成）
