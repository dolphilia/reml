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
- ✅ テストスイート（`tests/test_mem_alloc.c`）
  - 6 件のテストケースすべて成功
  - AddressSanitizer 統合
- ✅ ビルドシステム（`Makefile`）
  - macOS SDK 対応
  - デバッグ/リリースビルド切り替え
  - テスト実行自動化

### 次のステップ（Phase 1-5 §3-8）
- [ ] 参照カウント実装（`inc_ref`, `dec_ref`, 型別デストラクタ）
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
│   └── print_i64.c        # デバッグ出力
├── tests/
│   └── test_mem_alloc.c   # アロケータテスト
├── build/                 # ビルド成果物（自動生成）
│   ├── *.o                # オブジェクトファイル
│   ├── libreml_runtime.a  # 静的ライブラリ
│   └── test_*             # テストバイナリ
├── Makefile               # ビルドスクリプト
└── README.md              # このファイル
```
