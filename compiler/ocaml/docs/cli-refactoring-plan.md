# main.ml リファクタリング計画書

**作成日**: 2025-10-10
**Phase**: Phase 1-6 開発者体験整備
**対象**: Week 14-16

## 目次

1. [概要](#概要)
2. [現状分析](#現状分析)
3. [段階的移行戦略](#段階的移行戦略)
4. [モジュール設計](#モジュール設計)
5. [依存関係図](#依存関係図)
6. [リスクとフォローアップ](#リスクとフォローアップ)

---

## 概要

### 目的

現在の `main.ml`（245行）はオプション解析、パイプライン実行、エラーハンドリングが混在したモノリシックな実装となっている。これを責務ごとにモジュール化し、以下を実現する:

1. **保守性の向上**: 各モジュールの責務を明確化
2. **テスタビリティの向上**: ユニットテスト可能な粒度に分割
3. **拡張性の向上**: 新機能追加が容易な設計
4. **互換性の維持**: 既存 CLI インターフェースを保持

### スコープ

**Phase 1-6 で実施**:
- `options.ml` の導入（完了）
- `diagnostic_formatter.ml` の導入（Week 14 後半）
- `pipeline.ml` の導入（Week 15）

**Phase 2 以降で実施**:
- `config.ml`（設定ファイル対応）
- `lsp_server.ml`（LSP サーバ）

---

## 現状分析

### main.ml の構造（2025-10-10 時点）

```
main.ml (245行)
├─ オプション定義 (L9-L19)
│  └─ ref 変数によるグローバル状態
├─ Arg.parse 仕様 (L21-L31)
│  └─ speclist と anon_fun
├─ ヘルパー関数 (L36-L42)
│  ├─ output_filename
│  └─ get_basename
├─ リンク処理 (L50-L76)
│  └─ link_with_runtime
└─ メインロジック (L78-L244)
   ├─ オプション解析 (L79)
   ├─ ファイル読み込み (L88-L90)
   ├─ パーサー実行 (L96)
   ├─ AST 出力 (L99-L102)
   ├─ 型推論 (L106)
   ├─ Typed AST 出力 (L109-L112)
   ├─ Core IR 変換 (L118)
   ├─ 最適化 (L121-L129)
   ├─ LLVM IR 生成 (L132)
   ├─ IR 検証 (L135-L143)
   ├─ IR 出力 (L146-L153)
   ├─ Bitcode 出力 (L156-L161)
   ├─ ランタイムリンク (L164-L190)
   └─ エラーハンドリング (L192-L244)
```

### 問題点

1. **グローバル状態**: ref 変数を多用（`emit_ast`, `emit_tast` など10個以上）
2. **責務の混在**: オプション解析とビジネスロジックが同一ファイル
3. **テスト困難**: メインロジックが `let () = ...` ブロック内で完結
4. **エラーハンドリングの重複**: 各フェーズで個別に診断生成
5. **拡張困難**: 新しいオプション追加にはファイル全体を理解する必要

---

## 段階的移行戦略

### Phase 1: options.ml 導入（Week 14 前半）✅

**実施内容**:
- `compiler/ocaml/src/cli/options.ml` を作成
- ref 変数をレコード型に置き換え
- 環境変数対応（`NO_COLOR`, `REMLC_LOG`）

**main.ml の変更**:
- `Options.parse_args` を呼び出し
- ref 変数を `options` レコードに置き換え
- デフォルト値を `Options.default_options` から取得

**影響範囲**: 最小限（オプション解析部分のみ）

**完了条件**:
- [x] options.ml 実装完了
- [ ] main.ml が options.ml を利用
- [ ] 既存テスト 143 件が成功

### Phase 2: diagnostic_formatter.ml 導入（Week 14 後半）

**実施内容**:
- `compiler/ocaml/src/cli/diagnostic_formatter.ml` を作成
- ソースコードスニペット表示
- カラー出力対応
- JSON 出力対応

**main.ml の変更**:
- エラーハンドリング部分を `Diagnostic_formatter.format` に委譲
- `Diagnostic.to_string` の代わりに `Diagnostic_formatter.format` を使用

**影響範囲**: 中程度（エラー出力部分）

**完了条件**:
- [ ] diagnostic_formatter.ml 実装完了
- [ ] カラー出力が動作
- [ ] JSON 出力が動作
- [ ] 既存テスト成功

### Phase 3: pipeline.ml 導入（Week 15）

**実施内容**:
- `compiler/ocaml/src/cli/pipeline.ml` を作成
- パイプライン実行ロジックを分離
- トレース・統計収集機能を統合

**main.ml の変更**:
- メインロジック (L96-L190) を `Pipeline.run` に移動
- main.ml は薄いラッパーとなる（50行程度を目標）

**影響範囲**: 大（コア部分の全面リファクタリング）

**完了条件**:
- [ ] pipeline.ml 実装完了
- [ ] main.ml が 50-80 行程度に縮小
- [ ] `--trace`, `--stats` が動作
- [ ] 既存テスト成功

---

## モジュール設計

### 移行後の構成（Phase 3 完了後）

```
main.ml (50-80行)
├─ Cli.Options.parse_args       (オプション解析)
├─ Cli.Pipeline.run              (パイプライン実行)
└─ Cli.Diagnostic_formatter.format (診断出力)

compiler/ocaml/src/cli/
├─ options.ml                    (オプション定義)
├─ pipeline.ml                   (パイプライン制御)
├─ diagnostic_formatter.ml       (診断フォーマット)
├─ color.ml                      (カラー出力)
├─ json_formatter.ml             (JSON 出力)
├─ trace.ml                      (トレース機能)
├─ stats.ml                      (統計収集)
└─ help.ml                       (ヘルプメッセージ)
```

### 各モジュールの責務

#### options.ml ✅

**責務**: コマンドラインオプションの定義と解析

**主要型**:
- `options`: オプション設定レコード
- `output_format`: Text | Json
- `color_mode`: Auto | Always | Never

**主要関数**:
- `parse_args : string array -> (options, string) result`
- `default_options : options`

#### pipeline.ml

**責務**: コンパイルフェーズのオーケストレーション

**主要型**:
```ocaml
type phase = Parsing | TypeChecking | CoreIR | Optimization | CodeGen | Linking
type 'a pipeline_result = Success of 'a | Error of Diagnostic.t
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

**実装方針**:
- 各フェーズを `run_phase` でラップ
- トレース・統計収集を自動化
- エラー時は `Diagnostic.t` を返す

#### diagnostic_formatter.ml

**責務**: 診断情報のテキスト/JSON 出力

**主要型**:
```ocaml
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

**実装方針**:
- 仕様書 3-6 に準拠
- カラー出力は `color.ml` に委譲
- JSON 出力は `json_formatter.ml` に委譲

---

## 依存関係図

### モジュール依存関係（Phase 3 完了後）

```
main.ml
  ├─ depends on → cli (library)
  │   ├─ options.ml
  │   ├─ pipeline.ml
  │   │   ├─ depends on → options.ml
  │   │   ├─ depends on → trace.ml
  │   │   ├─ depends on → stats.ml
  │   │   └─ depends on → diagnostic_formatter.ml
  │   ├─ diagnostic_formatter.ml
  │   │   ├─ depends on → color.ml
  │   │   └─ depends on → json_formatter.ml
  │   ├─ color.ml
  │   ├─ json_formatter.ml
  │   ├─ trace.ml
  │   ├─ stats.ml
  │   └─ help.ml
  │
  └─ depends on → compiler/ocaml/src (existing modules)
      ├─ parser_driver.ml
      ├─ type_inference.ml
      ├─ core_ir/
      ├─ llvm_gen/
      └─ diagnostic.ml
```

### ビルド依存関係

**dune ファイル構成**:

```
compiler/ocaml/src/cli/dune
(library
 (name cli)
 (libraries))

compiler/ocaml/src/dune
(executable
 (name remlc)
 (public_name remlc-ocaml)
 (libraries cli parser typed_ast core_ir llvm_gen diagnostic type_error)
 (modules main))
```

---

## リスクとフォローアップ

### 高リスク

#### リスク1: 既存テストの破壊

**発生可能性**: 中
**影響度**: 高
**軽減策**:
- 段階的移行（Phase 1 → 2 → 3）
- 各フェーズ完了後に全テスト実行
- 回帰があれば即座にロールバック

#### リスク2: ビルドの失敗

**発生可能性**: 低
**影響度**: 高
**軽減策**:
- dune の依存関係を慎重に設定
- モジュール追加ごとにビルド確認

### 中リスク

#### リスク3: スケジュール遅延

**発生可能性**: 中
**影響度**: 中
**軽減策**:
- Phase 3 を Week 15 に実施（Week 14 は Phase 1-2 のみ）
- 必要に応じて Phase 3 を Week 16 に延期

#### リスク4: 診断出力の変更

**発生可能性**: 低
**影響度**: 低
**軽減策**:
- `diagnostic_formatter.ml` で既存の `Diagnostic.to_string` を呼び出す
- 段階的に拡張（カラー、JSON）

### フォローアップ

#### Phase 1 完了後（Week 14 前半）

- [ ] main.ml が options.ml を利用することを確認
- [ ] 既存テスト 143 件が成功
- [ ] `--help`, `--version` が動作

#### Phase 2 完了後（Week 14 後半）

- [ ] カラー出力が動作（`--color` オプション）
- [ ] JSON 出力が動作（`--format=json` オプション）
- [ ] ソースコードスニペット表示が動作

#### Phase 3 完了後（Week 15）

- [ ] main.ml が 50-80 行程度に縮小
- [ ] `--trace`, `--stats` が動作
- [ ] パイプライン実行が正常動作

---

## 実装ガイドライン

### コーディング規約

1. **関数型スタイル**: ref 変数を避け、不変データ構造を優先
2. **エラーハンドリング**: `result` 型を使用（例外は最小限）
3. **ドキュメント**: 各モジュールに目的と使用例を記載

### テスト戦略

1. **ユニットテスト**: 各モジュールに対応するテストファイルを作成
   - `tests/test_cli_options.ml`
   - `tests/test_cli_pipeline.ml`
   - `tests/test_diagnostic_formatter.ml`

2. **統合テスト**: CLI 全体の動作を検証
   - `tests/test_cli_integration.ml`

3. **スナップショットテスト**: 診断出力の回帰検出
   - `tests/cli/snapshots/`

### ドキュメント

1. **アーキテクチャ**: `tooling/cli/ARCHITECTURE.md`（既存）
2. **オプション仕様**: `tooling/cli/OPTIONS.md`（既存）
3. **リファクタリング計画**: `compiler/ocaml/docs/cli-refactoring-plan.md`（本ドキュメント）
4. **使用ガイド**: `docs/guides/cli-workflow.md`（Week 16 作成予定）

---

## 参考資料

- [Phase 1-6 計画書](../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md)
- [CLI アーキテクチャ](../../tooling/cli/ARCHITECTURE.md)
- [診断仕様](../../docs/spec/3-6-core-diagnostics-audit.md)
- [Phase 1-5 → 1-6 引き継ぎ](../../docs/plans/bootstrap-roadmap/1-5-to-1-6-handover.md)

---

**作成者**: Claude Code
**最終更新**: 2025-10-10
**ステータス**: Phase 1 実施中
