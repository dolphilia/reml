# Phase 1-6 → Phase 1-7 引き継ぎドキュメント

**作成日**: 2025-10-10
**Phase 1-6 完了日**: 2025-10-10
**Phase 1-7 開始予定**: 2025-10-10 以降

## Phase 1-6 の成果物

### ✅ 完了した実装

**診断出力システム強化（Week 14）**:
- ソースコードスニペット表示（`diagnostic_formatter.ml`）
- カラーコード対応（`color.ml`）
- JSON 出力フォーマット（`json_formatter.ml`）
- CLI オプション管理のモジュール化（`options.ml`）
- 診断フォーマット仕様書（`docs/guides/diagnostic-format.md`）

**トレース・ログ機能（Week 15）**:
- `--trace` オプション実装（`cli/trace.ml`）
- 統計情報収集（`cli/stats.ml`）
- GC 統計ベースのメモリ計測
- トレースと統計の統合ビュー
- トレース出力ガイド（`docs/guides/trace-output.md`）

**ヘルプ・ドキュメント整備（Week 16 部分完了）**:
- `--help` 出力の充実（セクション化、使用例）
- man ページ生成スクリプト（`tooling/cli/scripts/update-man-pages.sh`）
- CLI 使用ガイド初稿（`docs/guides/cli-workflow.md`）
- サンプルコード整備（`examples/cli/` 配下）

**CI 依存関係修正**:
- `yojson` を `dune-project` / `reml_ocaml.opam` に追加
- GitHub Actions でのビルドエラー解消

**詳細**: [compiler/ocaml/docs/phase1-6-completion-report.md](../../compiler/ocaml/docs/phase1-6-completion-report.md)

### ⏸️ Phase 2 へ延期

以下のタスクは基礎実装が完了しましたが、完全な実装は Phase 2 へ延期：

| タスク | 理由 | Phase 2 対応内容 |
|--------|------|------------------|
| 統計機能の拡張 | 基本機能は実装済み | `--metrics` フラグ、JSON/CSV スキーマ、自動記録 |
| ログレベル管理 | オプション定義は完了 | 出力量制御、カテゴリー分離 |
| CLI 統合テスト | 診断テストのみ完了 | 全オプション網羅、スモークテスト |
| ベンチマークスイート | サンプルは整備済み | 性能測定、基準値設定、10MB 入力 |

---

## Phase 1-7 の目標

Phase 1-7 では x86_64 Linux 検証インフラの構築に注力します：

### 主要タスク（Week 17-19）

1. **CI 設計とワークフロー定義**（Week 17）
   - `.github/workflows/bootstrap-linux.yml` の作成
   - トリガー設定（push, pull_request, schedule）
   - ステージ設計（Lint, Build, Test, Artifact）

2. **開発環境セットアップ**（Week 17-18）
   - OCaml 環境構築（opam, dune, menhir）
   - LLVM 15 環境構築（apt, キャッシュ）
   - システム依存関係（gcc, make, Valgrind）

3. **ビルドジョブ実装**（Week 18）
   - OCaml プロジェクトビルド
   - ランタイムビルド（`make runtime`）
   - CLI 生成（`remlc-ocaml`）

4. **テストジョブ実装**（Week 18-19）
   - 単体テスト実行（`dune runtest`）
   - 統合テスト実行
   - ゴールデンテスト実行

5. **LLVM 検証ステップ**（Week 19）
   - LLVM 検証パイプライン（`llvm-as`, `opt -verify`, `llc`）
   - リンクテスト（実行可能バイナリ生成）
   - メモリ検証（Valgrind, ASan）

6. **アーティファクト管理**（Week 19）
   - コンパイラバイナリ、ランタイムライブラリ
   - 中間生成物（AST, TAST, Core IR, LLVM IR）
   - テストレポート、診断ログ

7. **監査ログとメトリクス**（Week 19）
   - CI 実行結果の記録
   - `0-3-audit-and-metrics.md` への自動追記
   - GitHub Actions ステータスバッジ

8. **ローカル再現環境**（Week 19）
   - `scripts/ci-local.sh` の作成
   - Docker イメージ作成
   - ドキュメント整備

---

## 前提条件の確認

### Phase 1-6 から引き継ぐ実装

#### ✅ CLI 基盤（既存実装）

**ファイル**: `compiler/ocaml/src/main.ml`, `compiler/ocaml/src/cli/*.ml`

**主要オプション**:
```ocaml
(* 入力 *)
<file.reml>                    (* 入力ファイル *)
-                              (* stdin から読み込み *)

(* 出力 *)
--emit-ast                     (* AST を stdout に出力 *)
--emit-tast                    (* Typed AST を stdout に出力 *)
--emit-ir                      (* LLVM IR を出力ディレクトリに出力 *)
--emit-bc                      (* LLVM Bitcode を出力ディレクトリに出力 *)
--out-dir <dir>                (* 出力ディレクトリ *)

(* 診断 *)
--format <format>              (* text|json (デフォルト: text) *)
--color <mode>                 (* auto|always|never (デフォルト: auto) *)

(* デバッグ *)
--trace                        (* フェーズトレースを有効化 *)
--stats                        (* コンパイル統計を表示 *)
--verbose <level>              (* 詳細レベル: 0-3 (デフォルト: 1) *)

(* コンパイル *)
--target <triple>              (* ターゲットトリプル (デフォルト: x86_64-linux) *)
--link-runtime                 (* ランタイムライブラリとリンク *)
--runtime-path <path>          (* ランタイムライブラリのパス *)
--verify-ir                    (* 生成された LLVM IR を検証 *)
```

#### ✅ パイプライン統合（既存実装）

**フロー**: Parser → Typer → Core IR → Optimization → LLVM

**現在の処理**:
1. `Parser_driver.parse` で AST 生成
2. `Type_inference.infer_module` で型推論
3. `Core_ir.Desugar.desugar_module` で糖衣削除
4. `Core_ir.Pipeline.optimize` で最適化
5. `Llvm_gen.Codegen.codegen_module` で LLVM IR 生成

各フェーズで `Cli.Trace` によるトレースと `Cli.Stats` による統計収集が実行されます。

#### ✅ 診断システム（完全実装）

**ファイル**:
- `compiler/ocaml/src/diagnostic.ml` - 診断メッセージ構造
- `compiler/ocaml/src/cli/diagnostic_formatter.ml` - テキスト出力
- `compiler/ocaml/src/cli/json_formatter.ml` - JSON 出力
- `compiler/ocaml/src/cli/color.ml` - カラーコード処理

**現在の出力形式**:
```
/path/to/file.reml:2:5: エラー[E7001] (型システム): 型が一致しません
    2 | fn add(a: i64, b: String) -> i64 = a + b
      |                  ^^^^^^
補足: 期待される型: i64
補足: 実際の型:     String
```

#### ✅ トレース・統計システム（完全実装）

**ファイル**:
- `compiler/ocaml/src/cli/trace.ml` - フェーズトレース
- `compiler/ocaml/src/cli/stats.ml` - 統計情報収集

**トレース出力例**:
```
[TRACE] Parsing started
[TRACE] Parsing completed (0.008s, 512 bytes allocated)
[TRACE] TypeChecking started
[TRACE] TypeChecking completed (0.015s, 1024 bytes allocated)
...
[TRACE] Total: 0.060s
```

**統計出力例**:
```
[STATS] Tokens parsed: 12
[STATS] AST nodes: 8
[STATS] Unify calls: 15
[STATS] Optimization passes: 3
[STATS] LLVM instructions: 42
```

---

## Phase 1-7 開始前のチェックリスト

### 環境確認

- [x] OCaml 環境が正しく設定されているか (`opam env`)
- [x] すべてのテストが成功するか (`dune test` - 143/143 成功確認済み)
- [x] ビルドが通るか (`dune build`)
- [x] LLVM 18 が正しくインストールされているか (`llvm-config --version`)
- [x] ランタイムライブラリがビルド済みか (`runtime/native/build/libreml_runtime.a`)

### 仕様書の理解

- [ ] [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md) を読む（Phase 1-7 メイン）
- [ ] [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) を確認（メトリクス定義）
- [ ] [0-4-risk-handling.md](0-4-risk-handling.md) を確認（リスク管理）

### 計画書の確認

- [ ] Phase 1-7 マイルストーンの達成条件を理解
- [ ] 作業ブレークダウンを確認
- [ ] 既存実装との統合ポイントを把握

---

## Phase 1-6 から引き継ぐ技術的知見

### 1. CLI の現状

**動作確認済み**:
- 基本的なコンパイルフロー（Parser → Typer → Core IR → LLVM）
- 診断出力（テキスト、JSON、カラーコード）
- トレース・統計機能
- `--help` 出力

**未動作**:
- 10MB 入力ファイルでの性能測定
- ベンチマークスイートによる回帰検出

**影響**: Phase 1-7 で CI を構築する際、基本的なスモークテストは実施できるが、性能回帰テストは Phase 2 で追加する必要がある。

### 2. 診断システムの現状

**実装済み**:
- エラーコード体系（E7001-E7015）
- ソースコードスニペット表示
- カラーコード対応
- JSON 出力フォーマット

**未実装**:
- Fix-it 提案の自動生成（Phase 2）

**影響**: Phase 1-7 で CI の診断ログを保存する際、既存の JSON 出力を活用できる。

### 3. テストインフラの現状

**既存テスト**:
- ユニットテスト（143件）
- ゴールデンテスト（AST, TAST, Core IR, LLVM IR）
- CLI 診断テスト
- CLI トレース/統計テスト

**未実装**:
- CLI 統合テスト（全オプション網羅）
- スモークテスト
- パフォーマンステスト

**影響**: Phase 1-7 で CI を構築する際、既存テストフレームワークを活用し、段階的にテストを拡充できる。

### 4. ドキュメントの現状

**実装済み**:
- `docs/guides/diagnostic-format.md` - 診断フォーマット仕様
- `docs/guides/trace-output.md` - トレース出力仕様
- `docs/guides/cli-workflow.md` - CLI 使用ガイド初稿
- `examples/cli/README.md` - サンプル説明

**未実装**:
- CI 設定の詳細解説
- トラブルシューティングガイド
- ローカル再現手順

**影響**: Phase 1-7 でドキュメントを拡充し、CI の使用方法とローカル再現手順を明記する必要がある。

---

## 推奨される Phase 1-7 の進め方

### Week 17: CI 設計とワークフロー定義

1. **`.github/workflows/bootstrap-linux.yml` の作成**
   - トリガー設定（push, pull_request, schedule）
   - Ubuntu ランナーの指定（`ubuntu-latest`）
   - LLVM 15 以上の依存関係

2. **ステージ設計**
   - Lint: コードフォーマット、静的解析
   - Build: OCaml コンパイラ、ランタイムビルド
   - Test: 単体テスト、統合テスト、ゴールデンテスト
   - Artifact: 成果物の収集と保存

3. **依存関係グラフ**
   - ステージ間の依存関係定義（`needs`）
   - 並行実行可能なジョブの特定

**成果物**:
- `.github/workflows/bootstrap-linux.yml` 初版
- CI 設計ドキュメント

### Week 18: 開発環境セットアップとビルドジョブ

1. **OCaml 環境構築**
   - OCaml 4.14 以上のインストール
   - `opam` のセットアップとキャッシュ
   - `dune`, `menhir` 等の依存パッケージインストール

2. **LLVM 環境構築**
   - LLVM 15 のインストール（`apt` 経由）
   - `llvm-config` のパス設定
   - `actions/cache` による LLVM バイナリキャッシュ

3. **ビルドジョブ実装**
   - `dune build` の実行
   - `make runtime` の実行
   - ビルドログの保存

**成果物**:
- 依存関係インストールスクリプト
- ビルドジョブ設定

### Week 19: テストジョブと検証ステップ

1. **テストジョブ実装**
   - `dune runtest` の実行
   - テスト結果の JUnit XML 出力
   - ゴールデンテストの実行

2. **LLVM 検証ステップ**
   - `llvm-as` によるアセンブル検証
   - `opt -verify` による整合性チェック
   - `llc` によるコード生成テスト
   - Valgrind によるメモリリーク検出

3. **アーティファクト管理**
   - コンパイラバイナリ、ランタイムライブラリの保存
   - 中間生成物の保存
   - テストレポートの保存

4. **監査ログとメトリクス**
   - CI 実行結果の記録
   - `0-3-audit-and-metrics.md` への自動追記
   - GitHub Actions ステータスバッジ

5. **ローカル再現環境**
   - `scripts/ci-local.sh` の作成
   - Docker イメージ作成
   - ドキュメント整備

**成果物**:
- テストジョブ設定
- LLVM 検証ジョブ
- アーティファクト管理設定
- ローカル再現スクリプト

---

## Phase 1-6 から引き継ぐファイル

### コア実装

| ファイル | 説明 | Phase 1-7 での利用 |
|---------|------|-------------------|
| `src/main.ml` | CLI エントリポイント | CI でのコマンド実行 |
| `src/cli/options.ml` | オプション管理 | CI でのオプション指定 |
| `src/cli/diagnostic_formatter.ml` | 診断フォーマッター | CI での診断ログ保存 |
| `src/cli/json_formatter.ml` | JSON 出力 | CI での機械判読可能な出力 |
| `src/cli/trace.ml` | トレース機能 | CI での性能測定 |
| `src/cli/stats.ml` | 統計情報収集 | CI でのメトリクス記録 |

### テスト

| ファイル | 説明 | Phase 1-7 での利用 |
|---------|------|-------------------|
| `tests/test_cli_diagnostics.ml` | 診断テスト | CI でのテスト実行 |
| `tests/test_cli_trace.ml` | トレース/統計テスト | CI でのテスト実行 |
| `tests/test_llvm_golden.ml` | ゴールデンテスト | CI でのテスト実行 |

### ドキュメント

| ファイル | 説明 | Phase 1-7 での利用 |
|---------|------|-------------------|
| `docs/guides/cli-workflow.md` | CLI 使用ガイド | CI 設定の参考 |
| `docs/guides/diagnostic-format.md` | 診断フォーマット仕様 | CI ログ解析の参考 |
| `docs/guides/trace-output.md` | トレース出力仕様 | CI メトリクスの参考 |
| `compiler/ocaml/docs/phase1-6-completion-report.md` | Phase 1-6 完了報告 | 制約事項参照 |

### サンプルコード

| ファイル | 説明 | Phase 1-7 での利用 |
|---------|------|-------------------|
| `examples/cli/add.reml` | 基本的な算術演算 | CI スモークテスト |
| `examples/cli/type_error.reml` | 型エラーのデモ | CI 診断テスト |
| `examples/cli/trace_sample.reml` | トレース機能のサンプル | CI トレーステスト |
| `examples/cli/emit_suite.reml` | 各 `--emit-*` オプションの動作確認 | CI 出力検証 |

---

## Phase 1-7 で注意すべき制約

### 1. サンプルコード設計の制約

**動作するコード**:
```reml
fn add(a: i64, b: i64) -> i64 = a + b
fn main() -> i64 = add(2, 40)
```

**動作しないコード**（Phase 2 まで延期）:
```reml
// 文字列パラメータ（Core IR 変換でクラッシュ）
fn greet(name: String) -> String = name

// タプルリテラル（Core IR ノード未実装）
fn make_pair() -> (i64, i64) = (1, 2)
```

**対策**: Phase 1-7 の CI スモークテストは、プリミティブ型（i64, Bool）のみを使用するサンプルを使用する。

### 2. 性能測定の制約

**現状**:
- 小規模入力（`examples/cli/*.reml`）での測定は可能
- 10MB 入力での性能測定は未実施

**対策**: Phase 1-7 では基本的なスモークテストのみを実施し、性能回帰テストは Phase 2 で追加する。

### 3. テストカバレッジの制約

**現状**:
- ユニットテスト（143件）とゴールデンテスト（AST, TAST, Core IR, LLVM IR）は網羅的
- CLI 統合テスト（全オプション網羅）は未完了

**対策**: Phase 1-7 では既存テストを CI で実行し、CLI 統合テストは Phase 2 で追加する。

---

## リスク管理への登録

Phase 1-6 から引き継ぐリスク項目を [0-4-risk-handling.md](0-4-risk-handling.md) へ登録：

| リスク項目 | 影響 | 軽減策 |
|-----------|------|--------|
| 文字列パラメータ処理のクラッシュ | CI スモークテストの制約 | プリミティブ型のみ使用 |
| タプル/レコード未実装 | CI テストケースの制約 | Phase 2 で対応 |
| 性能測定基準値未設定 | 性能回帰の検出が困難 | Phase 2 で基準値設定 |
| CLI 統合テスト未完了 | CI のテストカバレッジが不十分 | Phase 2 で完全実装 |

---

## 連絡先とサポート

### ドキュメント

- **Phase 1-6 完了報告**: [compiler/ocaml/docs/phase1-6-completion-report.md](../../compiler/ocaml/docs/phase1-6-completion-report.md)
- **Phase 1-6 計画**: [1-6-developer-experience.md](1-6-developer-experience.md)
- **Phase 1-7 計画**: [1-7-linux-validation-infra.md](1-7-linux-validation-infra.md)

### 仕様書

- **メトリクス定義**: [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- **リスク管理**: [0-4-risk-handling.md](0-4-risk-handling.md)
- **診断仕様**: [../../spec/3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)

### 既存実装

- **CLI エントリポイント**: `compiler/ocaml/src/main.ml`
- **CLI モジュール**: `compiler/ocaml/src/cli/*.ml`
- **テストスイート**: `compiler/ocaml/tests/test_*.ml`

---

**引き継ぎ完了**: 2025-10-10
**Phase 1-7 開始**: 準備完了
**次回レビュー**: Phase 1-7 Week 19（CI 完成時）
