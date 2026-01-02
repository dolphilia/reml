% remlc-ocaml(1)
% Reml Bootstrap Team
% 2025-10-10

# 名前

remlc-ocaml — Reml OCaml ブートストラップ版コンパイラ CLI

# 書式

`remlc <入力ファイル.reml> [オプション]`

入力ファイルが省略された場合はエラーを返す。標準入力対応は Phase 2 で導入予定。

# 説明

`remlc-ocaml` は Reml 言語の Phase 1 実装で、構文解析から LLVM IR 生成までを一括実行する。  
`--emit-*` オプションで中間成果物を観測し、`--trace` や `--stats` でフェーズ別の挙動を診断できる。

# オプション

## 入力と出力制御

`--emit-ast`  
: 構文解析結果（AST）を標準出力に表示する。

`--emit-tast`  
: 型推論後の AST（Typed AST）を標準出力に表示する。

`--emit-ir`  
: LLVM IR (`.ll`) を出力ディレクトリに生成する。

`--emit-bc`  
: LLVM Bitcode (`.bc`) を出力ディレクトリに生成する。

`--out-dir <dir>`  
: 中間成果物の出力先ディレクトリを指定する（既定値: `.`）。

## 診断とフォーマット

`--format <text|json>`  
: 診断メッセージの出力形式を切り替える。`json` は LSP 互換の構造化出力。

`--color <auto|always|never>`  
: カラー表示を制御する。`NO_COLOR` 環境変数が設定されている場合は自動的に `never` になる。

## トレースと統計

`--trace`  
: コンパイルフェーズごとのトレースを標準エラーに出力する。

`--stats`  
: パースしたトークン数、AST ノード数、unify 呼び出し回数などの統計情報を標準エラーに出力する。

`--verbose <0-3>`  
: ログ詳細度を設定する。`REMLC_LOG` 環境変数でも同じ値を指定できる。

## コンパイル設定

`--target <triple>`  
: ターゲットトリプルを指定する（既定値: `x86_64-linux`）。

`--link-runtime`  
: ランタイムライブラリとリンクして実行可能ファイルを生成する。

`--runtime-path <path>`  
: ランタイム静的ライブラリのパスを指定する（既定値: `runtime/native/build/libreml_runtime.a`）。

`--verify-ir`  
: 生成した LLVM IR を検証する。

## ヘルプ

`--help`, `-help`  
: セクション化された詳細ヘルプを表示して終了する。

# 例

最小サンプルの IR を生成する:

```
opam exec -- dune exec -- \
  remlc examples/cli/add.reml --emit-ir
```

中間成果物をまとめて出力する:

```
opam exec -- dune exec -- \
  remlc examples/cli/emit_suite.reml \
  --emit-ast --emit-tast --emit-ir \
  --out-dir build/emit
```

トレースと統計を収集する:

```
opam exec -- dune exec -- \
  remlc examples/cli/trace_sample.reml \
  --trace --stats 2>trace.log
```

実行可能ファイルを生成し、ランタイムとリンクする:

```
opam exec -- dune exec -- \
  remlc examples/cli/add.reml \
  --link-runtime --out-dir build/bin
```

# 環境変数

`REMLC_LOG`  
: `error` / `warn` / `info` / `debug` のいずれかでログ詳細度を設定する。

`NO_COLOR`  
: 設定されている場合はカラー表示を無効化する。

# ファイル

`runtime/native/build/libreml_runtime.a`  
: 既定でリンクされるランタイム静的ライブラリ。

# 関連項目

`../toolingcli-workflow.md`,  
`../toolingtrace-output.md`,  
`../toolingdiagnostic-format.md`
