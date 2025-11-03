# 2.6 Windows x64 (MSVC ABI) 対応計画

## 目的
- Phase 2 マイルストーン M4 に向けて、`-target x86_64-pc-windows-msvc` のビルドパイプラインを確立し、Windows 環境でのスモークテストを完了させる。
- System V ABI との差分を整理し、Phase 3 のクロスコンパイル機能拡張に備える。

## スコープ
- **含む**: LLVM TargetMachine 設定、MSVC 呼出規約対応、名前マングリング、PE 生成、GitHub Actions (windows-latest) テスト、ランタイムビルド、MinGW (x86_64-w64-windows-gnu) 向けビルドラインの維持。
- **含まない**: ARM64 Windows、UWP 対応。必要に応じて別計画とする。
- **前提**: Phase 1 の x86_64 Linux ターゲットが安定、Phase 2 の型クラス/効果/FFI 実装が Windows でビルドできるよう調整済み。`docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md` で整理した環境を最新診断結果と突き合わせながら更新する。
## 現状診断（2025-10-20）

- 2025-11-03 に `tooling/toolchains/check-windows-bootstrap-env.ps1` を再実行し、コンソール出力を確認（リポジトリが読み取り専用のため JSON は未更新）。以下の表へ結果を反映した。
- OCaml 本体／dune／menhir、MSVC ツールチェーンのアクティベーション、7-Zip など複数項目が未整備であり、LLVM CLI は clang 19.1.1（MSVC 配布）と MSYS2 16.0.4 の混在状態にある。

| 分類 | 項目 | 状態 | 備考 |
| --- | --- | --- | --- |
| コア | OCaml / opam / dune / menhir | ✅ | `ocaml` 5.2.1 / `dune` 3.20.2 / `menhir` 20250912 を確認。`opam` 2.4.1 で `reml-521` スイッチが利用可能。 |
| コア | Bash (MSYS2 / Git) | ⚠️ | `check-windows-bootstrap-env.ps1` を別セッションで実行すると WindowsApps の WSL `bash.exe` が優先される。Git Bash を確実に選択できるよう PATH 調整が必要。 |
| LLVM | clang / llc / opt | ✅ | `clang`/`llc`/`opt` は LLVM 19.1.1 (MSVC 配布 ZIP) を参照。`llc --version` で `x86_64-pc-windows-msvc` を確認。 |
| LLVM | llvm-ar | ✅ | 同じく LLVM 19.1.1 のバイナリを利用。 |
| MSVC | cl / link / lib | ⚠️ | `reml-msvc-env` 実行後は 19.44.35219 を検出。ただし未実行のままスクリプトを回すと Missing 扱いになるため、自動化が必要。 |
| ビルド支援 | CMake / Ninja | ✅ | `cmake` 3.29.5 / `ninja` 1.12.1 を確認。CMake の最小要件を満たした。 |
| 補助ツール | jq / 7zip / pip | ✅ | `jq` 1.8.1 / `7z` 25.01 / `pip` 25.2 を確認。成果物圧縮とログ整形が可能。 |

- LLVM 19.1.1 Windows X64 配布物（`C:\llvm\LLVM-19.1.1-Windows-X64`）には `clang.exe`・`llc.exe`・`opt.exe`・`lld-link.exe`・`llvm-ar.exe` が揃っている一方、現在 PATH で優先される `C:\Program Files\LLVM\bin` には `llc`/`opt` が含まれていない。
- `lib` 直下には 721 本の `.lib` が配置され、`LLVMAArch64CodeGen.lib` や `clang*.lib`、`lib\cmake\llvm\LLVMConfig.cmake` 等を確認済み。`.lib` 前提は満たせるため、コード生成 CLI は MSYS2 LLVM 16.0.4 を併用する暫定構成とする。


## 作業ディレクトリ
- `compiler/ocaml/` : Windows 対応ビルド設定・ターゲット切替
- `runtime/native/windows`（想定）: MSVC ABI 向けランタイム実装
- `runtime/native/mingw/`（想定）: MinGW 向け差分実装と抽象化ヘッダー
- `tooling/ci`, `.github/workflows/` : Windows ランナーの CI 定義と補助スクリプト
- `tooling/toolchains/` : Windows 診断スクリプトとログ（`check-windows-bootstrap-env.ps1`, `reports/windows-env-check.json`）
- `docs/guides/llvm-integration-notes.md`, `docs/spec/3-9-core-async-ffi-unsafe.md` : Windows 章の更新
- `docs/notes/llvm-spec-status-survey.md` : プラットフォーム差分・リスクの記録

## ビルド検証ログ（2025-11-03）

- `compiler/ocaml` 直下で `opam exec -- dune build` を実行したところ、LLVM OCaml バインディング（`llvm`, `llvm.bitwriter`）が見つからずビルドに失敗した。
- エラーログ抜粋:

```text
File "src/llvm_gen/dune", line 25, characters 2-16:
  llvm.bitwriter
Error: Library "llvm.bitwriter" not found.
File "tests/dune", line 60, characters 2-6:
  llvm
Error: Library "llvm" not found.
```

- 判明した課題:
  - `opam list --installed` に `llvm` パッケージが存在せず、`conf-llvm-static` 19 のみが導入されている。
  - 読み取り専用セッションでは `opam install llvm` を実行できないため、Windows 向けセットアップ手順に LLVM バインディング導入ステップを組み込む必要がある。
- 推奨対応:
  1. 書き込み可能なセッションで `opam install llvm --yes`（または `opam pin add llvm 19-static`）を実行し、`llvm.bitwriter` を含む OCaml バインディングを追加する。
  2. `tooling/toolchains/setup-windows-toolchain.ps1` に LLVM バインディング導入チェックを追加し、詳細手順を `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` に追記する（TODO）。
  3. `opam exec -- dune build` を Windows smoke テストの必須項目として `docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md` に連携し、環境診断で検出できるようにする。

## 作業ブレークダウン

### 1. Toolchain 調査と環境準備（17-18週目）
**担当領域**: Windows ビルド環境構築

1.1. **LLVM/MSVC バージョン選定**
- LLVM は 2025-10-20 時点で MSYS2 LLVM 16.0.4 をベースラインとしつつ、他プラットフォームで採用している LLVM 19.x への移行可否を評価する。調査内容: MSYS2/公式バイナリの入手性、`opam` の `conf-llvm-static.19` ビルド成否、ビルド時間・ディスクコスト、ABI 差異。結果は `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` と `docs/notes/llvm-spec-status-survey.md` に追記する。
- MSVC ツールチェーンは Visual Studio Build Tools 2022（MSVC 19.40 以上）と Windows 11/10 SDK (10.0.22621 以上) を標準とし、`vcvarsall.bat`・`reml-msvc-env` からの呼び出しフローを設計する。
- MinGW (x86_64-w64-windows-gnu) は MSYS2 LLVM 16.0.4 / GCC 13 系の組み合わせを維持し、LLVM 19.x 採用時の置換手順と互換性確認をまとめる。
- バージョン決定後は `0-3-audit-and-metrics.md` の Toolchain セクションと `reports/windows-env-check.json` のバージョン欄を更新する。
- 取得済みの LLVM 19.1.1 Windows X64 配布物（`C:\llvm\LLVM-19.1.1-Windows-X64`）の PATH 連携、`lib`/`include` 参照方法、`lib\cmake\llvm` の利用手順を整備する。
- 2025-11-03 再診断結果:
  - `check-windows-bootstrap-env.ps1` は `clang` 19.1.1（`C:\Program Files\LLVM\bin`）を認識したが `llc`/`opt` は PATH 外のため未検出。
  - `C:\llvm\LLVM-19.1.1-Windows-X64\bin` に `llc.exe` と `opt.exe` が揃っているため、PATH 切替または Symlink 化で MSVC 向け CLI を補完する。
  - CLI が揃わない場合は `windows-llvm-build-investigation.md` の結論どおり MSYS2 LLVM 16.0.4（`C:\msys64\mingw64\bin`）を継続利用する。
- 2025-11-03 PATH 再構成:
  - PowerShell 7 プロファイル（`Documents\PowerShell\Microsoft.PowerShell_profile.ps1`）を更新し、`Add-RemlPathFront` 関数で `C:\llvm\LLVM-19.1.1-Windows-X64\bin` を最優先に追加。
  - ユーザー環境変数 `Path` にも同ディレクトリを追加済み。新規セッションで `llc`/`opt`/`clang` が MSVC 配布物を指すことを `Get-Command` で確認した。
  - MSYS2 CLI（16.0.4）はバックアップ用途として PATH 後段に配置。混在時の運用ルールを `windows-llvm-build-investigation.md` に反映予定。
- 2025-11-07 追記:
  - PowerShell プロファイルを読み込んだ状態で `check-windows-bootstrap-env.ps1` を実行し、`llc`/`opt`/`clang` が LLVM 19.1.1 (MSVC 配布 ZIP) を指すことを確認。
  - プロファイルを読み込まないサブプロセスでも同じ PATH になるよう、診断スクリプト側に `Add-RemlPathFront` 相当の初期化を追加するタスクを登録。

1.2. **開発環境セットアップ**
- Windows 10/11 でのビルド環境構築手順書作成
- LLVM のインストール手順（公式ビルド or 自前ビルド）
- OCaml / opam / dune / menhir の既存セットアップ状況を確認し、差分があれば `docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md` を更新する。
- MSVC コンパイラ（`cl.exe`）とリンカ（`link.exe`）の設定
- 環境変数の設定（PATH, INCLUDE, LIB）
- 2025-11-03 再診断結果:
  - `opam` 2.4.1 は検出されたが `ocaml`/`dune`/`menhir`/`yojson`/`camlzip` が未導入。`opam switch create reml-5.2.1 5.2.1` → `opam install --deps-only ./compiler/ocaml/reml_ocaml.opam` を次回実行する（履歴）。
  - PowerShell の既定 `bash` が WindowsApps の WSL エイリアスを指すため、`C:\Program Files\Git\bin` を PATH 先頭へ追加する手順を `2-3-windows-local-environment.md` に追記予定。
  - `7z` が未導入で成果物圧縮ができない。`winget install 7zip.7zip` を導入後、環境診断スクリプトの期待値を更新する。
- 2025-11-03 PATH 再構成:
  - PowerShell プロファイル関数で PATH 追加順序を一元管理。`C:\Program Files\7-Zip`、`%LOCALAPPDATA%\opam\reml-521\bin`、`%LOCALAPPDATA%\Microsoft\WinGet\Links` を同一関数経由で先頭に追加するよう調整。
  - `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned` を適用済み。プロファイル読込エラー（PSSecurityException）を解消し、定義済み関数 `reml-msvc-env` の動作を確認。
- 2025-11-07 フォローアップ:
  - `ocaml` 5.2.1 / `dune` 3.20.2 / `menhir` 20250912 の導入を確認。`reml-env-check` で各コマンドが Present = True となることを記録。
  - `winget install 7zip.7zip` と Kitware オフィシャルインストーラで `cmake` 3.29.5 を導入し、診断スクリプトでのバージョン要件を満たした。
  - 現行スクリプトは新しい PowerShell プロセスを起動する際にプロファイルを読み込まないため、Bash が WindowsApps（WSL）を指す。Git Bash を強制するか、プロファイルロードを明示する必要あり。

1.3. **CI 環境セットアップ**
- GitHub Actions `windows-latest` ランナーの調査
- キャッシュ戦略（LLVM/MSVC のキャッシュ）
- セットアップスクリプトの作成（PowerShell/Batch）
- 環境診断スクリプト（`tooling/toolchains/check-windows-bootstrap-env.ps1`）をジョブ冒頭で実行し、JSON 出力をアーティファクト化する。
- ビルド時間の初期計測
- 2025-11-03 検討メモ:
  - Actions `windows-latest` で `C:\llvm\LLVM-19.1.1-Windows-X64\bin` を `Add-Content $env:GITHUB_PATH` するセットアップ案を作成。
  - CI 冒頭で `tooling/toolchains/check-windows-bootstrap-env.ps1 -OutputJson ${{ runner.temp }}/windows-env-check.json` を実行し、アーティファクトとして保存する運用を追加する。
  - CMake 3.26.4 → 3.29 への更新が必要なため、CI では `choco install cmake --version 3.29.2` を使った整合化を検討する。
- 2025-11-03 フォローアップ:
  - ローカルで PowerShell プロファイル経由の PATH 調整が有効化されたため、Actions 側でも同等の PATH 付与スクリプトを `setup-windows-toolchain.ps1` として共通化する案をタスク化。
  - `check-windows-bootstrap-env.ps1` の PATH 検証結果と CLI 優先順位を CI ログへ明示するチェックポイントを追加。
- 2025-11-07 更新:
  - `tooling/toolchains/setup-windows-toolchain.ps1` を追加し、PATH 初期化と `reml-msvc-env` 呼び出しを共通化。`check-windows-bootstrap-env.ps1` は同スクリプトをドットソースしてから診断を実行する構成へ更新。
  - `reml-msvc-env` 実行後に `check-windows-bootstrap-env.ps1` を呼び出すと `cl`/`link`/`lib` (19.44.35219) が検出されることを確認。CI 側でも診断前に `vcvars64.bat` を呼び出す共通ステップが必要。
  - プロファイルを読み込むまで `cl.exe` が Missing になるため、セットアップスクリプトに `reml-msvc-env` 呼び出しを組み込むタスクを継続。

1.4. **LLVM 19 利用性の再評価**
- Linux/macOS で LLVM 19.x を採用している構成と依存バージョンを洗い出し、Windows で再現するための入手経路（MSYS2、LLVM 公式インストーラ、ソースビルド）を比較する。
- `opam install llvm` が要求する `conf-llvm-static.19` のビルドログを取得し、MSVC と MinGW の両ターゲットで利用可能か検証する。
- 調査の成否・阻害要因（欠落ライブラリ、ビルド時間、ABI 差分）と代替策を `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` と `docs/notes/llvm-spec-status-survey.md` に記録する。
- LLVM 16 → 19 への移行判定チェックリストを `0-3-audit-and-metrics.md` に追加し、フェーズ移行時の意思決定材料を整備する。
- ダウンロード済み配布物に同梱されている `libclang.lib`、`c++.lib`、`clang` 系 `.lib` のリンク可否と `LLVMConfig.cmake` 経由の検出結果を記録する。
- 2025-11-03 再評価:
  - 19.1.1 配布物の `.lib` 群は `conf-llvm-static.19` の要件を満たすため、`LLVMConfig.cmake` の検出テストを後続タスクに設定する。
  - CLI の併用ポリシー（MSVC 19 vs MinGW 16）を `0-3-audit-and-metrics.md` の Toolchain セクションで管理し、PATH 切替前提を共有する。
  - `docs/notes/llvm-spec-status-survey.md` に Program Files 版と ZIP 版でバンドル内容が異なる旨を追記し、配布差異を記録する。
- 2025-11-03 進捗:
  - ZIP 版配布物を PATH 先頭で利用できる状態に整理し、`Get-Command llc/opt/clang` で MSVC 配布物を参照していることを確認済み。
  - 次ステップでは `opam install conf-llvm-static.19` で `.lib` 検出の可否を確認し、結果を `windows-llvm-build-investigation.md` に追記する。
- 2025-11-07 メモ:
  - LLVM 19.1.1 の CLI を用いた `llc --version` / `opt --version` の確認を実施。`.lib` 群は引き続き揃っているが、`conf-llvm-static.19` インストールテストは未着手のためフォローアップが必要（→ 2025-11-07 再検証で完了）。
  - `opam reinstall conf-llvm-static.19 -y` を実行し、ZIP 版配布物同梱の `llvm-config.exe` (19.1.1) を検出できることを確認。`llvm-config --version` の出力と `conf-llvm-static` 成功ログを `windows-llvm-build-investigation.md` に反映予定。

### 進捗サマリ（2025-11-07）
- ✅ PATH 初期化と MSVC アクティベーションを `setup-windows-toolchain.ps1` に集約し、`check-windows-bootstrap-env.ps1` が自動で呼び出す構成へ統一。
- ✅ `reml-env-check -OutputJson reports/windows-env-check.json` を再実行し、LLVM 19.1.1 / MSVC 19.44.35219 / CMake 3.29.5 / 7-Zip 25.01 を検出できる最新診断ログを更新。
- ✅ `opam reinstall conf-llvm-static.19 -y` で LLVM 19.1.1 ZIP 配布物の `.lib` を認識できることを確認し、`llvm-config --version` で 19.1.1 を取得。
- ⚠️ CI ドキュメントへの PATH / `vcvars64.bat` 呼び出し手順の反映と、診断ログのアーティファクト化ルールが未整備。
- ⚠️ `windows-llvm-build-investigation.md` および `docs/notes/llvm-spec-status-survey.md` へ最新の `conf-llvm-static` 検証結果と ZIP 配布利用ポリシーを追記する作業が残存。

### 残タスク / 次ステップ
1. CI ドキュメントへ PATH 再構成と `vcvars64.bat` 呼び出し順序を追記し、`check-windows-bootstrap-env.ps1` のログをアーティファクト化する運用を確立する。

**成果物**: Toolchain 選定書（MSVC/MinGW/LLVM 16→19 評価）、セットアップ手順、CI スクリプト、診断ログ

### 2. ABI 差分の調査と整理（18-19週目）
**担当領域**: ABI 互換性調査
**ステータス**: ✅ 完了 (2025-11-09)

2.1. **Calling Convention の調査**
- System V AMD64 ABI (Linux) と x64 calling convention (Windows) の差分
- 整数・浮動小数点引数の渡し方の違い
- 構造体引数の扱い（値渡し vs ポインタ渡し）
- 戻り値の扱い（RVO/NRVO の差異）

2.2. **構造体レイアウトの調査**
- アライメント規則の差異
- パディングの挿入ルール
- ビットフィールドのレイアウト
- 可変長配列の扱い

2.3. **名前マングリングの調査**
- シンボル名のマングリング規則（`_` プレフィックス等）
- DLL エクスポート/インポート（`__declspec(dllexport/dllimport)`）
- `extern "C"` の挙動差異
- Phase 2 FFI タスクとの連携

- 2025-11-09 調査サマリ:
  - `docs/notes/llvm-spec-status-survey.md` §2.2.2a を拡張し、呼出規約／構造体レイアウト／名前マングリング／DLL 公開手順の比較表を追加。
  - Win64 Shadow Space と `byval`/`sret` 属性の適用条件を Reml 実装メモへ記録し、`examples/ffi/windows/struct_passing.reml` の期待値と照合。
  - `docs/guides/reml-ffi-handbook.md` の Win64 章更新 TODO を追記（ステップ3でのガイド同期タスク）。
- フォローアップ:
  - LLVM IR 拡張フェーズで `win64cc` 属性と Shadow Space 予約を実装・検証（ステップ3へ引き継ぎ）。
  - GitHub Actions `windows-latest` ジョブへ ABI 定点テストを追加し、Shadow Space 未確保時の診断を自動化（`docs/guides/ci-strategy.md` 更新タスク）。

**成果物**: ABI 差分レポート（`docs/notes/llvm-spec-status-survey.md` §2.2.2a 更新）、関連ガイド更新タスク

### 3. LLVM IR 生成の拡張（19-20週目）
**担当領域**: コード生成
**ステータス**: 🟡 進行中 (2025-11-12)

3.1. **ターゲット切替ロジックの実装**
- ✅ CLI フラグ `--target` を `compiler/ocaml/src/cli/options.ml` から `llvm_gen/target_config.ml` に伝搬し、`TargetMachine` 生成時に `x86_64-pc-windows-msvc` / `x86_64-w64-windows-gnu` の双方を選択できる設計草案を作成。CLI のヘルプ更新案を `docs/guides/llvm-integration-notes.md` に TODO として追加。
- ✅ LLVM `DataLayout` 文字列の差分を整理し、MSVC 向けは `e-m:w-i64:64-f80:128-n8:16:32:64-S128`、MinGW 向けは既存レイアウトを維持する方針を確定。`llc -mtriple=x86_64-pc-windows-msvc -filetype=obj smoke_test.ll` を用いた検証手順をドラフト化（`windows-llvm-build-investigation.md` 更新準備）。
- 🟡 CLI/ターゲット切替テスト: `compiler/ocaml/tests/llvm-ir/target_selection/` に MSVC/MinGW の比較ゴールデン (`msvc_shadow_space.ll`, `gnu_shadow_space.ll`) と CLI スナップショット (`test_cli_target_switch.ml`) を追加するブランチを作成済み。`dune runtest codegen` で新テストが実行されることを確認する手順メモを整備し、2025-11-15 までにレビューへ提出予定。

3.2. **Calling Convention の適用**
- ✅ `docs/notes/llvm-spec-status-survey.md` §2.2.2a の Shadow Space 調査結果を反映し、`compiler/ocaml/src/llvm_gen/abi.ml` へ `Win64` 分岐（`CallingConv.win64`, `sret`, `byval align 8`）を導入する実装メモを作成。構造体引数と戻り値の lowering ルールを整理。
- ✅ `examples/ffi/windows/struct_passing.reml` の期待 IR と照合し、MinGW/MSVC 共通で Shadow Space 32 バイトを確保する必要があることを確認。`llvm_gen/codegen.ml` での `add_function_attribute` 追加ポイントを特定。
- 🟡 Shadow Space FileCheck: `compiler/ocaml/tests/llvm-ir/win64_shadow_space.ll` に `CHECK: call void @llvm.frameescape` ベースの検証を追加する案を作成し、`tooling/scripts/run-win64-filecheck.ps1` で `llc -mtriple=x86_64-pc-windows-msvc` → `FileCheck` を自動化する下書きを作成。2025-11-18 までに `.ps1` 実装とテストデータを確定させる。

3.3. **デバッグ情報の生成**
- ✅ Windows では CodeView/PDB を既定、`--emit-dwarf` 指定時に DWARF を出力する切替を `compiler/ocaml/src/llvm_gen/codegen.ml`/`debug_info` で制御する設計をまとめ、Visual Studio / WinDbg での検証項目を列挙。
- ✅ `DICompileUnit` のソースパス正規化（`\\` → `/`）と `DW_LANG_C_plus_plus_17` など言語識別子の更新を含むチェックリストを作成し、`docs/guides/llvm-integration-notes.md` の追記箇所を特定。
- 🟡 PDB スモークテスト: `tooling/toolchains/test-pdb-smoke.ps1` で `llc -filetype=obj` → `lld-link /DEBUG` → `llvm-pdbutil -summary` を連鎖実行し、`Summary:` 内に `Publics` と `Globals` が出力されることを確認する自動化スクリプトの骨子を作成。CI 取り込みに向けてアーティファクト保存（`artifact: pdb-smoke-report.txt`）の仕様書きを 2025-11-19 までに確定する。

#### 進捗サマリ（2025-11-12）
- LLVM ターゲット切替経路と DataLayout 定義を確定し、CLI・生成器それぞれの修正ポイントを `compiler/ocaml/src/cli` / `llvm_gen` 配下に集約。
- Win64 calling convention の属性セット（`win64cc`/`sret`/`byval`/Shadow Space）を整理し、Phase 2 ABI 調査と整合する lowering ルールを明文化。
- PDB と DWARF の切替条件、および Visual Studio / WinDbg での検証シナリオを定義し、ガイド更新と CI 自動化へ橋渡しするタスクリストを作成。

#### フォローアップ
1. ターゲット切替テストブランチのレビューと `compiler/ocaml/tests/llvm-ir/target_selection` ゴールデン取り込み（担当: Windows チーム、期限 2025-11-15、参照: `compiler/ocaml/tests/llvm-ir`）。
2. Shadow Space FileCheck スクリプト (`tooling/scripts/run-win64-filecheck.ps1`) とテスト資産のコミット、`docs/notes/llvm-spec-status-survey.md` §2.2.2a への結果追記（期限 2025-11-18）。
3. PDB スモークテスト自動化 (`tooling/toolchains/test-pdb-smoke.ps1`) を CI に統合し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md`・`docs/guides/llvm-integration-notes.md` に運用手順を追記（期限 2025-11-19、担当: CI チーム）。

### 4. ランタイム C コードの移植（20-21週目）
**担当領域**: ランタイム実装
**ステータス**: 🟢 完了 (2025-11-12)

4.1. **プラットフォーム依存コードの分離**
- ✅ `runtime/native/include/reml_platform.h`・`reml_atomic.h` を新設し、MSVC/Clang/GCC 向けの属性・アトミック互換レイヤーを集約。`panic.c`, `mem_alloc.c`, `refcount.c`, `ffi_bridge.c` から直接のコンパイラ判定を排除。
- ✅ `_Thread_local` 非対応の MSVC に合わせて `REML_THREAD_LOCAL` を導入し、エラーステートやデバッグ統計のスレッドローカル管理を統一。

4.2. **Windows API への対応**
- ✅ `mem_alloc.c` を `VirtualAlloc`/`VirtualFree` ベースへ切替え、失敗時に `GetLastError` を取得。POSIX 側は既存の `malloc`/`free` 実装を保持。
- ✅ `panic.c` で `GetCurrentProcessId`・`localtime_s` を利用する共通ヘルパーを実装し、Windows/Unix 双方で同一フォーマットの診断メッセージを出力。
- ✅ `runtime/native/include/reml_os.h` と `src/os.c` を追加し、`CreateFileW`/`ReadFile`/`WriteFile`/`CreateThread` をラップしたクロスプラットフォーム OS API を提供（POSIX 版は `open`/`read`/`write`/`pthread`）。`reml_os_last_error_message` で `GetLastError` / `errno` の情報を統一的に取得可能。

4.3. **MSVC ビルドの実装**
- ✅ `runtime/native/CMakeLists.txt` を追加し、`cl.exe`/`lib.exe` による静的ライブラリ構築と `ctest` 連携をサポート。`REML_RUNTIME_ENABLE_DEBUG` オプションと `/W4` / `/permissive-` の警告設定を適用。
- ✅ `runtime/native/README.md` に Windows (MSVC) ビルド手順を追記し、`reml-msvc-env` → `cmake --build` → `ctest` のフローを明文化。
- ✅ MinGW 向け `Makefile` と MSVC 向け CMake が共存できるよう、OS 抽象レイヤー (`reml_platform.h`, `reml_os.h`) を整備し API 差分を吸収。

**成果物**: `runtime/native/include/reml_platform.h`, `runtime/native/include/reml_atomic.h`, `runtime/native/include/reml_os.h`, `runtime/native/src/os.c`, `runtime/native/CMakeLists.txt`, `runtime/native/README.md`（MSVC 手順追記）

#### フォローアップ
1. `reml_os_*` ラッパをランタイムの診断ログ／標準ライブラリ入出力へ組み込み、ユニットテストを追加する（Phase 2 テスト整備と連動）。
2. Windows 版での `reml_os_thread_*` 活用シナリオ（将来の GC/ワーカースレッド等）を整理し、`docs/spec/3-8-core-runtime-capability.md` の Capability テーブルへ反映。
3. `reml_os_last_error_message` を診断パイプライン（`docs/plans/bootstrap-roadmap/2-3-ffi-contract-extension.md`）と統合し、監査ログへエラーコードを記録するタスクを生成。

### 5. テスト実装とデバッグ（21-22週目）
**担当領域**: テスト整備

5.1. **スモークテストの実装**
- Parser のテスト（Windows パス対応）
- Typer のテスト
- LLVM IR 生成のテスト
- ランタイムリンクのテスト

5.2. **サンプルプログラムの実行**
- `examples/` 以下のプログラムを Windows でビルド
- 実行テスト（出力の検証）
- エラーケースのテスト
- メモリリークの検出（Application Verifier）

5.3. **デバッグとバグ修正**
- クラッシュの調査（WinDbg, Visual Studio）
- ABI 関連のバグ修正
- ランタイムのバグ修正
- エッジケースの追加テスト

#### 進捗サマリ（2025-11-15）
- ✅ スモークテスト対象を `compiler/ocaml/tests/dune` から抽出し、Windows で優先確認すべきテストバイナリと実行コマンドを洗い出した。実施順と確認観点を下表に整理。
- ✅ `examples/ffi/windows/`（`messagebox.reml` / `struct_passing.reml` / `ownership_transfer.reml`）を中心にサンプル実行パスを選定し、`remlc --target x86_64-pc-windows-msvc` でのビルド手順を PowerShell 前提で固めた。
- ✅ WinDbg / Visual Studio でのクラッシュ解析手順と PDB 取得経路を整理し、`tooling/toolchains/setup-windows-toolchain.ps1` を起点とした環境初期化フローに紐付けた。
- 🟡 ランタイム統合テストは `compiler/ocaml/tests/test_runtime_integration.sh` を PowerShell 版へ移植し、MSVC 版ランタイムと連携させる作業が未完。移植後に `Application Verifier` を含むリーク検出手順を自動化する。

#### Windows スモークテスト実行メモ（2025-11-15）
事前に `pwsh -NoLogo -File tooling/toolchains/setup-windows-toolchain.ps1 -NoCheck` を実行し、PATH と MSVC 環境を初期化する。

| カテゴリ | 実行コマンド例 | 主な確認ポイント | 状態 |
| --- | --- | --- | --- |
| Parser | `opam exec -- dune runtest compiler/ocaml/tests/test_parser.exe` | Windows パス区切り（`\\`）を含むエラーメッセージと `stdin` 経由入力の挙動 | 準備完了（要実行ログ） |
| Parser (補足) | `opam exec -- dune runtest compiler/ocaml/tests/test_parser_expectation.exe` | `expect` ファイルの改行コード（CRLF）差分検出 | 準備完了（要実行ログ） |
| Typer | `opam exec -- dune runtest compiler/ocaml/tests/test_type_inference.exe` | `StageRequirement` の検証が Linux と同一結果になるか | 準備完了（要実行ログ） |
| LLVM IR | `opam exec -- dune runtest compiler/ocaml/tests/test_cli_callconv_snapshot.exe` | `x86_64-pc-windows-msvc` 向け DataLayout / CallingConv の差分スナップショット | 準備完了（要実行ログ） |
| Runtime/FFI | `opam exec -- dune runtest compiler/ocaml/tests/test_ffi_lowering.exe` | `sret` / `byval` 属性と `lld-link` への引数構成 | 準備完了（要実行ログ） |
| Runtime Smoke | `opam exec -- dune runtest compiler/ocaml/tests/test_abi.exe` | Shadow Space / Struct 受け渡しの FileCheck マーカー | 準備完了（要実行ログ） |

#### サンプルプログラム検証メモ（2025-11-15）

| プログラム | ビルドコマンド例 | 確認ポイント | 状態 |
| --- | --- | --- | --- |
| `examples/ffi/windows/messagebox.reml` | `opam exec -- dune exec -- remlc .\examples\ffi\windows\messagebox.reml --target x86_64-pc-windows-msvc --emit-ir --link-runtime --out-dir .\build\windows\messagebox` | `MessageBoxW` 呼び出しでの `stdcall` / UNICODE 変換と `lld-link /SUBSYSTEM:WINDOWS` 指定 | 準備完了（生成物確認待ち） |
| `examples/ffi/windows/struct_passing.reml` | `opam exec -- dune exec -- remlc .\examples\ffi\windows\struct_passing.reml --target x86_64-pc-windows-msvc --emit-obj --out-dir .\build\windows\struct` | 構造体の `byval align 8` 振る舞いと `WinDbg` でのレジスタ確認 | 準備完了（生成物確認待ち） |
| `examples/ffi/windows/ownership_transfer.reml` | `opam exec -- dune exec -- remlc .\examples\ffi\windows\ownership_transfer.reml --target x86_64-pc-windows-msvc --emit-ir --link-runtime --out-dir .\build\windows\ownership` | `RuntimeBridge` 経由の参照カウントと `AuditEnvelope.metadata` の整合 | 準備完了（生成物確認待ち） |

#### デバッグ手順とフォローアップ
- WinDbg でのクラッシュ調査: `lld-link /DEBUG` で生成した `.pdb` を同ディレクトリへ配置し、`windbg.exe -z .\build\windows\struct\struct_passing.exe` を実行。例外発生時は `!analyze -v` の結果を `docs/notes/llvm-spec-status-survey.md` に転記。
- Visual Studio デバッガーでの動的検証: `devenv /debugexe` で実行し、`compiler/ocaml/src/llvm_gen/abi.ml` の `Win64` 分岐にブレークポイントを設定して引数レイアウトを検証。
- Application Verifier 導入手順: `appverif.exe /verify struct_passing.exe /tests Heaps Handles` をサンプルごとに実行し、ログを `%LOCALAPPDATA%\Reml\logs\appverifier\` へ収集。PowerShell 版ランタイム統合テストへ組み込み予定。

#### 残タスク / 次ステップ
1. `compiler/ocaml/tests/test_runtime_integration.sh` をベースに PowerShell 版統合テスト（仮称: `test_runtime_integration.ps1`）を実装し、MSVC 版ランタイムへのリンクと `LLVM` CLI 呼び出しを自動化する。
2. 上記スモークテスト runlist を GitHub Actions (windows-latest) で実行し、`reports/windows-smoke-*.log` をアーティファクト化する運用を確立する。
3. Application Verifier の結果を CI アーティファクトとして保存し、`docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` に検証ログの保存場所と運用ルールを追記する。

**成果物**: Windows テストスイート、バグ修正

### 6. GitHub Actions 統合（22-23週目）
**担当領域**: CI/CD

6.1. **Windows ジョブの追加**
- `.github/workflows/` へ Windows ジョブ追加
- セットアップステップ（LLVM/MSVC インストール）
- ビルドステップ（OCaml コンパイラ、Reml ツールチェーン）
- MinGW 向けの追加ジョブ（MSYS2 環境でのビルド）をマトリクスに組み込み、GNU ABI の回帰テストを維持する。
- テストステップ（全テストの実行）

6.2. **並行実行とキャッシュ**
- Linux/Windows ジョブの並行実行
- LLVM/MSVC のキャッシュ設定
- ビルド成果物のアーティファクト保存
- ビルド時間の最適化

6.3. **テスト結果の報告**
- テスト失敗時のログ出力
- 診断メッセージの CI への表示
- PR へのコメント自動投稿
- Phase 1/2 の CI との統合

**成果物**: Windows CI ジョブ、並行実行設定

### 7. ドキュメント整備（23週目）
**担当領域**: ドキュメント

7.1. **セットアップ手順の文書化**
- `docs/guides/llvm-integration-notes.md` への Windows セクション追加
- 環境変数の設定方法
- トラブルシューティング情報
- よくある質問（FAQ）

7.2. **ABI 差分の文書化**
- `docs/notes/llvm-spec-status-survey.md` への差分レポート追記
- Calling convention の比較表
- 構造体レイアウトの例示
- 名前マングリングの規則

7.3. **メトリクスの記録**
- `0-3-audit-and-metrics.md` へのビルド時間記録
- テストカバレッジ（Linux vs Windows）
- バイナリサイズの比較
- CI 実行時間の記録

**成果物**: 更新ガイド、差分レポート、メトリクス

### 8. コードサイニング調査と Phase 3 準備（23-24週目）
**担当領域**: リリース準備

8.1. **コードサイニング調査**
- コードサイニング証明書の必要性調査
- 取得方法（EV 証明書 vs Standard 証明書）
- コスト・期間の見積もり
- SmartScreen 対策の検討

8.2. **署名プロセスの設計**
- `signtool.exe` での署名自動化
- タイムスタンプサーバの設定
- CI での署名フロー（シークレット管理）
- Phase 4 リリース準備への引き継ぎ

8.3. **Phase 3 準備**
- Windows でのセルフホスト計画
- クロスコンパイル機能の拡張検討
- 残存課題の `0-4-risk-handling.md` への記録
- Windows 固有の最適化機会の特定

**成果物**: サイニング調査レポート、Phase 3 準備文書

## 成果物と検証
- Windows ジョブが安定稼働し、`llc -mtriple=x86_64-pc-windows-msvc` で生成したバイナリが実行可能であること。
- `llc -mtriple=x86_64-w64-windows-gnu` で生成したバイナリが従来どおり動作すること。
- ABI 差分がドキュメント化され、レビュー記録が残る。
- CLI で `--target x86_64-pc-windows-msvc` / `--target x86_64-w64-windows-gnu` を指定可能になり、テストケースが通過。

## リスクとフォローアップ
- CI の時間が延びる場合は nightly ジョブと PR ジョブを分離する。
- Windows 固有のファイルパス・改行問題に対応するため、テストで共通抽象化を導入。
- 署名や配布のプロセスは Phase 4 で本格化するため、必要な下調べを `docs/notes/` に記録。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [notes/llvm-spec-status-survey.md](../../notes/llvm-spec-status-survey.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
- [2-3-windows-local-environment.md](2-3-windows-local-environment.md)
- [windows-llvm-build-investigation.md](windows-llvm-build-investigation.md)
