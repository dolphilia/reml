# Core.Parse Improvement Survey

Ref: `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md`

## 1. Overview
このドキュメントは、Reml の `Core.Parse` をより実用的かつ開発者体験の高いものにするために、既存の優れたパーサーコンビネーターライブラリ（Haskell, Rust, OCaml, Scala）を調査・比較し、導入すべき機能や設計思想をまとめたものです。

## 2. Survey of Existing Libraries

### 2.1 Haskell: Parsec / Megaparsec
パーサーコンビネーターの「標準」とも言える存在。

*   **Parsec**:
    *   **Monadic Interface**: モナドを利用した直感的な記述（`do` 記法）。
    *   **`try` (Backtracking)**: デフォルトでは入力を消費するとバックトラックしない（LL(1)的）。`try` コンビネーターで明示的にバックトラックを許可する設計。これによりエラー位置の特定精度が高い。
    *   **User State**: パーサー内でユーザー定義の状態を伝播させる機能が標準化されている。
*   **Megaparsec**:
    *   Parsec の現代的改良版。
    *   **Error Reporting**: 非常に強力なエラー報告機能。「期待されるトークン」「予期しないトークン」だけでなく、カスタムヒントを表示可能。
    *   **Type Safety**: 入力ストリーム型とエラー型を型パラメータで柔軟に指定可能。

**Remlへの示唆**:
*   `<?>` (label) コンビネーターによる、文脈に応じた分かりやすいエラーメッセージ（"expected expression" など）。
*   `try` (attempt) と `commit` の使い分けによる、正確なエラー位置報告。

### 2.2 Rust: Nom / Chumsky
システムプログラミング言語らしく、性能とゼロコピー、そして最近はエラー回復に注力している。

*   **Nom**:
    *   **Zero-copy**: バイト列（`&[u8]`）や文字列スライス（`&str`）を直接扱い、コピーを回避する設計。
    *   **Bit-level parsing**: バイナリフォーマットの解析に非常に強い。
*   **Chumsky**:
    *   **Error Recovery**: 強力なエラー回復機能。パースエラーが発生しても処理を続行し、複数のエラーを報告する（`recover_with`）。IDE の解析エンジン向けに設計されている。
    *   **Token-based**: 入力をトークン列として扱うことを主眼に置いているが、文字ストリームも扱える。
    *   **Separation**: パーサーの「定義」と「実行」を分離している（コンビネーターを構築する段階と、それを走らせる段階）。

**Remlへの示唆**:
*   **Zero-copy**: `Text` 型のスライスを効率的に扱うことで、アロケーションを削減する（Remlのメモリモデルとの整合性確認が必要）。
*   **Error Recovery**: `recover` 戦略の拡充。DSL開発では「どこで間違ったか」だけでなく「間違いを無視して続きを解析する」機能が重要。

### 2.3 OCaml: Angstrom
ネットワークプロトコル向けの高性能パーサー。

*   **CPS (Continuation Passing Style)**: 継続渡しスタイルを採用し、非同期I/Oと相性が良い（データが届いた分だけパースし、中断・再開が可能）。
*   **Backtracking**: メモ化よりも効率的なバックトラック制御。

**Remlへの示唆**:
*   非同期データストリームの扱いは `docs/spec/2-7-core-parse-streaming.md` ですでに考慮されているが、Angstrom の中断・再開モデルは参考になる。

### 2.4 Scala: FastParse
Scala の柔軟な文法を活用し、普通のコードのように書けるパーサー。

*   **Direct Style**: モナド結合子 (`>>=`) ではなく、通常のメソッド呼び出しのように書ける文法（内部でマクロ等を使用）。
*   **Cut (`~/`)**: Parsec の `commit` に相当。バックトラックを禁止する「カット」演算子が非常に頻繁に使われ、これが高速化と正確なエラー報告の鍵となっている。

**Remlへの示唆**:
*   `Cut` の概念は重要。Reml では `commit` などの名前で、確信した分岐点でのバックトラック禁止を簡単に書けるようにすべき。

## 3. Analysis & Recommendations for Reml

Reml の `Core.Parse` は現在、基本的なコンビネーターは揃っているものの、DSL 開発の実用性（特にエラー報告と書きやすさ）において強化の余地があります。

### 3.1 Adopt "Cut" Semantics aggressively
エラーメッセージの品質向上とパフォーマンスのために、バックトラックを制御する機構（Cut）を導入・推奨すべきです。
*   **現状**: `attempt` でバックトラックを許可するが、逆の「ここで確定（以降バックトラック禁止）」を明示する手段が弱い、あるいは慣習化されていない。
*   **提案**: `commit` や `cut` といったコンビネーターを導入し、「ここまで読めたら、このルールであることは確定なので、失敗したら親ルールに戻らず即座にエラーにする」という記述を容易にする。これにより、「`if` の条件式で失敗したのに、`if` 全体が失敗扱いになって別の文として解析しようとして変なエラーが出る」といった事態を防げる。

### 3.2 Enhanced Error Labeling
ユーザーがパーサーの各部分に名前を付けられるようにする。
*   **機能**: `label(parser, "expression")` や演算子 `<?>` のようなもの。
*   **効果**: エラー時に "unexpected token '(', expected expression" のように、文脈に沿ったメッセージを出せる。現在は低レベルなトークン列のエラーになりがち。

### 3.3 Lexing Helpers (Token Parsers)
完全な「Lexer」と「Parser」の分離（トークンストリーム化）を強制するのではなく、パーサーコンビネーターの中で字句解析的な処理を簡単に行えるヘルパー群を充実させる（Scannerlessに近いアプローチ）。
*   **必要なもの**:
    *   `symbol(str)`: 空白スキップ込みの文字列一致。
    *   `lexeme(p)`: パーサー `p` の後に空白をスキップするラッパー。
    *   `integer`, `float`: 数値解析。
    *   `stringLiteral`: エスケープシーケンス対応の文字列解析。
*   現状の `examples/language-impl-comparison/reml/basic_interpreter_combinator.reml` でも自前定義しているが、これを `Core.Parse.Lex` などの標準モジュールとして提供・強化する。Phase 9 の `autoWhitespace` はこの方向性と合致する。

### 3.4 Input Abstraction & Zero-copy
*   **現状**: `Text` を入力としているが、部分文字列の生成（`substring`）コストが懸念される。
*   **提案**: `Input` 型を、`Text` そのものではなく、`Text` への参照とオフセット・長さを持つ「スライス」として扱う、あるいはイテレータとして扱うことで、文字列コピーを発生させない設計にする。これはRustのNomなどの設計に近い。

### 3.5 Left Recursion Handling
*   多くのプログラミング言語の文法（式など）は左再帰を含む。PEG/コンビネーターは通常左再帰を扱えない。
*   `chainl1` などで回避は可能だが、「左再帰ガード（Left Recursion Guard）」のような仕組みがあれば、より直感的に文法を記述できる可能性がある（Phase 7/10 で言及あり）。

## 4. Proposed Features for Phase 4.1 Plan

上記調査に基づき、`4-1-core-parse-combinator-plan` に以下の要素を強調/追加することを推奨します。

1.  **`Core.Parse.Cut` / `commit`**: エラー品質向上のためのバックトラック制御。
2.  **`Core.Parse.Label`**: 人間可読なエラーメッセージのための注釈機能。
3.  **`Core.Parse.Lex` Enhancement**: 一般的なプログラミング言語のトークン（識別子、リテラル、コメント）を処理する堅牢なプリセット。
4.  **Zero-copy Strategy**: `Text` をコピーせずパースするための入力モデルの最適化（内部実装レベルの変更含む）。

## 5. 次のアクション（計画化）
本メモを元にした具体タスクは、既存 Phase 4.1 計画と混在しないよう `docs/plans/core-parse-improvement/` にドラフトとして分割しました。

- 計画群: `docs/plans/core-parse-improvement/README.md`
