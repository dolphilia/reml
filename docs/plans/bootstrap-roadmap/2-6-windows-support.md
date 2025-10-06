# 2.6 Windows x64 (MSVC ABI) 対応計画

## 目的
- Phase 2 マイルストーン M4 に向けて、`-target x86_64-pc-windows-msvc` のビルドパイプラインを確立し、Windows 環境でのスモークテストを完了させる。
- System V ABI との差分を整理し、Phase 3 のクロスコンパイル機能拡張に備える。

## スコープ
- **含む**: LLVM TargetMachine 設定、MSVC 呼出規約対応、名前マングリング、PE 生成、GitHub Actions (windows-latest) テスト、ランタイムビルド。
- **含まない**: ARM64 Windows、MinGW、UWP 対応。必要に応じて別計画とする。
- **前提**: Phase 1 の x86_64 Linux ターゲットが安定、Phase 2 の型クラス/効果/FFI 実装が Windows でビルドできるよう調整済み。

## 作業ディレクトリ
- `compiler/ocaml/` : Windows 対応ビルド設定・ターゲット切替
- `runtime/native/windows`（想定）: MSVC ABI 向けランタイム実装
- `tooling/ci`, `.github/workflows/` : Windows ランナーの CI 定義と補助スクリプト
- `docs/guides/llvm-integration-notes.md`, `docs/spec/3-9-core-async-ffi-unsafe.md` : Windows 章の更新
- `docs/notes/llvm-spec-status-survey.md` : プラットフォーム差分・リスクの記録

## 作業ブレークダウン

### 1. Toolchain 調査と環境準備（17-18週目）
**担当領域**: Windows ビルド環境構築

1.1. **LLVM/MSVC バージョン選定**
- LLVM バージョンの決定（Phase 1 と統一 or 最新版）
- MSVC ツールチェーンのバージョン選定（Visual Studio 2022 推奨）
- Windows SDK バージョンの決定
- `0-3-audit-and-metrics.md` へのバージョン記録

1.2. **開発環境セットアップ**
- Windows 10/11 でのビルド環境構築手順書作成
- LLVM のインストール手順（公式ビルド or 自前ビルド）
- MSVC コンパイラ（`cl.exe`）とリンカ（`link.exe`）の設定
- 環境変数の設定（PATH, INCLUDE, LIB）

1.3. **CI 環境セットアップ**
- GitHub Actions `windows-latest` ランナーの調査
- キャッシュ戦略（LLVM/MSVC のキャッシュ）
- セットアップスクリプトの作成（PowerShell/Batch）
- ビルド時間の初期計測

**成果物**: Toolchain 選定書、セットアップ手順、CI スクリプト

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
- `--target x86_64-pc-windows-msvc` フラグの処理
- LLVM `TargetMachine` の初期化（Windows ターゲット）
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
- ビルドスクリプトの作成（CMake or 手動）
- Phase 1 ランタイムとの統合

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
- `.github/workflows/` に Windows ジョブ追加
- セットアップステップ（LLVM/MSVC インストール）
- ビルドステップ（OCaml コンパイラ、Reml ツールチェーン）
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
- ABI 差分がドキュメント化され、レビュー記録が残る。
- CLI で `--target x86_64-pc-windows-msvc` を指定可能になり、テストケースが通過。

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
