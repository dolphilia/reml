# Reml C言語実装計画: アーキテクチャ

本文書では、C言語実装におけるアーキテクチャ設計とディレクトリ構造について記述します。

## 1. ディレクトリ構造

プロジェクトは `compiler/c` に配置し、標準的な CMake プロジェクト構造に従います。

```
compiler/c/
├── CMakeLists.txt          # ルートビルド設定
├── cmake/                  # カスタムCMakeモジュール
│   └── Find...cmake
├── docs/                   # 実装固有のドキュメント
├── include/                # 公開ヘッダ
│   └── reml/
│       ├── ast/
│       ├── lexer/
│       ├── parser/
│       ├── typeck/
│       └── util/
├── src/                    # ソースコード
│   ├── main.c              # CLIエントリーポイント
│   ├── driver/             # コンパイルドライバ
│   ├── lexer/              # レキサー実装
│   ├── parser/             # パーサー実装
│   ├── ast/                # ASTノードと操作
│   ├── sema/               # 意味解析 (名前解決, 型チェック)
│   ├── codegen/            # コード生成 (LLVM連携)
│   └── util/               # 内部ユーティリティ
├── tests/                  # テストスイート
│   ├── unit/
│   └── integration/
└── deps/                   # サードパーティ依存 (git submodules または FetchContent)
```

## 2. コンポーネント設計

コンパイラはパイプラインとして構成されます：

1.  **Driver**: CLI引数の処理、ファイルI/O、パイプラインの指揮を担当。
2.  **Lexer**: 入力ソースをトークン化。
3.  **Parser**: `docs/spec/2-1-parser-type.md` および `docs/spec/1-5-formal-grammar-bnf.md` に詳述。ASTを生成。
4.  **Semantic Analysis (Sema/意味解析)**:
    -   名前解決 (Name Resolution)
    -   型推論 (`docs/spec/1-2-types-Inference.md`)
    -   効果チェック (`docs/spec/1-3-effects-safety.md`)
5.  **Codegen**: 実行可能コード/IRを生成。
    -   ターゲット: LLVM IR (初期段階)。

## 3. ライブラリ選定戦略

「信頼性が高い」かつ「C言語フレンドリー」なライブラリを優先します。

| 機能 | 候補ライブラリ | 備考 |
| :--- | :--- | :--- |
| **データ構造** | `uthash` および同作者の `utarray`/`utstring`/`utlist`/`utstack` | 小規模で扱いやすい構成を維持。必要がない限り GLib のような重いフレームワークは避ける。 |
| **引数解析** | `argparse` (cofyc), `docopt.c`, または自作 | シンプルな CLI 解析。 |
| **JSON** | `yyjson` | 設定の読み込みや AST のエクスポート用。高速でメモリ効率が良い。 |
| **BigInt** | `libtommath` または `gmp` | 仕様で `BigInt` が要求される。`libtommath` はライセンスが寛容 (WTFPL/Public Domain)。 |
| **LLVM** | `LLVM C API` | コアバックエンド。C API が不十分な場合のみ C++ ラッパーを使用。 |
| **Unicode** | `utf8.h` または `libunistring` | Reml は Unicode に敏感 (`docs/spec/1-4-test-unicode-model.md`)。 |
| **アリーナ/アロケータ** | `mimalloc`, `rpmalloc`, または自作アリーナ | AST/型情報などの短命オブジェクトをまとめて解放する用途。 |
| **テストフレームワーク** | `cmocka`, `criterion` | ユニットテストと統合テストの基盤。 |
| **ログ/診断** | `log.c` (rxi) または自作 | 解析・型検査・コード生成のトレースと診断出力。 |
| **字句解析支援** | `re2c` | 仕様に準拠した高速レキサー生成。 |
| **ファイル/ディレクトリ操作** | `tinydir` または自作 | examples/ や tests/ の走査用途。 |
| **スレッド/並列** | `tinycthread` または自作薄ラッパー | ビルド/解析の並列化を行う場合に限定して導入。 |

### C++ との相互作用
-   C++ ライブラリ（Cに公開されていない LLVM C++ API コンポーネントなど）が必要な場合、専用の `src/cpp_bridge/` ディレクトリを作成します。
-   これらのファイルは C++17 としてコンパイルされます。
-   これらは `include/reml/cpp_bridge/` に配置されたヘッダで、厳格な `extern "C"` インターフェースを公開します。

## 4. ビルドシステム

-   **CMake** をビルドの唯一の真実のソースとします。
-   **プロファイル**:
    -   `Debug`: デバッグ情報とサニタイザ (AddressSanitizer, UBSanitizer) を有効化。
    -   `Release`: 最適化を有効化 (O3)。
-   **テスト**:
    -   `CTest` 統合。
    -   `tests/unit` 内のユニットテスト。
    -   既存の Reml サンプルを実行するエンドツーエンドテスト。
