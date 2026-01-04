# 0.3 Remlコードスタイルガイド

> 目的：Reml のコード例と実装ドキュメントを一貫した表記で記述し、仕様全体の可読性と相互参照性を高める。
> 参照：言語構文は [1.1 構文仕様](1-1-syntax.md)、型・効果は [1.2 型システムと推論](1-2-types-Inference.md) / [1.3 効果システムと安全性](1-3-effects-safety.md) を基準とする。

---

## 1. 適用範囲と原則

- 本ガイドは仕様書上のコード片・サンプル実装・DSL 断片を含む Reml コード全般に適用する。
- **式指向・左から右への逐次評価・`Option`/`Result` ベースの失敗制御**という言語哲学を損なわないこと。【F:1-1-syntax.md†L346-L454】
- 仕様例は読者がそのままコピーして動作確認できることを目指し、暗黙の前提や依存順序を明示する。
- コード例は**日本語コメント**で説明しつつ、識別子や API 名は仕様で定義された英語ベースの命名規約に従う。

## 2. ファイル構造とモジュール記述

- 1 ファイル = 1 モジュールの原則に従い、冒頭に `module` 宣言を配置する。省略する場合も暗黙のモジュール名を意識する。【F:1-1-syntax.md†L61-L99】
- `module` の次に `use` をグルーピングして記述する。
  - ルート参照（`use ::Core.Parse`）、相対参照（`use self.syntax`）、ローカルエイリアス（`use Foo as Bar`）の順に並べ、グループ間は 1 行空ける。
  - ネストした import は 1 行でまとめ、3 階層を超える場合は複数行に分割する。
- 公開 API を定義するファイルでは、**公開宣言 → 型定義 → 関数/実装 → 内部補助関数**の順で並べる。DSL エントリーポイントは `pub` + `@dsl_export` を最上部に置く。【F:1-1-syntax.md†L83-L105】
- サンプルコードも同じ順番を踏襲し、依存関係が前方参照にならないようにする。

## 3. フォーマット規約

### 3.1 文字コードと行長

- 文字コードは UTF-8。視認性を保つため、行長は **100 文字**を目安に折り返す。
- 長いリテラルやフォーマッタ指示子を含む場合は複数行文字列（`"""`）や `r"..."` で表現する。

### 3.2 インデントと空白

- インデントは**半角スペース 2 個**（タブ禁止）。
- ブロックは K&R スタイル：`fn foo() {
  ...
}` のように開き波括弧は同一行に置き、閉じ波括弧は宣言と同じインデントに戻す。
- 1 行の中で複数の式を記述する場合のみ `;` を使い、基本は 1 行 1 文。
- 2 つ以上の論理ブロック（宣言グループ・制御構造）を分ける際は空行 1 行を挿入する。

### 3.3 配列・タプル・レコード

- リテラルは 1 行で収まる場合は `let point = { x: 1, y: 2 }` のように記述し、複数行では末尾カンマを付けて揃える。

  ```reml
  let matrix = [
    [1, 0, 0],
    [0, 1, 0],
    [0, 0, 1],
  ]
  ```

- フィールドアクセスは `value.field` / `tuple.0` を用い、`value . field` のように空白を挿入しない。【F:1-1-syntax.md†L505-L514】

### 3.4 パイプとチェーン

- `|>` は 1 段につき 1 行を基本とし、長いチェーンでは次行をインデントして視線を揃える。

  ```reml
  fn pipeline_example(source) {
    source
      |> tokenize()
      |> parse(rule = grammar.entry)
      |> map_errors(normalize)
  }
  ```

- `_` 占位による挿入位置指定は必要箇所のみ。複数箇所に同じ値を渡す場合はラムダを使う方が読みやすい。【F:1-1-syntax.md†L494-L499】

### 3.5 制御構文

- `if`/`else` は 1 行で書ける短文でもブロックを許容するが、ネストが深くなる場合は `match` を優先する。

  ```reml
  fn choose_value(cond: bool, value: Option<i64>) -> i64 {
    let base = if cond then 1 else 0

    match value with
    | Some(v) -> v + base
    | None    -> base
  }
  ```

- `match` アームは `|` を行頭に揃え、`->` の右側は 1 行で収まらない場合にのみ改行して 2 スペース継続インデントする。

### 3.6 関数シグネチャ

- 引数が 80 文字を超える場合は 1 引数ごとに改行し、括弧直後と閉じ括弧前で改行する。
- デフォルト引数や名前付き引数は `name: Type = default` / `fn call(arg = value)` の順で書き、`=` の前後に空白を入れる。【F:1-1-syntax.md†L353-L358】

## 4. 命名規約

| 区分 | 規約 | 例 |
| --- | --- | --- |
| パッケージ/モジュール | `lowercase.with.dot`。先頭 `::` でルート指定可能。 | `module core.syntax`, `use ::Core.Parse` |
| 公開型・トレイト・コンストラクタ | `PascalCase`。略語は大文字 1 文字に圧縮。 | `type ExprTree`, `trait Show`, `Some(value)` |
| 効果・operation | `PascalCase` で効果タグを表し、操作は動詞で始める。 | `effect Console`, `operation log` |
| 関数・メソッド | 標準ライブラリにならい **`snake_case` を基本**とする。既存 API との互換性が必要な場合のみ `camelCase` を許容する。 | `fn parse_module`, `fn and_then` |
| 変数・フィールド | `snake_case`。パターンマッチの束縛も同様。 | `let total_bytes`, `{ x, y: start_y }` |
| 定数 | `UPPER_SNAKE_CASE`。 | `const MAX_RETRIES = 3` |
| ジェネリック型変数 | `T`, `U`, `E` のような単一大文字。意味を明確化したいときは `Input`, `Error` のような PascalCase。 | `fn map<T, U>` |
| DSL 属性・アノテーション | 既存仕様に合わせて `@lower_snake_case`。 | `@must_use`, `@dsl_export` |

- 仕様書で CamelCase の例が残っている場合は、背景説明を添えつつ現行規約への移行を推奨する（例：外部 API 互換の `andThen`）。
- モジュール名とファイル名は一致させ、複合語は `.` 区切りで階層化する（例：`Core.Parse.Op`）。

## 5. 宣言のスタイル

### 5.1 `let` / `var`

- 不変データは `let` を既定とし、変更が必要な場合のみ `var` を用いる。【F:1-1-syntax.md†L409-L420】
- `var` に対する再代入は `:=` を使い、1 行に複数の再代入を書かない。
- パターン束縛で構造を分解し、未使用の値は `_` で明示する。
- `List.fold` や `Iter.try_fold` を優先し、蓄積目的の `var` を避ける。解析器サンプルのような逐次更新は fold で置き換え、可読性と効果スコープを両立させる。【F:3-1-core-prelude-iteration.md†L160-L214】

### 5.2 関数・無名関数

- 短い関数は式形式で表記し、複数行の場合はブロックを用いる。

  ```reml
  fn add(lhs: i64, rhs: i64) -> i64 = lhs + rhs
  
  fn fact(n: i64) -> i64 {
    if n <= 1 then 1 else n * fact(n - 1)
  }
  ```

- ラムダは `|args| expr` を基本とし、副作用や複数式がある場合のみブロック `{ ... }` にする。【F:1-1-syntax.md†L401-L405】
- 効果タグ（`@pure`, `effect {io}` など）は宣言直前に置き、複数付与するときは 1 行にまとめる。

### 5.3 型・トレイト・実装

- ADT の各バリアントは縦に並べ、コンストラクタ引数を 1 行で記述できない場合は括弧内で改行・継続インデント。
- `trait` 内のメソッド宣言はシグネチャの末尾に `;` を付けず、既定実装を持つ場合はブロックで囲む。
- `impl` ブロックの順番は「関連定数 → コンストラクタ → 公開メソッド → 内部メソッド」。一貫した順序で読者が API を把握しやすくする。

### 5.4 効果宣言とハンドラ

- `effect` 宣言ではタグと目的をコメントで補足し、`operation` 名には動詞を用いる。【F:1-1-syntax.md†L200-L240】
- ハンドラは `handle expr with` ブロック内でアームを `| operation(args) -> ...` の形式で揃える。複雑なハンドラは段階的にモジュールへ分割する。

## 6. 失敗制御と安全性

- エラー処理は `Result` / `Option` を基本とし、`?` で早期伝播するスタイルを徹底する。【F:3-1-core-prelude-iteration.md†L25-L70】
- `panic` はデバッグ用途に限定し、例外的な解説には `effect {debug}` の意図を明示する。
- `defer` を使う場合はリソース獲得と解放を対で記述し、複数の `defer` が並ぶときは後入れ先出し順をコメントで残す。【F:1-1-syntax.md†L409-L460】
- `unsafe` ブロックにはリスクの理由と境界条件をコメントとして必ず記載する。【F:1-1-syntax.md†L422-L432】

## 7. コメントとドキュメンテーション

- 項目レベルの説明には `///`、ファイル頭の概要は `//!` を使用する。本文中の補足や TODO は `//` で記述する。
- コメントは原則日本語で書き、仕様へのリンクや関連セクションを `[参照] (target.md)` のような Markdown リンク形式で添える（実際の記述では `]` と `(` を隣接させてリンク化する）。
- サンプルコードに出力例が必要な場合はコードブロック直後に `// => ...` 形式で結果を示す。
- DSL 属性や複雑な型推論の前提がある場合は、前行に 1 行コメントで意図を説明する。

## 8. サンプルの品質基準

- 例として示すコードは **実際にコンパイル可能**であることを前提とし、未定義 API を用いる場合は仮定である旨をコメントで明記する。
- 効果タグ・型パラメータ・`use` が必要になるサンプルは省略せず、読者が自力で復元する必要がないようにする。
- 複数のスタイル候補を比較する際は「推奨」「非推奨」を明確にラベル付けする。
- 既存仕様と競合する新しいスタイルを提示する場合は、本ファイルへの追記とともに関連ドキュメントの更新を行う。
- Chapter 1 のコード片は `examples/docs-examples/spec/1-1-syntax/*.reml` へ配置し、`cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --emit-diagnostics <sample> --emit-typeck-debug reports/spec-audit/ch1/<sample>-YYYYMMDD-typeck.json --emit-impl-registry tmp/<sample>-impls.json` を基本手順とする。出力された `typeck` JSON には `schema_version = "3.0.0-alpha"`、`stage_trace`、`used_impls` が含まれていることを確認し、Streaming 経路の検証では `cargo test --manifest-path compiler/frontend/Cargo.toml streaming_metrics -- --nocapture` を併用して `reports/spec-audit/ch1/streaming_metrics-YYYYMMDD-log.md`／`reports/spec-audit/summary.md` にコマンドと結果を記録する。
- Rust Frontend が受理できなかった期間のフォールバック (`*_rustcap.reml`) は履歴として `examples/` に残すが、監査ベースラインや CLI/Streaming チェックリストには正準サンプルのみを使う。`use_nested.reml` / `effect_handler.reml` は 2025-11-21 時点で Streaming Runner を含むすべての経路で診断 0 件となり、`reports/spec-audit/ch1/streaming_use_nested-YYYYMMDD-diagnostics.json` / `streaming_effect_handler-YYYYMMDD-diagnostics.json` と `reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` / `effect_handler-YYYYMMDD-trace.md` を組み合わせて証跡を残す。必要に応じて `reports/spec-audit/ch2/streaming/` にも同名ログを複製し、`docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表` の `ERR-001` 行と紐付ける。
- Trace Coverage は `syntax:expr::<kind>` / `syntax:effect::<kind>` / `syntax:handler::<name>` / `syntax:operation::resume` で揃え、`scripts/poc_dualwrite_compare.sh effect_handler --trace` の出力を `reports/spec-audit/ch1/trace-coverage-YYYYMMDD.md` にまとめて管理する。`FrontendDiagnostic.extensions.trace_ids` に保存された ID と `reports/spec-audit/ch1/*-trace.md` の内容が一致しているかを確認し、`Trace coverage >= 4` を満たしたら `docs/notes/process/spec-integrity-audit-checklist.md` の `SYNTAX-003` を更新する。
- module_parser の後方互換テストは `cargo test --manifest-path compiler/frontend/Cargo.toml parser::module -- --nocapture` を正規手順とし、`reports/spec-audit/ch1/module_parser-YYYYMMDD-parser-tests.md` にログと `CI_RUN_ID`、`git rev-parse HEAD` を必ず追記する。dual-write 比較は `module_parser-YYYYMMDD-dualwrite.md` へ保存し、`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` の Rust Frontend パーサ拡張ステップへリンクさせる。

---

## 付録：フォーマットチェックリスト

1. `module` と `use` の順序は正しいか。
2. インデントが 2 スペースで統一されているか。
3. `snake_case` / `PascalCase` の命名規則に従っているか。
4. パイプチェーンが読みやすく改行されているか。
5. `Result` / `Option` を使った失敗制御が `?` と併用されているか。
6. コメントが日本語で記述され、前提や非推奨項目が明示されているか。
