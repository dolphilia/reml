# 技術的負債リスト

**最終更新**: 2025-10-06
**Phase**: Phase 1 → Phase 2 移行時

このドキュメントは、Phase 1 で発見された既知の問題と技術的負債を記録し、Phase 2 以降での対応を計画するものです。

## 分類

- 🔴 **Critical**: 即座に対応が必要
- 🟠 **High**: Phase 2 で対応すべき
- 🟡 **Medium**: Phase 2-3 で対応
- 🟢 **Low**: Phase 3 以降で対応可

---

## 🟠 High Priority（Phase 2 で対応）

### 1. レコードパターンの複数アーム制限

**分類**: パーサの制限
**優先度**: 🟠 High
**発見日**: 2025-10-06

#### 問題の詳細

レコードパターンで以下の形式を複数アームで使用すると、パーサが構文エラーを報告する：

```reml
// 失敗するケース
let _ = match record with
| { x: Some(value), y } -> value + y  // 1st arm: OK
| { x: None, y } -> y                  // 2nd arm: パースエラー
```

**根本原因（2025-10-06 更新）**: `parser.mly` の `record_pattern_entry` で先頭フィールドを解析する際に、`pattern -> ident` と `primary_expr -> ident` の縮約が Menhir 上で衝突しており（`parser.conflicts` state 238/239）、裸の識別子パターンを式として確定させてしまう。結果として直後の `,` や `..` を受理できず、構文エラーを報告する。

**詳細調査結果（2025-10-06）**:
- 先頭フィールドが `field: None` や `field: Some` のような裸の識別子パターン（引数なしコンストラクタ/変数）で、直後に短縮形フィールドや `..` rest が続くと単一アームでも失敗する。
- 同じ構造でも先頭フィールドが `field: Some(value)` のように括弧付きコンストラクタであれば成功する。
- 先頭 bare コンストラクタの前に短縮形フィールドを置く、または後続フィールドを `field: pattern` 形式にすると成功する。
- エラーメッセージは常に `構文エラー: 入力を解釈できません` で、診断位置は後続フィールド先頭（例: `3:16`、`3:14`）に固定される。
- 既存のレコードパターン網羅テストでは未捕捉だったため、`compiler/ocaml/tests/test_pattern_matching.ml:333` の `test_record_pattern_limitations` を追加し、成功/失敗の境界条件を固定化した。
- `record_pattern_entry` に先頭フィールド専用の非終端を導入して `pattern -> ident` を分離する案を検証したが、Menhir の state 238/239 の reduce/reduce 衝突は解消されず、依然として `tests/tmp_record_issue.reml` が失敗することを確認した（コード変更は差分影響が大きいためロールバック済み）。
- 既存処理系の調査では、OCaml 本体がレコードパターンを専用規則 `record_pat_field` で解析し、`ラベル -> (型注釈)? -> (= パターン)?` の順で必ず `ラベル` を消費することで衝突を回避している（`/Users/dolphilia/.opam/5.2.1/.opam-switch/sources/ocaml-base-compiler.5.2.1/parsing/parser.mly:3003`）。パターン略記（pun）の場合は `= pattern` を省略しても構文木構築時に `pat_of_label` へ差し替えるため、Menhir 側で裸識別子をパターンとして扱う場面がなくなる。
- Rust (`rustc` パーサ) など LR 系実装では、先頭トークンの分類を lexer 段階で細分化し（例: `IDENT` と `FIELD_IDENT` を分ける）、さらに「パターン文脈」情報を再帰下降パーサに持たせて `record_pat_field` を式コンテキストと切り離している。Reml で同手法を採用する場合、lexer でフィールド名を区別するか、Menhir のパラメータ化非終端で「record-field context」を明示する必要がある。
- 新規アプローチとして、Lexer で先頭大文字の識別子を `UPPER_IDENT` トークンへ分類し、パターン側では `UPPER_IDENT` をゼロ引数コンストラクタとして解釈するように修正した。これにより `{ x: None, y }`・`{ x: None, .. }` など問題だったケースが成功することをパターンテスト・パーサユニットテストの両方で確認した。
- `UPPER_IDENT` 化によって `record_pattern_entry` が式文脈とトークンを共有しなくなったため、Menhir の `state 238/239` で発生していた `pattern` vs `primary_expr` の競合が解消され、既存の期待失敗テストを成功シナリオに更新済み。
- モジュール修飾付き列挙子（例: `Option.None`, `DSL.Node(tag)`）をパターンに記述するケースを追加テストし、複数セグメントの `ident` シーケンスを `PatConstructor` へ写像する規則を導入した。これにより `compiler/ocaml/tests/test_parser.ml:307` で追加したシナリオがパース可能になり、`Option.None` も `{ x: Option.None }` のように扱えることを確認した。
- ゴールデン AST テスト `tests/qualified_patterns.reml` / `tests/golden/qualified_patterns.golden` を追加し、モジュール修飾付き列挙子を含むパターンが AST に正しく反映されることをスナップショットで担保した。
- `primary_expr` は `ident`（`IDENT` と `UPPER_IDENT` を包含）を `Var` として受理する設計であり、Lexer 分割後も式文脈の挙動が変わらないことをドライバ実行で確認済み。Phase 2 で仕様書 §1-1 識別子規則へ反映する。

#### 回避策

以下のいずれかの方法で回避可能：

1. すべてのフィールドを `field: pattern` 形式に揃える：
   ```reml
   | { x: Some(value) } -> value
   | { x: None } -> 0
   ```
2. 短縮形フィールドを先頭に移動してから bare コンストラクタを記述する：
   ```reml
   | { y, x: None } -> y
   ```
3. rest パターンを使用する場合はダミーフィールドを追加して順序を変える、または rest の直前を `field: pattern` にする。

#### 対応計画

**Phase 2 Week 1-2**:
- Lexer を `IDENT` / `UPPER_IDENT` に二分し、パターン側でゼロ引数コンストラクタを正しく構築する変更を実装済み。追加で以下を確認する。
  - モジュールパス付き列挙子（例: `Option.None`）や DSL 固有の識別子が適切に分類されるかの追加テスト。
  - `UPPER_IDENT` を式文脈で `Var` として扱ってよいか（仕様レビュー）を Phase 2 タスクに追加。
- Menhir の conflict resolution を再確認し、残存する shift/reduce 警告がレコードパターンに影響しないことをレポート。
- テストスイート強化（ゴールデン AST / `--emit-ast` 出力の比較）を実施し、今回の修正で新たな回帰がないことを保証する。

**成功基準**:
- 複数アームでのレコードパターン + コンストラクタ + 短縮形が動作（`tests/test_pattern_matching.ml` 追加ケースで検証済み）
- 既存テストが全て成功（`tests/test_parser.exe` / `tests/test_pattern_matching.exe` 実行済み）
- 仕様に基づく追加シナリオ（モジュール修飾列挙子など）のテストが整備されること

---

## 🟡 Medium Priority（Phase 2-3 で対応）

### 3. Unicode XID 識別子の未対応

**分類**: 機能未実装
**優先度**: 🟡 Medium
**発見日**: Phase 1 開始時（計画的延期）

#### 問題の詳細

現在の Lexer は ASCII 識別子のみをサポート：

```ocaml
let identifier = ['a'-'z' 'A'-'Z' '_']['a'-'z' 'A'-'Z' '0'-'9' '_']*
```

仕様書 [1-1-syntax.md](../../../docs/spec/1-1-syntax.md) では Unicode XID（`XID_Start` + `XID_Continue*`）が要求されている。

#### 影響範囲

- 非 ASCII 文字を含む識別子が使用できない
- 例: `変数名`, `変量`, `π`, `α` など

#### 対応計画

**Phase 2 Week 6-7**（余裕があれば）:
- Unicode ライブラリの選定（`uutf`, `uucp` など）
- Lexer の Unicode 対応実装
- Unicode テストケースの追加

**Phase 3**（確実に対応）:
- 本格的な Unicode 対応
- 正規化処理の実装

**成功基準**:
- Unicode 識別子のパース成功
- XID 仕様への準拠

---

### 4. AST Printer の改善

**分類**: 開発者体験
**優先度**: 🟡 Medium
**発見日**: パターンマッチ検証時

#### 問題の詳細

現在の `ast_printer.ml` はフラットな出力で、深いネスト構造が読みにくい。

**改善案**:
- インデント付き Pretty Print
- 色付き出力（オプション）
- JSON/S-expression 形式の出力

#### 対応計画

**Phase 2 Week 8**:
- Pretty Printer の実装
- `--emit-ast --format=json` オプションの追加

---

## 🟡 Medium Priority（Phase 2-3 で対応）

### 7. 型エラー生成順序の問題

**分類**: 型推論エンジンの設計
**優先度**: 🟡 Medium
**発見日**: 2025-10-07（Phase 2 Week 9）

#### 問題の詳細

現在の型推論エンジンは**制約ベースの双方向型推論**を実装していますが、一部のエラーケースで期待されるエラー型ではなく、汎用的な `UnificationFailure` が報告されます。これは、型推論の順序と単一化のタイミングに起因します。

**失敗するテストケース（7件）**:

1. **E7007: BranchTypeMismatch** - if式の分岐型不一致
   - 期待: `BranchTypeMismatch { then_ty: i64, else_ty: String, ... }`
   - 実際: `UnificationFailure (i64, String, ...)`
   - 原因: 253行目の `unify` が汎用的な型不一致エラーを返す

2. **E7005: NotAFunction** (2件) - 非関数型への関数適用
   - 期待: `NotAFunction (i64, ...)`
   - 実際: `UnificationFailure (i64, (i64 -> t0), ...)`
   - 原因: `infer_call` で関数型を期待する際の単一化エラー

3. **E7006: ConditionNotBool** (2件) - 条件式が非Bool型
   - 期待: `ConditionNotBool (i64, ...)`
   - 実際: `UnificationFailure (i64, Bool, ...)`
   - 原因: 241行目の `unify s1 cond_ty ty_bool` が汎用エラーを返す

4. **E7014: NotATuple** - 非タプル型へのタプルパターン
   - 期待: `NotATuple (i64, ...)`
   - 実際: `UnificationFailure (i64, (t0, t1), ...)`
   - 原因: パターン推論での型構築時の単一化エラー

#### 根本原因

現在の実装では、`Constraint.unify` が常に `UnificationFailure` を返します。しかし、**文脈に応じた専用エラー型**を生成するには、呼び出し側で型チェックの意図を認識し、適切なエラーを構築する必要があります。

例:
```ocaml
(* 現在の実装 *)
let* s2 = unify s1 cond_ty ty_bool cond.expr_span in  (* UnificationFailure を返す *)

(* 理想的な実装 *)
match unify s1 cond_ty ty_bool cond.expr_span with
| Ok s2 -> ...
| Error _ when not (is_bool cond_ty) -> Error (ConditionNotBool (cond_ty, cond.expr_span))
| Error e -> Error e
```

#### 影響範囲

- **機能面**: エラーは正しく検出されるが、診断メッセージの品質が低下
- **ユーザー体験**: 「型不一致」という汎用的なメッセージではなく、「条件式はBool型が必要です」のような具体的なメッセージが望ましい
- **テスト**: 24件中7件が失敗（診断品質の検証ができない）

#### 対応計画

**Phase 2 後半（Week 10-12）**:
1. `unify` 呼び出しの文脈を解析し、以下のパターンで専用エラーを生成：
   - `unify expected_ty actual_ty` の直後に型カテゴリをチェック
   - 関数適用コンテキスト → `NotAFunction`
   - 条件式コンテキスト → `ConditionNotBool`
   - 分岐型統一コンテキスト → `BranchTypeMismatch`
   - パターンマッチコンテキスト → `NotATuple`, `NotARecord`

2. ヘルパー関数の導入:
   ```ocaml
   val unify_as_function : substitution -> ty -> span -> (substitution * ty, type_error) result
   val unify_as_bool : substitution -> ty -> span -> (substitution, type_error) result
   val unify_branches : substitution -> ty -> ty -> span -> (substitution, type_error) result
   ```

3. エラー判定ロジックの追加:
   ```ocaml
   let is_function_type = function TArrow _ -> true | _ -> false
   let is_bool_type ty = type_equal ty ty_bool
   let is_tuple_type = function TTuple _ -> true | _ -> false
   ```

**Phase 3**:
- 型推論エンジンの全面的なリファクタリング（必要に応じて）
- より高度な型エラー回復戦略の実装

#### 回避策

現在のテストでは、以下の方針で対処：
- 汎用的な `UnificationFailure` を許容する
- 重要なのは**エラーが検出されること**であり、エラー型の精度は次優先
- `test_type_errors.ml` の該当テストは KNOWN ISSUE としてマーク

**成功基準**:
- 全7件のテストが専用エラー型を報告するように修正
- エラーメッセージが仕様書 2-5 の診断フォーマットに準拠
- 既存の成功テスト（17件）が引き続き成功

#### 診断統合での新たな発見（2025-10-07 Week 9-10）

**背景**: CLI統合と診断出力の改善タスクにて、`Type_error.to_diagnostic` の実装を完了しました。

**判明した事実**:

1. **診断変換は正常に動作**
   - 全15種類の型エラー（E7001-E7015）に対する日本語診断メッセージの生成は成功
   - `to_diagnostic_with_source` により、正確な行列番号を含む診断が生成される
   - FixIt（修正提案）の自動生成も機能している

2. **問題の本質は診断変換ではなく型推論側**
   - `to_diagnostic` は受け取った `type_error` を正しく診断に変換する
   - しかし、型推論エンジンが `UnificationFailure` を返す時点で、文脈情報が失われている
   - 診断層での補正では、失われた文脈を復元できない

3. **具体的な診断出力例**:
   ```bash
   # ConditionNotBool が期待されるケース（実際は UnificationFailure）
   /tmp/diagnostic_test.reml:2:19: エラー[E7001] (型システム): 型が一致しません
   補足: 期待される型: i64
   補足: 実際の型:     Bool
   ```
   - エラーコードが E7001（汎用的な型不一致）になっている
   - 本来は E7006（条件式がBool型でない）が正しい

4. **影響の明確化**:
   - ユーザーは型エラーを**検出できる**（機能は正常）
   - しかし、エラーの**文脈情報**（「条件式として使われている」など）が失われる
   - 診断メッセージの品質が低下し、修正方法が分かりにくくなる

**結論**:
- 診断統合タスクは完了したが、根本的な問題（型推論エンジン側）は残存
- 対応は Phase 2 後半（Week 10-12）で型推論エンジンの修正として実施する必要がある
- 診断システム側の準備は完了しており、型推論エンジンが正しいエラー型を返せば、即座に高品質な診断が出力される

**追加推奨事項**:

1. **文脈依存の unify ヘルパー関数の実装**（優先度: High）
   ```ocaml
   (* Type_inference.ml に追加 *)
   let unify_as_bool s ty span =
     match unify s ty ty_bool span with
     | Ok s' -> Ok s'
     | Error _ -> Error (ConditionNotBool (ty, span))

   let unify_as_function s ty span =
     match ty with
     | TArrow _ -> Ok s
     | _ -> Error (NotAFunction (ty, span))
   ```

2. **型推論の各コンテキストで専用ヘルパーを使用**
   - `infer_if` → `unify_as_bool` を使用
   - `infer_call` → `unify_as_function` を使用
   - 分岐の型統一 → 専用エラー `BranchTypeMismatch` を生成

3. **テスト駆動での修正**
   - 失敗している7件のテストケースを一つずつ修正
   - 各修正が他のテスト（39件）を破壊しないことを確認

**優先順位の再評価**:
- 当初 🟡 Medium Priority としていたが、診断品質への影響が大きいため 🟠 High Priority に引き上げを推奨
- Phase 2 完了前（Week 10-12）に対応することで、ユーザー体験が大幅に向上する

---

## 🟢 Low Priority（Phase 3 以降）

### 5. 性能測定の未実施

**分類**: 計測・最適化
**優先度**: 🟢 Low
**計画**: Phase 3

#### 内容

Phase 1 で以下の性能測定が未実施：

- 10MB ソースファイルの解析時間
- メモリ使用量のプロファイリング
- O(n) 特性の検証

#### 対応計画

**Phase 3**:
- ベンチマークスイートの作成
- 性能測定と最適化
- [0-3-audit-and-metrics.md](../../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md) への記録

---

### 6. エラー回復の強化

**分類**: 診断品質
**優先度**: 🟢 Low
**計画**: Phase 3

#### 改善案

- 期待トークン集合の提示
- より詳細な診断メッセージ
- 複数エラーの同時報告

#### 対応計画

**Phase 3**:
- エラー回復戦略の実装
- 診断メッセージの改善

---

## 除外項目（対応不要）

## 対応状況トラッキング

| ID | 項目 | 優先度 | ステータス | 担当 Phase | 備考 |
|----|------|--------|-----------|-----------|------|
| 1  | レコードパターン複数アーム | 🟠 High | 暫定解消（要レビュー） | Phase 2 W1-2 | Lexer 分割 + テスト強化 |
| 2  | Unicode XID | 🟡 Medium | 未対応 | Phase 2-3 | ライブラリ選定 |
| 3  | AST Printer 改善 | 🟡 Medium | 未対応 | Phase 2 W8 | Pretty Print |
| 4  | 性能測定 | 🟢 Low | 未対応 | Phase 3 | ベンチマーク |
| 5  | エラー回復強化 | 🟢 Low | 未対応 | Phase 3 | 診断改善 |
| 6  | 型エラー生成順序 | 🟠 High | 診断層完了・型推論層未対応 | Phase 2 W10-12 | 診断統合完了（W9-10）、型推論修正が必要 |

---

## ✅ 解決済み項目

- **2025-10-06**: Handler 宣言のパースを仕様準拠に更新し、`tests/test_parser.ml` の TODO ケースを廃止（`compiler/ocaml/src/parser.mly` の `handler_body` を `handler_entry` 列挙へ置換）。

---

## 更新履歴

- **2025-10-06**: 初版作成（Phase 1 完了時）
  - レコードパターン問題を記録
  - Handler パース問題を記録
  - Unicode XID 未対応を記録
- **2025-10-06**: Handler パース問題を解消し、追跡リストから除外
- **2025-10-07**: Phase 2 Week 9 更新
  - 型エラー生成順序の問題を追加（ID: 6）
  - 7件のテスト失敗を分析・文書化
  - Phase 2 後半での対応計画を策定
- **2025-10-07**: Phase 2 Week 9-10 更新（CLI統合と診断出力の改善完了後）
  - 型エラー生成順序（ID: 6）に新たな発見を追記
  - 診断層の実装は完了、問題の本質は型推論層にあることを明確化
  - 優先度を 🟡 Medium → 🟠 High に引き上げを推奨
  - 文脈依存の unify ヘルパー関数の実装案を追加
  - 対応状況トラッキング表を更新（診断層完了・型推論層未対応）

---

**次回更新予定**: Phase 2 Week 10-12（型エラー生成順序の修正時）
