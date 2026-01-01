# LLVM IR検証パイプライン

**作成日**: 2025-10-09
**Phase**: Phase 3 Week 15-16
**計画書**: [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §6

## 概要

このドキュメントは、生成されたLLVM IRの品質を保証するための検証パイプラインの設計と使い方を説明します。

検証パイプラインは以下の3段階で構成されます:

1. **llvm-as**: LLVM IRアセンブル (`.ll` → `.bc`)
2. **opt -verify**: LLVM検証パス実行（IR妥当性検証）
3. **llc**: ネイティブコード生成 (`.bc` → `.o`)

全ての段階が成功した場合のみ、LLVM IRは「検証成功」と判定されます。

## アーキテクチャ

```
Reml Source
    ↓ (Parser)
   AST
    ↓ (Type Inference)
 Typed AST
    ↓ (Desugar)
  Core IR
    ↓ (Optimization)
Optimized Core IR
    ↓ (Codegen)
  LLVM IR (.ll)
    ↓
┌───────────────────────────────────┐
│  検証パイプライン                    │
│  1. llvm-as (.ll → .bc)           │
│  2. opt -verify (.bc 検証)         │
│  3. llc (.bc → .o)                │
└───────────────────────────────────┘
    ↓
検証済みLLVM IR
```

## コンポーネント

### 1. 検証スクリプト

**ファイル**: `compiler/ocaml/scripts/verify_llvm_ir.sh`

**役割**: LLVM IRファイルを受け取り、3段階の検証を実行するシェルスクリプト。

**使い方**:
```bash
./compiler/ocaml/scripts/verify_llvm_ir.sh <input.ll>
```

**終了コード**:
- `0`: 検証成功
- `1`: 引数エラー
- `2`: llvm-as失敗
- `3`: opt -verify失敗
- `4`: llc失敗

**出力例**:
```
=========================================
LLVM IR 検証パイプライン
=========================================
入力: test.ll

[1/3] llvm-as: アセンブル (.ll → .bc)...
✓ llvm-as 成功
[2/3] opt -verify: LLVM 検証パス実行...
✓ opt -verify 成功
[3/3] llc: ネイティブコード生成 (.bc → .o)...
✓ llc 成功

=========================================
検証成功 ✓
=========================================
生成物:
  - ビットコード: test.bc
  - オブジェクトファイル: test.o
```

### 2. OCamlラッパーモジュール

**ファイル**: `src/llvm_gen/verify.ml/mli`

**役割**: 検証スクリプトをOCamlから呼び出し、エラーを診断形式へ変換する。

**主要API**:

```ocaml
(** LLVM IR を検証 *)
val verify_llvm_ir : Llvm.llmodule -> verification_result

(** LLVM IR ファイルを検証 *)
val verify_llvm_ir_file : string -> verification_result

(** 検証エラーを診断形式へ変換 *)
val error_to_diagnostic : verification_error -> Ast.span option -> Diagnostic.t
```

**使用例**:
```ocaml
(* LLVM モジュール生成 *)
let llmodule = Codegen.codegen_module ~target_name:"x86_64-linux" core_ir_module in

(* 検証実行 *)
match Verify.verify_llvm_ir llmodule with
| Ok () ->
    print_endline "検証成功"
| Error err ->
    let diag = Verify.error_to_diagnostic err None in
    Diagnostic.print diag
```

### 3. テストスイート

**ファイル**: `tests/test_llvm_verify.ml`

**カバレッジ**:
- 正常ケース: 7件（基本関数、関数呼び出し、条件分岐、算術演算、空関数、CFG、let束縛）
- エラーケース: ファイルベースで別途実装予定
- 境界値: 空関数、大きなブロック、ネスト深い制御フロー

**実行方法**:
```bash
dune exec -- ./tests/test_llvm_verify.exe
```

## 検証項目

### 1. llvm-as: アセンブル検証

**チェック内容**:
- LLVM IR構文の正当性
- トークン化・パース可否
- 基本的な型情報の整合性

**検出されるエラー例**:
- 構文エラー（未閉じ括弧、不正なトークン等）
- 不正な型注釈
- 不正な命令フォーマット

### 2. opt -verify: IR妥当性検証

**チェック内容**:
- SSA形式の正当性（φノード、支配関係）
- 型整合性（命令と値の型一致）
- 基本ブロックの構造（終端命令の存在、到達可能性）
- 関数シグネチャの一貫性

**検出されるエラー例**:
- 型不一致（例: `i32` と `i64` の混在）
- 未定義シンボル参照
- 無効な終端命令（例: ブロック末尾に `ret` がない）
- 到達不能コード
- φノードの不整合

### 3. llc: コード生成検証

**チェック内容**:
- ターゲット固有の制約（x86_64 Linux）
- ABI準拠（System V calling convention）
- レジスタ割り当て可能性
- メモリアラインメント

**検出されるエラー例**:
- ターゲット非対応の命令
- ABI違反（構造体引数・戻り値の渡し方）
- スタックアラインメント違反

## CI統合

### GitHub Actions設定

**ファイル**: `.github/workflows/ocaml-dune-test.yml`

**追加ステップ**:
1. LLVM 18ツールチェーンのインストール
2. `llvm-as`, `opt`, `llc`のシンボリックリンク作成
3. 検証テスト実行
4. 失敗時のLLVM IRアーティファクト保存

### アーティファクト保存

検証失敗時、以下のファイルが保存されます:
- `/tmp/reml_verify_*.ll`: 検証失敗したLLVM IR
- `/tmp/reml_verify_*.bc`: ビットコード（llvm-as成功時のみ）

保持期間: 7日間

## 診断エラーコード

| コード | 意味 | 原因コンポーネント |
|--------|------|-------------------|
| E9001  | llvm-as エラー | LLVM IRアセンブラ |
| E9002  | opt -verify エラー | LLVM検証パス |
| E9003  | llc エラー | LLVMコード生成器 |
| E9004  | スクリプトエラー | 検証パイプライン実行 |

## トラブルシューティング

### 問題: llvm-as が見つからない

**症状**:
```
エラー: llvm-as が見つかりません。LLVM 15+ をインストールしてください。
```

**解決策**:
1. LLVM 15以上をインストール:
   ```bash
   # Ubuntu/Debian
   sudo apt-get install llvm-18 llvm-18-tools

   # macOS (Homebrew)
   brew install llvm@18
   ```

2. 環境変数を設定:
   ```bash
   export LLVM_AS=/usr/bin/llvm-as-18
   export OPT=/usr/bin/opt-18
   export LLC=/usr/bin/llc-18
   ```

### 問題: opt -verify が失敗する

**症状**:
```
[2/3] opt -verify: LLVM 検証パス実行...
エラー: opt -verify が失敗しました（終了コード: 3）
```

**解決策**:
1. LLVM IRを手動で確認:
   ```bash
   cat <input.ll>
   ```

2. opt単体で詳細エラーを確認:
   ```bash
   opt -verify <input.ll> 2>&1 | less
   ```

3. Core IR生成・最適化パスを見直し（型不一致・未定義変数等）

### 問題: llc が失敗する

**症状**:
```
[3/3] llc: ネイティブコード生成 (.bc → .o)...
エラー: llc が失敗しました（終了コード: 4）
```

**解決策**:
1. ターゲット設定を確認（x86_64 Linux向けか）
2. llc単体で詳細エラーを確認:
   ```bash
   llc -mtriple=x86_64-unknown-linux-gnu <input.bc> 2>&1 | less
   ```

3. ABI実装を見直し（構造体引数・戻り値の属性設定）

## 今後の拡張

### Phase 2以降の計画

1. **Windows x64対応**
   - ターゲット: `x86_64-pc-windows-msvc`
   - 呼び出し規約: Win64
   - 検証スクリプトの拡張（ターゲット別検証）

2. **ARM64対応**
   - ターゲット: `aarch64-unknown-linux-gnu` / `aarch64-apple-darwin`
   - クロスコンパイル検証

3. **詳細診断の強化**
   - LLVM診断出力のパース（行番号・カラム位置の抽出）
   - Span情報へのマッピング精度向上
   - FixIt提案の自動生成

4. **性能プロファイリング**
   - 検証時間の計測
   - ボトルネック分析

## 参考資料

- [1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) — LLVM IR生成計画
- [guides/llvm-integration-notes.md](../../../docs/guides/compiler/llvm-integration-notes.md) — LLVM統合設計
- [3-6-core-diagnostics-audit.md](../../../docs/spec/3-6-core-diagnostics-audit.md) — 診断形式仕様
- [LLVM Language Reference](https://llvm.org/docs/LangRef.html)
- [LLVM Verifier](https://llvm.org/docs/Passes.html#verify-module-verifier)

---

**最終更新**: 2025-10-09
