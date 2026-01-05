# 第7章: 型チェックと型推論

## 1. 概要 (Introduction)

本章では、Reml コンパイラのフロントエンドにおける中核機能である型チェックと型推論について解説します。

Reml の型システムは、Hindley-Milner 型推論をベースとしつつ、副作用（Effect）や Capability の概念を統合した独自の体系を持っています。このフェーズの主な責任は、構文解析によって生成された「生の」AST（抽象構文木）を入力とし、各式の型を決定し、型安全性を検証し、最終的に意味解析済みの「型付き AST（Typed AST）」と「中間表現（MIR）」を出力することです。

実装の主体となるのは `compiler/frontend/src/typeck` モジュールです。このモジュールは、単なる型合わせだけでなく、ジェネリクスの単一化（Unification）、制約（Constraint）の解決、そして Effect System に基づく権限監査の基礎データ生成も担います。

### なぜ型推論が必要なのか？

Reml は静的型付け言語ですが、ユーザーがすべての場所で型注釈を書く必要はありません。コンパイラはプログラムの文脈から変数の型を自動的に導き出します。例えば、`let x = 10` という宣言があれば、`x` は整数型であると推論されます。

このプロセスを実現するために、Reml は **制約ベース（Constraint-Based）** のアプローチを採用しています。プログラムを走査しながら「この式は Int でなければならない」「この変数は関数でなければならない」といった制約を収集し、それらを連立方程式のように解くことで全体の型を決定します。

### 入力と出力

- **入力**: `Parser AST` (`compiler/frontend/src/parser/ast.rs`)
  - 構文解析フェーズの出力であり、ソースコードの構造をそのまま表現しています。型情報は部分的、あるいは欠落しています。
- **出力**: `TypecheckReport` (`compiler/frontend/src/typeck/driver.rs`)
  - 以下の情報を含む包括的なレポートです。
    - **Typed AST**: すべての式に型情報が付与された AST。
    - **MIR (Mid-level IR)**: 後続のコンパイルフェーズ（LLVM IR 生成など）で利用しやすいように単純化された中間表現。
    - **Violations**: 型不一致や未定義変数などの検出されたエラー一覧。

## 2. データ構造 (Key Data Structures)

型推論の中核となるデータ構造について解説します。これらは主に `compiler/frontend/src/typeck/types.rs` および `constraint.rs` で定義されています。

### 2.1 型の表現 (`Type`)

Reml における「型」は、`Type` 列挙型で表現されます。

```rust
// compiler/frontend/src/typeck/types.rs

pub enum Type {
    Var(TypeVariable),    // 型変数 (推論中の未確定な型)
    Builtin(BuiltinType), // 組み込み型 (Int, Bool, Str 等)
    Arrow {               // 関数型 (引数 -> 戻り値)
        parameters: Vec<Type>,
        result: Box<Type>,
    },
    App {                 // 型適用 (List<Int> 等)
        constructor: SmolStr,
        arguments: Vec<Type>,
    },
    Slice {               // スライス ([T])
        element: Box<Type>,
    },
    Ref {                 // 参照 (&T, &mut T)
        target: Box<Type>,
        mutable: bool,
    },
}
```

- **`Type::Var(TypeVariable)`**: 推論の過程で「まだ決まっていない型」を表すプレースホルダです。`TypeVariable` は一意な ID (`u32`) を持ちます。推論が進むにつれて、この変数は具体的な型（例: `Int`）に「束縛」されていきます。
- **`Type::App`**: ジェネリクス型やユーザー定義型を表します。例えば `Option<Int>` は、コンストラクタ `"Option"` と引数 `Int` を持つ `App` として表現されます。

### 2.2 制約 (`Constraint`)

型推論エンジンは、コードを解析しながら「制約」を生成します。

```rust
// compiler/frontend/src/typeck/constraint.rs

pub enum Constraint {
    Equal {
        left: Type,
        right: Type,
    },
    HasCapability {
        ty: Type,
        capability: SmolStr,
        context: CapabilityContext,
    },
    ImplBound {
        ty: Type,
        implementation: SmolStr,
    },
}
```

- **`Equal`**: 最も基本的な制約で、「左辺の型と右辺の型は等しくなければならない」ことを示します。例えば、「変数 `x` に `10` を代入する」という式からは、`Equal { left: Type(x), right: Int }` という制約が生まれます。
- **`HasCapability`**: Reml 特有の制約で、ある型（あるいは式）が特定の Capability（権限）を要求していることを示します。これは Effect System の監査に使用されます。

### 2.3 代入表 (`Substitution`)

制約解決の結果は `Substitution`（代入表）に蓄積されます。これは「型変数 `t0` は `Int` である」といったマッピングを保持する構造体です。

```rust
// compiler/frontend/src/typeck/constraint.rs

pub struct Substitution {
    entries: IndexMap<TypeVariable, Type>,
}
```

`ConstraintSolver` はこの代入表を更新しながら単一化（Unification）を進めます。最終的にすべての型変数が具体的な型に解決されるのが理想ですが、ジェネリック関数の場合は型変数が残ることもあります（これは「多相」として扱われます）。

## 3. アルゴリズムと実装 (Core Logic)

型推論の実行フローを追跡します。エントリポイントは `compiler/frontend/src/typeck/driver.rs` にある `TypecheckDriver::infer_module` です。

### 3.1 推論の全体フロー

`infer_module` (および内部で呼ぶ `infer_module_from_ast`) は、以下の手順で処理を進めます。

1. **環境の初期化**:
   - `TypeEnv`（型環境）を作成し、`Int` や `Bool` などの `Prelude`（標準組み込み）型を登録します (`register_prelude_type_decls`)。

2. **宣言の収集 (First Pass)**:
   - モジュール内のすべての型宣言 (`struct`, `enum` など) と関数シグネチャを走査し、環境に登録します。
   - Reml では関数が定義される順序に関係なく呼び出せるよう、関数本体の解析前にすべてのシグネチャを登録する必要があります。これをこのフェーズで行います (`register_function_decls`)。

3. **関数本体の推論 (Second Pass)**:
   - 各関数の本体（式）を走査し、制約を収集・解決します。
   - ここで `infer_expr` や `infer_stmt` といった再帰的な関数が活躍します。

4. **レポートの作成**:
   - 収集された型情報と診断結果を `TypecheckReport` にまとめて返します。

### 3.2 単一化 (Unification) のメカニズム

型推論の核心は `ConstraintSolver::unify` メソッドにあります。

```rust
// compiler/frontend/src/typeck/constraint.rs:134

pub fn unify(&mut self, left: Type, right: Type) -> Result<(), ConstraintSolverError> {
    let left = self.substitution.apply(&left);   // 現在の知識で具体化
    let right = self.substitution.apply(&right);

    match (left, right) {
        (Type::Var(v), ty) => self.bind_variable(v, ty), // 型変数の束縛
        (ty, Type::Var(v)) => self.bind_variable(v, ty),
        (Type::Builtin(l), Type::Builtin(r)) if l == r => Ok(()),
        // ... (構造的な比較: Arrow, App, Tuple など)
        _ => Err(ConstraintSolverError::Mismatch(left, right)),
    }
}
```

- **Occurs Check**: `bind_variable` では「出現チェック（Occurs Check）」が行われます。これは、例えば `t0 = List<t0>` のような再帰的な型定義を防ぐためのものです。もし型変数 `t0` が代入しようとする型 `List<t0>` の中に含まれていれば、無限に展開されてしまうため、エラーとします (`ConstraintSolverError::Occurs`)。

### 3.3 Active Pattern の推論

Reml の特徴的な機能である Active Pattern もこのフェーズで処理されます。Active Pattern は関数のように振る舞いますが、パターンマッチの文脈で使用されます。

`TypecheckDriver` は Active Pattern の定義を解析する際、その「戻り値の型」を厳密にチェックします。

- **Partial Pattern**: `Option<T>` を返す必要があります。失敗する（`None` を返す）可能性があるからです。
- **Total Pattern**: 常に成功する必要があるため、`Option` や `Result` でラップされていない値を返すか、あるいは網羅的な型（単一バリアントの Enum など）を返す必要があります。

このロジックは `infer_module_from_ast` 内のループで `classify_active_pattern_return` を呼び出し、結果を検証することで実装されています。

## 4. エラー処理 (Error Handling)

型推論中に発生した不整合は、即座にコンパイルを停止させるのではなく、可能な限り `violations` リストに蓄積されます。これにより、一度のコンパイルで複数のエラーを報告できます。

### 主な違反の種類 (`TypecheckViolation`)

- **`Mismatch`**: 期待される型と実際の型が異なる場合。
  - 例: `let x: Int = "hello"`
- **`Msg` (Occurs Check)**: 循環的な型定義が検出された場合。
- **`ActivePatternReturnContract`**: Active Pattern が規約に従った型を返していない場合。
- **`IntrinsicInvalidType`**: `@intrinsic` 属性がついた関数が、コンパイラが期待するシグネチャと一致しない場合。

また、そもそも構文解析に失敗して AST が存在しない場合には、`ast_unavailable` という特別な違反を生成し、後続のパイプラインに「型チェックどころではない」ことを伝えます。

## 5. 発展的トピック (Advanced Topics)

### 5.1 Effect System と Capability 監査

Reml の型システムは、単なるデータ型の整合性だけでなく、副作用の権限（Capability）も管理します。

型推論フェーズでは、コード内で使用されている Effect（例: IO 操作、ネットワークアクセス）を検出し、それらが現在の「Stage（ステージ）」で許可されているかを検証します。

- **Stage**: `stable`, `beta`, `experimental` などの段階。例えば、実験的な機能は `experimental` ステージでのみ許可されます。
- **Iterator Stage**: 特殊なケースとして、イテレータの実装（`Array`, `Option`, `Result` など）ごとに異なる安定性要件がある場合があります。`constraint/iterator.rs` の `solve_iterator` は、イテレータの種類に応じて適切な Stage Profile (`IteratorStageProfile`) を割り当て、監査に必要な情報を生成します。

### 5.2 Dual Write と Type Row Mode

`compiler/frontend/src/typeck/env.rs` には `TypeRowMode` という設定があります。これは Reml の Effect System が進化する過程で、新旧の型システム表現を共存させるための仕組みです。

- **`Integrated`**: 標準モード。Effect 情報を型システムの一部として完全に統合して扱います。
- **`DualWrite`**: 移行期間用のモード。新旧両方の形式で情報を出力し、デバッグや検証を支援します。`DualWriteGuards` はこのモード時に、比較用のレポートをファイルシステム（ `reports/dual-write/` 配下）に書き出します。

## 6. 章末まとめ (Checkpoint)

- `TypecheckDriver` は AST を入力として受け取り、型推論を実行して `TypecheckReport` を生成します。
- 型は `Type` 列挙型、制約は `Constraint` 列挙型で表現され、単一化（Unification）アルゴリズムによって型変数が解決されます。
- 型推論は、単なる型の決定だけでなく、Active Pattern の契約検証や、Effect/Capability の監査データの生成も行います。
- エラーは `TypecheckViolation` として集約され、ユーザーフレンドリーな診断メッセージの生成に利用されます。

次章「第8章: 意味解析」では、型情報が確定した AST をもとに、変数のスコープ解決や名前解決の詳細、そしてより高度な意味的検証のプロセスを追っていきます。
