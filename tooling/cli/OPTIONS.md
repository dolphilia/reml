# Reml CLI オプションリファレンス

**Phase**: 1-6 開発者体験整備
**作成日**: 2025-10-10
**ステータス**: 設計フェーズ

## 目次

1. [概要](#概要)
2. [入力オプション](#入力オプション)
3. [出力オプション](#出力オプション)
4. [診断オプション](#診断オプション)
5. [デバッグオプション](#デバッグオプション)
6. [コンパイルオプション](#コンパイルオプション)
7. [その他](#その他)
8. [使用例](#使用例)

---

## 概要

`remlc-ocaml` コマンドは以下の形式で実行します：

```bash
remlc [OPTIONS] <file.reml>
```

または標準入力から読み込む場合：

```bash
remlc [OPTIONS] -
```

---

## 入力オプション

### `<file.reml>`

**説明**: コンパイルする Reml ソースファイルのパス

**必須**: はい

**例**:
```bash
remlc hello.reml
```

### `-`

**説明**: 標準入力からソースコードを読み込む

**Phase**: Phase 2（Phase 1-6 では未実装）

**例**:
```bash
echo "fn main() -> i64 = 42" | remlc -
```

---

## 出力オプション

### `--emit-ast`

**説明**: 抽象構文木（AST）を標準出力に出力する

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 1（実装済み）

**例**:
```bash
remlc --emit-ast hello.reml
```

**出力例**:
```
Module:
  Function: main
    Params: []
    ReturnType: i64
    Body: Literal(Int(42))
```

### `--emit-tast`

**説明**: 型付き AST（Typed AST）を標準出力に出力する

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 2（実装済み）

**例**:
```bash
remlc --emit-tast hello.reml
```

**出力例**:
```
Module:
  Function: main : () -> i64
    Body: Literal(Int(42)) : i64
```

### `--emit-ir`

**説明**: LLVM IR（テキスト形式）を出力ディレクトリに生成する

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 3（実装済み）

**例**:
```bash
remlc --emit-ir hello.reml --out-dir ./output
```

**出力ファイル**: `./output/hello.ll`

### `--emit-bc`

**説明**: LLVM Bitcode（バイナリ形式）を出力ディレクトリに生成する

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 3（実装済み）

**例**:
```bash
remlc --emit-bc hello.reml --out-dir ./output
```

**出力ファイル**: `./output/hello.bc`

### `--out-dir <dir>`

**説明**: 出力ファイルを生成するディレクトリ

**型**: 文字列

**デフォルト**: `.` (カレントディレクトリ)

**Phase**: Phase 1（実装済み）

**例**:
```bash
remlc --emit-ir hello.reml --out-dir ./build
```

---

## 診断オプション

### `--format <format>`

**説明**: 診断メッセージの出力形式

**型**: `text` | `json`

**デフォルト**: `text`

**Phase**: Phase 1-6（Week 14）

**例**:

**テキスト形式**:
```bash
remlc --format=text hello.reml
```
出力:
```
hello.reml:2:5: エラー[E7001] (型システム): 型が一致しません
    2 |   a + "hello"
      |       ^^^^^^^ 期待される型: i64、実際の型: String
```

**JSON 形式**:
```bash
remlc --format=json hello.reml
```
出力:
```json
{
  "diagnostics": [
    {
      "severity": "error",
      "code": "E7001",
      "domain": "type",
      "message": "型が一致しません",
      "location": {
        "file": "hello.reml",
        "line": 2,
        "column": 5
      }
    }
  ]
}
```

### `--color <mode>`

**説明**: カラー出力の制御

**型**: `auto` | `always` | `never`

**デフォルト**: `auto`

**Phase**: Phase 1-6（Week 14）

**動作**:
- `auto`: TTY（端末）への出力時のみカラー表示
- `always`: 常にカラー表示（パイプ時も）
- `never`: カラー表示を無効化

**環境変数**: `NO_COLOR` が設定されている場合は自動的に `never` となる

**例**:
```bash
remlc --color=always hello.reml
```

---

## デバッグオプション

### `--trace`

**説明**: コンパイルフェーズのトレースを有効化

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 1-6（Week 15）

**例**:
```bash
remlc --trace hello.reml
```

**出力例**:
```
[TRACE] Parsing started
[TRACE] Parsing completed (0.008s, 512 bytes allocated)
[TRACE] TypeChecking started
[TRACE] TypeChecking completed (0.015s, 1024 bytes allocated)
[TRACE] CoreIR started
[TRACE] CoreIR completed (0.012s, 768 bytes allocated)
[TRACE] CodeGen started
[TRACE] CodeGen completed (0.025s, 2048 bytes allocated)
[TRACE] Total: 0.060s
```

### `--stats`

**説明**: コンパイル統計情報を表示

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 1-6（Week 15）

**例**:
```bash
remlc --stats hello.reml
```

**出力例**:
```
[STATS] Tokens parsed: 42
[STATS] AST nodes: 18
[STATS] Unify calls: 35
[STATS] Optimization passes: 3
[STATS] LLVM instructions: 127
```

### `--verbose <level>`

**説明**: ログの詳細度レベル

**型**: `0` | `1` | `2` | `3`

**デフォルト**: `1`

**Phase**: Phase 1-6（Week 15）

**レベル**:
- `0`: エラーのみ
- `1`: 警告を含む（デフォルト）
- `2`: 情報メッセージを含む
- `3`: デバッグ情報を含む

**環境変数**: `REMLC_LOG=debug` で `--verbose=3` と同等

**例**:
```bash
remlc --verbose=3 hello.reml
```

---

## コンパイルオプション

### `--target <triple>`

**説明**: ターゲットトリプル（プラットフォーム指定）

**型**: 文字列

**デフォルト**: `x86_64-linux`

**Phase**: Phase 1（実装済み）

**サポートターゲット**（Phase 1）:
- `x86_64-linux` — x86_64 Linux (System V ABI)

**Phase 2 以降で追加予定**:
- `x86_64-macos` — x86_64 macOS
- `aarch64-macos` — ARM64 macOS (Apple Silicon)
- `x86_64-windows` — x86_64 Windows (MSVC ABI)

**例**:
```bash
remlc --target=x86_64-linux --emit-ir hello.reml
```

### `--link-runtime`

**説明**: ランタイムライブラリとリンクして実行可能ファイルを生成

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 1-5（実装済み）

**前提条件**: ランタイムライブラリがビルド済みであること

**例**:
```bash
remlc --link-runtime hello.reml
```

**出力ファイル**: `./hello`（実行可能ファイル）

**内部処理**:
1. LLVM IR → オブジェクトファイル（`llc`）
2. オブジェクトファイル + ランタイム → 実行可能ファイル（`cc`）

### `--runtime-path <path>`

**説明**: ランタイムライブラリのパス

**型**: 文字列

**デフォルト**: `runtime/native/build/libreml_runtime.a`

**Phase**: Phase 1-5（実装済み）

**例**:
```bash
remlc --link-runtime --runtime-path=/usr/local/lib/libreml_runtime.a hello.reml
```

### `--verify-ir`

**説明**: 生成された LLVM IR を検証する

**型**: フラグ

**デフォルト**: 無効

**Phase**: Phase 3（実装済み）

**検証内容**:
- LLVM IR の構文チェック
- 型整合性チェック
- 関数シグネチャチェック

**例**:
```bash
remlc --emit-ir --verify-ir hello.reml
```

**成功時**:
```
LLVM IR verification passed.
LLVM IR written to: ./hello.ll
```

**失敗時**:
```
/tmp/test.reml:1:1: エラー[E8002] (LLVM IR 生成): 検証に失敗しました
補足: 関数 'main' のシグネチャが不正です
```

---

## その他

### `--help`

**説明**: ヘルプメッセージを表示

**型**: フラグ

**Phase**: Phase 1（実装済み）

**例**:
```bash
remlc --help
```

### `--version`

**説明**: バージョン情報を表示

**型**: フラグ

**Phase**: Phase 1-6（Week 15）

**例**:
```bash
remlc --version
```

**出力例**:
```
remlc-ocaml 0.1.0 (Phase 1 OCaml implementation)
LLVM version: 18.1.0
Target: x86_64-linux
```

---

## 使用例

### 例1: 基本的なコンパイル

```bash
remlc hello.reml
```

デフォルトでは何も出力されません（エラーがない場合）。

### 例2: AST を確認

```bash
remlc --emit-ast hello.reml
```

### 例3: LLVM IR を生成

```bash
remlc --emit-ir hello.reml --out-dir ./build
```

出力: `./build/hello.ll`

### 例4: 実行可能ファイルを生成

```bash
remlc --link-runtime hello.reml
./hello
```

### 例5: トレースとともにコンパイル

```bash
remlc --trace --stats hello.reml
```

出力:
```
[TRACE] Parsing started
[TRACE] Parsing completed (0.008s, 512 bytes allocated)
...
[STATS] Tokens parsed: 42
[STATS] AST nodes: 18
...
```

### 例6: JSON 形式でエラーを出力

```bash
remlc --format=json error.reml
```

出力:
```json
{
  "diagnostics": [
    {
      "severity": "error",
      "code": "E7001",
      "message": "型が一致しません",
      ...
    }
  ]
}
```

### 例7: CI/CD でのコンパイル

```bash
remlc --format=json --verify-ir --emit-ir hello.reml > diagnostics.json
```

### 例8: カラー出力を無効化

```bash
remlc --color=never hello.reml
```

または環境変数で:
```bash
NO_COLOR=1 remlc hello.reml
```

### 例9: 詳細なデバッグ情報

```bash
remlc --verbose=3 --trace --stats hello.reml
```

### 例10: ターゲット指定

```bash
remlc --target=x86_64-linux --emit-ir hello.reml
```

---

## オプションの組み合わせ

### 推奨される組み合わせ

**開発中**:
```bash
remlc --trace --stats hello.reml
```

**CI/CD**:
```bash
remlc --format=json --verify-ir --emit-ir hello.reml
```

**デバッグ**:
```bash
remlc --verbose=3 --trace --emit-ast --emit-tast hello.reml
```

**リリースビルド**:
```bash
remlc --link-runtime --verify-ir hello.reml
```

### 非推奨の組み合わせ

**`--emit-ast` と `--emit-tast` の同時指定**:
- 両方指定すると出力が混在するため、個別に実行することを推奨

**`--color=always` とリダイレクト**:
- ファイルやパイプにリダイレクトする場合は `--color=never` を推奨

---

## 環境変数

### `REMLC_LOG`

**説明**: ログレベルを制御

**値**: `error` | `warn` | `info` | `debug`

**例**:
```bash
REMLC_LOG=debug remlc hello.reml
```

### `NO_COLOR`

**説明**: カラー出力を無効化（標準仕様）

**値**: 任意（設定されていれば有効）

**例**:
```bash
NO_COLOR=1 remlc hello.reml
```

---

## Phase 2 以降の予定オプション

### `--config <file>`

**説明**: 設定ファイル（`reml.toml`）のパス

**Phase**: Phase 2

**例**:
```bash
remlc --config=./config/dev.toml hello.reml
```

### `--incremental`

**説明**: インクリメンタルコンパイルを有効化

**Phase**: Phase 3

**例**:
```bash
remlc --incremental hello.reml
```

### `--cache-dir <dir>`

**説明**: インクリメンタルコンパイルのキャッシュディレクトリ

**Phase**: Phase 3

**例**:
```bash
remlc --incremental --cache-dir=./cache hello.reml
```

### `--plugin <path>`

**説明**: プラグインを読み込む

**Phase**: Phase 2

**例**:
```bash
remlc --plugin=./my-plugin.so hello.reml
```

---

## トラブルシューティング

### エラー: ランタイムライブラリが見つからない

```
Error: runtime library not found: runtime/native/build/libreml_runtime.a
Please build the runtime first with: make -C runtime/native runtime
```

**対処法**:
```bash
make -C runtime/native runtime
```

### エラー: 不明なオプション

```
Error: unknown option '--foo'
```

**対処法**: `remlc --help` でオプション一覧を確認

### 警告: カラー出力が効かない

**原因**: リダイレクトやパイプ時は `--color=auto` では無効化される

**対処法**:
```bash
remlc --color=always hello.reml | less -R
```

---

## 参考資料

- [CLI アーキテクチャ](./ARCHITECTURE.md)
- [Phase 1-6 計画書](../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md)
- [診断仕様](../../docs/spec/3-6-core-diagnostics-audit.md)

---

**作成者**: Claude Code
**最終更新**: 2025-10-10
**ステータス**: 設計フェーズ
