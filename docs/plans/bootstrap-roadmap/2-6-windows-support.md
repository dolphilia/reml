# 2.6 Windows x64 (MSVC ABI) 対応計画

## 目的
- Phase 2 マイルストーン M4 に向けて、`-target x86_64-pc-windows-msvc` のビルドパイプラインを確立し、Windows 環境でのスモークテストを完了させる。
- System V ABI との差分を整理し、Phase 3 のクロスコンパイル機能拡張に備える。

## スコープ
- **含む**: LLVM TargetMachine 設定、MSVC 呼出規約対応、名前マングリング、PE 生成、GitHub Actions (windows-latest) テスト、ランタイムビルド、MinGW (x86_64-w64-windows-gnu) 向けビルドラインの維持。
- **含まない**: ARM64 Windows、UWP 対応。必要に応じて別計画とする。
- **前提**: Phase 1 の x86_64 Linux ターゲットが安定、Phase 2 の型クラス/効果/FFI 実装が Windows でビルドできるよう調整済み。`docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md` で整理した環境を最新診断結果と突き合わせながら更新する。
## 現状診断（2025-10-20）

- `tooling/toolchains/check-windows-bootstrap-env.ps1` を 2025-10-20 に再実行し、`reports/windows-env-check.json` を最新化済み。以降の作業は診断ログを常に添付する。
- OCaml・LLVM・ビルド支援ツールは整備済みで、未導入なのは MSVC ツールチェーンのみ。

| 分類 | 項目 | 状態 | 備考 |
| --- | --- | --- | --- |
| コア | OCaml / opam / dune / menhir | ✅ | `opam switch 5.2.1` で整備済み。 |
| コア | Bash (MSYS2 / Git) | ✅ | `C:\Program Files\Git\bin\bash.exe` を使用。 |
| LLVM | clang / llc / opt | ✅ | MSYS2 LLVM 16.0.4（`x86_64-w64-windows-gnu`）。 |
| LLVM | llvm-ar | ✅ | 同上。 |
| MSVC | cl / link / lib | ❌ | Visual Studio Build Tools 2022 の導入が未完了。 |
| ビルド支援 | CMake / Ninja | ✅ | MSYS2 配布版 3.29 系 / 1.11 系を確認。 |
| 補助ツール | jq / 7zip / pip | ✅ | ログ整形と成果物圧縮に使用。 |

- LLVM 19.1.1 Windows X64 配布物 (`C:\llvm\LLVM-19.1.1-Windows-X64`) を取得済み。`bin`/`include`/`lib`/`libexec`/`share` が揃い、`clang.exe`・`llc.exe`・`lld-link.exe`・`llvm-ar.exe` などの主要 CLI と 148 本の実行ファイルが含まれる。
- `lib` 直下には 721 本の `.lib` が配置され、`LLVMAArch64CodeGen.lib` などのコアライブラリに加えて `clang*.lib` や `c++.lib` を確認済み。`lib\cmake\llvm\LLVMConfig.cmake` 等も存在し、CMake での検出に利用できる。


## 作業ディレクトリ
- `compiler/ocaml/` : Windows 対応ビルド設定・ターゲット切替
- `runtime/native/windows`（想定）: MSVC ABI 向けランタイム実装
- `runtime/native/mingw/`（想定）: MinGW 向け差分実装と抽象化ヘッダー
- `tooling/ci`, `.github/workflows/` : Windows ランナーの CI 定義と補助スクリプト
- `tooling/toolchains/` : Windows 診断スクリプトとログ（`check-windows-bootstrap-env.ps1`, `reports/windows-env-check.json`）
- `docs/guides/llvm-integration-notes.md`, `docs/spec/3-9-core-async-ffi-unsafe.md` : Windows 章の更新
- `docs/notes/llvm-spec-status-survey.md` : プラットフォーム差分・リスクの記録

## 作業ブレークダウン

### 1. Toolchain 調査と環境準備（17-18週目）
**担当領域**: Windows ビルド環境構築

1.1. **LLVM/MSVC バージョン選定**
- LLVM は 2025-10-20 時点で MSYS2 LLVM 16.0.4 をベースラインとしつつ、他プラットフォームで採用している LLVM 19.x への移行可否を評価する。調査内容: MSYS2/公式バイナリの入手性、`opam` の `conf-llvm-static.19` ビルド成否、ビルド時間・ディスクコスト、ABI 差異。結果は `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` と `docs/notes/llvm-spec-status-survey.md` に追記する。
- MSVC ツールチェーンは Visual Studio Build Tools 2022（MSVC 19.40 以上）と Windows 11/10 SDK (10.0.22621 以上) を標準とし、`vcvarsall.bat`・`reml-msvc-env` からの呼び出しフローを設計する。
- MinGW (x86_64-w64-windows-gnu) は MSYS2 LLVM 16.0.4 / GCC 13 系の組み合わせを維持し、LLVM 19.x 採用時の置換手順と互換性確認をまとめる。
- バージョン決定後は `0-3-audit-and-metrics.md` の Toolchain セクションと `reports/windows-env-check.json` のバージョン欄を更新する。
- 取得済みの LLVM 19.1.1 Windows X64 配布物（`C:\llvm\LLVM-19.1.1-Windows-X64`）の PATH 連携、`lib`/`include` 参照方法、`lib\cmake\llvm` の利用手順を整備する。

1.2. **開発環境セットアップ**
- Windows 10/11 でのビルド環境構築手順書作成
- LLVM のインストール手順（公式ビルド or 自前ビルド）
- OCaml / opam / dune / menhir の既存セットアップ状況を確認し、差分があれば `docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md` を更新する。
- MSVC コンパイラ（`cl.exe`）とリンカ（`link.exe`）の設定
- 環境変数の設定（PATH, INCLUDE, LIB）

1.3. **CI 環境セットアップ**
- GitHub Actions `windows-latest` ランナーの調査
- キャッシュ戦略（LLVM/MSVC のキャッシュ）
- セットアップスクリプトの作成（PowerShell/Batch）
- 環境診断スクリプト（`tooling/toolchains/check-windows-bootstrap-env.ps1`）をジョブ冒頭で実行し、JSON 出力をアーティファクト化する。
- ビルド時間の初期計測

1.4. **LLVM 19 利用性の再評価**
- Linux/macOS で LLVM 19.x を採用している構成と依存バージョンを洗い出し、Windows で再現するための入手経路（MSYS2、LLVM 公式インストーラ、ソースビルド）を比較する。
- `opam install llvm` が要求する `conf-llvm-static.19` のビルドログを取得し、MSVC と MinGW の両ターゲットで利用可能か検証する。
- 調査の成否・阻害要因（欠落ライブラリ、ビルド時間、ABI 差分）と代替策を `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` と `docs/notes/llvm-spec-status-survey.md` に記録する。
- LLVM 16 → 19 への移行判定チェックリストを `0-3-audit-and-metrics.md` に追加し、フェーズ移行時の意思決定材料を整備する。
- ダウンロード済み配布物に同梱されている `libclang.lib`、`c++.lib`、`clang` 系 `.lib` のリンク可否と `LLVMConfig.cmake` 経由の検出結果を記録する。

**成果物**: Toolchain 選定書（MSVC/MinGW/LLVM 16→19 評価）、セットアップ手順、CI スクリプト、診断ログ

### 2. ABI 差分の調査と整理（18-19週目）
**担当領域**: ABI 互換性調査

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

**成果物**: ABI 差分レポート、`docs/notes/llvm-spec-status-survey.md` への追記

### 3. LLVM IR 生成の拡張（19-20週目）
**担当領域**: コード生成

3.1. **ターゲット切替ロジックの実装**
- `--target x86_64-w64-windows-gnu`（MinGW）での既存コード生成経路を維持し、smoke テストを追加する。
- `--target x86_64-pc-windows-msvc` フラグの処理
- LLVM `TargetMachine` の初期化（Windows ターゲット／MinGW と MSVC の両対応）
- データレイアウトの設定
- トリプルの設定と検証

3.2. **Calling Convention の適用**
- 関数シグネチャへの calling convention 属性付与
- `cc 0` (C calling convention) の適用
- 構造体引数の lowering ロジック
- 戻り値の lowering ロジック

3.3. **デバッグ情報の生成**
- Windows PDB 形式のデバッグ情報生成
- DWARF vs PDB の選択ロジック
- ソースマッピングの正確性確認
- デバッガ（Visual Studio/WinDbg）での動作確認

**成果物**: 拡張 LLVM IR 生成、ターゲット切替

### 4. ランタイム C コードの移植（20-21週目）
**担当領域**: ランタイム実装

4.1. **プラットフォーム依存コードの分離**
- Linux 固有のコード（`#ifdef __linux__`）の抽出
- Windows 固有のコード（`#ifdef _WIN32`）の追加
- 共通コードの抽象化
- ヘッダファイルの整理（`<windows.h>` vs `<unistd.h>`）

4.2. **Windows API への対応**
- ファイル IO（`CreateFile`, `ReadFile`, `WriteFile` 等）
- メモリ管理（`VirtualAlloc`, `VirtualFree` 等）
- スレッド（`CreateThread`, `WaitForSingleObject` 等）
- エラーハンドリング（`GetLastError`）

4.3. **MSVC ビルドの実装**
- `cl.exe` でのコンパイル設定（`/O2`, `/W4` 等）
- `link.exe` での静的ライブラリ生成（`.lib` ファイル）
- ビルドスクリプトの作成（CMake or 既存）
- Phase 1 のランタイムとの整合
- MinGW 版とのインターフェース差分（例: エラーコード、`errno` の扱い）を比較し共通化ポイントを洗い出す。

**成果物**: Windows 対応ランタイム、MSVC ビルド設定

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
