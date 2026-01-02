# フェーズ 4: コード生成 (LLVM) と実行

このフェーズでは、チェック済みの AST を LLVM バックエンドに接続し、実行可能ファイルを生成します。

## 4.1 LLVM セットアップとブリッジ
- **ライブラリ**: `LLVM C API` + `src/cpp_bridge/` (C++17)。
- **タスク**:
  1.  CMake 経由で LLVM ライブラリをリンク (`find_package(LLVM)`)。
  2.  LLVM コンテキストの初期化 (`LLVMContextCreate`)。
  3.  モジュールの作成 (`LLVMModuleCreateWithName`)。
  4.  ビルダーの作成 (`LLVMCreateBuilder`)。
  5.  **C++ ブリッジ**: 高度な機能（例: C API が不足している場合のデバッグ情報メタデータ生成）のためのラッパー。

## 4.2 IR 生成 (Codegen)
- **入力**: 型付き AST。
- **タスク**:
  1.  **値**: `LLVMConstInt`, `LLVMConstReal` を使用してリテラルの `LLVMValueRef` を生成。
  2.  **制御フロー**:
      - `if/else`: `LLVMBuildCondBr`, BasicBlocks。
      - ループ: `LLVMBuildBr` (必要なら phi ノード、または可変変数用 `alloca`)。
  3.  **関数**: `LLVMAddFunction`, パラメータ処理。
  4.  **変数**: `LLVMBuildAlloca` (スタック変数)。
- **最適化**: 基本的なパスを実行 (`LLVMPassManagerBuilder`)。

## 4.3 ランタイムエントリーポイント
- **戦略**: 最小限の C ランタイム (CRT) またはカスタム `main` にリンク。
- **タスク**:
  1.  Reml のエントリーポイントを呼び出す `main` 関数ラッパーを生成。
  2.  GC（後で必要な場合）またはグローバル状態の初期化。

## 4.4 オブジェクトファイル出力
- **ターゲット**: ネイティブマシンコード。
- **タスク**:
  1.  ターゲットマシンの初期化 (`LLVMGetTargetFromTriple`)。
  2.  `.o` ファイルの出力 (`LLVMTargetMachineEmitToFile`)。
  3.  システムリンカと `.o` をリンク (`clang` ドライバを経由して `ld` を呼び出すか、埋め込みなら `lld`)。

## 4.5 JIT サポート（オプション/後期）
- **目標**: `reml run` または REPL での即時実行。
- **ライブラリ**: `LLVMOrcJIT` (おそらく C++ ブリッジが必要)。
- **タスク**:
  - JIT セッションのセットアップ。
  - JIT へのモジュール追加。
  - シンボルの検索と実行。

## 4.6 検証
- **目標**: `hello_world.reml` をコンパイルおよび実行できる (int を返す)。
- **テスト**:
  - `tests/integration/codegen_test.py`:
    - ソースコンパイル -> バイナリ実行 -> stdout/終了コード確認。
  - `LLVM IR` 出力の正当性を目視確認。

## チェックリスト
- [ ] LLVM が CMake で正常にリンクされた。
- [ ] `codegen` モジュールが算術演算に対して有効な IR を生成する。
- [ ] 制御フロー (if/loop) が正しい BasicBlocks を生成する。
- [ ] オブジェクトファイルを出力できる。
- [ ] `main.c` からバイナリをリンクして実行できる。
