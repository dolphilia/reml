# Phase 3 → Phase 2 引き継ぎドキュメント

**作成日**: 2025-10-09
**Phase 3 完了日**: 2025-10-09
**Phase 2 開始予定**: 2025-10-09 以降

## Phase 3 の成果物

### 完了した実装

✅ **M3: CodeGen MVP** - 完全実装（2025-10-09達成）

**Week 9-11: Core IR 最適化パス**
- Core IR データ構造（ir.ml: 384行）
- 糖衣削除（desugar.ml: 638行）
- CFG構築（cfg.ml: 430行）
- 定数畳み込み（const_fold.ml: 519行）
- 死コード削除（dce.ml: 377行）
- 最適化パイプライン（pipeline.ml: 216行）
- テスト: 42/42 成功

**Week 12-17: LLVM IR 生成**
- LLVM型マッピング（type_mapping.ml: 280行、テスト: 35/35）
- ターゲット設定（target_config.ml: 180行）
- IRビルダー（codegen.ml: 662行）
- ABI実装（abi.ml: 200行、テスト: 61/61）
- 検証パイプライン（verify.ml: 240行、テスト: 2/2）
- ゴールデンテスト（3件）
- 関数本体生成完了（Week 17）

詳細は [phase3-m3-completion-report.md](phase3-m3-completion-report.md) を参照。

---

## Phase 2 の目標

Phase 2 では以下のマイルストーンを達成します：

### 主要タスク（Week 17-34）

1. **型クラス実装**（2-1: Week 17-24）
   - 辞書渡し vs モノモルフィゼーション評価
   - 基本型クラス（Eq, Ord, Show等）
   - 型クラス制約と推論

2. **効果システム**（2-2: Week 24-30）
   - 代数的効果の型推論
   - 効果ハンドラー実装
   - 効果安全性検証

3. **FFI実装**（2-3: Week 30-32）
   - C FFI基盤
   - 安全性保証
   - 型マッピング拡張

4. **診断強化**（2-4: Week 32-33）
   - エラーメッセージ改善
   - 診断品質向上
   - フィックスイット提案

5. **仕様差分解消**（2-5: Week 33-34）
   - 仕様書との整合性確認
   - 未実装機能の補完

6. **Windows対応**（2-6: Week 17-24 並行）
   - Windows x64 ABI（8バイト閾値）
   - MSVC ツールチェーン統合
   - クロスプラットフォームCI

---

## 前提条件の確認

### 開発環境

- [x] OCaml >= 4.14 (推奨: 5.2.1)
- [x] Dune >= 3.0
- [x] Menhir >= 20201216
- [x] LLVM 18 (Phase 3 で統合済み)
- [x] opam パッケージマネージャ

### Phase 3 完了時点の成果物

**Core IR関連**:
- [x] Core IR データ構造 (`src/core_ir/ir.ml`)
- [x] 糖衣削除パス (`src/core_ir/desugar.ml`)
- [x] CFG構築 (`src/core_ir/cfg.ml`)
- [x] 定数畳み込み (`src/core_ir/const_fold.ml`)
- [x] 死コード削除 (`src/core_ir/dce.ml`)
- [x] 最適化パイプライン (`src/core_ir/pipeline.ml`)
- [x] IR Printer (`src/core_ir/ir_printer.ml`)

**LLVM生成関連**:
- [x] LLVM型マッピング (`src/llvm_gen/type_mapping.ml`)
- [x] ターゲット設定 (`src/llvm_gen/target_config.ml`)
- [x] コードジェネレーション (`src/llvm_gen/codegen.ml`)
- [x] ABI実装 (`src/llvm_gen/abi.ml`)
- [x] 検証パイプライン (`src/llvm_gen/verify.ml`)

**テストインフラ**:
- [x] Core IR最適化テスト（42件）
- [x] LLVM型マッピングテスト（35件）
- [x] ABIテスト（61件）
- [x] LLVM IR検証テスト（2件）
- [x] ゴールデンテスト（3件）
- [x] CI/CD パイプライン (`.github/workflows/ocaml-dune-test.yml`)

### 仕様書の準備状況

- [x] [1-1-syntax.md](../../../docs/spec/1-1-syntax.md) - 構文仕様（完了）
- [x] [1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md) - 型システム仕様（完了）
- [x] [2-5-error.md](../../../docs/spec/2-5-error.md) - エラー仕様（Phase 2 で拡張済み）
- [x] [3-6-core-diagnostics-audit.md](../../../docs/spec/3-6-core-diagnostics-audit.md) - 診断・監査（Phase 3 で参照済み）
- [x] LLVM 連携ガイド - [guides/llvm-integration-notes.md](../../../docs/guides/llvm-integration-notes.md)（Phase 3 で実装済み）

---

## Phase 3 から引き継ぐタスク

### 1. High優先度タスク（Phase 2前半で対応: Week 17-20）

| ID | タスク | 発見箇所 | 推奨期限 | 理由 |
|----|--------|----------|----------|------|
| H1 | 型マッピングのTODO解消 | type_mapping.ml:75,135,186 | Week 20 | 型定義構造取得、ジェネリック展開、サイズ計算の実装 |
| H2 | ABI判定ロジックのテスト実装 | README.md:200 | Week 18 | Windows対応前に検証必須 |
| H3 | ゴールデンテストの拡充 | README.md:372 | Week 20 | 関数定義、分岐、再帰のカバレッジ追加 |
| H4 | CFG線形化の完成 | cfg.ml:300 | Week 20 | 複雑な制御フローの正確な表現 |

#### H1: 型マッピングのTODO解消（詳細）

**発見箇所**: `compiler/ocaml/src/llvm_gen/type_mapping.ml:75,135,186`

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

#### H2: ABI判定ロジックのテスト実装

**現状**:
- Week 15でABIテストスイート実装済み(61/61成功)
- 境界値テスト(15/16/17バイト構造体)追加済み
- エッジケーステスト(空タプル、FAT pointer)追加済み

**Phase 2対応**:
- Windows x64 ABI(8バイト閾値)追加時の回帰テスト準備
- LLVM 18型付き属性への移行テスト

#### H3: ゴールデンテストの拡充

**現状**: 3件（basic_arithmetic, control_flow, function_calls）

**拡充計画**:
- ネストした制御フロー(if-else-if)
- 再帰関数(factorial, fibonacci)
- クロージャとキャプチャ
- タプル/レコードの構築と分解
- パターンマッチの最適化

#### H4: CFG線形化の完成

**現状**:
```ocaml
(* TODO: 関数本体の線形化 *)
let linearize_function (fn : Ir.function_def) : Ir.block list =
  []
```

**対応内容**:
- 関数本体の式をブロックリストへ線形化
- エントリブロック・リターンブロックの明示的生成
- 複雑な制御フロー(ループ、多段if)の正確な表現

---

### 2. Medium優先度タスク（Phase 2中盤で対応: Week 20-30）

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

---

### 3. 技術的負債

#### LLVM 18型付き属性のバインディング制限

**分類**: LLVM統合 / ABI実装
**優先度**: 🟡 Medium
**ステータス**: 回避策実装済み（Phase 3 Week 14-15）

**問題**: llvm-ocamlバインディングで `create_type_attr` が未サポート

**現在の回避策**: 文字列属性として実装
```ocaml
let add_sret_attr llctx llvm_fn _ret_ty param_index =
  let attr_kind = Llvm.AttrIndex.Param param_index in
  let sret_attr = Llvm.create_string_attr llctx "sret" "" in
  Llvm.add_function_attr llvm_fn sret_attr attr_kind
```

**Phase 2対応計画**:
1. llvm-ocamlバインディング拡張（`create_type_attr` のC stubs実装）
2. 手動FFI実装（代替案）
3. 検証強化（ABI属性の正確性確認）

**参考**: [technical-debt.md §10](technical-debt.md)

---

## 既存コードベースの構造

### ディレクトリ構成（Phase 3完了時点）

```
compiler/ocaml/
├── src/                          # コンパイラ本体
│   ├── ast.ml                   # AST 定義
│   ├── token.ml                 # トークン定義
│   ├── lexer.mll                # 字句解析器
│   ├── parser.mly               # 構文解析器
│   ├── parser_driver.ml         # パーサドライバ
│   ├── diagnostic.ml            # 診断メッセージ
│   ├── ast_printer.ml           # AST プリンター
│   ├── types.ml                 # 型表現とスキーム
│   ├── type_env.ml              # 型環境
│   ├── constraint.ml            # 型制約と単一化
│   ├── typed_ast.ml             # 型付きAST
│   ├── type_inference.ml        # 型推論エンジン
│   ├── type_error.ml            # 型エラーと診断
│   ├── core_ir/                 # Core IR実装（Phase 3 Week 9-11）
│   │   ├── ir.ml               # Core IR データ構造
│   │   ├── ir_printer.ml       # IR Pretty Printer
│   │   ├── desugar.ml          # 糖衣削除
│   │   ├── cfg.ml              # CFG構築
│   │   ├── const_fold.ml       # 定数畳み込み
│   │   ├── dce.ml              # 死コード削除
│   │   └── pipeline.ml         # 最適化パイプライン
│   ├── llvm_gen/                # LLVM生成実装（Phase 3 Week 12-17）
│   │   ├── type_mapping.ml     # LLVM型マッピング
│   │   ├── target_config.ml    # ターゲット設定
│   │   ├── codegen.ml          # コードジェネレーション
│   │   ├── abi.ml              # ABI実装
│   │   └── verify.ml           # 検証パイプライン
│   └── main.ml                  # CLI エントリポイント
├── tests/                        # テストコード
│   ├── test_lexer.ml            # Lexer ユニットテスト
│   ├── test_parser.ml           # Parser ユニットテスト
│   ├── test_pattern_matching.ml # パターンマッチ専用テスト
│   ├── test_golden.ml           # ゴールデンテスト
│   ├── test_types.ml            # 型システムユニットテスト
│   ├── test_type_inference.ml   # 型推論テスト
│   ├── test_type_errors.ml      # 型エラーテスト
│   ├── test_let_polymorphism.ml # let多相テスト
│   ├── test_cfg.ml              # CFGテスト
│   ├── test_const_fold.ml       # 定数畳み込みテスト
│   ├── test_dce.ml              # DCEテスト
│   ├── test_pipeline.ml         # パイプラインテスト
│   ├── test_llvm_type_mapping.ml # LLVM型マッピングテスト
│   ├── test_abi.ml              # ABIテスト
│   ├── test_llvm_verify.ml      # LLVM検証テスト
│   └── test_llvm_golden.ml      # LLVMゴールデンテスト
└── docs/                         # 実装ドキュメント
    ├── parser_design.md
    ├── environment-setup.md
    ├── phase1-completion-report.md
    ├── phase2-completion-report.md
    ├── phase3-m3-completion-report.md    # Phase 3完了報告
    ├── phase3-handover.md
    ├── phase3-remaining-tasks.md
    ├── phase3-to-phase2-handover.md      # このファイル
    ├── technical-debt.md
    ├── llvm-type-mapping.md
    └── llvm-ir-verification.md
```

---

## Phase 2 開始前のチェックリスト

### 環境確認

- [ ] OCaml 環境が正しく設定されているか (`opam env`)
- [ ] すべてのテストが成功するか (`dune test` - 143/143成功確認済み)
- [ ] ビルドが通るか (`dune build`)
- [ ] LLVM 18 が正しくインストールされているか (`llvm-config --version`)

### 仕様書の理解

- [ ] [2-1-type-class-design.md](../../../docs/plans/bootstrap-roadmap/2-1-type-class-design.md) を読む（Phase 2メイン）
- [ ] [2-2-effect-system.md](../../../docs/plans/bootstrap-roadmap/2-2-effect-system.md) を読む
- [ ] [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) を再確認（Phase 3完了分）

### 計画書の確認

- [ ] [2-0-phase2-stabilization.md](../../../docs/plans/bootstrap-roadmap/2-0-phase2-stabilization.md) を読む
- [ ] Phase 2 マイルストーンの達成条件を理解
- [ ] 作業ブレークダウンを確認

### ツールとリソース

- [ ] 型クラス実装の参考資料を準備（Haskell、Rust trait、OCaml module）
- [ ] Windows開発環境の準備（WSL2/VM/クロスコンパイル）
- [ ] ベンチマーク用のサンプルコードを準備

---

## 推奨される Phase 2 の進め方

### Week 17-20: 基盤整備と型クラス設計

1. **High優先度タスク着手**
   - H1: 型マッピングのTODO解消
   - H2: Windows x64 ABI検証準備
   - H3: ゴールデンテスト拡充
   - H4: CFG線形化の完成

2. **型クラス戦略決定**
   - 辞書渡し vs モノモルフィゼーション評価
   - 型クラス制約の型表現設計
   - 実装方針の確定

### Week 20-24: 型クラス実装

1. 基本型クラス実装（Eq, Ord, Show等）
2. 型クラス制約と推論
3. インスタンス定義と検証
4. テストスイート整備

### Week 24-30: 効果システム実装

1. 代数的効果の型推論
2. 効果ハンドラー実装
3. 効果安全性検証
4. エラーハンドリング統合

### Week 30-34: 統合と安定化

1. FFI実装（Week 30-32）
2. 診断強化（Week 32-33）
3. 仕様差分解消（Week 33-34）
4. Phase 2完了報告書作成

### 並行タスク

**Week 17-24: Windows対応**
- Windows x64 ABI（8バイト閾値）実装
- MSVCツールチェーン統合
- クロスプラットフォームCI構築

**Week 20-30: Medium優先度タスク**
- M1-M9の段階的実装
- 配列リテラル、Unicode XID、Switch文等

---

## リスク管理への登録

Phase 3から引き継ぐリスク項目を [0-4-risk-handling.md](../../../docs/plans/bootstrap-roadmap/0-4-risk-handling.md) へ登録：

| リスク項目 | 影響 | 軽減策 |
|-----------|------|--------|
| H1未対応のままPhase 2突入 | 型マッピング不整合 | Phase 2 Week 17-20で優先対応 |
| H3不足による回帰未検出 | 品質低下 | CI強化、Phase 2でカバレッジ目標設定 |
| M1-M9の積み残し | Phase 2スケジュール遅延 | 優先順位付けと段階リリース |
| LLVM 18型付き属性制限 | Windows/ARM64対応遅延 | Phase 2でバインディング拡張 |
| 型クラス実装方式の選択 | 性能・保守性への影響 | Week 20までに評価完了、方針決定 |
| Windows ABI検証不足 | クロスプラットフォーム不整合 | Week 18でテスト環境整備 |

---

## 連絡先とサポート

### ドキュメント

- Phase 3完了報告: [phase3-m3-completion-report.md](phase3-m3-completion-report.md)
- 残タスクリスト: [phase3-remaining-tasks.md](phase3-remaining-tasks.md)
- 技術的負債リスト: [technical-debt.md](technical-debt.md)
- LLVM統合ガイド: [llvm-type-mapping.md](llvm-type-mapping.md), [llvm-ir-verification.md](llvm-ir-verification.md)

### 仕様書

- Phase 2計画: [2-0-phase2-stabilization.md](../../../docs/plans/bootstrap-roadmap/2-0-phase2-stabilization.md)
- 型クラス計画: [2-1-type-class-design.md](../../../docs/plans/bootstrap-roadmap/2-1-type-class-design.md)
- 効果システム: [2-2-effect-system.md](../../../docs/plans/bootstrap-roadmap/2-2-effect-system.md)
- 全体計画: [1-0-phase1-bootstrap.md](../../../docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md)

### 計画書

- Phase 1全体: [1-0-phase1-bootstrap.md](../../../docs/plans/bootstrap-roadmap/1-0-phase1-bootstrap.md)
- Core IR計画: [1-3-core-ir-min-optimization.md](../../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
- LLVM統合: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md)

---

**引き継ぎ完了**: 2025-10-09
**Phase 2 開始**: 準備完了
**次回レビュー**: Phase 2 Week 20（型クラス戦略決定時）
