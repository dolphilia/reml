# Reml CLI アーキテクチャ設計

**Phase**: 1-6 開発者体験整備
**作成日**: 2025-10-10
**ステータス**: 設計フェーズ

## 目次

1. [概要](#概要)
2. [設計原則](#設計原則)
3. [アーキテクチャ](#アーキテクチャ)
4. [モジュール設計](#モジュール設計)
5. [データフロー](#データフロー)
6. [拡張性](#拡張性)
7. [実装計画](#実装計画)

---

## 概要

### 目的

`remlc-ocaml` CLI は Phase 1 の OCaml 実装コンパイラのエントリポイントとして、以下の機能を提供する：

1. **コンパイルパイプライン**: ソースファイルから LLVM IR / 実行可能ファイルまでの変換
2. **診断出力**: エラー・警告メッセージの可読性向上
3. **観測機能**: コンパイルフェーズのトレース、統計情報収集
4. **開発者体験**: 直感的なオプション体系、充実したヘルプ

### スコープ

**Phase 1-6 で実装**:
- コマンドラインオプション解析
- パイプラインオーケストレーション
- 診断出力フォーマッター（テキスト、JSON）
- トレース・ログ機能
- 統計情報収集

**Phase 2 以降で実装**:
- Language Server Protocol (LSP) サーバ
- 設定ファイル（`reml.toml`）対応
- プラグインシステム
- インクリメンタルコンパイル

---

## 設計原則

### 1. モジュール分離

**原則**: 各機能を独立したモジュールに分離し、依存関係を最小化する。

**理由**:
- テスタビリティの向上
- 保守性の向上
- 再利用性の向上

### 2. 仕様準拠

**原則**: 仕様書 [3-6-core-diagnostics-audit.md](../../docs/spec/3-6-core-diagnostics-audit.md) に準拠した診断システムを実装する。

**理由**:
- セルフホスト版との互換性確保
- LSP 連携の基盤整備
- 監査・メトリクス収集の一貫性

### 3. 段階的拡張

**原則**: 基本機能を優先し、高度な機能は Phase 2 以降に延期する。

**理由**:
- Phase 1-6 のスケジュール遵守（Week 14-16）
- 早期フィードバックの獲得
- リスク最小化

### 4. 開発者体験優先

**原則**: エラーメッセージの可読性、ヘルプの充実、直感的なオプション体系を重視する。

**理由**:
- Reml の設計指針（[0-1-project-purpose.md](../../docs/spec/0-1-project-purpose.md) §2.2）に準拠
- ユーザー満足度の向上
- コミュニティ成長の促進

---

## アーキテクチャ

### 全体構成

```
┌──────────────┐
│  ユーザー      │
└──────┬───────┘
       │ コマンドライン
       ▼
┌──────────────────────────────────────┐
│  CLI エントリポイント (main.ml)        │
│  - オプション解析                       │
│  - ヘルプ表示                          │
│  - 設定ロード（Phase 2）               │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│  パイプラインオーケストレーター          │
│  (pipeline.ml)                        │
│  - フェーズ管理                        │
│  - トレース制御                        │
│  - 統計収集                           │
└──────┬───────────────────────────────┘
       │
       ├─────────────┬──────────────┬─────────────┐
       ▼             ▼              ▼             ▼
┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐
│  Parser   │ │  Typer    │ │  Core IR  │ │  Codegen  │
│  Driver   │ │  Driver   │ │  Pipeline │ │  Driver   │
└───────┬───┘ └───────┬───┘ └───────┬───┘ └───────┬───┘
        │             │              │             │
        └─────────────┴──────────────┴─────────────┘
                       │
                       ▼
        ┌──────────────────────────────┐
        │  診断システム                  │
        │  (diagnostic_formatter.ml)    │
        │  - テキスト出力               │
        │  - JSON 出力                 │
        │  - カラーコード               │
        └──────────────────────────────┘
```

### レイヤー構造

#### Layer 1: CLI インタフェース
- **責務**: コマンドライン引数の解析、ヘルプ表示、エラーハンドリング
- **主要モジュール**: `main.ml`, `options.ml`, `help.ml`

#### Layer 2: パイプライン制御
- **責務**: コンパイルフェーズのオーケストレーション、トレース、統計収集
- **主要モジュール**: `pipeline.ml`, `trace.ml`, `stats.ml`

#### Layer 3: 診断・出力
- **責務**: エラー・警告メッセージのフォーマット、JSON 出力
- **主要モジュール**: `diagnostic_formatter.ml`, `json_formatter.ml`, `color.ml`

#### Layer 4: コンパイラコア
- **責務**: パース、型推論、IR 生成
- **モジュール**: 既存の `compiler/ocaml/src/` 配下のモジュール

---

## モジュール設計

### 1. `options.ml` — オプション定義

**責務**: コマンドラインオプションの定義と解析

**型定義**:
```ocaml
(* オプション設定 *)
type options = {
  (* 入力 *)
  input_file: string;
  use_stdin: bool;

  (* 出力 *)
  emit_ast: bool;
  emit_tast: bool;
  emit_ir: bool;
  emit_bc: bool;
  out_dir: string;

  (* 診断 *)
  format: output_format;
  color: color_mode;

  (* デバッグ *)
  trace: bool;
  stats: bool;
  verbose: int;

  (* コンパイル *)
  target: string;
  link_runtime: bool;
  runtime_path: string;
  verify_ir: bool;
}

and output_format = Text | Json

and color_mode = Auto | Always | Never
```

**主要関数**:
```ocaml
val parse_args : string array -> (options, string) result
val default_options : options
```

### 2. `pipeline.ml` — パイプラインオーケストレーション

**責務**: コンパイルフェーズの管理、トレース制御

**型定義**:
```ocaml
(* コンパイルフェーズ *)
type phase =
  | Parsing
  | TypeChecking
  | CoreIR
  | Optimization
  | CodeGen
  | Linking

(* パイプライン結果 *)
type 'a pipeline_result =
  | Success of 'a
  | Error of Diagnostic.t

(* パイプラインコンテキスト *)
type context = {
  options: Options.options;
  source: string;
  trace_enabled: bool;
  stats_enabled: bool;
}
```

**主要関数**:
```ocaml
val run : context -> unit pipeline_result
val run_phase : context -> phase -> (unit -> 'a) -> 'a pipeline_result
```

### 3. `diagnostic_formatter.ml` — 診断フォーマッター

**責務**: 診断情報のテキスト出力フォーマット

**型定義**:
```ocaml
(* フォーマットオプション *)
type format_options = {
  color: Color.color_mode;
  show_source: bool;
  context_lines: int;
}
```

**主要関数**:
```ocaml
val format_diagnostic : format_options -> string -> Diagnostic.t -> string
val format_snippet : string -> Diagnostic.span -> string
```

**出力例**:
```
/tmp/test.reml:2:5: エラー[E7001] (型システム): 型が一致しません
    1 | fn add(a: i64, b: i64) -> i64 =
    2 |   a + "hello"
      |       ^^^^^^^ 期待される型: i64、実際の型: String
補足: 数値演算には整数型が必要です
```

### 4. `json_formatter.ml` — JSON 出力フォーマッター

**責務**: 診断情報の JSON 出力（LSP 互換）

**主要関数**:
```ocaml
val format_diagnostics : Diagnostic.t list -> string
```

**出力例**:
```json
{
  "diagnostics": [
    {
      "severity": "error",
      "code": "E7001",
      "domain": "type",
      "message": "型が一致しません",
      "location": {
        "file": "/tmp/test.reml",
        "line": 2,
        "column": 5
      },
      "notes": [
        "期待される型: i64",
        "実際の型: String"
      ]
    }
  ]
}
```

### 5. `color.ml` — カラーコード対応

**責務**: ANSI エスケープシーケンスによるカラー出力

**型定義**:
```ocaml
type color_mode = Auto | Always | Never

type color =
  | Red
  | Yellow
  | Blue
  | Green
  | Gray
```

**主要関数**:
```ocaml
val colorize : color_mode -> color -> string -> string
val should_use_color : color_mode -> bool
```

### 6. `trace.ml` — トレース・ログ機能

**責務**: コンパイルフェーズのトレース、時間計測

**型定義**:
```ocaml
type phase_info = {
  phase: Pipeline.phase;
  start_time: float;
  end_time: float;
  memory_before: Gc.stat;
  memory_after: Gc.stat;
}
```

**主要関数**:
```ocaml
val start_phase : Pipeline.phase -> unit
val end_phase : Pipeline.phase -> unit
val print_summary : unit -> unit
```

**出力例**:
```
[TRACE] Parsing started
[TRACE] Parsing completed (0.008s, 512 bytes allocated)
[TRACE] TypeChecking started
[TRACE] TypeChecking completed (0.015s, 1024 bytes allocated)
[TRACE] Total: 0.023s
```

### 7. `stats.ml` — 統計情報収集

**責務**: コンパイル統計の収集・表示

**型定義**:
```ocaml
type stats = {
  token_count: int;
  ast_node_count: int;
  unify_calls: int;
  optimization_passes: int;
  llvm_instructions: int;
}
```

**主要関数**:
```ocaml
val collect : unit -> stats
val print : stats -> unit
```

### 8. `help.ml` — ヘルプメッセージ生成

**責務**: `--help` オプションの出力生成

**主要関数**:
```ocaml
val print_help : unit -> unit
val print_version : unit -> unit
```

**出力例**:
```
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
```

---

## データフロー

### 正常系フロー

```
ユーザー
  │
  └─> main.ml: オプション解析
        │
        └─> pipeline.ml: パイプライン実行
              │
              ├─> Parser_driver.parse
              │     └─> AST
              │
              ├─> Type_inference.infer_compilation_unit
              │     └─> Typed AST
              │
              ├─> Core_ir.Desugar.desugar_compilation_unit
              │     └─> Core IR
              │
              ├─> Core_ir.Pipeline.optimize_module
              │     └─> Optimized IR
              │
              └─> Codegen.codegen_module
                    └─> LLVM Module
                          │
                          └─> 出力（.ll, .bc, 実行可能ファイル）
```

### エラーフロー

```
エラー発生
  │
  └─> Diagnostic.t 生成
        │
        ├─> [--format=text]
        │     └─> diagnostic_formatter.ml
        │           └─> テキスト出力
        │
        └─> [--format=json]
              └─> json_formatter.ml
                    └─> JSON 出力
```

### トレースフロー

```
[--trace] オプション指定
  │
  └─> pipeline.ml: 各フェーズ実行前後
        │
        └─> trace.ml: 時間計測・メモリ計測
              │
              └─> [フェーズ完了後]
                    └─> trace.ml: サマリ出力
```

---

## 拡張性

### Phase 2 での拡張計画

#### 1. 設定ファイル対応（`reml.toml`）

**新規モジュール**: `config.ml`

**機能**:
- `reml.toml` の読み込み
- CLI オプションとのマージ（CLI 優先）
- プロファイル対応（`[profile.dev]`, `[profile.release]`）

#### 2. LSP サーバ

**新規モジュール**: `lsp_server.ml`

**機能**:
- Language Server Protocol 実装
- リアルタイム診断
- 自動補完

#### 3. プラグインシステム

**新規モジュール**: `plugin.ml`

**機能**:
- カスタムリントルール
- コードジェネレータ
- カスタムパス

### 拡張ポイント

#### 1. 新しい出力フォーマット追加

`diagnostic_formatter.ml` にフォーマット関数を追加：

```ocaml
val format_html : Diagnostic.t -> string
val format_markdown : Diagnostic.t -> string
```

#### 2. 新しいトレース情報追加

`stats.ml` に統計項目を追加：

```ocaml
type stats = {
  (* 既存 *)
  token_count: int;
  ast_node_count: int;
  (* 新規 *)
  type_errors: int;
  warnings: int;
}
```

#### 3. 新しいコマンドラインオプション追加

`options.ml` にフィールドを追加：

```ocaml
type options = {
  (* 既存 *)
  input_file: string;
  (* 新規 *)
  incremental: bool;
  cache_dir: string;
}
```

---

## 実装計画

### Week 14 前半: 基盤整備

**タスク**:
1. ディレクトリ構造作成
2. `options.ml` 実装
3. 既存 `main.ml` のリファクタリング計画

**成果物**:
- `tooling/cli/ARCHITECTURE.md`（本ドキュメント）
- `tooling/cli/OPTIONS.md`
- `tooling/cli/options.ml`（スケルトン）

### Week 14 後半: 診断強化

**タスク**:
1. `diagnostic_formatter.ml` 実装
2. `color.ml` 実装
3. `json_formatter.ml` 実装

**成果物**:
- ソースコードスニペット表示
- カラー出力対応
- JSON 出力対応

### Week 15 前半: 観測機能

**タスク**:
1. `trace.ml` 実装
2. `stats.ml` 実装
3. `pipeline.ml` 実装

**成果物**:
- フェーズトレース
- 統計情報収集
- パイプラインオーケストレーション

### Week 15 後半: UX 向上

**タスク**:
1. `help.ml` 実装
2. サンプルコード整備
3. ドキュメント更新

**成果物**:
- 充実したヘルプメッセージ
- 使用例 4 件
- `docs/guides/cli-workflow.md` 更新

### Week 16: 統合テスト

**タスク**:
1. CLI スナップショットテスト
2. パフォーマンステスト
3. ドキュメント検証

**成果物**:
- テストスイート
- Phase 1-6 完了報告書

---

## 参考資料

- [Phase 1-6 計画書](../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md)
- [診断仕様](../../docs/spec/3-6-core-diagnostics-audit.md)
- [プロジェクト目的](../../docs/spec/0-1-project-purpose.md)
- [Phase 1-5 → 1-6 引き継ぎ](../../docs/plans/bootstrap-roadmap/1-5-to-1-6-handover.md)

---

**作成者**: Claude Code
**最終更新**: 2025-10-10
**ステータス**: 設計フェーズ
