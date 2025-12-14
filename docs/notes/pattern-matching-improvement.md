# パターンマッチ機能強化の提案と調査

## 1. 目的

Reml の「DSLファースト」および「安全性・実用性」という目標に基づき、現在のパターンマッチ機能をより強力かつ表現豊かにするための強化案を提案します。
本調査では、Rust, F#, OCaml, Swift, Scala などの先進的な言語機能を比較検討し、Reml に最適な機能セットを導出します。

## 2. Reml の現状 (Current Status)

`docs/spec/1-1-syntax.md` および `examples/spec_core/chapter1/match_expr/` に基づく現状の機能は以下の通りです。

*   **基本構文**: `match expr with | pat -> body`
*   **サポートされるパターン**:
    *   ワイルドカード (`_`)
    *   リテラル (整数, 文字列, 真偽値)
    *   変数束縛
    *   タプル (`(a, b)`)
    *   レコード (`{x, y}`, punning対応)
    *   ADTコンストラクタ (`Some(x)`, `Option.None`)
*   **ガード句**: `when expr` (例: `Some(x) when x > 10 -> ...`)
*   **エイリアス**: `pat as name`

**課題・不足点**:
*   **Or-pattern (論理和)**: 複数のパターンを `|` でまとめる構文が明示されていない（例: `| A | B -> ...`）。
*   **Active Patterns (能動的パターン)**: DSL記述に不可欠な「分解ロジックのカスタマイズ」機能がない。
*   **Slice Patterns**: リストや配列の柔軟なマッチング (`[head, ..tail]`) が不足している。
*   **Range Patterns**: 範囲指定 (`1..10`) がない。

## 3. 他言語の調査 (Survey)

### Rust
*   **特徴**:
    *   **網羅性 (Exhaustiveness)**: コンパイル時に漏れを厳密にチェック。
    *   **Or-patterns**: `| Ok(x) | Err(x) => ...` のようにネスト内でも使用可能。
    *   **Range Patterns**: `1..=5` で範囲マッチ。
    *   **Slice Patterns**: `[first, .., last]` で可変長配列の先頭・末尾をマッチ。
    *   **Binding Guards**: `name @ pat` でパターン全体を変数に束縛しつつ分解。
*   **評価**: 安全性とパフォーマンス重視。Remlの「安全性」目標と合致する。特に Slice Patterns はパーサ記述に有用。

### F# (Active Patterns)
*   **特徴**:
    *   **Active Patterns (`(|...|)`)**: 関数呼び出しをパターンとして扱える機能。
        *   例: `(|Int|_|)` というパーサをパターン内で `match input with | Int n -> ...` のように自然に書ける。
    *   **Partial Active Patterns**: 失敗する可能性のある分解（`Some`/`None` に相当）。
*   **評価**: **Reml にとって最も重要**。DSLファーストアプローチにおいて、複雑な解析ロジック（正規表現マッチ、構文解析、特定構造の抽出）を宣言的なパターン構文に隠蔽できる。

### OCaml / Haskell
*   **特徴**:
    *   **View Patterns (Haskell)**: `pat -> exp` のように関数適用結果に対してマッチ。
    *   **Polymorphic Variants (OCaml)**: 事前定義なしでタグを使える。
    *   **Guards**: `when` (OCaml) / `|` guards (Haskell)。
*   **評価**: OCaml の `as` 構文や `when` 構文は既に Reml に採用されている。View Patterns は強力だが、F# の Active Patterns の方が視覚的にパターンらしく読みやすい場合が多い。

### Swift
*   **特徴**:
    *   `switch` 文が非常に強力。`where` 句での条件詳細化。
    *   Enum の Associated Values との一体化。
*   **評価**: 構文が自然言語に近く読みやすい。

## 4. Reml への導入提案

Reml の価値観（読みやすさ、DSLファースト、安全性）に基づき、以下の優先度で機能を導入することを提案します。

### 優先度：最高 (Must Have)

1.  **Active Patterns (能動的パターン)**
    *   **理由**: DSL記述能力を飛躍的に向上させるため。パーサコンビネータの結果をパターンとして扱えれば、解析ロジックが極めて宣言的になる。
    *   **提案構文**: F#風の定義 `pattern (|Integer|_|) ...` を導入し、`expr` 内で分解ロジックを利用可能にする。

2.  **Or-patterns (パターン論理和)**
    *   **理由**: コードの重複を防ぎ、可読性を高める。
    *   **提案構文**: `| A(x) | B(x) -> ...` (トップレベルだけでなくネスト内部でも `Some(A | B)` のように許可)

### 優先度：高 (Should Have)

3.  **Slice Patterns (スライスパターン)**
    *   **理由**: 配列やバッファ処理（バイナリ解析DSLなど）で非常に強力。
    *   **提案構文**: `[head, ..tail]` や `[.., last]`。

4.  **Range Patterns (範囲パターン)**
    *   **理由**: 数値範囲のチェック（エラーコード判定など）に頻出。
    *   **提案構文**: `1..10` (境界含むかどうかは `..=` 等で区別検討)。

### 優先度：中 (Nice to Have)

5.  **Binding Operator (`@`)**
    *   **理由**: 現在の `as` (後置) と Rust の `@` (前置) の比較。構造分解しつつ元の値も欲しいケース。
    *   **検討**: Reml は現在 `pat as name` を採用しているため、これを継続推奨とし、必要なら `name @ pat` のエイリアス導入を検討。

6.  **Regular Expression Literals in Patterns**
    *   **理由**: テキスト処理DSL向け。
    *   **提案**: `match text with | r"^\d+" as digits -> ...` のような正規表現リテラルマッチ。Active Patterns の糖衣構文として実装可能。

## 5. 次のアクション

1.  **Or-patterns / Range Patterns の仕様策定**: 既存のパーサ・型システムへの影響が比較的小さいため、早期に仕様化する。
2.  **Active Patterns の設計**: 関数呼び出しセマンティクスとの整合、副作用の扱い、網羅性チェックへの影響を慎重に設計する。
