# compiler/runtime/native ワークスペース

Phase 1 の最小ランタイムおよび Phase 2 以降の Capability 拡張を実装する領域です。詳細タスクは [`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`](../../docs/plans/bootstrap-roadmap/1-5-runtime-integration.md) と後続フェーズの計画書を参照してください。

## 実装状況（2025-10-10）

### 完了した実装
- ✅ ディレクトリ構成の確定（`include/`, `src/`, `tests/`）
- ✅ ランタイム API ヘッダ（`include/reml_runtime.h`）
- ✅ メモリアロケータ（`src/mem_alloc.c`）
  - malloc ベース実装
  - 8 バイト境界自動調整
  - ヘッダ初期化（refcount=1, type_tag）
  - デバッグ支援（アロケーション追跡、二重解放検出）
- ✅ パニックハンドラ（`src/panic.c`）
  - エラーメッセージ出力
  - タイムスタンプ・PID 記録
  - 異常終了処理
- ✅ デバッグユーティリティ（`src/print_i64.c`）
- ✅ 参照カウント実装（`src/refcount.c`）
  - inc_ref / dec_ref 基本操作（単一スレッド、Phase 1）
  - 型別デストラクタディスパッチ（STRING, TUPLE, RECORD, CLOSURE, ADT）
  - 再帰的な子オブジェクト解放
  - デバッグ統計（inc/dec/destroy カウンタ）
- ✅ テストスイート（`tests/test_mem_alloc.c`, `tests/test_refcount.c`, `tests/test_ffi_bridge.c`, `tests/test_os.c`）
  - メモリアロケータ：6 件のテストケース成功
  - 参照カウント：8 件のテストケース成功
  - AddressSanitizer 統合：リーク・ダングリングゼロ
- ✅ ビルドシステム（`Makefile`）
  - macOS SDK 対応
  - デバッグ/リリースビルド切り替え
  - テスト実行自動化

### 次のステップ（Phase 1-5 §4-8）
- [ ] LLVM 連携統合（コンパイラ側からの呼び出し検証）
- [x] CI 統合（GitHub Actions でのランタイムビルド・テスト）
- [ ] 監査・メトリクス計測に関する補助スクリプトの配置
- [ ] Windows/MSVC 対応や追加 Capability を Phase 2 計画と同期

## CI 統合

### GitHub Actions での自動テスト

ランタイムのビルドとテストは `.github/workflows/ocaml-dune-test.yml` で自動実行されます：

1. **ランタイムビルド**: `make runtime` で `libreml_runtime.a` を生成
2. **基本テスト**: `make test` でメモリアロケータ・参照カウントのテストを実行（14件）
3. **Valgrind 検証**: リリースビルドのテストバイナリに対してリーク・ダングリング検出を実行
4. **AddressSanitizer 検証**: `DEBUG=1` で再ビルドし、ASan を有効にしたテストを実行
5. **アーティファクト収集**:
   - 成功時: `libreml_runtime.a` と `.o` ファイルを 30 日保持
   - 失敗時: テストバイナリとログを 7 日保持

### ローカルでの再現手順

CI と同じテストをローカルで実行する方法：

```bash
# 基本テスト
make clean && make runtime && make test

# Valgrind 統合テスト（リリースビルドを使用）
for test in build/test_*; do
  valgrind --leak-check=full --error-exitcode=1 "$test"
done

# AddressSanitizer テスト
make clean && DEBUG=1 make runtime && DEBUG=1 make test
```

Valgrind と AddressSanitizer は同時に有効化するとメモリマップが衝突するため、上記のようにビルドを分けて実行してください。

### テスト方針: 一時ファイル作成

`tests/test_os.c` では POSIX 環境で `tmpnam` を使用せず、`mkstemp` により一意なパスを確保したうえで書き込み・読み込みを検証する。
セキュリティ上の理由と macOS での API 非推奨警告を避けるため、同テストは `mkstemp` を標準とする。

### Docker での検証

CI 環境と同じ Ubuntu 22.04 + LLVM 18 環境で検証する場合：

```bash
# Docker イメージを使用してランタイムテストを実行
scripts/docker/run-runtime-tests.sh --tag ghcr.io/reml/bootstrap-runtime:local

# カスタムコマンドを実行（Valgrind 対応）
scripts/docker/run-runtime-tests.sh -- "cd compiler/runtime/native && make clean && make runtime && make test && for t in build/test_*; do valgrind --leak-check=full --error-exitcode=1 \"\$t\"; done"

# AddressSanitizer テスト
scripts/docker/run-runtime-tests.sh -- "cd compiler/runtime/native && make clean && DEBUG=1 make runtime && DEBUG=1 make test"
```

### アーティファクトの取得

GitHub Actions の成果物は以下から取得できます：

- **ランタイムライブラリ**: Actions タブ → 該当ワークフロー → Artifacts → `runtime-artifacts`
- **失敗時のテストバイナリ**: Actions タブ → 失敗したワークフロー → Artifacts → `runtime-test-failures`

保持期間:
- `runtime-artifacts`: 30 日
- `runtime-test-failures`: 7 日

## ビルド方法

```bash
# ランタイムライブラリのビルド
make runtime

# テストの実行
make test

# デバッグビルド（AddressSanitizer 有効）
make clean && DEBUG=1 make runtime && DEBUG=1 make test

# クリーン
make clean
```

### Windows (MSVC) でのビルド

1. PowerShell で `reml-msvc-env` を実行して MSVC ツールチェーンを有効化する（`tooling/toolchains/setup-windows-toolchain.ps1`が提供）。
2. CMake 3.20 以上でビルドディレクトリを生成する:

   ```powershell
   cmake -S compiler/runtime/native -B compiler/runtime/native/build-msvc -DREML_RUNTIME_ENABLE_DEBUG=OFF
   ```

3. `cl.exe` / `lib.exe` を用いたビルドを実行する:

   ```powershell
   cmake --build compiler/runtime/native/build-msvc --config Release
   ```

4. テストを実行する:

   ```powershell
   ctest --test-dir compiler/runtime/native/build-msvc --output-on-failure
   ```

`REML_RUNTIME_ENABLE_DEBUG=ON` を指定すると `DEBUG` マクロが定義され、参照カウントとアロケータの統計出力が有効になる。MSVC ビルドでは `VirtualAlloc`/`VirtualFree` によるメモリ管理と Windows API ベースの診断出力を利用する。

## ディレクトリ構成

```
compiler/runtime/native/
├── include/
│   ├── reml_runtime.h     # ランタイム API 定義
│   ├── reml_embed.h       # 埋め込み API 定義
│   ├── reml_platform.h    # プラットフォーム判定マクロ
│   ├── reml_atomic.h      # アトミック操作互換レイヤー
│   └── reml_os.h          # ファイル/スレッド抽象化 API
├── src/
│   ├── mem_alloc.c        # メモリアロケータ
│   ├── panic.c            # パニックハンドラ
│   ├── print_i64.c        # デバッグ出力
│   ├── refcount.c         # 参照カウント実装
│   ├── ffi_bridge.c       # FFI ブリッジ補助
│   └── os.c               # Windows/POSIX 共通 OS ラッパー
├── tests/
│   ├── test_mem_alloc.c   # アロケータテスト
│   ├── test_refcount.c    # 参照カウントテスト
│   ├── test_ffi_bridge.c  # FFI ブリッジテスト
│   └── test_os.c          # OS 抽象レイヤーテスト
├── build/                 # ビルド成果物（自動生成）
│   ├── *.o                # オブジェクトファイル
│   ├── libreml_runtime.a  # 静的ライブラリ
│   └── test_*             # テストバイナリ
├── Makefile               # ビルドスクリプト
└── README.md              # このファイル
```
