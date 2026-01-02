# Reml C言語実装計画: ロードマップ

本ロードマップでは、開発を論理的なフェーズに分解します。

## フェーズ 1: プロジェクトのブートストラップ
**目標**: ビルド環境の確立と "Hello World"。
- [x] `compiler/c` ディレクトリの初期化。
- [x] コンパイラフラグ（C11, C++17, Sanitizers）を設定した `CMakeLists.txt` のセットアップ。
- [x] `main.c`（CLI引数解析）の実装。
- [x] macOS/Linux/Windows ビルド用の CI/CD スタブ（Github Actions）のセットアップ。
- [x] `ctest` がダミーテストを実行できることを確認。

## フェーズ 2: フロントエンドの基礎 (Lexer & Parser)
**目標**: Reml 構文を抽象構文木 (AST) に解析する。
- [x] コンテナライブラリの選定と統合（例: `kvec.h`, `uthash`）。
- [x] `Lexer` の実装: ソースファイルのトークン化（Unicode対応）。
- [x] C言語での AST 構造体の定義（最小構成）。
- [x] `Parser` の実装: 再帰下降法など（最小構成）。
- [x] AST Printer の実装（デバッグ/テスト用）。
- [x] テスト: 単純なスニペットに対して解析された AST を期待される出力と比較。

## フェーズ 3: 意味解析と型の概念
**目標**: AST の検証と名前解決。
- [x] `SymbolTable` / `Environment` の実装。
- [x] 名前解決 (Scopes) の実装。
- [x] 基本的な型定義の構造体実装。
- [x] 型チェックのシム実装（最初は正しい型だけを受け入れる）。
- [x] フェーズ3の最小サブセット確定（ADT/レコード/参照/トレイト/効果行は保留）。

## フェーズ 4: コード生成 (LLVM) / 実行
**目標**: 単純なプログラムの実行。
- [x] LLVM の統合（C API または C++ ブリッジ経由）。
- [x] 基本的な算術とプリミティブ（`Int`, `Float`）用の `Codegen` 実装。
- [x] `CodeEmitter` の実装: オブジェクトファイルまたは実行可能ファイルの出力。
- [x] 検証: 整数を返す単純な `main` 関数をコンパイルして実行。

## フェーズ 5: 高度な機能と仕様準拠
**目標**: Reml 独自の機能（Effects, パターンマッチング, Strings）をサポート。
- [ ] `BigInt` サポートの実装（ライブラリ統合）。
- [ ] `String` / Unicode 処理の実装。
- [ ] ADT とレコード型の導入。
- [ ] 参照型 (`&T`, `&mut T`) の導入。
- [ ] パターンマッチングのコンパイル実装（Desugaring）。
- [ ] トレイト/型クラスの導入（演算子解決の一般化）。
- [ ] Algebraic Effects の実装（または初期段階では簡略化されたランタイムモデル）。
- [ ] 効果行 (`! Σ`) の導入。
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
