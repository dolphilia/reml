# runtime/native ワークスペース

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
- ✅ テストスイート（`tests/test_mem_alloc.c`, `tests/test_refcount.c`）
  - メモリアロケータ：6 件のテストケース成功
  - 参照カウント：8 件のテストケース成功
  - AddressSanitizer 統合：リーク・ダングリングゼロ
- ✅ ビルドシステム（`Makefile`）
  - macOS SDK 対応
  - デバッグ/リリースビルド切り替え
  - テスト実行自動化

### 次のステップ（Phase 1-5 §4-8）
- [ ] LLVM 連携統合（コンパイラ側からの呼び出し検証）
- [ ] CI 統合（GitHub Actions でのランタイムビルド・テスト）
- [ ] 監査・メトリクス計測に関する補助スクリプトの配置
- [ ] Windows/MSVC 対応や追加 Capability を Phase 2 計画と同期

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

## ディレクトリ構成

```
runtime/native/
├── include/
│   └── reml_runtime.h     # ランタイム API 定義
├── src/
│   ├── mem_alloc.c        # メモリアロケータ
│   ├── panic.c            # パニックハンドラ
│   ├── print_i64.c        # デバッグ出力
│   └── refcount.c         # 参照カウント実装
├── tests/
│   ├── test_mem_alloc.c   # アロケータテスト
│   └── test_refcount.c    # 参照カウントテスト
├── build/                 # ビルド成果物（自動生成）
│   ├── *.o                # オブジェクトファイル
│   ├── libreml_runtime.a  # 静的ライブラリ
│   └── test_*             # テストバイナリ
├── Makefile               # ビルドスクリプト
└── README.md              # このファイル
```
