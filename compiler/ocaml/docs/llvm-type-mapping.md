# LLVM IR 型マッピング技術文書

**作成日**: 2025-10-09
**Phase**: Phase 3 Week 12-13
**関連計画書**: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §2

## 概要

このドキュメントは、Reml の型システム（`Types.ty`）と LLVM IR の型システム（`Llvm.lltype`）の対応付けを定義する。Phase 3 では x86_64 Linux (System V ABI) を主要ターゲットとし、後のフェーズで Windows x64、ARM64 への対応を追加予定。

## 型マッピング表

### プリミティブ型

| Reml 型 | LLVM IR 型 | サイズ (bytes) | アラインメント (bytes) | 備考 |
|---------|-----------|----------------|------------------------|------|
| `Bool` | `i1` | 1 | 1 | 論理値 |
| `Char` | `i32` | 4 | 4 | Unicode scalar value (U+0000～U+10FFFF) |
| `i8` | `i8` | 1 | 1 | 符号付き8ビット整数 |
| `i16` | `i16` | 2 | 2 | 符号付き16ビット整数 |
| `i32` | `i32` | 4 | 4 | 符号付き32ビット整数 |
| `i64` | `i64` | 8 | 8 | 符号付き64ビット整数 |
| `isize` | `i64` | 8 | 8 | ポインタサイズ整数 (x86_64) |
| `u8` | `i8` | 1 | 1 | 符号なし8ビット整数 |
| `u16` | `i16` | 2 | 2 | 符号なし16ビット整数 |
| `u32` | `i32` | 4 | 4 | 符号なし32ビット整数 |
| `u64` | `i64` | 8 | 8 | 符号なし64ビット整数 |
| `usize` | `i64` | 8 | 8 | ポインタサイズ符号なし整数 (x86_64) |
| `f32` | `float` | 4 | 4 | IEEE 754 単精度浮動小数 |
| `f64` | `double` | 8 | 8 | IEEE 754 倍精度浮動小数 |
| `()` | `void` | 0 | 1 | 単位型 |
| `Never` | `void` | 0 | 1 | 到達不能型（実際には使用されない） |

### 複合型

#### タプル型

Reml のタプル型は LLVM の構造体型に対応する。

| Reml 型 | LLVM IR 型 | 備考 |
|---------|-----------|------|
| `(i64, Bool)` | `{ i64, i1 }` | 要素の順序を保証 |
| `(i32, f64, Bool)` | `{ i32, double, i1 }` | 3要素以上も同様 |

**アラインメント**: 最大要素のアラインメントに従う。

#### レコード型

Reml のレコード型は LLVM の名前付き構造体型に対応する（Phase 3 では無名構造体として実装）。

| Reml 型 | LLVM IR 型 | 備考 |
|---------|-----------|------|
| `{ x: i64, y: i64 }` | `{ i64, i64 }` | フィールド順序を保証 |
| `{ name: String, age: i32 }` | `{ { ptr, i64 }, i32 }` | String は FAT pointer |

**アラインメント**: 最大フィールドのアラインメントに従う。

#### 配列型とスライス型

| Reml 型 | LLVM IR 型 | サイズ (bytes) | 備考 |
|---------|-----------|----------------|------|
| `[i32]` (スライス) | `{ ptr, i64 }` | 16 | FAT pointer (data, len) |
| `[i64; 5]` (固定長) | `[5 x i64]` | 40 | 固定長配列 |
| `String` | `{ ptr, i64 }` | 16 | FAT pointer (UTF-8 data, byte len) |

**FAT pointer 構造**:
```llvm
%fat_ptr = type { ptr, i64 }
  ; ptr:  データへのポインタ
  ; i64:  要素数（スライス）またはバイト長（String）
```

#### 関数型

| Reml 型 | LLVM IR 型 | 備考 |
|---------|-----------|------|
| `i64 -> i64` | `i64 (i64)` | 関数ポインタ型 |
| `Bool -> ()` | `void (i1)` | 戻り値が unit の場合 |
| `(i32, i32) -> i64` | `i64 ({ i32, i32 })` | タプル引数 |

**クロージャ**: クロージャは環境キャプチャがある場合、以下の構造体として表現される。

```llvm
%closure = type { ptr, ptr }
  ; ptr:  環境ポインタ (env_ptr)
  ; ptr:  関数ポインタ (code_ptr)
```

#### 代数的データ型 (ADT)

Reml の ADT は **tagged union** として表現される。

```llvm
%adt = type { i32, %payload }
  ; i32:      タグ（バリアント識別子）
  ; %payload: ペイロード（最大幅のバリアントデータ）
```

**例**: `Option<i64>`

```llvm
%Option_i64 = type { i32, i64 }
  ; tag = 0: None
  ; tag = 1: Some(i64)
```

**アラインメント**: ペイロードのアラインメント（最低4バイト、タグのため）。

## DataLayout 設定

### x86_64 Linux (System V ABI)

```
e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64
```

**詳細**:
- `e`: リトルエンディアン
- `m:e`: ELF マングリング
- `p:64:64`: ポインタ64ビット、アラインメント64ビット
- `f64:64:64`: double 64ビット、アラインメント64ビット
- `v128:128:128`: ベクトル型128ビット、アラインメント128ビット
- `a:0:64`: 集成体アラインメント64ビット

### ターゲットトリプル

```
x86_64-unknown-linux-gnu
```

## 呼び出し規約

### System V AMD64 (x86_64 Linux)

LLVM では `cc ccc` (C calling convention) として自動処理される。

**引数渡し**:
- 整数引数: RDI, RSI, RDX, RCX, R8, R9
- 浮動小数引数: XMM0-XMM7
- スタック: 残りの引数（16バイトアラインメント）

**戻り値**:
- 整数: RAX
- 浮動小数: XMM0
- 構造体: メモリ経由（`sret` 属性）

**特殊処理**:
- 構造体引数・戻り値は LLVM 属性（`sret`, `byval`）で表現
- System V / Windows の差異は `llvm::DataLayout` から自動取得

## ABI 互換性ノート

### 構造体レイアウト

- **自然境界アラインメント**: デフォルトで自然境界に配置
- **パディング**: フィールド間に必要に応じてパディングを挿入
- **エンディアン**: リトルエンディアン（x86_64）

### 所有権とメモリ管理

Phase 3 では **参照カウント (RC)** ベースのランタイムを使用。

- **ヒープ割り当て**: `mem_alloc(size)` → `ptr`
- **参照カウント**: `inc_ref(ptr)`, `dec_ref(ptr)`
- **解放**: `mem_free(ptr)`

**FFI 境界での処理**:
- `extern` 関数へ渡す場合: `inc_ref` で寿命延長
- `extern` 関数から受け取る場合: 呼び出し側が `reml_release_*` を呼ぶ契約

詳細は [docs/guides/ffi/reml-ffi-handbook.md](../../guides/reml-ffi-handbook.md) 参照（Phase 2 以降で作成予定）。

## 実装ノート

### 型キャッシュ

再帰的型定義に対応するため、型変換結果をハッシュテーブルでメモ化。

```ocaml
type type_mapping_context = {
  llctx: Llvm.llcontext;
  llmodule: Llvm.llmodule;
  mutable type_cache: (Types.ty, Llvm.lltype) Hashtbl.t;
}
```

### 型変数の処理

型推論後は型変数 (`TVar`) は存在しないはず。LLVM IR 生成時に `TVar` が残存している場合はエラーとする。

```ocaml
| TVar tv ->
    failwith (Printf.sprintf "型変数 %s が LLVM IR 生成時に残存"
                (string_of_type_var tv))
```

### 将来拡張

**Phase 2 以降**:
- ジェネリック型の完全サポート（モノモルフィゼーション）
- ユーザ定義型 (`TCUser`) の構造解決
- Windows x64、ARM64 ターゲット対応
- デバッグ情報（DWARF）の付与

## テストカバレッジ

[compiler/ocaml/tests/test_llvm_type_mapping.ml](../tests/test_llvm_type_mapping.ml) にて以下を検証:

- プリミティブ型の全種類（15件）
- 複合型（タプル、レコード、配列）（5件）
- 関数型（3件）
- FAT pointer 構造（2件）
- サイズとアラインメント（6件）
- ターゲット設定（5件）

**合計**: 36件のテストケース

## 参考資料

- [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §2
- [docs/guides/compiler/llvm-integration-notes.md](../../../docs/guides/compiler/llvm-integration-notes.md) §5
- [docs/spec/1-2-types-Inference.md](../../../docs/spec/1-2-types-Inference.md)
- [System V Application Binary Interface (AMD64)](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf)
- [LLVM Language Reference Manual](https://llvm.org/docs/LangRef.html)
