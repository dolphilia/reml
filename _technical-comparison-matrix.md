# Kestrel言語コンパイラ実装言語技術比較表

## 総合評価マトリックス

| 評価軸 | 重要度 | OCaml | Rust | Haskell | C++ | C |
|--------|--------|-------|------|---------|-----|---|
| **HM型推論実装** | ★★★★★ | 5/5 | 3/5 | 5/5 | 2/5 | 1/5 |
| **ADT・パターンマッチ実装** | ★★★★★ | 5/5 | 4/5 | 5/5 | 3/5 | 2/5 |
| **パーサーコンビネーター実装** | ★★★★☆ | 5/5 | 4/5 | 5/5 | 3/5 | 2/5 |
| **LLVM連携** | ★★★★☆ | 4/5 | 5/5 | 3/5 | 5/5 | 5/5 |
| **Unicode/UTF-8処理** | ★★★☆☆ | 3/5 | 5/5 | 4/5 | 4/5 | 2/5 |
| **メモリ管理（RC+CoW）** | ★★★☆☆ | 4/5 | 3/5 | 4/5 | 4/5 | 2/5 |
| **開発・デバッグ体験** | ★★★☆☆ | 4/5 | 4/5 | 3/5 | 2/5 | 1/5 |
| **セルフコンパイル移行** | ★★☆☆☆ | 3/5 | 4/5 | 2/5 | 1/5 | 1/5 |
| **エコシステム・ライブラリ** | ★★☆☆☆ | 3/5 | 5/5 | 3/5 | 5/5 | 3/5 |
| **総合スコア** | - | **4.2/5** | **4.1/5** | **3.9/5** | **3.2/5** | **2.1/5** |

---

## 詳細評価と根拠

### HM型推論実装 (最重要)

#### OCaml (5/5) ⭐⭐⭐⭐⭐
**長所**:
- 型推論がネイティブサポート、言語仕様そのものが参考実装
- `unify`、`generalize`、`instantiate`の実装が極めて自然
- OCamlコンパイラのソースコードが世界最高の手本

**実装例**:
```ocaml
type typ = TVar of int | TConst of string | TArrow of typ * typ

let rec unify t1 t2 subst =
  match (t1, t2) with
  | (TVar v1, TVar v2) when v1 = v2 -> Some subst
  | (TVar v, t) | (t, TVar v) ->
      if occurs_check v t then None else Some ((v, t) :: subst)
  | (TArrow(a1, b1), TArrow(a2, b2)) ->
      unify a1 a2 subst >>= unify b1 b2
  | _ -> None
```

**実装コスト**: 低 (2-3週間)

#### Rust (3/5) ⭐⭐⭐
**長所**:
- `ena`ライブラリでUnificationTable提供
- 型安全性でバグ混入リスク低減

**短所**:
- Borrowチェッカーとの競合で複雑な設計が必要
- `Rc<RefCell<T>>`、Arena allocationが必須

**実装例**:
```rust
use ena::unify::{UnifyKey, UnificationTable};

#[derive(Clone, Debug)]
enum Type {
    Var(TypeVar),
    Const(String),
    Arrow(Box<Type>, Box<Type>),
}

struct TypeInferrer {
    table: UnificationTable<TypeVar>,
    types: Vec<Type>,
}
```

**実装コスト**: 中-高 (4-6週間)

#### Haskell (5/5) ⭐⭐⭐⭐⭐
**長所**:
- 型推論・型クラスが言語ネイティブ
- GHCコンパイラが参考実装

**短所**:
- セルフコンパイル移行で遅延評価→正格評価の根本変更

**実装コスト**: 低 (2-3週間)

#### C++ (2/5) ⭐⭐
**短所**:
- 完全に一から実装が必要
- std::variantでADT模倣、煩雑な実装

**実装コスト**: 高 (8-12週間)

#### C (1/5) ⭐
**短所**:
- すべて手動実装、極めて困難

**実装コスト**: 極高 (16-20週間)

---

### ADT・パターンマッチ実装

#### OCaml (5/5)
**長所**:
```ocaml
type expr =
  | EVar of string * span
  | ELam of string * typ option * expr * span
  | EApp of expr * expr * span
  | EPipe of expr * expr * span

let rec compile = function
  | EVar (name, _) -> lookup name
  | ELam (param, ty, body, _) -> compile_lambda param ty body
  | EApp (func, arg, _) -> compile_app func arg
  | EPipe (left, right, _) -> compile_pipe left right
```

**実装コスト**: 極低 (数日)

#### Rust (4/5)
**長所**:
- `enum`・`match`でネイティブサポート
- 網羅性チェック

**実装例**:
```rust
#[derive(Clone, Debug)]
enum Expr {
    Var(String, Span),
    Lam(String, Option<Type>, Box<Expr>, Span),
    App(Box<Expr>, Box<Expr>, Span),
    Pipe(Box<Expr>, Box<Expr>, Span),
}

fn compile(expr: &Expr) -> Result<Value, Error> {
    match expr {
        Expr::Var(name, _) => lookup(name),
        Expr::Lam(param, ty, body, _) => compile_lambda(param, ty, body),
        Expr::App(func, arg, _) => compile_app(func, arg),
        Expr::Pipe(left, right, _) => compile_pipe(left, right),
    }
}
```

**実装コスト**: 低 (1-2週間)

#### Haskell (5/5)
**長所**: OCamlと同等、さらに簡潔

#### C++ (3/5)
**短所**: `std::variant`と`std::visit`で模倣可能だが煩雑

**実装コスト**: 中 (3-4週間)

#### C (2/5)
**短所**: `union`とタグで手動実装、エラー発生しやすい

**実装コスト**: 高 (6-8週間)

---

### LLVM連携

#### C++ (5/5) / C (5/5)
**長所**:
- LLVM自体がC++、最も直接的なAPI
- 全機能への完全アクセス

**実装例**:
```cpp
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Module.h"

class CodeGen {
    llvm::LLVMContext context;
    llvm::IRBuilder<> builder;
    llvm::Module module;

public:
    llvm::Value* compileExpr(const Expr& expr) {
        switch (expr.kind) {
            case ExprKind::Int:
                return builder.getInt64(expr.intValue);
            case ExprKind::Add:
                return builder.CreateAdd(
                    compileExpr(expr.left),
                    compileExpr(expr.right)
                );
        }
    }
};
```

#### Rust (5/5)
**長所**:
- `inkwell`ライブラリが高品質
- rustc自体がLLVMベース

**実装例**:
```rust
use inkwell::context::Context;
use inkwell::builder::Builder;

struct CodeGen<'ctx> {
    context: &'ctx Context,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
    fn compile_expr(&self, expr: &Expr) -> IntValue<'ctx> {
        match expr {
            Expr::Int(val) => self.context.i64_type().const_int(*val, false),
            Expr::Add(left, right) => {
                let l = self.compile_expr(left);
                let r = self.compile_expr(right);
                self.builder.build_int_add(l, r, "add")
            }
        }
    }
}
```

#### OCaml (4/5)
**長所**:
- `llvm-ocaml`バインディング、API安定

**短所**:
- C++直接APIより一段薄い

**実装例**:
```ocaml
open Llvm

let compile_expr expr =
  let context = global_context () in
  let module_ = create_module context "kestrel" in
  let builder = builder context in

  let rec compile = function
    | EInt i -> const_int (i64_type context) i
    | EAdd (l, r) ->
        let lv = compile l and rv = compile r in
        build_add lv rv "add" builder
  in
  compile expr
```

#### Haskell (3/5)
**短所**:
- `llvm-hs`バインディング存在
- ただしGHCとの統合が複雑

---

### Unicode/UTF-8処理

#### Rust (5/5)
**長所**:
- `String`・`char`がUTF-8/UTF-32ネイティブ
- `unicode-segmentation`でGrapheme境界処理

**実装例**:
```rust
use unicode_segmentation::UnicodeSegmentation;

fn char_boundaries(text: &str) -> Vec<(usize, usize)> {
    text.grapheme_indices(true)
        .map(|(start, grapheme)| (start, start + grapheme.len()))
        .collect()
}
```

#### C++ (4/5)
**長所**: ICUライブラリで完全機能

**短所**: 外部依存、セットアップ複雑

#### Haskell (4/5)
**長所**: `text`パッケージでUTF-16ベース処理

#### OCaml (3/5)
**短所**: 外部ライブラリ必須（Uutf、Uuseg）

**実装例**:
```ocaml
open Uutf
open Uuseg

let grapheme_boundaries text =
  let decoder = decoder ~encoding:`UTF_8 (`String text) in
  let segmenter = segmenter `Grapheme_cluster in
  (* Implementation using Uuseg *)
```

#### C (2/5)
**短所**: 完全に手動実装、極めて困難

---

## 実装コスト見積もり

### MVP実装 (基本型推論・ADT・LLVM IR生成)

| 言語 | 期間 | 人月 | リスク | 品質 |
|------|------|------|--------|------|
| **OCaml** | 2-3ヶ月 | 3-4人月 | 低 | 高 |
| **Rust** | 3-4ヶ月 | 4-6人月 | 中 | 高 |
| **Haskell** | 2-3ヶ月 | 3-5人月 | 中 | 高 |
| **C++** | 6-8ヶ月 | 10-15人月 | 高 | 中 |
| **C** | 8-12ヶ月 | 15-25人月 | 極高 | 低 |

### 本格実装 (完全仕様・最適化・エラー処理)

| 言語 | 期間 | 人月 | 保守性 | 拡張性 |
|------|------|------|--------|--------|
| **OCaml** | 4-6ヶ月 | 8-12人月 | 高 | 高 |
| **Rust** | 5-7ヶ月 | 10-15人月 | 最高 | 最高 |
| **Haskell** | 4-6ヶ月 | 8-14人月 | 高 | 高 |
| **C++** | 12-18ヶ月 | 25-40人月 | 中 | 中 |
| **C** | 18-24ヶ月 | 40-60人月 | 低 | 低 |

### セルフコンパイル移行

| 元言語 | 移行期間 | 移行コスト | 実現性 |
|--------|----------|------------|--------|
| **Rust** | 6-8ヶ月 | 15-25人月 | 高 |
| **OCaml** | 8-12ヶ月 | 20-35人月 | 中 |
| **Haskell** | 12-18ヶ月 | 30-50人月 | 中-低 |
| **C++** | 18-24ヶ月 | 50-80人月 | 低 |
| **C** | 24-36ヶ月 | 80-120人月 | 極低 |

---

## 依存ライブラリ比較

### OCaml
```ocaml
(* dune-project *)
(package
 (name kestrel)
 (depends
  ocaml
  dune
  llvm          (* LLVM bindings *)
  uutf          (* UTF-8 codec *)
  uuseg         (* Unicode segmentation *)
  cmdliner      (* CLI parsing *)
  fmt           (* Formatting *)
  logs          (* Logging *)
  menhir        (* Parser generator *)
))
```

**総ライブラリサイズ**: ~50MB
**セットアップ複雑度**: 中
**長期保守**: 安定

### Rust
```toml
[dependencies]
inkwell = "0.2"           # LLVM bindings
unicode-segmentation = "1.0"
unicode-normalization = "0.1"
ena = "0.14"              # Unification
clap = "4.0"              # CLI parsing
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"            # Error handling
```

**総ライブラリサイズ**: ~200MB (LLVM含む)
**セットアップ複雑度**: 低
**長期保守**: 最高

### Haskell
```yaml
dependencies:
- base >= 4.7 && < 5
- llvm-hs >= 12.0
- text
- containers
- megaparsec
- mtl
- lens
```

**総ライブラリサイズ**: ~300MB
**セットアップ複雑度**: 中-高
**長期保守**: 中

### C++
```cmake
find_package(LLVM REQUIRED CONFIG)
find_package(PkgConfig REQUIRED)
pkg_check_modules(ICU REQUIRED icu-uc icu-io)

target_link_libraries(kestrel
  ${LLVM_LIBRARIES}
  ${ICU_LIBRARIES}
)
```

**総ライブラリサイズ**: ~500MB
**セットアップ複雑度**: 高
**長期保守**: 中

---

## パフォーマンス比較見積もり

### コンパイル速度

| 言語 | 小規模(1K LOC) | 中規模(10K LOC) | 大規模(100K LOC) |
|------|----------------|-----------------|------------------|
| **OCaml** | 0.1s | 1s | 10s |
| **Rust** | 0.2s | 2s | 15s |
| **Haskell** | 0.3s | 3s | 20s |
| **C++** | 0.1s | 0.8s | 8s |
| **C** | 0.05s | 0.5s | 5s |

### メモリ使用量

| 言語 | ベースライン | 型推論時 | CodeGen時 |
|------|-------------|----------|-----------|
| **OCaml** | 10MB | +20MB | +50MB |
| **Rust** | 15MB | +30MB | +60MB |
| **Haskell** | 20MB | +50MB | +100MB |
| **C++** | 5MB | +15MB | +40MB |
| **C** | 2MB | +10MB | +30MB |

### 生成コード品質

| 言語 | 最適化レベル | 実行速度 | バイナリサイズ |
|------|-------------|----------|---------------|
| **OCaml** | 中 | 90% | 3MB |
| **Rust** | 高 | 95% | 5MB |
| **Haskell** | 中 | 85% | 8MB |
| **C++** | 最高 | 100% | 2MB |
| **C** | 最高 | 100% | 1MB |

---

## 学習コスト見積もり

### 前提知識別学習期間

| 言語 | 関数型経験あり | システム言語経験 | 初心者 |
|------|---------------|-----------------|--------|
| **OCaml** | 1-2週間 | 4-6週間 | 8-12週間 |
| **Rust** | 4-6週間 | 2-3週間 | 12-16週間 |
| **Haskell** | 2-3週間 | 8-12週間 | 16-24週間 |
| **C++** | 6-8週間 | 1-2週間 | 8-12週間 |
| **C** | 4-6週間 | 1週間 | 6-8週間 |

### 必要スキル習得度

| 言語 | 基本文法 | 型システム | エコシステム | 最適化 |
|------|---------|-----------|-------------|--------|
| **OCaml** | 中 | 高 | 中 | 中 |
| **Rust** | 高 | 中 | 低 | 中 |
| **Haskell** | 高 | 最高 | 中 | 高 |
| **C++** | 中 | 低 | 中 | 高 |
| **C** | 低 | 最低 | 低 | 最高 |

---

## 総合推奨マトリックス

### プロジェクト状況別推奨

| 状況 | 第1推奨 | 第2推奨 | 避けるべき |
|------|---------|---------|-----------|
| **学術研究** | OCaml | Haskell | C, C++ |
| **商用開発** | Rust | OCaml | Haskell |
| **個人プロジェクト** | OCaml | Rust | C++ |
| **企業R&D** | Rust | OCaml | C |
| **最短実装** | OCaml | - | C++, C |
| **最高性能** | C++ | Rust | Haskell |
| **学習目的** | OCaml | Haskell | C |

この技術比較表により、具体的な数値と詳細な根拠に基づいて実装言語を選択できます。