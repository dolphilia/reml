# 1.3 Core IR と最小最適化計画

## ✅ 完了ステータス

**完了日**: 2025-10-07
**状態**: **完了**
**期間**: Week 9-11（Phase 3）

### 実装統計
- **総コード行数**: 約5,642行（Core IR関連実装）
- **実装ファイル**: 7ファイル（ir.ml, desugar.ml, cfg.ml, const_fold.ml, dce.ml, pipeline.ml, ir_printer.ml）
- **テスト**: 42件（test_core_ir, test_desugar, test_cfg, test_const_fold, test_dce, test_pipeline）
- **テスト結果**: 全て成功（42/42）

### 成果物
| コンポーネント | ファイル | 行数 | テスト | 状態 |
|--------------|---------|------|--------|------|
| Core IR 型定義 | `src/core_ir/ir.ml` | 384行 | ✅ | 完了 |
| IR Printer | `src/core_ir/ir_printer.ml` | 348行 | ✅ | 完了 |
| 糖衣削除 | `src/core_ir/desugar.ml` | 638行 | ✅ | 完了 |
| CFG構築 | `src/core_ir/cfg.ml` | 430行 | ✅ | 完了 |
| 定数畳み込み | `src/core_ir/const_fold.ml` | 519行 | 26/26 | 完了 |
| 死コード削除 | `src/core_ir/dce.ml` | 377行 | 9/9 | 完了 |
| パイプライン統合 | `src/core_ir/pipeline.ml` | 216行 | 7/7 | 完了 |

### 次のステップ
Phase 3 Week 12-16: [1-4-llvm-targeting.md](1-4-llvm-targeting.md)（LLVM IR 生成）

---

## 目的
- Phase 1 マイルストーン M3 に向けて、Parser/TypeChecker の出力を Core IR へ正規化し、LLVM 生成に渡す手前の最小最適化を整備する。
- `docs/guides/compiler/llvm-integration-notes.md` の Core IR 設計方針を OCaml 実装で具現化し、Phase 2 以降の最適化拡張に備える。

## スコープ
- **含む**: Core IR データ構造の定義、構文糖の剥離、ベーシックブロック構成、定数畳み込み、死コード削除 (DCE)、簡易な代入伝播。
- **含まない**: 高度な最適化（ループ最適化、共通部分式除去、インライン展開）。これらは Phase 2 の検討対象。
- **前提**: TypedAST が安定しており、型付き情報を参照しながら IR を生成できること。

## 作業ディレクトリ
- `compiler/ocaml/src/ir`（想定）: IR 型定義、Desugar/Optimization パス
- `compiler/ocaml/tests/ir` : IR ゴールデン出力、最適化パスの回帰テスト
- `compiler/ocaml/docs` : IR 設計、最適化方針、既知の制約をまとめる
- `docs/notes/llvm-spec-status-survey.md` : Core IR と LLVM の整合性およびベンチ結果を追記

## 作業ブレークダウン

### ✅ 1. Core IR データ構造設計（9週目）
**担当領域**: IR型定義とデータモデル
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/ir.ml` (384行)

1.1. **基本IR型の定義**
- `Expr`: `Literal | Var | App | Let | If | Match | Primitive | Closure | DictLookup | CapabilityCheck | ...`
- `Stmt`: `Assign | Return | Jump | Branch | Phi | EffectMarker` の定義
- `Block`: ラベル + 命令列 + 終端命令
- `Function`: 引数・ブロックリスト・戻り型の構造体、辞書/Capability/効果情報を保持するフィールド

1.2. **型情報の保持**
- TypedASTからの型情報マッピング
- プリミティブ型（整数族/浮動小数/Bool/Char/String）、複合型（タプル/レコード/配列/スライス）の IR 型表現
- 辞書インスタンスや Capability 情報の型付けメタデータ

1.3. **メタデータ設計**
- Span情報の引き継ぎ（診断用）
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) 準拠のタグ付けと効果集合（`Σ`）の追跡
- Capability/Stage 参照を保持し、最適化時に破壊しないガードを設ける
- 最適化可否フラグ（DCE除外マーカー）

**成果物**: `core_ir/ir.ml`, IR型定義ドキュメント

### ✅ 2. 糖衣削除（Desugaring）パス（9-10週目）
**担当領域**: 高階構文の正規化
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/desugar.ml` (638行)

2.1. **パターンマッチの変換**
- `match` 式を条件分岐・フィールドアクセス・タグ検査へ分解
- ネストパターンの段階的展開
- 網羅性検査結果の反映（未到達分岐の除去）
- 代数的データ型の `{tag, payload}` 表現を生成

2.2. **パイプ演算子の展開**
- `a |> b |> c` → `let t1 = b(a) in c(t1)` への変換
- 一時変数の自動生成と命名規則
- Span情報の保存（エラー追跡用）
- クロージャ環境の `{env_ptr, code_ptr}` 表現と RC フックの付与

2.3. **let再束縛の正規化**
- 同名変数の再束縛をSSA形式へ変換準備
- φノード挿入位置の事前マーキング
- 変数生存区間の初期計算

**成果物**: `core_ir/desugar.ml`, 糖衣削除テスト

### ✅ 3. ベーシックブロック生成（10週目）
**担当領域**: 制御フローグラフ構築
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/cfg.ml` (430行)

3.1. **CFG構築アルゴリズム**
- 制御フロー分岐点の検出
- ブロック境界の決定（分岐・合流・ループ）
- ラベル自動生成とリンク

3.2. **SSA前提の準備**
- 支配木の構築（簡易版）
- φノード挿入位置の決定
- 変数定義・使用の追跡

3.3. **制御フロー検証**
- 到達不能ブロックの検出
- 無限ループの警告
- CFGの整形性チェック

**成果物**: `core_ir/cfg.ml`, CFG可視化ツール

### ✅ 4. 定数畳み込みパス（10-11週目）
**担当領域**: 最適化基盤
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/const_fold.ml` (519行)
**テスト**: `compiler/ocaml/tests/test_const_fold.ml` (26/26 成功)

4.1. **定数評価エンジン**
- 算術演算・論理演算の定数評価
- 文字列結合・比較の畳み込み
- 型安全な評価（オーバーフロー検出）

4.2. **定数伝播**
- 不変束縛の追跡
- 条件分岐の静的評価（`if true then A` → `A`）
- 畳み込み結果の型情報保持
  - Let 束縛のスコープを保持するため、定数環境への追加は一時的に行い、畳み込み後に復元する（Phase 3 Week 10 対応）

4.3. **畳み込み適用戦略**
- 再帰的な畳み込み（不動点まで反復）
- 適用回数上限の設定
- 診断用ノード保護（DCE前処理）

**成果物**: `core_ir/const_fold.ml`, 畳み込みテスト

### ✅ 5. 死コード削除（DCE）パス（11週目）
**担当領域**: 不要コード除去
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/dce.ml` (377行)
**テスト**: `compiler/ocaml/tests/test_dce.ml` (9/9 成功)

5.1. **生存解析**
- 使用されない変数の検出
- 副作用を持つ式の保護リスト作成
- 到達不能コードのマーキング

5.2. **DCE適用**
- 未使用束縛の削除
- 到達不能ブロックの除去
- メタデータタグによる除外処理

5.3. **安全性検証**
- 診断情報保持の確認
- 副作用保存の検証
- DCE前後のセマンティクス同値性チェック

**成果物**: `core_ir/dce.ml`, DCE検証テスト

### ✅ 6. 最適化パイプライン統合（11週目）
**担当領域**: パス管理とオーケストレーション
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/pipeline.ml` (216行)
**テスト**: `compiler/ocaml/tests/test_pipeline.ml` (7/7 成功)

6.1. **パイプライン設計**
- パス実行順序の定義（Desugar → CFG → ConstFold → DCE）
- 不動点反復の実装（畳み込み→DCE→畳み込み...）
- 停止条件の設定（変更なし or 上限回数）
- 効果メタデータ・Capability 情報・辞書インスタンスが各パスで保持されることを検証

6.2. **パス設定管理**
- 最適化レベルのフラグ（`-O0`, `-O1`）
- 個別パスの有効化/無効化
- デバッグモードでの中間結果出力

6.3. **統計収集**
- 各パスの実行時間計測
- 最適化効果の定量化（削除ノード数等）
- `0-3-audit-and-metrics.md` への記録

**成果物**: `core_ir/pipeline.ml`, パイプライン設定

### ✅ 7. IR検査ツールと出力（11-12週目）
**担当領域**: 診断と可視化
**完了日**: 2025-10-07
**実装**: `compiler/ocaml/src/core_ir/ir_printer.ml` (348行)
**備考**: Pretty Printer 実装済み、`--emit-core` CLI は Phase 3 後半で実装予定

7.1. **Pretty Printer実装**
- 人間可読なIR出力フォーマット
- インデント・色付け（オプション）
- Span情報のアノテーション表示

7.2. **`--emit-core` CLI実装**
- Core IRダンプの出力
- 中間段階の保存（各パス後）
- 差分表示機能（最適化前後）

7.3. **検証ツール**
- IR整形性チェッカー（未定義変数検出等）
- 型情報の一貫性検証
- CFG構造の検証（到達可能性、支配関係）

**成果物**: `core_ir/printer.ml`, `--emit-core` CLI

### ✅ 8. テストとドキュメント（12週目）
**担当領域**: 品質保証と文書化
**完了日**: 2025-10-07
**テスト統計**: 42件のテストが全て成功
**ドキュメント**: 本完了報告、`phase3-week10-11-completion.md`

8.1. **ゴールデンテスト**
- `examples/language-impl-comparison/` のIR変換テスト
- 最適化前後のスナップショット比較
- CI自動検証の統合

8.2. **単体テスト整備**
- 各パスの境界ケーステスト
- エラーケース（不正IR、無限ループ等）
- 性能回帰テスト（10MB相当コード）

8.3. **技術文書作成**
- Core IR仕様書の作成
- 最適化パスの設計文書
- M3マイルストーン達成報告
- Phase 2への引き継ぎ（高度な最適化TODO）

**成果物**: 完全なテストスイート、Core IR仕様書

## 成果物と検証
- OCaml モジュール `core_ir/` を追加し、`dune runtest core_ir` で各種パスの単体テストを実行。
- Core IR の SSA 検証を簡易チェック（未定義変数の検出等）で実施し、CI に組み込む。
- Core IR ダンプと LLVM IR の比較レポートを自動生成し、`0-3-audit-and-metrics.md` にサマリを記録。

## リスクとフォローアップ
- SSA 生成を Phase 1 で扱わない場合でも、phi ノード設計を前提にしておかないと Phase 2 でリファクタが発生する恐れがあるため、構造だけ先行定義する。
- DCE の適用範囲が広すぎると診断用ノードが除去される可能性があるので、[3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) に示されたメタデータを保持するタグを IR 上に確保する。
- Core IR から LLVM IR への写像を検証するため、IR 変換時のタグ情報をログに出力し、`0-3-audit-and-metrics.md` で追跡する。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
