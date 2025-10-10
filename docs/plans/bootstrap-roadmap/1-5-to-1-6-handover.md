# Phase 1-5 → Phase 1-6 引き継ぎドキュメント

**作成日**: 2025-10-10
**Phase 1-5 完了日**: 2025-10-10
**Phase 1-6 開始予定**: 2025-10-10 以降

## Phase 1-5 の成果物

### ✅ 完了した実装

**LLVM連携**:
- ランタイム関数宣言生成（`mem_alloc`, `inc_ref`, `dec_ref`, `panic`, `print_i64`, `memcpy`）
- 文字列リテラル生成時の `mem_alloc` 呼び出し
- リンクヘルパー（`runtime_link.ml`）
- CLI 統合（`--link-runtime` オプション）

**ランタイム実装**:
- 最小ランタイム API（6関数）
- メモリアロケータ（malloc ベース、8バイト境界調整）
- 参照カウント（RC操作、型別デストラクタ）
- パニックハンドラ（stderr出力、exit(1)）

**テストインフラ**:
- 統合テストスクリプト（`tests/test_runtime_integration.sh`）
- メモリ検証スクリプト（`scripts/verify_memory.sh`）
- ランタイム単体テスト（14/14成功）

**詳細**: [phase1-5-completion-report.md](../../compiler/ocaml/docs/phase1-5-completion-report.md)

### ⏳ Phase 2 へ延期

以下のタスクは技術的課題により Phase 2 へ延期：

| タスク | 理由 | Phase 2 対応内容 |
|--------|------|------------------|
| タプル/レコード生成時の `mem_alloc` | Core IR に `TupleConstruct` ノードが未実装 | Core IR拡張 + 糖衣削除パス実装 |
| スコープ終了時の `dec_ref` 挿入 | FAT pointer の構造体型判定に所有権解析が必要 | 所有権解析 + 型情報ベース判定 |
| 実行可能ファイル生成 E2E テスト | 文字列パラメータ処理の安定化が必要 | Core IR 変換パイプライン安定化 |
| メモリリーク検証 | 実行可能バイナリ生成が未完了 | Valgrind/ASan 包括検証 |

---

## Phase 1-6 の目標

Phase 1-6 では開発者体験（DX）の整備に注力します：

### 主要タスク（Week 14-16）

1. **診断出力システム強化**（Week 14）
   - エラー・警告メッセージの改善
   - ソースコードスニペット表示
   - カラーコード対応

2. **トレース・ログ機能**（Week 14-15）
   - `--trace` オプション実装
   - 各フェーズの時間計測
   - 統計情報収集

3. **ヘルプ・ドキュメント整備**（Week 15-16）
   - `--help` 出力の充実
   - CLI使用ガイドの作成
   - サンプルコード整備

4. **CI統合とテスト**（Week 16）
   - CLI スナップショットテスト
   - パフォーマンステスト
   - ドキュメント検証

---

## 前提条件の確認

### Phase 1-5 から引き継ぐ実装

#### ✅ CLI 基盤（既存実装）
**ファイル**: `compiler/ocaml/src/main.ml`

**既存オプション**:
```ocaml
let speclist = [
  ("--emit-ast", Arg.Set emit_ast, "Emit AST to stdout");
  ("--emit-tast", Arg.Set emit_tast, "Emit Typed AST to stdout");
  ("--emit-ir", Arg.Set emit_ir, "Emit LLVM IR (.ll) to output directory");
  ("--emit-bc", Arg.Set emit_bc, "Emit LLVM Bitcode (.bc) to output directory");
  ("--verify-ir", Arg.Set verify_ir, "Verify generated LLVM IR");
  ("--link-runtime", Arg.Set link_runtime, "Link with runtime library");
  ("--runtime-path", Arg.Set_string runtime_path, "Path to runtime library");
  ("--out-dir", Arg.Set_string out_dir, "Output directory");
  ("--target", Arg.Set_string target, "Target triple");
]
```

#### ✅ パイプライン統合（既存実装）
**フロー**: Parser → Typer → Core IR → LLVM

**現在の処理**:
1. `Parser_driver.parse` でAST生成
2. `Type_inference.infer_module` で型推論
3. `Core_ir.Desugar.desugar_module` で糖衣削除
4. `Core_ir.Pipeline.optimize` で最適化
5. `Llvm_gen.Codegen.codegen_module` でLLVM IR生成

#### ✅ 診断システム（部分実装）
**ファイル**:
- `compiler/ocaml/src/diagnostic.ml` - 診断メッセージ構造
- `compiler/ocaml/src/type_error.ml` - 型エラー診断

**現在の出力形式**:
```
/path/to/file.reml:2:5: エラー[E7001] (型システム): 型が一致しません
補足: 期待される型: i64
補足: 実際の型:     Bool
```

---

## Phase 1-6 開始前のチェックリスト

### 環境確認

- [x] OCaml 環境が正しく設定されているか (`opam env`)
- [x] すべてのテストが成功するか (`dune test` - 143/143成功確認済み)
- [x] ビルドが通るか (`dune build`)
- [x] LLVM 18 が正しくインストールされているか (`llvm-config --version`)
- [x] ランタイムライブラリがビルド済みか (`runtime/native/build/libreml_runtime.a`)

### 仕様書の理解

- [ ] [1-6-developer-experience.md](1-6-developer-experience.md) を読む（Phase 1-6メイン）
- [ ] [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) を読む（診断仕様）
- [ ] [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) を確認（指標定義）

### 計画書の確認

- [ ] Phase 1-6 マイルストーンの達成条件を理解
- [ ] 作業ブレークダウンを確認
- [ ] 既存実装との統合ポイントを把握

---

## Phase 1-5 から引き継ぐ技術的知見

### 1. LLVM連携の現状

**動作確認済み**:
- 基本的な算術演算（`fn add(a: i64, b: i64) -> i64 = a + b`）
- 文字列リテラル生成（`"Hello, Reml!"`）
- ランタイム関数宣言

**未動作**:
- 文字列パラメータを含む関数（Core IR変換でクラッシュ）
- タプル/レコードリテラル（Core IR ノード未実装）

**影響**: Phase 1-6 で使用するサンプルコードは、上記の制限を考慮して設計する必要がある。

### 2. 診断システムの現状

**実装済み**:
- エラーコード体系（E7001-E7015）
- 型エラーの専用診断（`ConditionNotBool`, `BranchTypeMismatch` 等）
- 行・列番号の正確な報告

**未実装**:
- ソースコードスニペット表示
- カラーコード対応
- Fix-it提案の自動生成

**影響**: Phase 1-6 で診断出力を拡張する際、既存の `Diagnostic` 構造体を活用できる。

### 3. テストインフラの現状

**既存テスト**:
- ユニットテスト（143件）
- ゴールデンテスト（3件）
- 統合テスト（`test_runtime_integration.sh`）

**未実装**:
- CLI スナップショットテスト
- パフォーマンステスト
- ドキュメント検証テスト

**影響**: Phase 1-6 で CLI テストを追加する際、既存テストフレームワークを拡張できる。

---

## 推奨される Phase 1-6 の進め方

### Week 14: 診断出力システム強化

1. **ソースコードスニペット表示**
   - `Diagnostic` に `source: string` フィールドを追加
   - Span情報から前後2行を抽出
   - ポインタ表示（`^^^`）の実装

2. **カラーコード対応**
   - `--color=auto|always|never` オプション追加
   - ANSI エスケープシーケンスの実装
   - エラー=赤、警告=黄、情報=青

3. **JSON出力フォーマット**
   - `--format=json` オプション追加
   - `diagnostic_schema.json` の定義
   - 機械判読可能な構造化出力

**成果物**:
- `compiler/ocaml/src/cli/diagnostic_formatter.ml` （新規）
- `compiler/ocaml/src/diagnostic.ml` （拡張）
- `docs/guides/diagnostic-format.md` （新規）

### Week 15: トレース・ログ機能

1. **`--trace` 実装**
   - 各フェーズの開始・終了ログ
   - 時間計測（`Unix.gettimeofday`）
   - メモリ使用量スナップショット（`Gc.stat`）

2. **統計情報収集**
   - パースしたトークン数
   - 型推論のunify呼び出し回数
   - 最適化パスの適用回数
   - 生成されたLLVM IR命令数

3. **`--verbose` レベル管理**
   - レベル0: エラーのみ
   - レベル1: 警告含む
   - レベル2: 情報含む
   - レベル3: デバッグ情報含む

**成果物**:
- `compiler/ocaml/src/cli/trace.ml` （新規）
- `compiler/ocaml/src/cli/stats.ml` （新規）
- `docs/guides/trace-output.md` （新規）

### Week 16: ヘルプ・ドキュメント整備とCI統合

1. **`--help` 出力の充実**
   - オプション一覧と説明
   - 使用例の追加
   - セクション分け（入力、出力、診断、デバッグ）

2. **CLI使用ガイド作成**
   - `docs/guides/cli-workflow.md` の作成
   - サンプルコード整備
   - トラブルシューティング

3. **CI統合**
   - CLIスナップショットテスト
   - パフォーマンステスト（10MB入力）
   - ドキュメント検証

**成果物**:
- `compiler/ocaml/src/main.ml` （拡張）
- `docs/guides/cli-workflow.md` （新規）
- `compiler/ocaml/tests/cli/snapshots/` （新規）

---

## Phase 1-5 から引き継ぐファイル

### コア実装

| ファイル | 説明 | Phase 1-6での利用 |
|---------|------|-------------------|
| `src/main.ml` | CLI エントリポイント | 拡張ポイント |
| `src/diagnostic.ml` | 診断メッセージ構造 | 診断出力強化で拡張 |
| `src/type_error.ml` | 型エラー診断 | 診断出力で参照 |
| `src/parser_driver.ml` | パーサドライバ | トレース統合で拡張 |
| `src/type_inference.ml` | 型推論エンジン | 統計情報収集で拡張 |

### テスト

| ファイル | 説明 | Phase 1-6での利用 |
|---------|------|-------------------|
| `tests/test_runtime_integration.sh` | 統合テスト | CLI テスト参考 |
| `tests/test_llvm_golden.ml` | ゴールデンテスト | CLI スナップショット参考 |

### ドキュメント

| ファイル | 説明 | Phase 1-6での利用 |
|---------|------|-------------------|
| `docs/phase1-5-completion-report.md` | Phase 1-5完了報告 | 制約事項参照 |
| `compiler/ocaml/README.md` | コンパイラREADME | CLI使用例追加 |

---

## Phase 1-6 で注意すべき制約

### 1. サンプルコード設計の制約

**動作するコード**:
```reml
fn add(a: i64, b: i64) -> i64 = a + b
fn main() -> i64 = add(2, 40)
```

**動作しないコード**（Phase 2 まで延期）:
```reml
// 文字列パラメータ（Core IR変換でクラッシュ）
fn greet(name: String) -> String = name

// タプルリテラル（Core IR ノード未実装）
fn make_pair() -> (i64, i64) = (1, 2)
```

**対策**: Phase 1-6 のサンプルコードは、プリミティブ型（i64, Bool）のみを使用する。

### 2. 診断メッセージの制約

**実装済み**:
- 型エラー（15種類）
- パースエラー
- 行・列番号の正確な報告

**未実装**:
- ソースコードスニペット
- Fix-it提案
- 関連エラーのグループ化

**対策**: Phase 1-6 で段階的に拡張する。

### 3. パフォーマンステストの制約

**現状**:
- 10MB入力の性能測定は未実施
- メモリプロファイリングは未実施

**対策**: Phase 1-6 で基準値を測定し、`0-3-audit-and-metrics.md` に記録する。

---

## リスク管理への登録

Phase 1-5 から引き継ぐリスク項目を [0-4-risk-handling.md](0-4-risk-handling.md) へ登録：

| リスク項目 | 影響 | 軽減策 |
|-----------|------|--------|
| 文字列パラメータ処理のクラッシュ | サンプルコード設計に制約 | プリミティブ型のみ使用 |
| タプル/レコード未実装 | CLI デモに制約 | Phase 2 で対応 |
| E2E テスト未完了 | 統合検証が不十分 | Phase 2 で完全実装 |
| メモリリーク検証未実施 | ランタイム品質が未確認 | Phase 2 で Valgrind/ASan 実施 |

---

## 連絡先とサポート

### ドキュメント

- **Phase 1-5完了報告**: [phase1-5-completion-report.md](../../compiler/ocaml/docs/phase1-5-completion-report.md)
- **Phase 1-5計画**: [1-5-runtime-integration.md](1-5-runtime-integration.md)
- **Phase 1-6計画**: [1-6-developer-experience.md](1-6-developer-experience.md)

### 仕様書

- **診断仕様**: [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- **監査・メトリクス**: [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)

### 既存実装

- **CLI エントリポイント**: `compiler/ocaml/src/main.ml`
- **診断システム**: `compiler/ocaml/src/diagnostic.ml`
- **型エラー**: `compiler/ocaml/src/type_error.ml`

---

**引き継ぎ完了**: 2025-10-10
**Phase 1-6 開始**: 準備完了
**次回レビュー**: Phase 1-6 Week 16（CLI 完成時）
