# Reml C言語実装計画: ロードマップ

本ロードマップでは、開発を論理的なフェーズに分解します。

## フェーズ 1: プロジェクトのブートストラップ
**目標**: ビルド環境の確立と "Hello World"。
- [ ] `compiler/c` ディレクトリの初期化。
- [ ] コンパイラフラグ（C11, C++17, Sanitizers）を設定した `CMakeLists.txt` のセットアップ。
- [ ] `main.c`（CLI引数解析）の実装。
- [ ] macOS/Linux/Windows ビルド用の CI/CD スタブ（Github Actions）のセットアップ。
- [ ] `ctest` がダミーテストを実行できることを確認。

## フェーズ 2: フロントエンドの基礎 (Lexer & Parser)
**目標**: Reml 構文を抽象構文木 (AST) に解析する。
- [ ] コンテナライブラリの選定と統合（例: `kvec.h`, `uthash`）。
- [ ] `Lexer` の実装: ソースファイルのトークン化（Unicode対応）。
- [ ] C言語での AST 構造体の定義。
- [ ] `Parser` の実装: 再帰下降法など（`compiler/rust` のロジックを参考にしつつ C で実装）。
- [ ] AST Printer の実装（デバッグ/テスト用）。
- [ ] テスト: 単純なスニペットに対して解析された AST を期待される出力と比較。

## フェーズ 3: 意味解析と型の概念
**目標**: AST の検証と名前解決。
- [ ] `SymbolTable` / `Environment` の実装。
- [ ] 名前解決 (Scopes) の実装。
- [ ] 基本的な型定義の構造体実装。
- [ ] 型チェックのシム実装（最初は正しい型だけを受け入れる）。

## フェーズ 4: コード生成 (LLVM) / 実行
**目標**: 単純なプログラムの実行。
- [ ] LLVM の統合（C API または C++ ブリッジ経由）。
- [ ] 基本的な算術とプリミティブ（`Int`, `Float`）用の `Codegen` 実装。
- [ ] `CodeEmitter` の実装: オブジェクトファイルまたは実行可能ファイルの出力。
- [ ] 検証: 整数を返す単純な `main` 関数をコンパイルして実行。

## フェーズ 5: 高度な機能と仕様準拠
**目標**: Reml 独自の機能（Effects, パターンマッチング, Strings）をサポート。
- [ ] `BigInt` サポートの実装（ライブラリ統合）。
- [ ] `String` / Unicode 処理の実装。
- [ ] パターンマッチングのコンパイル実装（Desugaring）。
- [ ] Algebraic Effects の実装（または初期段階では簡略化されたランタイムモデル）。
- [ ] `Type Inference` の実装（Hindley-Milner または双方向）。

## フェーズ 6: 標準ライブラリとセルフホスティング
**目標**: `examples/practical` の実行。
- [ ] 最小限のランタイム（C実装の標準ライブラリ）とのリンク。
- [ ] `Core` ライブラリの機能をランタイムコードへ移植。
- [ ] `examples/spec_core` テストの実行。
- [ ] `examples/practical` アプリケーションの実行。

## 成功の指標
- `examples/spec_core/01_basics.reml` を実行できる。
- `cmake --build . --target test` がパスする。
- macOS (Clang), Linux (GCC), Windows (MSVC) でクリーンにビルドできる。
