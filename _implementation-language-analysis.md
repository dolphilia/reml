# Kestrel言語コンパイラ実装言語選定分析レポート

## エグゼクティブサマリー

本レポートは、Kestrel言語コンパイラのセルフコンパイル実現に向けた最初の実装言語選定について、仕様書の詳細調査と多角的分析に基づく包括的な提案を行います。

**主要推奨事項**：
1. **第1推奨: OCaml** - 型推論・ADT実装に最適、最短期間でのBootstrap実現可能
2. **第2推奨: Rust** - セルフコンパイル移行が容易、エコシステムが豊富
3. **段階的移行戦略** - MVP実装 → 本格実装 → セルフコンパイル

---

## 1. Kestrel言語の特徴・要件分析

### 1.1 言語コア特徴

#### 型システム
- **Hindley-Milner型推論**: Algorithm Wによる型推論、量化・インスタンス化
- **代数的データ型(ADT)**: Sum型・Product型、パターンマッチング
- **トレイト（型クラス）システム**: 演算子オーバーロード、辞書パッシング
- **ジェネリクス**: ランク1多相、モノモルフィゼーション
- **値制限**: 効果のある式は単相に制限

#### 効果システム
- **Pure関数デフォルト**: 副作用なしがデフォルト
- **効果フラグ**: `mut`, `io`, `ffi`, `panic`, `unsafe`
- **属性による制約**: `@pure`, `@no_panic`, `@no_alloc`
- **Result/Option**: 例外なし、型によるエラー処理

#### メモリ管理
- **不変データデフォルト**: 参照透明性重視
- **参照カウント + Copy-on-Write**: 関数型スタイルで実用性能
- **defer文**: スコープ終端でのリソース解放保証
- **unsafe境界**: FFI・原始操作の明示的分離

#### Unicode文字モデル
- **UTF-8前提**: ソースコード・文字列
- **3層モデル**: Byte/Char（Unicode scalar value）/Grapheme（extended grapheme cluster）

### 1.2 パーサーコンビネーター特化設計

#### コア設計（Nest.Parse）
- **12-15個の最小コンビネータ**: map, then, or, many, cut, label等
- **consumed/committed意味論**: 2ビットで選択・コミット制御
- **ゼロコピー入力**: 不変ビュー、オフセット操作のみ
- **トランポリン・末尾最適化**: スタック安全保証

#### エラーシステム
- **最遠位置エラー**: farthest-first原則
- **期待集合**: Token/Rule/Class/Custom等の構造化期待
- **cut/label/recover/trace**: 高品質診断のための四点セット
- **FixIt提案**: IDE向け修復候補生成

#### 実行戦略
- **デフォルト**: LL(*)相当の前進解析
- **オプション**: Packrat（線形時間）、左再帰（seed-growing）
- **スライディング窓メモ化**: メモリ使用量制御
- **ストリーミング・インクリメンタル**: REPL・IDE向け

### 1.3 コンパイラパイプライン要件

#### 段階的処理
1. **構文解析**: 自己記述可能（Nest.Parse使用）
2. **意味解析**: 名前解決・HM型推論・制約解決
3. **Core IR降格**: 糖衣剥がし（パイプ・パターンマッチ・辞書パッシング）
4. **MIR最適化**: モノモルフィゼーション・インライン・CFG構築
5. **LLVM IR生成**: 型レイアウト・呼出規約・メモリ管理

#### ターゲット
- **LLVM IRコード生成**: 現代的最適化・バックエンド
- **C互換FFI**: extern "C"呼出規約
- **セルフコンパイル**: Bootstrap完了後の自己記述移行

---

## 2. 実装言語候補の詳細評価

### 2.1 評価軸の定義

| 評価軸 | 重要度 | 説明 |
|--------|--------|------|
| HM型推論実装 | ★★★★★ | Unification、Generalization/Instantiation、制約解決 |
| ADT・パターンマッチ実装 | ★★★★★ | AST表現、パターン解析、変換処理 |
| パーサーコンビネーター実装 | ★★★★☆ | 関数合成、モナド的操作、エラー処理 |
| LLVM連携 | ★★★★☆ | バインディング品質、API安定性、コード生成 |
| Unicode/UTF-8処理 | ★★★☆☆ | 文字境界、正規化、エンコーディング |
| メモリ管理（RC+CoW） | ★★★☆☆ | 参照カウント、Copy-on-Write、RAII |
| 開発・デバッグ体験 | ★★★☆☆ | ツール支援、エラー診断、生産性 |
| セルフコンパイル移行 | ★★☆☆☆ | 構文類似性、意味論ギャップ、移植コスト |
| エコシステム・ライブラリ | ★★☆☆☆ | 依存ライブラリ、コミュニティ、長期保守性 |

### 2.2 各言語の詳細評価

#### 2.2.1 OCaml ⭐⭐⭐⭐⭐ (総合スコア: 4.6/5)

**強み**:

*型推論・ADT実装 (5/5)*:
- HM型推論がネイティブサポート
- OCamlコンパイラ自体が参考実装として活用可能
- variant型・パターンマッチでKestrel ASTを直接表現
- unificationアルゴリズムの実装が極めて自然

*パーサーコンビネーター実装 (5/5)*:
- 関数型言語特性（高階関数・クロージャ・immutability）
- モナド風操作の実装が直感的
- エラー処理がResult型で統一

*コンパイラ実装実績*:
- OCamlコンパイラ（世界最高レベルのHM型推論実装）
- Coq証明支援系
- F*関数型言語
- ReasonML transpiler

**弱み**:

*LLVM連携 (4/5)*:
- ocaml-llvmバインディング存在、API安定
- ただし、C++直接APIと比べると一段薄いレイヤー

*Unicode処理 (3/5)*:
- 標準ライブラリは基本的なUTF-8サポートのみ
- Uutf（UTF-8 codec）、Uuseg（text segmentation）等の外部ライブラリ必要
- 実装コストは増加するが、品質は十分

*セルフコンパイル移行 (3/5)*:
- 構文ギャップが大きい（ML系だが詳細は異なる）
- ただし意味論は近いため、AST変換は比較的容易

**推奨実装戦略**:
```ocaml
(* HM型推論の実装例 *)
type typ =
  | TVar of int
  | TConst of string
  | TApp of typ * typ list
  | TArrow of typ * typ

let unify typ1 typ2 subst =
  (* Algorithm W unification *)
  ...

(* ADTの実装例 *)
type expr =
  | EVar of string
  | ELam of string * typ option * expr
  | EApp of expr * expr
  | ELet of string * expr * expr
```

#### 2.2.2 Rust ⭐⭐⭐⭐ (総合スコア: 4.0/5)

**強み**:

*LLVM連携 (5/5)*:
- rustc自体がLLVMベース、豊富な実績
- inkwell、llvm-sysライブラリが成熟
- LLVMバージョン追従が良好

*メモリ・型安全性 (5/5)*:
- Kestrelの安全性志向と完全一致
- Borrowチェッカーによるメモリ安全保証
- enum・matchでADTをネイティブ表現

*エコシステム (5/5)*:
- cargo/crates.ioによる豊富なライブラリ
- unicode-segmentation、unicode-normalizationなど
- 活発なコミュニティ・長期サポート

*セルフコンパイル移行 (4/5)*:
- 構文（match、enum、impl）がKestrelに最も近い
- 意味論（所有権、borrow）は異なるが、移植は段階的に可能

**弱み**:

*HM型推論実装 (3/5)*:
- RustのOwnership systemとHM型推論の組み合わせが複雑
- 可変状態管理（Rc<RefCell<T>>、Arena allocation）が必要
- ライフタイム管理が型推論実装を複雑化

*関数型スタイル (3/5)*:
- Immutabilityがデフォルトでない
- クロージャキャプチャが複雑（move、借用）
- パーサーコンビネーター実装で所有権との競合発生可能

**推奨実装戦略**:
```rust
// HM型推論の実装例
use ena::unify::{UnifyKey, UnificationTable};

#[derive(Clone, Debug)]
enum Type {
    Var(TypeVar),
    Const(String),
    App(Box<Type>, Vec<Type>),
    Arrow(Box<Type>, Box<Type>),
}

struct TypeInferrer {
    table: UnificationTable<TypeVar>,
    // Rcを使って循環参照を避けつつ可変状態を管理
}
```

#### 2.2.3 Haskell ⭐⭐⭐⭐ (総合スコア: 4.25/5)

**強み**:

*純粋関数型一致 (5/5)*:
- Kestrelのpure-by-default設計と完全一致
- Immutability、参照透明性がデフォルト

*HM型推論・ADT (5/5)*:
- GHCコンパイラが世界最高の参考実装
- データ型、パターンマッチが言語ネイティブ
- 型クラスシステムがKestrelトレイトの直接モデル

*パーサーコンビネーター (5/5)*:
- Parsec、Megaparsec等の最成熟ライブラリ
- モナド変換子による効果の分離
- エラー処理の洗練

**弱み**:

*LLVM連携 (3/5)*:
- llvm-hsバインディングは存在
- ただしGHCの複雑なRTSとの統合が困難
- 既存Haskell→LLVMパスが複雑

*実行時性能 (3/5)*:
- 遅延評価によるメモリ使用パターンが予測困難
- Kestrelは正格評価のため、パフォーマンス特性が異なる

*セルフコンパイル移行 (2/5)*:
- 遅延評価→正格評価の根本的な変更が必要
- モナド中心→直接スタイルの大幅変更

#### 2.2.4 C++ ⭐⭐⭐ (総合スコア: 3.25/5)

**強み**:

*LLVM連携 (5/5)*:
- LLVM自体がC++製、最も直接的なAPI
- 全機能への完全アクセス
- パフォーマンス最適化の余地が最大

*フレキシビリティ (4/5)*:
- 任意の設計パターンを実装可能
- メモリレイアウトの完全制御
- 最適化の余地が豊富

**弱み**:

*型推論・ADT実装 (2/5)*:
- std::variantでADT模倣可能だが煩雑
- HM型推論は完全に一から実装
- パターンマッチング構文なし

*開発効率・安全性 (2/5)*:
- 手動メモリ管理によるバグリスク
- 実装コード量が膨大
- デバッグ・保守性の問題

*セルフコンパイル移行 (1/5)*:
- 構文・パラダイムが根本的に異なる
- 移行コストが極めて高い

#### 2.2.5 C ⭐⭐ (総合スコア: 2.5/5)

**強み**:

*LLVM連携 (5/5)*:
- LLVM-C APIで直接アクセス
- 最小限のオーバーヘッド

*軽量性 (4/5)*:
- 最小限のランタイム依存
- 全環境での移植性

**弱み**:

*抽象化レベル (1/5)*:
- HM型推論実装が極めて困難
- ADT、パターンマッチを手動実装
- 開発効率が極めて低い

*安全性 (1/5)*:
- 手動メモリ管理
- 型安全性なし
- バグ混入リスク極大

---

## 3. 推奨実装言語と戦略

### 3.1 総合評価と推奨順位

| 順位 | 言語 | 総合スコア | 主要推奨理由 |
|------|------|-----------|-------------|
| 🥇 | **OCaml** | 4.6/5 | HM型推論・ADT実装最適、最短Bootstrap |
| 🥈 | **Rust** | 4.0/5 | セルフコンパイル移行容易、エコシステム豊富 |
| 🥉 | **Haskell** | 4.25/5 | 関数型完全一致、型推論参考実装豊富 |
| 4位 | C++ | 3.25/5 | LLVM直接アクセス、最大性能 |
| 5位 | C | 2.5/5 | 最小依存、軽量 |

### 3.2 第1推奨：OCaml採用の詳細根拠

#### 3.2.1 技術的優位性

**型推論実装の自然さ**:
```ocaml
(* OCamlでの型推論実装例 *)
let rec unify t1 t2 subst =
  match (t1, t2) with
  | (TVar v1, TVar v2) when v1 = v2 -> Some subst
  | (TVar v, t) | (t, TVar v) ->
      if occurs_check v t then None
      else Some ((v, t) :: subst)
  | (TArrow(a1, b1), TArrow(a2, b2)) ->
      unify a1 a2 subst >>= unify b1 b2
  | (TConst c1, TConst c2) when c1 = c2 -> Some subst
  | _ -> None
```

**ADT表現の直接性**:
```ocaml
(* Kestrel ASTの直接表現 *)
type expr =
  | EVar of string * span
  | ELam of string * typ option * expr * span
  | EApp of expr * expr * span
  | EPipe of expr * expr * span
  | EMatch of expr * (pattern * expr) list * span

type pattern =
  | PVar of string
  | PConst of literal
  | PConstruct of string * pattern list
  | PTuple of pattern list
```

#### 3.2.2 実装戦略

**Phase 1: MVP Bootstrap (2-3ヶ月)**
- 基本型推論（単相→多相）
- 最小ADT・パターンマッチ
- シンプルなLLVM IR生成
- 算術・条件・関数のみ

**Phase 2: Full Implementation (4-6ヶ月)**
- 完全HM型推論 + 制約解決
- Nest.Parse標準ライブラリ
- 効果システム実装
- 最適化パス

**Phase 3: Self-Compilation (6-12ヶ月)**
- Kestrel自己記述版作成
- OCaml→Kestrel移植
- Bootstrap完了

#### 3.2.3 依存ライブラリ戦略

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
  menhir        (* Parser generator for bootstrap *)
))
```

### 3.3 第2推奨：Rust採用の場合

#### 3.3.1 技術戦略

**型推論実装アプローチ**:
```rust
use ena::unify::{UnifyKey, UnificationTable};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TypeVar(u32);

impl UnifyKey for TypeVar {
    type Value = Option<Type>;
    fn index(&self) -> u32 { self.0 }
    fn from_index(u: u32) -> Self { TypeVar(u) }
    fn tag() -> &'static str { "TypeVar" }
}

// Arena allocation for AST nodes
struct InferCtx {
    types: UnificationTable<TypeVar>,
    constraints: Vec<Constraint>,
    scopes: Vec<HashMap<String, Scheme>>,
}
```

**パーサーコンビネーター実装**:
```rust
// Nom-style combinator with custom error handling
type IResult<I, O> = Result<(I, O), ParseError>;

trait Parser<I, O> {
    fn parse(&self, input: I) -> IResult<I, O>;

    fn map<F, O2>(self, f: F) -> Map<Self, F>
    where F: Fn(O) -> O2;

    fn and_then<F, P2>(self, f: F) -> AndThen<Self, F>
    where F: Fn(O) -> P2, P2: Parser<I, O>;
}
```

#### 3.3.2 セルフコンパイル移行の容易さ

Rustの構文がKestrelに最も近いため、段階的移行が可能：

```rust
// Rust
enum Expr {
    Var(String),
    App(Box<Expr>, Box<Expr>),
    Lam(String, Box<Expr>),
}

match expr {
    Expr::Var(name) => ...,
    Expr::App(func, arg) => ...,
    Expr::Lam(param, body) => ...,
}
```

```kestrel
// Kestrel (target)
type Expr =
  | Var(String)
  | App(Expr, Expr)
  | Lam(String, Expr)

match expr with
| Var(name) -> ...
| App(func, arg) -> ...
| Lam(param, body) -> ...
```

### 3.4 実装フェーズ戦略

#### Phase 1: Rapid Prototyping (2-3ヶ月)
**目標**: 基本概念実証
- 選択言語でミニマルコンパイラ実装
- 基本型推論（let多相）
- シンプルADT・パターンマッチ
- LLVM IR出力（関数・条件・算術）

**成果物**:
- Proof of Concept実装
- LLVM IR生成まで一貫動作
- 性能・技術負債の評価

#### Phase 2: Production Implementation (4-6ヶ月)
**目標**: 本格コンパイラ実装
- 完全HM型推論・制約解決
- Nest.Parse標準ライブラリ
- 効果システム（pure/io/mut/unsafe）
- 最適化パス（インライン・モノモルフィゼーション）

**成果物**:
- Production-ready Kestrelコンパイラ
- セルフコンパイル準備完了

#### Phase 3: Self-Compilation (6-12ヶ月)
**目標**: Bootstrap完了
- Kestrel言語での自己記述版作成
- 段階的移植・検証
- 元実装からの移行完了

**成果物**:
- 完全セルフコンパイル達成
- Bootstrapping完了

---

## 4. リスク分析と対策

### 4.1 OCaml採用時のリスク

| リスク | 影響度 | 対策 |
|--------|--------|------|
| Unicode処理の複雑性 | 中 | Uutf/Uusegライブラリ活用、専用モジュール分離 |
| LLVMバインディング制約 | 低 | C++ラッパー作成、必要に応じてFFI直接呼び出し |
| エコシステム規模 | 低 | 必要ライブラリは存在、最小依存戦略 |
| セルフコンパイル移行コスト | 中 | 段階的移行、構文変換ツール作成 |

### 4.2 Rust採用時のリスク

| リスク | 影響度 | 対策 |
|--------|--------|------|
| HM型推論実装複雑性 | 高 | ena等の既存ライブラリ活用、Arena allocation |
| Borrowチェッカー競合 | 中 | Rc<RefCell<T>>、適切なデータ構造設計 |
| 開発初期の学習コスト | 中 | チーム内Rust習熟、プロトタイプでのリスク軽減 |

### 4.3 共通リスク

| リスク | 影響度 | 対策 |
|--------|--------|------|
| LLVM APIバージョン依存 | 中 | 安定版固定、互換性レイヤー作成 |
| パフォーマンス要件未達 | 中 | プロファイリング継続、最適化段階的実装 |
| 仕様変更による大幅修正 | 高 | モジュラー設計、テスト自動化 |

---

## 5. 結論と推奨事項

### 5.1 最終推奨事項

**第1選択: OCaml**を強く推奨します。

**根拠**:
1. **技術的適合性**: HM型推論・ADT実装でOCamlが圧倒的優位
2. **開発効率**: 最短期間でのBootstrap実現
3. **安定性**: 実績豊富、技術リスク最小
4. **参考実装**: OCamlコンパイラが世界最高レベルの手本

**代替案**: プロジェクト制約（エコシステム重視、長期保守性、チームスキル）によってはRustも有効

### 5.2 成功要因

1. **段階的実装**: MVP→本格実装→セルフコンパイル
2. **早期検証**: Phase 1での技術概念実証
3. **モジュラー設計**: 言語移行を前提とした疎結合
4. **テスト自動化**: 仕様変更耐性の確保

### 5.3 次のステップ

1. **実装言語の最終決定** (1週間)
2. **開発環境セットアップ** (1週間)
3. **Phase 1実装開始** (2-3ヶ月)
4. **技術評価・改善** (1ヶ月)
5. **Phase 2移行判断** (継続評価)

この分析に基づいて、Kestrel言語の実装者は十分な情報を持って適切な実装言語を選択し、効率的なBootstrap戦略を策定できると考えます。