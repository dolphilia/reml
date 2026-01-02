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

| 機能 | 候補ライブラリ | 採否 | 備考 |
| :--- | :--- | :--- | :--- |
| **データ構造** | `uthash` + `utarray`/`utstring`/`utlist`/`utstack` | **採用** | 小規模で扱いやすい構成を維持。必要がない限り GLib のような重いフレームワークは避ける。 |
| **引数解析** | `argparse` (cofyc) | **採用** | CLI の多層コマンドを簡潔に扱える。 |
| **JSON** | `yyjson` | **採用** | 設定の読み込みや診断/監査の JSON 出力で必須。高速でメモリ効率が良い。 |
| **TOML** | `tomlc99` | **採用** | `reml.toml` 読み込みが必須（`docs/spec/3-7-core-config-data.md`）。互換モードはレキサー層で吸収する。 |
| **BigInt** | `libtommath`（必要時のみ `gmp`） | **採用** | 仕様で `BigInt` が要求される。`gmp` は性能要件が明確になった場合に限定。 |
| **LLVM** | `LLVM C API` | **採用** | コアバックエンド。C API が不十分な場合のみ C++ ラッパーを使用。 |
| **Unicode** | `utf8proc` + `libgrapheme` | **採用** | NFC 正規化/分類/幅計算と UAX #29 のグラフェム分割を満たす（`docs/spec/1-4-test-unicode-model.md`）。詳細は下記の検証メモを参照。 |
| **アリーナ/アロケータ** | 自作アリーナ（`mimalloc`/`rpmalloc` は任意） | **採用** | AST/型情報などの短命オブジェクトをまとめて解放する用途。 |
| **テストフレームワーク** | `cmocka` | **採用** | CMake/CTest 連携が容易で軽量。 |
| **ログ/診断** | `log.c` (rxi) | **採用** | 内部ログに限定。診断 JSON 形式は独自実装で統一する。 |
| **字句解析支援** | `re2c` | **採用** | 仕様に準拠した高速レキサー生成。 |
| **ファイル/ディレクトリ操作** | `tinydir` | **採用** | examples/ や tests/ の走査用途。 |
| **スレッド/並列** | `tinycthread` | **保留** | 初期実装は単一スレッド前提。並列化が必要になった段階で導入。 |
| **UUID** | `uuid4`（単一ヘッダ/実装） | **採用** | `audit_id` や診断 ID を生成するために必須。 |
| **ハッシュ** | `BLAKE3` C 公式実装 | **採用** | マニフェスト/キャッシュ/監査のハッシュ用途。 |
| **JSON Schema 検証** | `jsonschema` (C) または自作 | **保留** | diagnostics/report のスキーマ検証は後期に回す。 |

### Unicode ライブラリ妥当性の検証メモ

- **正規化 (NFC)**: `utf8proc` の NFC 正規化で `String` 内部表現を統一できるか検証する。無効 UTF-8 やサロゲートの扱いは `docs/spec/1-4-test-unicode-model.md` の規約に合わせ、字句段階エラーへ落とし込めること。
- **幅計算 (display_width)**: `utf8proc_charwidth` を素朴に使うだけでは ZWJ 絵文字や合成列の幅が合わない可能性があるため、グラフェム単位で幅を集計する設計が必要。差分が残る場合は「幅補正テーブル（例: Emoji 互換幅）」を追加する前提で比較テストを行う。
- **分割 (UAX #29)**: `libgrapheme` が拡張書記素クラスタに準拠していることを前提に、Unicode バージョンと UAX #29 テストデータの一致を確認する。仕様更新時は `Unicode 15.x` との差分を記録し、C 実装側の更新手順を明文化する。

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
