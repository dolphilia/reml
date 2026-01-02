# フェーズ 4: コード生成 (LLVM) と実行

このフェーズでは、チェック済みの AST を LLVM バックエンドに接続し、実行可能ファイルを生成します。

## 4.0 スコープと前提
- **入力**: フェーズ3で検証済みの AST（名前解決・型検査済み）。
- **対象サブセット**:
  - リテラル（`Int`, `Float`, `Bool`）と基本算術。
  - 変数定義と参照（ローカルのみ）。
  - 関数定義・呼び出し（第一級関数やクロージャは対象外）。
  - `if/else` と単純なループ（`while` 相当）。
- **非対象**: 文字列、ADT、参照型、効果、例外、GC、並列実行。
- **ゴール**: `hello_world.reml` がネイティブ実行できる最小パスを確立する。

## 4.1 LLVM セットアップとブリッジ
- **ライブラリ**: `LLVM C API` + `src/cpp_bridge/` (C++17)。
- **タスク**:
  1.  CMake 経由で LLVM ライブラリをリンク (`find_package(LLVM)`)。
  2.  LLVM の初期化 (`LLVMInitializeNativeTarget`, `LLVMInitializeNativeAsmParser`, `LLVMInitializeNativeAsmPrinter`)。
  3.  ターゲットトリプル取得と設定 (`LLVMGetDefaultTargetTriple`, `LLVMSetTarget`)。
  4.  コンテキストの初期化 (`LLVMContextCreate`)。
  5.  モジュールの作成 (`LLVMModuleCreateWithName`)。
  6.  データレイアウト設定 (`LLVMCreateTargetDataLayout`, `LLVMSetDataLayout`)。
  7.  ビルダーの作成 (`LLVMCreateBuilder`)。
  8.  **C++ ブリッジ**: C API が不足する機能（デバッグ情報、ORC JIT、属性設定）用のラッパー。
  9.  エラーハンドリング: `LLVMCreateTargetMachine` や `LLVMTargetMachineEmitToFile` の失敗を診断へ落とし込む。

## 4.2 IR 生成 (Codegen)
- **入力**: 型付き AST。
- **タスク**:
  1.  **型降格 (Type Lowering)**:
      - Reml 型を LLVM 型に対応付け（`Int`/`Float`/`Bool` の幅は仕様に合わせる）。
      - サイズ/アライメントの表を作成し、型レイアウト計算の基礎にする。
      - フェーズ4の既定はフェーズ3のプリミティブ (`i64`/`f64`) に合わせる。
      - 型降格表:
        - `Int` -> `i64` (signed 64-bit)
        - `Float` -> `double` (IEEE754 binary64)
        - `Bool` -> `i1` (条件分岐は `i1`、必要なら `zext`/`trunc`)
        - `Unit` -> `void`
  2.  **値**: `LLVMConstInt`, `LLVMConstReal` を使用してリテラルの `LLVMValueRef` を生成。
  3.  **制御フロー**:
      - `if/else`: `LLVMBuildCondBr`, BasicBlocks。
      - ループ: `LLVMBuildBr` (必要なら phi ノード、または可変変数用 `alloca`)。
  4.  **関数**: `LLVMAddFunction`, パラメータ処理。
  5.  **変数**: `LLVMBuildAlloca` (スタック変数)。
  6.  **スコープ管理**: `alloca` の配置、ローカル変数のテーブル化、ブロック終了時の可視性管理。
  7.  **診断**: コード生成不能な AST ノードに対してフェーズ4専用の診断 ID を定義する。
- **最適化**: 基本的なパスを実行 (`LLVMPassManagerBuilder`)。

## 4.3 ランタイムエントリーポイント
- **戦略**: 最小限の C ランタイム (CRT) またはカスタム `main` にリンク。
- **タスク**:
  1.  Reml のエントリーポイントを呼び出す `main` 関数ラッパーを生成。
  2.  GC（後で必要な場合）またはグローバル状態の初期化。
  3.  **ABI 取り決め**:
      - エントリーポイントは `reml_main` とし、シグネチャは `i64 reml_main(void)` を既定とする。
      - C 側の `main` は `int main(int argc, char** argv)` を生成し、現時点では `argv` を無視する。
      - 退出コードは `(int)reml_main()` を返す（プラットフォームにより下位 8/32 ビットが使用される）。
      - ランタイム初期化に失敗した場合は `1` を返し、診断を stderr へ出力する（詳細は後続フェーズで拡張）。

## 4.4 オブジェクトファイル出力
- **ターゲット**: ネイティブマシンコード。
- **タスク**:
  1.  ターゲットマシンの初期化 (`LLVMGetTargetFromTriple`)。
  2.  `.o` ファイルの出力 (`LLVMTargetMachineEmitToFile`)。
  3.  システムリンカと `.o` をリンク (`clang` ドライバを経由して `ld` を呼び出すか、埋め込みなら `lld`)。
  4.  デバッグ支援: `--emit-llvm` や `--emit-ir` のような中間出力オプションを追加（初期はファイルダンプのみ）。

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
  - `tests/unit/test_codegen.c`:
    - 最小 AST から IR を生成し、`LLVMVerifyModule` で検証。
  - `LLVM IR` 出力の正当性を目視確認。

## 4.7 完了条件
- `Int`/`Float` の演算が LLVM IR に落とされる。
- `if/else` と単純ループが BasicBlock を構成できる。
- `hello_world.reml` がネイティブ実行できる。
- 失敗時に診断が JSON で出力される。

## チェックリスト
- [ ] LLVM が CMake で正常にリンクされた。
- [ ] `codegen` モジュールが算術演算に対して有効な IR を生成する。
- [ ] 制御フロー (if/loop) が正しい BasicBlocks を生成する。
- [ ] オブジェクトファイルを出力できる。
- [ ] `main.c` からバイナリをリンクして実行できる。
- [ ] `LLVMVerifyModule` の検証をパスする。
