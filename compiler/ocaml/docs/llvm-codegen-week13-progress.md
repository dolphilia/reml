# LLVM IR Code Generation - Week 13 Progress Report

**日付**: 2025-10-09
**フェーズ**: Phase 3 Week 13-14
**タスク**: LLVM IRビルダー実装（モジュール・関数・基本ブロック生成）

## 実装概要

docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §4 に基づき、Core IRからLLVM IRへの変換を実装しました。

### 実装済みコンポーネント

#### 1. コードジェネレーションコンテキスト (`codegen.ml`: 30-75行)

```ocaml
type codegen_context = {
  llctx: Llvm.llcontext;
  llmodule: Llvm.llmodule;
  builder: Llvm.llbuilder;
  type_ctx: Type_mapping.type_mapping_context;
  target: Target_config.target_config;

  mutable fn_map: (string, Llvm.llvalue) Hashtbl.t;
  mutable var_map: (var_id, Llvm.llvalue) Hashtbl.t;
  mutable block_map: (label, Llvm.llbasicblock) Hashtbl.t;
}
```

- LLVM コンテキスト・モジュール・ビルダーの統合管理
- ターゲット設定の適用（x86_64 Linux System V ABI）
- 関数・変数・ブロックマッピングのHashtable管理

#### 2. ランタイム関数宣言 (`codegen.ml`: 90-114行)

- `mem_alloc: (i64) -> ptr` - メモリ割り当て
- `inc_ref: (ptr) -> void` - 参照カウント増加
- `dec_ref: (ptr) -> void` - 参照カウント減少
- `panic: (ptr, i64) -> void` - パニックハンドラ（noreturn属性付き）

外部リンケージで宣言され、runtime/nativeで実装される想定。

#### 3. 式のコード生成 (`codegen.ml`: 126-413行)

**対応済み式種別**:

- **Literal** (146-186行): リテラル値の生成
  - 整数（Int64→i8/i16/i32/i64）
  - 浮動小数（Float→f32/f64）
  - Bool→i1
  - Char→i32
  - String→FAT pointer `{ptr, i64}`（グローバル文字列定数経由）
  - Unit→undef（void型）

- **Var** (190-193行): 変数参照
  - 変数マップからLLVM値を取得

- **App** (197-206行): 関数適用
  - `Llvm.build_call`による関数呼び出し生成

- **Let** (210-218行): let束縛
  - 束縛値をコード生成し、変数マップに登録
  - 本体式を評価

- **If** (222-256行): if式
  - then/else/mergeブロックの3ブロック構造
  - φノードによる値のマージ

- **Primitive** (260-398行): プリミティブ演算
  - **算術演算**: Add, Sub, Mul, Div, Mod（整数/浮動小数を型で判定）
  - **比較演算**: Eq, Ne, Lt, Le, Gt, Ge（icmp/fcmp）
  - **論理演算**: And, Or, Not
  - **ビット演算**: BitAnd, BitOr, BitXor, BitNot, Shl, Shr

- **TupleAccess** (402-404行): タプル要素アクセス
  - `Llvm.build_extractvalue`

**Phase 1では未対応**（Phase 2以降で実装予定）:
- Match式、Closure、DictLookup、CapabilityCheck、ADT操作
- RecordAccess、ArrayAccess

#### 4. 終端命令のコード生成 (`codegen.ml`: 416-445行)

- **TermReturn**: 返り値と共に関数から復帰
- **TermJump**: 無条件ジャンプ
- **TermBranch**: 条件分岐（then/elseラベル）
- **TermSwitch**: switch文（Phase 1では未実装）
- **TermUnreachable**: 到達不能マーカー

#### 5. 文のコード生成 (`codegen.ml`: 449-502行)

- **Assign**: 変数への代入（変数マップ更新）
- **Return**: 関数復帰
- **Jump**: 無条件ジャンプ
- **Branch**: 条件分岐
- **Phi**: φノード生成（SSA対応）
- **EffectMarker**: 効果マーカー（Phase 1では無視）
- **ExprStmt**: 式文

#### 6. 関数・グローバル変数・モジュール生成 (`codegen.ml`: 515-642行)

- **codegen_function_decl**: 関数宣言生成
  - パラメータ型と返り値型の変換
  - System V calling convention設定
  - パラメータ名の設定

- **codegen_global_def**: グローバル変数生成
  - 型変換と変数宣言
  - 可変性設定
  - 初期化式（Phase 1では未実装）

- **codegen_blocks**: 基本ブロック生成
  - 2フェーズアプローチ（全ブロック作成→命令生成）
  - 前方参照対応

- **codegen_module**: モジュール全体生成
  - ランタイム関数宣言
  - グローバル変数生成
  - 関数宣言→関数本体の順で生成

#### 7. LLVM IR出力 (`codegen.ml`: 651-661行)

- **emit_llvm_ir**: テキスト形式（`.ll`）出力
- **emit_llvm_bc**: ビットコード形式（`.bc`）出力

### インターフェース (`codegen.mli`)

公開API:
- `create_codegen_context`
- `get_llmodule`, `get_builder`
- `declare_runtime_functions`
- `codegen_function_decl`, `codegen_global_def`, `codegen_blocks`
- `codegen_module`
- `emit_llvm_ir`, `emit_llvm_bc`

### ビルド設定

**dune設定更新** (`src/llvm_gen/dune`):
- モジュール追加: `codegen`
- ライブラリ依存追加: `llvm.bitwriter`

```lisp
(library
 (name llvm_gen)
 (public_name reml_ocaml.llvm_gen)
 (wrapped false)
 (modules type_mapping target_config codegen)
 (libraries llvm llvm.bitwriter reml_ocaml.core_ir reml_parser))
```

## 技術的特徴

### LLVM 18 Opaque Pointer対応

- `Llvm.pointer_type ctx.llctx`（型引数なし）を使用
- 型情報は`load`/`store`命令に付与
- GEP（GetElementPtr）でopaque pointerを操作

### System V ABI (x86_64 Linux)

- DataLayout: `e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64`
- Calling convention: `ccc`（C calling convention）
- LLVMの自動ABI処理に委譲（手動レジスタ割当なし）

### FAT Pointer実装

String型のFAT pointer `{ptr, i64}`:
```ocaml
let str_const = Llvm.const_stringz ctx.llctx s in
let str_global = Llvm.define_global "str_const" str_const ctx.llmodule in
let str_ptr = Llvm.const_gep ptr_ty str_global [| zero |] in
Llvm.const_struct ctx.llctx [| str_ptr; len_const |]
```

### 型による演算分岐

浮動小数型判定ヘルパー:
```ocaml
let is_float_type ty =
  match ty with
  | TCon (TCFloat _) -> true
  | _ -> false
```

整数加算 vs 浮動小数加算:
```ocaml
if is_float_type lhs.expr_ty then
  Llvm.build_fadd lhs_val rhs_val "fadd_tmp" ctx.builder
else
  Llvm.build_add lhs_val rhs_val "add_tmp" ctx.builder
```

## 統計情報

- **実装ファイル**: `codegen.ml`（662行）、`codegen.mli`（113行）
- **総コード行数**: 775行
- **対応する式種別**: 9種類（Phase 1スコープ）
- **対応するプリミティブ演算**: 17種類
- **対応する終端命令**: 4種類（Switchは未実装）
- **対応する文**: 6種類

## 既知の課題

### 現在の問題

**Core_ir.Ir型インポートエラー**:
- `open Core_ir.Ir`でモジュール内の型が正しく見えない
- OCamlのモジュールシステムにおける名前空間の問題
- 修正方法: `module IR = Core_ir.Ir`でモジュールエイリアスを作成し、`IR.var_id`等で参照

### Phase 1スコープ外（Phase 2以降で実装）

- Match式の決定木生成
- Closure（クロージャ変換）
- 型クラス辞書参照（DictLookup）
- Capability動的チェック
- ADT（代数的データ型）操作
- Record/Arrayアクセス
- グローバル変数の初期化式
- Switch終端命令

## 次のステップ

### Week 13-14 残タスク

1. **型エラー修正**
   - `module IR = Core_ir.Ir`エイリアス導入
   - 全型参照を`IR.xxx`形式に修正

2. **ビルド検証**
   - `dune build`で警告なくビルド成功
   - LLVM バインディングとの統合確認

3. **ユニットテスト実装**
   - `tests/test_llvm_codegen.ml`作成
   - モジュール生成テスト
   - 関数宣言テスト
   - 基本ブロック生成テスト
   - 式コード生成テスト（各式種別）

4. **技術文書作成**
   - `docs/llvm-codegen-architecture.md`
   - README更新（Week 13-14ダッシュボード）

### Week 15-16 計画

- ABI・呼び出し規約の詳細実装
- LLVM IR検証パイプライン（`opt -verify`統合）
- `--emit-ir` CLI統合
- ゴールデンテスト（LLVM IR期待値比較）
- エンドツーエンドテスト

## 参考資料

- 計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md)
- LLVM統合ガイド: [docs/guides/llvm-integration-notes.md](../../../docs/guides/llvm-integration-notes.md)
- Core IR仕様: [src/core_ir/ir.ml](../../src/core_ir/ir.ml)
- 型マッピング実装: [src/llvm_gen/type_mapping.ml](../../src/llvm_gen/type_mapping.ml)
- ターゲット設定: [src/llvm_gen/target_config.ml](../../src/llvm_gen/target_config.ml)

---

**作成者**: Claude (AI Assistant)
**日付**: 2025-10-09
**Phase**: Phase 3 Week 13-14
