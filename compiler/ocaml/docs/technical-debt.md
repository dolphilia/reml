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
- パーサの文法ルール `record_pattern_entry_list` を調査
- Menhir の conflict resolution を確認
- 修正と回帰テストの追加
- `pattern` 文法をコンテキスト別に分離する際は、単純な非終端分割では衝突が残るため、(a) `IDENT` を大文字・小文字でトークン分割する、(b) Menhir のパラメータ付き非終端で「パターン文脈」を持ち回る、等の追加ディスアンビギュエーションが必要。

**成功基準**:
- 複数アームでのレコードパターン + コンストラクタ + 短縮形が動作
- 既存テストが全て成功

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
| 1  | レコードパターン複数アーム | 🟠 High | 未対応 | Phase 2 W1-2 | パーサ修正 |
| 2  | Unicode XID | 🟡 Medium | 未対応 | Phase 2-3 | ライブラリ選定 |
| 3  | AST Printer 改善 | 🟡 Medium | 未対応 | Phase 2 W8 | Pretty Print |
| 4  | 性能測定 | 🟢 Low | 未対応 | Phase 3 | ベンチマーク |
| 5  | エラー回復強化 | 🟢 Low | 未対応 | Phase 3 | 診断改善 |

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

---

**次回更新予定**: Phase 2 Week 4（中間レビュー時）
