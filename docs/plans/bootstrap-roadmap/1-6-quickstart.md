# Phase 1-6 開発者体験整備 クイックスタートガイド

**対象フェーズ**: Phase 1-6 開発者体験整備
**推定期間**: Week 14-16（3週間）
**前提**: Phase 1-5 完了（ランタイム連携実装済み）

## 目次

1. [概要](#概要)
2. [環境準備](#環境準備)
3. [Week 14: 診断出力システム強化](#week-14-診断出力システム強化)
4. [Week 15: トレース・ログ機能](#week-15-トレースログ機能)
5. [Week 16: ヘルプ・ドキュメント整備](#week-16-ヘルプドキュメント整備)
6. [完了条件](#完了条件)

---

## 概要

Phase 1-6 では、`remlc-ocaml` CLI の開発者体験を向上させます。具体的には：

- **診断出力**: エラーメッセージの可読性向上（ソースコードスニペット、カラー対応）
- **トレース**: コンパイルフェーズの可視化と性能分析
- **ドキュメント**: CLI使用ガイドとサンプルコード整備

### Phase 1-6 の位置付け

```
Phase 1-3: コンパイラコア実装 ✅
  ├─ Phase 1: Parser & Frontend ✅
  ├─ Phase 2: Typer MVP ✅
  └─ Phase 3: Core IR & LLVM ✅

Phase 1-5: ランタイム連携 ✅
  └─ 最小ランタイム API 実装 ✅

Phase 1-6: 開発者体験整備 ← 今ここ
  ├─ 診断出力強化
  ├─ トレース・ログ
  └─ ドキュメント整備

Phase 1-7: Linux検証 (次フェーズ)
```

---

## 環境準備

### 1. Phase 1-5 完了確認

```bash
cd /Users/dolphilia/github/kestrel/compiler/ocaml

# すべてのテストが成功することを確認
opam exec -- dune test

# ビルドが成功することを確認
opam exec -- dune build

# ランタイムライブラリが存在することを確認
ls -l ../../runtime/native/build/libreml_runtime.a
```

### 2. 関連ドキュメント確認

必須ドキュメント:
- [1-6-developer-experience.md](1-6-developer-experience.md) - 計画書
- [1-5-to-1-6-handover.md](1-5-to-1-6-handover.md) - 引き継ぎ情報
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) - 診断仕様

### 3. 既存実装の確認

```bash
# CLIエントリポイント
cat src/main.ml | head -50

# 診断システム
cat src/diagnostic.ml | head -50

# 型エラー診断
cat src/type_error.ml | grep -A 5 "to_diagnostic"
```

---

## Week 14: 診断出力システム強化

### 目標

エラーメッセージの可読性を向上させ、開発者がエラーを素早く理解・修正できるようにする。

### タスク1: ソースコードスニペット表示

**実装ファイル**: `compiler/ocaml/src/cli/diagnostic_formatter.ml` (新規)

**実装内容**:
```ocaml
(* ソースコードスニペットを抽出して表示 *)
val format_snippet : source:string -> span:Ast.span -> string

(* 例:
     3 | fn add(a: i64, b: String) -> i64 = a + b
       |                  ^^^^^^ 型が一致しません
*)
```

**手順**:
1. `src/cli/` ディレクトリを作成
2. `diagnostic_formatter.ml` を実装
3. `Diagnostic.t` にソースコード文字列を追加
4. `main.ml` から呼び出し

**テスト**:
```bash
# エラーを含むファイルを作成
cat > /tmp/test_error.reml << 'EOF'
fn add(a: i64, b: String) -> i64 = a + b
EOF

# 診断出力を確認
opam exec -- dune exec -- remlc /tmp/test_error.reml 2>&1
```

**期待出力**:
```
/tmp/test_error.reml:1:18: エラー[E7001] (型システム): 型が一致しません
    1 | fn add(a: i64, b: String) -> i64 = a + b
      |                  ^^^^^^
補足: 期待される型: i64
補足: 実際の型:     String
```

### タスク2: カラーコード対応

**実装ファイル**: `compiler/ocaml/src/cli/color.ml` (新規)

**実装内容**:
```ocaml
type color_mode = Auto | Always | Never

val colorize : color_mode -> severity:string -> text:string -> string

(* ANSI エスケープシーケンス *)
val red : string -> string
val yellow : string -> string
val blue : string -> string
```

**手順**:
1. `color.ml` を実装
2. `--color=auto|always|never` オプションを追加
3. 環境変数 `NO_COLOR` に対応
4. `diagnostic_formatter.ml` と統合

**テスト**:
```bash
# カラー出力確認
opam exec -- dune exec -- remlc /tmp/test_error.reml --color=always 2>&1

# カラー無効確認
opam exec -- dune exec -- remlc /tmp/test_error.reml --color=never 2>&1
```

### タスク3: JSON出力フォーマット

**実装ファイル**: `compiler/ocaml/src/cli/json_formatter.ml` (新規)

**実装内容**:
```ocaml
(* JSON形式で診断を出力 *)
val format_diagnostics : Diagnostic.t list -> string

(* 出力例:
{
  "diagnostics": [
    {
      "severity": "error",
      "code": "E7001",
      "message": "型が一致しません",
      "location": {
        "file": "/tmp/test_error.reml",
        "line": 1,
        "column": 18
      },
      "notes": [
        "期待される型: i64",
        "実際の型:     String"
      ]
    }
  ]
}
*)
```

**手順**:
1. `json_formatter.ml` を実装
2. `--format=json` オプションを追加
3. LSP互換性を考慮した構造
4. スキーマ定義 `docs/schemas/diagnostic.schema.json` を作成

**テスト**:
```bash
# JSON出力確認
opam exec -- dune exec -- remlc /tmp/test_error.reml --format=json 2>&1 | jq .
```

### Week 14 完了条件

- [ ] ソースコードスニペット表示が動作する
- [ ] カラーコード対応が動作する（`--color` オプション）
- [ ] JSON出力フォーマットが動作する（`--format=json` オプション）
- [ ] 既存テストがすべて成功する（143/143）
- [ ] `docs/guides/diagnostic-format.md` を作成

---

## Week 15: トレース・ログ機能

### 目標

コンパイルフェーズの実行時間と統計情報を可視化し、パフォーマンス分析を可能にする。

### タスク1: `--trace` オプション実装

**実装ファイル**: `compiler/ocaml/src/cli/trace.ml` (新規)

**実装内容**:
```ocaml
(* フェーズトレース *)
type phase =
  | Parsing
  | TypeChecking
  | CoreIR
  | Optimization
  | CodeGen

val start_phase : phase -> unit
val end_phase : phase -> unit

(* 出力例:
[TRACE] Parsing started
[TRACE] Parsing completed (0.012s)
[TRACE] TypeChecking started
[TRACE] TypeChecking completed (0.034s)
...
*)
```

**手順**:
1. `trace.ml` を実装
2. 各フェーズの開始・終了を記録
3. `Unix.gettimeofday` で時間計測
4. `Gc.stat` でメモリ使用量取得
5. `--trace` オプションを追加

**統合箇所**:
```ocaml
(* main.ml *)
let () =
  if !trace then Trace.start_phase Parsing;
  let ast = Parser_driver.parse lexbuf in
  if !trace then Trace.end_phase Parsing;
  (* ... *)
```

**テスト**:
```bash
# トレース出力確認
cat > /tmp/test_trace.reml << 'EOF'
fn main() -> i64 = 42
EOF

opam exec -- dune exec -- remlc /tmp/test_trace.reml --trace 2>&1
```

**期待出力**:
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

### タスク2: 統計情報収集

**実装ファイル**: `compiler/ocaml/src/cli/stats.ml` (新規)

**実装内容**:
```ocaml
(* 統計情報収集 *)
type stats = {
  token_count: int;
  ast_node_count: int;
  unify_calls: int;
  optimization_passes: int;
  llvm_instructions: int;
}

val collect : unit -> stats
val print : stats -> unit
```

**収集箇所**:
- `Lexer`: トークン数
- `Parser`: ASTノード数
- `Type_inference`: unify呼び出し回数
- `Pipeline`: 最適化パス適用回数
- `Codegen`: LLVM IR命令数

**テスト**:
```bash
# 統計情報確認
opam exec -- dune exec -- remlc /tmp/test_trace.reml --trace --stats 2>&1
```

**期待出力**:
```
[STATS] Tokens parsed: 12
[STATS] AST nodes: 8
[STATS] Unify calls: 15
[STATS] Optimization passes: 3
[STATS] LLVM instructions: 42
```

### タスク3: `--verbose` レベル管理

**実装内容**:
```ocaml
(* ログレベル *)
type log_level = Error | Warning | Info | Debug

val set_level : log_level -> unit
val log : log_level -> string -> unit
```

**オプション**:
- `--verbose=0`: エラーのみ
- `--verbose=1`: 警告含む（デフォルト）
- `--verbose=2`: 情報含む
- `--verbose=3`: デバッグ情報含む

**環境変数**: `REMLC_LOG=debug`

### Week 15 完了条件

- [ ] `--trace` オプションが動作する
- [ ] 統計情報収集が動作する（`--stats` オプション）
- [ ] `--verbose` レベル管理が動作する
- [ ] 既存テストがすべて成功する
- [ ] `docs/guides/trace-output.md` を作成

---

## Week 16: ヘルプ・ドキュメント整備

### 目標

CLI の使いやすさを向上させ、ユーザーが自己解決できるドキュメントを整備する。

### タスク1: `--help` 出力の充実

**実装内容**:
```bash
$ remlc --help
remlc-ocaml - Reml compiler (Phase 1 OCaml implementation)

USAGE:
  remlc [OPTIONS] <file.reml>

INPUT:
  <file.reml>          Input Reml source file
  -                    Read from stdin

OUTPUT:
  --emit-ast           Emit AST to stdout
  --emit-tast          Emit Typed AST to stdout
  --emit-ir            Emit LLVM IR to output directory
  --emit-bc            Emit LLVM Bitcode to output directory
  --out-dir <dir>      Output directory (default: .)

DIAGNOSTICS:
  --format <format>    Output format: text|json (default: text)
  --color <mode>       Color mode: auto|always|never (default: auto)

DEBUG:
  --trace              Enable phase tracing
  --stats              Show compilation statistics
  --verbose <level>    Verbosity level: 0-3 (default: 1)

COMPILATION:
  --target <triple>    Target triple (default: x86_64-linux)
  --link-runtime       Link with runtime library
  --runtime-path <path> Path to runtime library

EXAMPLES:
  remlc hello.reml --emit-ir
  remlc hello.reml --link-runtime -o hello
  remlc hello.reml --trace --stats
```

**手順**:
1. `main.ml` のヘルプメッセージを拡張
2. セクション分け（INPUT, OUTPUT, DIAGNOSTICS, etc.）
3. 使用例を追加

### タスク2: CLI使用ガイド作成

**ファイル**: `docs/guides/cli-workflow.md` (新規)

**内容**:
1. **基本的な使い方**
   - ソースファイルのコンパイル
   - LLVM IR の生成
   - 実行可能ファイルの生成

2. **診断出力の活用**
   - エラーメッセージの読み方
   - JSON出力の活用（CI統合）

3. **トレース・デバッグ**
   - パフォーマンス分析
   - 統計情報の解釈

4. **トラブルシューティング**
   - よくあるエラー
   - デバッグ方法

### タスク3: サンプルコード整備

**ディレクトリ**: `examples/cli/` (新規)

**サンプル**:
1. `hello.reml` - 最小限の例
2. `arithmetic.reml` - 算術演算
3. `control_flow.reml` - 制御フロー
4. `type_error.reml` - 型エラーのデモ

**制約**: Phase 1-5 の制約により、文字列パラメータとタプルは使用不可

### タスク4: CI統合

**ファイル**: `compiler/ocaml/tests/cli/test_cli_snapshots.ml` (新規)

**テスト内容**:
1. **スナップショットテスト**
   - 各サンプルの出力を記録
   - 回帰検出

2. **パフォーマンステスト**
   - 10MB入力の解析時間
   - メモリ使用量

3. **ドキュメント検証**
   - `--help` 出力の妥当性
   - サンプルコードの動作確認

### Week 16 完了条件

- [ ] `--help` 出力が充実している
- [ ] `docs/guides/cli-workflow.md` を作成
- [ ] サンプルコード（4件）を作成
- [ ] CLI スナップショットテストが動作する
- [ ] 既存テストがすべて成功する
- [ ] Phase 1-6 完了報告書を作成

---

## 完了条件

Phase 1-6 完了の判定基準：

### 機能要件

- [x] ソースコードスニペット表示
- [x] カラーコード対応（`--color` オプション）
- [x] JSON出力フォーマット（`--format=json` オプション）
- [x] トレース機能（`--trace` オプション）
- [x] 統計情報収集（`--stats` オプション）
- [x] `--verbose` レベル管理
- [x] 充実した `--help` 出力

### ドキュメント

- [x] `docs/guides/diagnostic-format.md`
- [x] `docs/guides/trace-output.md`
- [x] `docs/guides/cli-workflow.md`
- [x] サンプルコード（4件）

### テスト

- [x] 既存テストがすべて成功（143/143）
- [x] CLI スナップショットテスト
- [x] パフォーマンステスト（基準値測定）

### 成果物

- [x] Phase 1-6 完了報告書（`compiler/ocaml/docs/phase1-6-completion-report.md`）
- [x] `0-3-audit-and-metrics.md` への記録

---

## 次のステップ

Phase 1-6 完了後は **Phase 1-7: Linux検証インフラ** へ進みます：

- Docker コンテナ整備
- クロスコンパイル環境構築
- CI/CD 統合
- メモリ検証自動化

詳細は [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md) を参照してください。

---

**作成日**: 2025-10-10
**対象者**: Phase 1-6 実装担当者
**想定期間**: 3週間（Week 14-16）
