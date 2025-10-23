# 2025-10-23 セッション（Ubuntu 24.04 LTS 初期環境確認）

## セッションメタデータ
- **ホスト情報**: `uname -a` と `lsb_release -a` を取得済み。`hostnamectl` の詳細も併記。
- **実行ステップ**:
  1. `uname -a` / `lsb_release -a` で OS 情報を確認。
  2. `hostnamectl` でホストおよびファームウェア情報を確認。
  3. `dpkg -l | rg llvm` で system LLVM バージョンを確認。
  4. `dpkg -l | rg clang` で system Clang バージョンを確認。
  5. `sudo apt update && sudo apt upgrade` を実行し、`logs/2025-ubuntu2404-apt-upgrade.log` に保存。
- **結果概要**: すべてのコマンドが正常終了。`apt upgrade` は追加パッケージのダウンロードと適用を完了。
- **仮説**: LLVM 18/19/20 が混在しているため、opam スイッチ構築時は `$OPAM_SWITCH_PREFIX` 内ライブラリを優先させないとリンク競合が再発する可能性が高い。
- **CI 連携メモ**: GitHub Actions では system LLVM バージョンを明示的に固定し、ローカル環境との差分ログ（`llvm-config --version`）を収集する必要がある。

## uname -a 出力
```text
Linux dolphilia-HP-ProDesk-405-G8-Desktop-Mini-PC 6.14.0-33-generic #33~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Fri Sep 19 17:02:30 UTC 2 x86_64 x86_64 x86_64 GNU/Linux
```

## lsb_release -a 出力
```text
No LSB modules are available.
Distributor ID: Ubuntu
Description:    Ubuntu 24.04.3 LTS
Release:        24.04
Codename:       noble
```

## hostnamectl 出力
※ 絵文字はログ統一のため削除。

```text
Static hostname: dolphilia-HP-ProDesk-405-G8-Desktop-Mini-PC
Icon name: computer-desktop
Chassis: desktop
Machine ID: 51e5490d40104cd9a501853b1f5e4a60
Boot ID: 0b500fbd67bc426e962ec429dd9038a7
Operating System: Ubuntu 24.04.3 LTS
Kernel: Linux 6.14.0-33-generic
Architecture: x86-64
Hardware Vendor: HP
Hardware Model: HP ProDesk 405 G8 Desktop Mini PC
Firmware Version: T25 Ver. 02.15.00
Firmware Date: Mon 2024-12-30
Firmware Age: 9month 3w 2d
```

## dpkg -l | rg llvm
```text
ii  libllvm18:amd64                               1:18.1.3-1ubuntu1                        amd64        Modular compiler and toolchain technologies, runtime library
ii  libllvm19:amd64                               1:19.1.1-1ubuntu1~24.04.2                amd64        Modular compiler and toolchain technologies, runtime library
ii  libllvm20:amd64                               1:20.1.2-0ubuntu1~24.04.2                amd64        Modular compiler and toolchain technologies, runtime library
```

## dpkg -l | rg clang
```text
ii  libclang-cpp18                                1:18.1.3-1ubuntu1                        amd64        C++ interface to the Clang library
ii  libclang1-18                                  1:18.1.3-1ubuntu1                        amd64        C interface to the Clang library
```
