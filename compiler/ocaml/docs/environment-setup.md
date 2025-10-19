# OCaml 開発環境セットアップガイド

**最終更新**: 2025-10-09
**対象 Phase**: Phase 1-3

このドキュメントでは、Reml OCaml コンパイラ（Phase 1-3）の開発環境をセットアップする手順を説明します。

## 目次

- [前提条件](#前提条件)
- [macOS (Apple Silicon/Intel)](#macos-apple-siliconintel)
- [Linux](#linux)
- [Windows (WSL推奨)](#windows-wsl推奨)
- [LLVM セットアップ（Phase 3 以降）](#llvm-セットアップphase-3-以降)
- [環境の確認](#環境の確認)
- [トラブルシューティング](#トラブルシューティング)

---

## 前提条件

以下のツールが必要です:

**Phase 1-2（必須）**:
- **OCaml**: >= 4.14 (推奨: 5.2.1)
- **Dune**: >= 3.0
- **Menhir**: >= 20201216
- **opam**: OCamlパッケージマネージャ

**Phase 3 以降（LLVM IR 生成に必要）**:
- **LLVM**: >= 15.0 (推奨: 15.0.7, 16.0.x, 17.0.x)
- **LLVM OCaml bindings**: opam経由でインストール

---

## macOS (Apple Silicon/Intel)

### 1. Homebrewのインストール

```bash
# Homebrewがインストールされていない場合
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

### 2. opamのインストール

```bash
brew install opam
```

### 3. opamの初期化

```bash
opam init --auto-setup --yes
eval $(opam env)
```

### 4. OCaml コンパイラのインストール

```bash
# OCaml 5.2.1をインストール（推奨）
opam switch create 5.2.1

# または、既存のスイッチを使用
opam switch 5.2.1

# 環境変数を更新
eval $(opam env --switch=5.2.1)
```

### 5. Dune と Menhir のインストール

```bash
opam install dune menhir --yes
```

### 6. 環境変数の永続化（オプション）

シェルの設定ファイル（`~/.zshrc` または `~/.bash_profile`）に以下を追加:

```bash
# opam環境変数の自動読み込み
eval $(opam env --switch=5.2.1)
```

---

## Linux

### Ubuntu/Debian系

```bash
# opamのインストール
sudo apt update
sudo apt install opam build-essential

# opamの初期化
opam init --auto-setup --yes
eval $(opam env)

# OCaml 5.2.1のインストール
opam switch create 5.2.1
eval $(opam env --switch=5.2.1)

# Dune と Menhir のインストール
opam install dune menhir --yes
```

### Fedora/RHEL系

```bash
# opamのインストール
sudo dnf install opam gcc make

# opamの初期化
opam init --auto-setup --yes
eval $(opam env)

# OCaml 5.2.1のインストール
opam switch create 5.2.1
eval $(opam env --switch=5.2.1)

# Dune と Menhir のインストール
opam install dune menhir --yes
```

### Arch Linux

```bash
# opamのインストール
sudo pacman -S opam base-devel

# opamの初期化
opam init --auto-setup --yes
eval $(opam env)

# OCaml 5.2.1のインストール
opam switch create 5.2.1
eval $(opam env --switch=5.2.1)

# Dune と Menhir のインストール
opam install dune menhir --yes
```

---

## Windows (WSL推奨)

Windowsでは、WSL (Windows Subsystem for Linux) を使用することを推奨します。

### 1. WSL2のインストール

```powershell
# PowerShellを管理者権限で実行
wsl --install
```

### 2. Ubuntu on WSLで開発環境をセットアップ

WSL内でUbuntuを起動し、上記の「Ubuntu/Debian系」の手順に従ってください。

### MSYS2環境 (Phase 2-3推奨)

Phase 2-3のWindows FFI契約拡張では、MSYS2環境を推奨します。

#### 1. MSYS2のインストール

[MSYS2公式サイト](https://www.msys2.org/)からインストーラをダウンロードし、実行してください。

#### 2. LLVM 16.0.4のインストール

```bash
# MSYS2 MinGW64シェルで実行
pacman -Syu
pacman -S mingw-w64-x86_64-llvm
```

**重要**: LLVM OCamlバインディングは **不要** です。

- Phase 2-3では外部プロセス (`llc`/`opt`) 呼び出しで対応します
- `opam install llvm` は試行しないでください (ビルド失敗します)
- 詳細: `compiler/ocaml/docs/technical-debt.md` §21

#### 3. OCamlとduneのインストール

```bash
# opamのインストール (WinGetを推奨)
winget install OCaml.opam

# PowerShellで環境変数を設定
$env:PATH = "$env:LOCALAPPDATA\Microsoft\WinGet\Links;" + $env:PATH

# opam初期化
opam init --disable-sandboxing

# OCaml 5.2.1スイッチの作成
opam switch create reml-521 5.2.1

# 必要パッケージのインストール (llvmを除く)
opam install dune menhir yojson ocamlformat
```

**注意**: `llvm` パッケージは **インストールしない** でください。

#### 4. PATH設定

PowerShellプロファイルに以下を追加:

```powershell
# MSYS2 LLVM (完全版) を優先
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH

# OCaml tools
$env:PATH = "C:\Users\<username>\AppData\Local\opam\reml-521\bin;" + $env:PATH

# WinGet Links
$env:PATH = "$env:LOCALAPPDATA\Microsoft\WinGet\Links;" + $env:PATH
```

詳細手順: `docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md`

### ネイティブWindows（WSL以外、非推奨）

WSLを使用しない場合は、上記のMSYS2環境を参照してください。
従来のネイティブWindowsセットアップは複雑であり、[OCaml公式サイト](https://ocaml.org/install#windows)を参照してください。

---

## LLVM セットアップ（Phase 3 以降）

Phase 3 以降で LLVM IR 生成機能を使用する場合は、LLVM ツールチェーンが必要です。

### macOS

```bash
# LLVM のインストール（バージョン 15-18 が対応）
# LLVM 18 を推奨（Phase 3 Week 12 で検証済み）
brew install llvm@18

# LLVM パスを環境変数に追加（~/.zshrc または ~/.bash_profile）
export PATH="/opt/homebrew/opt/llvm@18/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/llvm@18/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm@18/include"

# LLVM OCaml bindings のインストール
opam install llvm.18-static --yes
```

**Apple Silicon (M1/M2/M3) の注意事項**:
- LLVM は `/opt/homebrew/opt/llvm@18` にインストールされます
- Intel Mac の場合は `/usr/local/opt/llvm@18` です
- LLVM 15-19 の範囲で動作しますが、LLVM 18 を推奨（opaque pointer 対応済み）

### Linux

#### Ubuntu/Debian

```bash
# LLVM のインストール（バージョン 15-18 を推奨）
sudo apt update
sudo apt install llvm-18 llvm-18-dev

# または LLVM 15 を使用する場合
# sudo apt install llvm-15 llvm-15-dev

# LLVM OCaml bindings のインストール
opam install llvm.18-static --yes
```

#### Fedora/RHEL

```bash
# LLVM のインストール
sudo dnf install llvm18 llvm18-devel

# または LLVM 15 を使用する場合
# sudo dnf install llvm15 llvm15-devel

# LLVM OCaml bindings のインストール
opam install llvm.18-static --yes
```

### LLVM バージョンの確認

```bash
llvm-config --version
# 期待: 15.0.x, 16.0.x, 17.0.x, または 18.1.x
```

### LLVM OCaml bindings の確認

```bash
opam list | grep llvm
# 期待出力例:
# conf-llvm-static  18     Virtual package relying on llvm static library installation
# llvm              18-static  The OCaml bindings distributed with LLVM
```

### トラブルシューティング: opaque pointer 対応

LLVM 18 では opaque pointer がデフォルトになり、型付きポインタ API が廃止されています。
本プロジェクトは LLVM 18 の opaque pointer に対応済みです。

LLVM 15 を使用する場合は、以下を実行:

```bash
brew install llvm@15  # macOS
opam install llvm.15.0.7+nnp-3 --yes
```

---

## 環境の確認

セットアップが完了したら、以下のコマンドでバージョンを確認します:

```bash
# OCamlバージョン
ocaml --version
# 期待: The OCaml toplevel, version 5.2.1 (またはそれ以上)

# Duneバージョン
dune --version
# 期待: 3.0以上

# Menhirバージョン
menhir --version
# 期待: 20201216以上

# opamバージョン
opam --version
# 期待: 2.0以上
```

すべてのコマンドが正常に実行されれば、環境セットアップは完了です。

---

## トラブルシューティング

### `opam: command not found`

- **macOS**: `brew install opam`
- **Linux**: パッケージマネージャでopamをインストール（上記参照）
- **PATH設定**: `echo $PATH` を確認し、opamのバイナリパスが含まれているか確認

### `eval $(opam env)` が動作しない

シェルの種類によって構文が異なる場合があります:

```bash
# bash/zsh
eval $(opam env)

# fish
eval (opam env)
```

### Menhirのバージョンが古い

```bash
opam update
opam upgrade menhir
```

### OCamlのバージョンを切り替えたい

```bash
# 利用可能なスイッチを確認
opam switch list

# 特定のバージョンに切り替え
opam switch 5.2.1
eval $(opam env --switch=5.2.1)
```

### ビルドエラー: `dune: command not found`

opam環境変数が読み込まれていない可能性があります:

```bash
eval $(opam env --switch=5.2.1)
```

シェル再起動後も自動で読み込むには、`~/.zshrc` または `~/.bash_profile` に上記を追加してください。

---

## 次のステップ

環境セットアップが完了したら、[README.md](../README.md) の「ビルド方法」セクションに進んでください。

```bash
cd /path/to/kestrel/compiler/ocaml
dune build
dune test
```
