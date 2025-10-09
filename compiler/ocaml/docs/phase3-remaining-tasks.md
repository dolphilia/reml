# Phase 3 残タスクと優先度分類

**作成日**: 2025-10-09
**対象フェーズ**: Phase 3 Week 9-17 (Core IR & LLVM 生成)
**基準計画書**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md)

このドキュメントは、Phase 1-3 (Week 9-17) で作成されたコードと文書に残るTODO・未実装箇所を抽出し、**1-4 LLVM Targeting (Week 13-16) 完了前に対応すべきタスク**と**Phase 2以降へ延期可能なタスク**に分類したものです。

## 目次

1. [1-4 LLVM Targeting完了前の必須タスク](#1-14-llvm-targeting完了前の必須タスク)
2. [Phase 2以降への延期タスク](#2-phase-2以降への延期タスク)
3. [タスク詳細](#3-タスク詳細)
4. [フォローアップ](#4-フォローアップ)

---

## 1. 1-4 LLVM Targeting完了前の必須タスク

### 1.1 Critical (M3達成条件)

M3マイルストーン「Core IR → LLVM IR の降格と最小ランタイム連携を実装」の達成に**必須**のタスク。

| ID | タスク | 発見箇所 | 対応期限 | 備考 |
|----|--------|----------|----------|------|
| C1 | 基本関数定義のLLVM IR生成完了 | README.md:372 | Week 16 | 現在はランタイム宣言のみ。関数本体生成が必要 |
| C2 | LLVM IR検証パイプライン完了 | 1-4 §6 | Week 15-16 | llvm-as/opt/llc の統合テスト必須 |
| C3 | `--emit-ir` CLI統合 | 1-4 §7 | Week 16 | M3デモに必要 |

**判定基準**: 上記3タスクが完了し、`examples/language-impl-comparison/` の基本サンプルがLLVM IR経由で実行可能になること。

### 1.2 High (品質向上)

M3達成には必須ではないが、**Phase 3完了前に対応することで品質・保守性が大幅向上**するタスク。

| ID | タスク | 発見箇所 | 推奨期限 | 理由 |
|----|--------|----------|----------|------|
| H1 | 型マッピングのTODO解消 | type_mapping.ml:75,135,186 | Week 16 | 型定義構造取得、ジェネリック展開、サイズ計算の実装 |
| H2 | ABI判定ロジックのテスト実装 | README.md:200 | Week 15 | Phase 2のWindows対応前に検証必須 |
| H3 | ゴールデンテストの拡充 | README.md:372 | Week 16 | 関数定義、分岐、再帰のカバレッジ追加 |
| H4 | CFG線形化の完成 | cfg.ml:300 | Week 16 | 複雑な制御フローの正確な表現 |

---

## 2. Phase 2以降への延期タスク

### 2.1 Medium (機能拡張 - Phase 2対応)

Phase 2で**型クラス・効果システム・Windows対応**を実装する際に必要となるタスク。

| ID | タスク | 発見箇所 | 対応Phase | 備考 |
|----|--------|----------|-----------|------|
| M1 | 配列リテラル型推論 | technical-debt.md §8 | Phase 2前半 | `infer_literal`拡張 |
| M2 | Unicode XID完全対応 | lexer.mll:48, technical-debt.md | Phase 2-3 | uutf/uucpライブラリ統合 |
| M3 | Switch文のLLVM IR生成 | codegen.ml:590 | Phase 2 | ADTパターンマッチ最適化 |
| M4 | レコードフィールドアクセス | codegen.ml:537 | Phase 2 | レコード型の完全サポート |
| M5 | 配列アクセスIR生成 | codegen.ml:541 | Phase 2 | 配列/スライス型の実装 |
| M6 | グローバル変数初期化 | codegen.ml:755-756 | Phase 2 | 定数初期化のみPhase 1で対応済み |
| M7 | 型クラス辞書の型表現 | type_env.ml:163 | Phase 2後半 | 辞書渡し実装時に拡張 |
| M8 | より具体的な型エラー | type_error.ml:208 | Phase 2 | 診断品質向上 |
| M9 | 類似変数名の提案 | type_error.ml:405 | Phase 2 | エラー回復強化 |

### 2.2 Low (将来計画 - Phase 3-4対応)

長期的な改善・拡張計画。Phase 3セルフホスト化またはPhase 4エコシステム統合で対応。

| ID | タスク | 発見箇所 | 対応Phase | 備考 |
|----|--------|----------|-----------|------|
| L1 | AST Printer改善 | technical-debt.md §3 | Phase 3 | Pretty Print, JSON出力 |
| L2 | 性能測定実施 | technical-debt.md §4 | Phase 3 | 10MBファイル、メモリプロファイリング |
| L3 | エラー回復強化 | technical-debt.md §5 | Phase 3 | 期待トークン集合、複数エラー報告 |
| L4 | 未対応エスケープ処理 | lexer.mll:33 | Phase 3-4 | 完全なエスケープシーケンス対応 |
| L5 | 環境キャプチャ実装 | desugar.ml:121 | Phase 3 | クロージャの完全実装 |
| L6 | 累乗演算子サポート | desugar.ml:148 | Phase 3 | 数値演算拡張 |
| L7 | 代入文・defer文 | desugar.ml:259,263 | Phase 3-4 | 命令型機能の追加 |
| L8 | コンストラクタタグマッピング | desugar.ml:656 | Phase 3 | ADT実装の完成 |
| L9 | 複雑パターン関数引数 | desugar.ml:692 | Phase 3 | パターンマッチ拡張 |
| L10 | モジュール名・型定義変換 | desugar.ml:738-740 | Phase 3 | セルフホスト準備 |

---

## 3. タスク詳細

### C1: 基本関数定義のLLVM IR生成完了

**優先度**: Critical
**発見箇所**: `compiler/ocaml/README.md:372`
**影響範囲**: M3マイルストーン達成条件

**現状**:
- Week 17でLLVM関数本体生成を実装済み
- `codegen_module`がCore IR基本ブロックをLLVM基本ブロックへ展開
- φノード遅延解決、SRet/ByVal属性付与を完了

**対応内容**:
- ✅ `codegen.ml`の関数本体生成パイプライン実装済み
- ✅ `begin_function`/`end_function`フロー確立
- ✅ `emit_return`でunit戻り値とsretハンドリング統合
- 🔧 ゴールデンテスト拡充が残課題(H3参照)

**検証方法**:
- `tests/llvm-ir/golden/*.ll.golden` の期待値比較
- `llvm-as` → `opt -verify` → `llc` パイプライン成功

**関連計画書**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §4-6

---

### C2: LLVM IR検証パイプライン完了

**優先度**: Critical
**発見箇所**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §6
**影響範囲**: CI/CD品質保証

**現状**:
- Week 15-16で`Verify`モジュール実装完了
- `scripts/verify_llvm_ir.sh`による3段階検証
- CI統合(`.github/workflows/ocaml-dune-test.yml`)済み

**対応内容**:
- ✅ llvm-as/opt/llcの統合スクリプト実装
- ✅ OCamlラッパー(`verify.ml/mli`)実装
- ✅ 診断変換(`error_to_diagnostic`)実装
- ✅ テストスイート(`test_llvm_verify.ml`)実装

**検証方法**:
- 正常ケース: LLVM APIで組み立てたモジュールの検証成功
- エラーケース: 壊れた`.ll`ファイルで`AssembleError`検出

**関連計画書**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §6

---

### C3: `--emit-ir` CLI統合

**優先度**: Critical
**発見箇所**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §7
**影響範囲**: M3デモンストレーション

**現状**:
- Week 16でCLIフラグ統合確認済み
- `--emit-ir`/`--emit-bc`/`--verify-ir`/`--out-dir`実装

**対応内容**:
- ✅ `src/main.ml`のCLIパイプライン統合
- ✅ IR出力フォーマット(.ll/.bc)対応
- 🔧 `--out-dir`との組み合わせゴールデンテスト追加(Week 16後半)

**検証方法**:
- `dune exec -- remlc --emit-ir input.reml` の動作確認
- 出力IRが`llvm-as`でアセンブル可能

**関連計画書**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §7

---

### H1: 型マッピングのTODO解消

**優先度**: High
**発見箇所**: `compiler/ocaml/src/llvm_gen/type_mapping.ml:75,135,186`
**影響範囲**: 型システムの完全性

**TODOリスト**:

1. **Line 75**: 型定義から構造を取得
   ```ocaml
   | TCustom name ->
       (* TODO: 型定義から構造を取得 *)
       i8_ptr (* プレースホルダー *)
   ```
   - 対応: ADT/レコード定義を`TypeEnv`から参照し、LLVMタグ付きユニオン/構造体へマッピング
   - Phase 2で型定義テーブル整備後に実装

2. **Line 135**: ジェネリック型の展開
   ```ocaml
   | TApp (TVar _, _) ->
       (* TODO: ジェネリック型の展開 *)
       i8_ptr
   ```
   - 対応: 型パラメータのインスタンス化情報を型推論から受け取り、具体型へ展開
   - Phase 2の型クラス実装と連携

3. **Line 186**: DataLayoutサイズ計算
   ```ocaml
   let get_type_size lltype =
     (* TODO: Llvm_target.DataLayout.size_of_type を使用 *)
     match Llvm.classify_type lltype with ...
   ```
   - 対応: `Llvm_target.DataLayout.size_of_type`を正式に使用
   - 現在の手動計算は推定値のため、ABI厳密性が求められるPhase 2で修正

**推奨対応時期**: Phase 2前半（Week 17-20）

**関連**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §2

---

### H2: ABI判定ロジックのテスト実装

**優先度**: High
**発見箇所**: `compiler/ocaml/README.md:200`, `compiler/ocaml/docs/phase3-week14-15-abi-completion.md`
**影響範囲**: マルチターゲット対応の品質保証

**現状**:
- Week 15でABIテストスイート実装済み(61/61成功)
- 境界値テスト(15/16/17バイト構造体)追加済み
- エッジケーステスト(空タプル、FAT pointer)追加済み

**対応内容**:
- ✅ System V ABI(16バイト閾値)の検証完了
- ✅ sret/byval属性のLLVM IR出力確認
- 🔧 Phase 2でWindows x64 ABI(8バイト閾値)追加時の回帰テスト準備

**推奨対応時期**: Phase 2前半（Windows対応開始時）

**関連**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §5, §8

---

### H3: ゴールデンテストの拡充

**優先度**: High
**発見箇所**: `compiler/ocaml/README.md:372`, `compiler/ocaml/tests/test_llvm_golden.ml`
**影響範囲**: 回帰検出能力

**現状**:
- Week 16で基本ゴールデンテスト実装(3件)
- 対象: basic_arithmetic, control_flow, function_calls
- 現在の期待値: ランタイム関数宣言のみ(関数本体未含)

**拡充計画**:
1. **Week 16後半**: 関数本体を含む期待値更新
2. **Phase 2**: 以下のケース追加
   - ネストした制御フロー(if-else-if)
   - 再帰関数(factorial, fibonacci)
   - クロージャとキャプチャ
   - タプル/レコードの構築と分解
   - パターンマッチの最適化

**推奨対応時期**: Phase 3 Week 16後半 + Phase 2継続

**関連**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §8

---

### H4: CFG線形化の完成

**優先度**: High
**発見箇所**: `compiler/ocaml/src/core_ir/cfg.ml:300`
**影響範囲**: Core IR品質

**現状**:
```ocaml
(* TODO: 関数本体の線形化
 * Phase 1 では関数定義を直接 block に変換していないため、
 * 関数のエントリポイントと終了処理を明示的に扱う必要がある *)
let linearize_function (fn : Ir.function_def) : Ir.block list =
  []
```

**対応内容**:
- 関数本体の式をブロックリストへ線形化
- エントリブロック・リターンブロックの明示的生成
- 複雑な制御フロー(ループ、多段if)の正確な表現

**推奨対応時期**: Phase 3 Week 16

**関連**: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) §3

---

### M1-M9, L1-L10: 詳細省略

Medium/Lowタスクの詳細は`technical-debt.md`および各計画書を参照。

---

## 4. フォローアップ

### 4.1 Phase 2への引き継ぎ事項

以下のタスクをPhase 2計画書([2-0-phase2-stabilization.md](../../../docs/plans/bootstrap-roadmap/2-0-phase2-stabilization.md))へ引き継ぐ:

1. **型システム拡張** (M1, M7, M8, M9)
   - 配列リテラル型推論
   - 型クラス辞書表現
   - 診断品質向上

2. **LLVM IR生成拡張** (M3, M4, M5, M6)
   - Switch文、レコード、配列、グローバル変数

3. **マルチターゲット対応** (H1, H2)
   - Windows x64 ABI検証
   - 型マッピング完全化

### 4.2 リスク管理への登録

以下を`docs/plans/bootstrap-roadmap/0-4-risk-handling.md`へ登録:

| リスク項目 | 影響 | 軽減策 |
|-----------|------|--------|
| H1未対応のままPhase 2突入 | 型マッピング不整合 | Phase 2 Week 17で優先対応 |
| H3不足による回帰未検出 | 品質低下 | CI強化、Phase 2でカバレッジ目標設定 |
| M1-M9の積み残し | Phase 2スケジュール遅延 | 優先順位付けと段階リリース |

### 4.3 M3マイルストーン達成判定

**判定基準**:
- ✅ C1: 基本関数のLLVM IR生成完了
- ✅ C2: LLVM IR検証パイプライン動作
- ✅ C3: `--emit-ir` CLI統合

**達成状況** (2025-10-09 Week 17):
- C1: ✅ 完了(codegen.ml実装済み)
- C2: ✅ 完了(Verify実装済み)
- C3: ✅ 完了(CLI統合済み)

**M3達成**: ✅ **達成済み（2025-10-09正式確認）**

**完了報告書**: [phase3-m3-completion-report.md](phase3-m3-completion-report.md)

### 4.4 Phase 3完了とPhase 2への移行

**Phase 3完了日**: 2025-10-09

**達成サマリー**:
- 総コード行数: 約13,000行（Core IR 5,642行 + LLVM生成 7,300行）
- 実装ファイル: 19ファイル
- テスト成功率: 100%（143/143テスト成功）
- LLVM 18完全対応、System V ABI準拠実装

**Phase 2への引き継ぎ**:
- High優先度タスク（H1-H4）の対応
- Medium優先度タスク（M1-M9）の段階的実装
- 技術的負債の解消（LLVM 18型付き属性など）

**詳細**: [phase3-to-phase2-handover.md](phase3-to-phase2-handover.md)

### 4.5 次ステップ（Phase 2: Week 17-34）

1. **Week 17-18**: Phase 2開始準備、型クラス戦略決定
2. **Week 17-20**: High優先度タスク着手（H1-H4）
3. **Week 17-24**: 型クラス実装（2-1）、Windows対応（2-6）
4. **Week 24-34**: 効果システム（2-2）、FFI（2-3）、診断強化（2-4）、仕様差分解消（2-5）

---

## 更新履歴

- **2025-10-09**: 初版作成（Phase 3 Week 17時点の残タスク整理）
  - コードベース・ドキュメント内のTODO/未実装箇所を抽出
  - 1-4完了前の必須タスク(Critical/High)と延期タスク(Medium/Low)に分類
  - M3マイルストーン達成判定基準を明確化
- **2025-10-09**: M3達成確認と更新（Phase 3完了）
  - M3マイルストーン正式達成確認（C1/C2/C3すべて完了）
  - Phase 3完了サマリー追加（達成統計、技術的成果）
  - Phase 2への引き継ぎ事項を明確化
  - 完了報告書へのリンク追加（phase3-m3-completion-report.md）
  - Phase 2次ステップ（Week 17-34）の計画追加
