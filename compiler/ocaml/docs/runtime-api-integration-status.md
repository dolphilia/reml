# ランタイム API 統合状況レポート

**作成日**: 2025-10-10
**Phase**: Phase 1-5（ランタイム連携）— §1 API 定義完了
**次ステップ**: §2 メモリアロケータ実装

---

## 概要

`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §1「ランタイムAPI設計」に基づき、最小ランタイム API のヘッダ定義と初期構造を整備しました。

---

## 成果物

### 1. ヘッダファイル

**ファイル**: `runtime/native/include/reml_runtime.h`

**内容**:
- **バージョン定義**: `REML_RUNTIME_VERSION_*` マクロ（0.1.0）
- **ヒープオブジェクトヘッダ**: `reml_object_header_t` （refcount + type_tag、8バイト）
- **型タグ enum**: `reml_type_tag_t` （9 種類の基本型）
- **6 関数の API 定義**:
  - `void* mem_alloc(size_t size)` — ヒープメモリ割り当て
  - `void mem_free(void* ptr)` — ヒープメモリ解放
  - `void inc_ref(void* ptr)` — 参照カウントインクリメント
  - `void dec_ref(void* ptr)` — 参照カウントデクリメント + 解放
  - `void panic(const char* msg)` — 異常終了（noreturn）
  - `void print_i64(int64_t value)` — デバッグ用整数出力
- **内部ヘルパーマクロ**: `REML_GET_HEADER(ptr)` — ペイロードからヘッダへの逆引き

**検証結果**:
```bash
$ gcc -c -I runtime/native/include runtime/native/src/print_i64.c -o /tmp/print_i64.o -Wall -Wextra
# 成功 — ヘッダが妥当な C コードとしてコンパイル可能
```

### 2. ディレクトリ構造

```
runtime/native/
├── include/
│   └── reml_runtime.h        ✅ 完了
├── src/
│   └── print_i64.c           ✅ 完了（簡易実装例）
└── tests/
    └── (Phase 1-5 §7 で追加予定)
```

### 3. ドキュメント更新

- **計画書**: `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §1.1 に `panic` シグネチャの注意事項を追記
- **README**: `compiler/ocaml/README.md` の Phase 1-5 セクションを更新し、API 定義完了をチェックマーク付きで記録

---

## コンパイラ側との整合性

### 現在の宣言（`compiler/ocaml/src/llvm_gen/codegen.ml:176-200`）

| 関数 | LLVM IR シグネチャ | ランタイムヘッダ | 整合性 |
|------|-------------------|----------------|--------|
| `mem_alloc` | `(i64) -> ptr` | `void* mem_alloc(size_t)` | ✅ 一致 |
| `inc_ref` | `(ptr) -> void` | `void inc_ref(void*)` | ✅ 一致 |
| `dec_ref` | `(ptr) -> void` | `void dec_ref(void*)` | ✅ 一致 |
| `panic` | `(ptr, i64) -> void noreturn` | `void panic(const char*) noreturn` | ⚠️ シグネチャ形式の差異あり（後述） |

### `panic` のシグネチャに関する設計決定

**問題**:
- **LLVM IR 側**: `panic(ptr, i64)` — 文字列を FAT ポインタ形式 `{ptr, len}` として渡す
- **C 実装側**: `panic(const char*)` — NULL 終端文字列を想定

**採用した方針**:
1. **LLVM IR 側の実装を優先** — コンパイラが生成する IR は `panic(ptr, i64)` のまま維持
2. **C 実装側での吸収** — ランタイム実装で `panic(const char* msg)` として受け取り、長さパラメータは無視
3. **計画書への注記** — `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §1.1 に「Phase 1 では FAT ポインタ形式を採用」と明記

**理由**:
- Phase 1 では文字列リテラルが FAT ポインタとして扱われるため、LLVM IR 側の整合性を優先
- C 実装側は NULL 終端を前提とすることで、`panic("Out of memory")` のような単純な呼び出しが可能
- 長さパラメータ（i64）は将来的にバイナリセーフなエラーメッセージ対応で利用可能

### 宣言されていない関数

| 関数 | ヘッダ定義 | LLVM IR 宣言 | 理由 |
|------|-----------|-------------|------|
| `mem_free` | ✅ あり | ❌ なし | 実装内部（`dec_ref`）でのみ使用、コンパイラから直接呼ばれない |
| `print_i64` | ✅ あり | ❌ なし | デバッグ用ユーティリティ、必要に応じて後で宣言可能 |

**対応不要**: これらの関数はランタイム内部で使用されるため、LLVM IR での明示的な宣言は Phase 1 では不要。

---

## 型タグ定義

### `reml_type_tag_t` enum（Phase 1 対応範囲）

| タグ | 値 | 用途 | Phase 1 実装状況 |
|------|---|------|----------------|
| `REML_TAG_INT` | 1 | 整数型（i32, i64） | 予定 |
| `REML_TAG_FLOAT` | 2 | 浮動小数点型（f32, f64） | 予定 |
| `REML_TAG_BOOL` | 3 | 真偽値型 | 予定 |
| `REML_TAG_STRING` | 4 | 文字列型（FAT pointer） | 予定 |
| `REML_TAG_TUPLE` | 5 | タプル型 | 予定 |
| `REML_TAG_RECORD` | 6 | レコード型 | 予定 |
| `REML_TAG_CLOSURE` | 7 | クロージャ | 予定 |
| `REML_TAG_ADT` | 8 | 代数的データ型 | 予定 |

**Phase 2 以降で追加予定**: `REML_TAG_ARRAY`, `REML_TAG_SLICE`, `REML_TAG_CUSTOM`, ...

### ヒープオブジェクトヘッダ構造

```c
typedef struct {
    uint32_t refcount;  // 参照カウント（初期値: 1）
    uint32_t type_tag;  // 型タグ（reml_type_tag_t）
} reml_object_header_t;
```

**メモリレイアウト**:
```
[reml_object_header_t (8 bytes)] [payload (n bytes)]
 ↑                                ↑
 ヘッダ                            mem_alloc が返すポインタ
```

**アラインメント**: 8 バイト境界（System V ABI / Windows x64 ABI 準拠）

---

## 検証項目

### 静的チェック（完了）

- [x] `reml_runtime.h` が単体でコンパイル可能
- [x] `print_i64.c` がヘッダをインクルードしてコンパイル可能
- [x] 警告なし（`-Wall -Wextra`）

### ドキュメント整合性（完了）

- [x] 計画書 §1.1 の関数一覧と `reml_runtime.h` の宣言が一致
- [x] `docs/guides/compiler/llvm-integration-notes.md` §5.4 との整合を確認
- [x] `panic` のシグネチャ差異が明示的に文書化されている

### コンパイラ統合（次ステップで検証）

- [ ] `mem_alloc` の実装が完了後、コンパイラから呼び出せることを確認
- [ ] `inc_ref` / `dec_ref` の挿入箇所を特定し、リンクテスト
- [ ] `panic` のエラーメッセージ出力を検証

---

## 次ステップ

### Phase 1-5 §2: メモリアロケータ実装

**ファイル**: `runtime/native/src/mem_alloc.c`

**実装内容**:
1. `malloc` ベースの単純実装
2. ヘッダ領域の初期化（refcount=1, type_tag 設定）
3. 8 バイト境界への自動調整
4. アロケーション失敗時の `panic` 呼び出し

**前提条件**: ✅ API 定義完了（本レポート）

**スケジュール**: Phase 1-5 Week 13-14（計画書 §2）

---

## 参考資料

- **計画書**: `docs/plans/bootstrap-roadmap/1-5-runtime-integration.md`
- **LLVM 統合ガイド**: `docs/guides/compiler/llvm-integration-notes.md` §5
- **仕様調査**: `docs/notes/llvm-spec-status-survey.md` §2.5
- **コンパイラ実装**: `compiler/ocaml/src/llvm_gen/codegen.ml:176-200`
- **ABI 実装**: `compiler/ocaml/src/llvm_gen/abi.ml`, `compiler/ocaml/src/llvm_gen/llvm_attr.ml`

---

**報告者**: Claude Code
**レビュー待ち**: Phase 1-5 §2 着手前に本レポートを確認すること
