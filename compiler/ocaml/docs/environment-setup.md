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

### ネイティブWindows（非推奨）

ネイティブWindowsでのセットアップは複雑です。[OCaml公式サイト](https://ocaml.org/install#windows)を参照してください。

---

## LLVM セットアップ（Phase 3 以降）

Phase 3 以降で LLVM IR 生成機能を使用する場合は、LLVM ツールチェーンが必要です。

### macOS

```bash
# LLVM のインストール
brew install llvm@15

# LLVM パスを環境変数に追加（~/.zshrc または ~/.bash_profile）
export PATH="/opt/homebrew/opt/llvm@15/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/llvm@15/lib"
export CPPFLAGS="-I/opt/homebrew/opt/llvm@15/include"

# LLVM OCaml bindings のインストール
opam install llvm --yes
```

**Apple Silicon (M1/M2) の注意事項**:
- LLVM は `/opt/homebrew/opt/llvm@15` にインストールされます
- Intel Mac の場合は `/usr/local/opt/llvm@15` です

### Linux

#### Ubuntu/Debian

```bash
# LLVM のインストール
sudo apt update
sudo apt install llvm-15 llvm-15-dev

# LLVM OCaml bindings のインストール
opam install llvm --yes
```

#### Fedora/RHEL

```bash
# LLVM のインストール
sudo dnf install llvm15 llvm15-devel

# LLVM OCaml bindings のインストール
opam install llvm --yes
```

### LLVM バージョンの確認

```bash
llvm-config --version
# 期待: 15.0.x または 16.0.x, 17.0.x
```

### LLVM OCaml bindings の確認

```bash
opam list | grep llvm
# 期待: llvm.15.0.x+... (バージョンは LLVM に対応)
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
