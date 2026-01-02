# フェーズ 1: プロジェクトの立ち上げと環境構築

このフェーズでは、堅牢な開発環境、ビルドシステム、CI パイプラインの確立に焦点を当てます。

## 1.1 ディレクトリ初期化
- **アクション**: `1-0-architecture.md` で定義されたディレクトリ構造を作成する。
- **詳細**:
  - `compiler/c/src/` (main, driver, util)
  - `compiler/c/include/reml/`
  - `compiler/c/tests/`
  - `compiler/c/cmake/`
  - `compiler/c/deps/`
- **成果物**: 空のディレクトリとプレースホルダー。

## 1.2 依存関係管理戦略
- **決定されたライブラリセット**:
  - `uthash`, `argparse`, `yyjson`, `tomlc99`, `libtommath`, `utf8proc`, `libgrapheme`, `cmocka`, `log.c`, `tinydir`, `uuid4`, `BLAKE3`。
- **アクション**:
  - これらの C 依存関係を管理するために `FetchContent` (CMake) または `git submodule` を使用する。
  - バージョン固定を処理するための `cmake/FetchDependencies.cmake` を作成する。

## 1.3 ビルドシステム (CMake)
- **目標**: クロスプラットフォームビルドのサポート (macOS, Linux, Windows)。
- **設定**:
  - `CMakeLists.txt`:
    - 最小バージョン: 3.10
    - 言語: C11, C++17 (LLVM ブリッジ用)
    - デフォルトビルドタイプ: `Debug`
  - **フラグ**:
    - 警告: `-Wall -Wextra -Wpedantic` (GCC/Clang), `/W4` (MSVC)。
    - サニタイザ: Debug モードで `AddressSanitizer` と `UndefinedBehaviorSanitizer` を有効化。
- **成果物**: バイナリを生成する動作可能な CMake ビルド。

## 1.4 コマンドラインインターフェース (CLI) の骨子
- **ライブラリ**: `argparse`
- **実装**:
  - `src/main.c`: エントリーポイント。
  - 基本コマンドの実装: `reml version`, `reml help`。
- **検証**: `reml version` が現在のバージョンを表示する。

## 1.5 テストインフラ
- **ライブラリ**: `cmocka`
- **セットアップ**:
  - CMake で `enable_testing()`。
  - `cmocka` に接続する `tests/unit/test_main.c` を作成。
- **CI**:
  - `.github/workflows/c-build.yml` を作成。
  - マトリックス: `ubuntu-latest`, `macos-latest`, `windows-latest`。
  - ステップ: Configure, Build, Test (CTest)。

## 1.6 ログシステム
- **ライブラリ**: `log.c`
- **ラッパー**: ログマクロを抽象化し、CLI フラグ (`-v`, `-vv`) で詳細レベルを制御するための `src/util/logger.h` を作成する。

## チェックリスト
- [ ] ディレクトリ構造が作成された。
- [ ] 厳格な警告フラグを設定した `CMakeLists.txt` が構成された。
- [ ] 依存関係が取得/ベンダリングされた。
- [ ] `reml --version` が動作する。
- [ ] `logging` モジュールが動作する。
- [ ] ダミーテストが `ctest` でパスする。
- [ ] CI パイプラインが 3 つの OS すべてでパスする。
