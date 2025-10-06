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

**根本原因**: レコードパターンの末尾フィールドが短縮形（`y`）の場合、閉じ括弧 `}` の後に次のアーム `|` が続くと、パーサが正しく処理できない。

**詳細調査結果**:
- 単一アームでは動作する
- `{ field: pattern }` の明示形式では動作する
- `{ field: pattern, .. }` の rest パターンでも同じ問題が発生

#### 回避策

以下のいずれかの方法で回避可能：

1. 各フィールドを明示的に記述：
   ```reml
   | { x: Some(value) } -> value
   | { x: None } -> 0
   ```

2. 単一アームの match を使用：
   ```reml
   | { x: Some(value), y } -> value + y
   ```

#### 対応計画

**Phase 2 Week 1-2**:
- パーサの文法ルール `record_pattern_entry_list` を調査
- Menhir の conflict resolution を確認
- 修正と回帰テストの追加

**成功基準**:
- 複数アームでのレコードパターン + コンストラクタ + 短縮形が動作
- 既存テストが全て成功

---

### 2. Handler 宣言のパース問題

**分類**: パーサの予期しない動作
**優先度**: 🟠 High
**発見日**: 2025-10-06

#### 問題の詳細

`test_parser.ml` の以下のテストが予期せず成功する：

```ocaml
expect_fail "handler: block body (todo)" handler_src
```

**想定される動作**: パースエラーで失敗すべき
**実際の動作**: パース成功

#### 調査結果

Handler のブロック本体が仕様書で未定義または TODO 状態のため、パーサが不完全な実装で通してしまっている可能性。

#### 対応計画

**Phase 2 Week 3**:
- [1-1-syntax.md](../../../docs/spec/1-1-syntax.md) の Handler 仕様を再確認
- Handler パーサルールの実装状態を調査
- 正しい文法定義に修正

**成功基準**:
- Handler の仕様準拠パース
- テストケースの更新

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

### Handler の TODO テスト

**理由**: 仕様が Phase 2 以降で確定するため、現時点では TODO が適切

---

## 対応状況トラッキング

| ID | 項目 | 優先度 | ステータス | 担当 Phase | 備考 |
|----|------|--------|-----------|-----------|------|
| 1  | レコードパターン複数アーム | 🟠 High | 未対応 | Phase 2 W1-2 | パーサ修正 |
| 2  | Handler パース問題 | 🟠 High | 未対応 | Phase 2 W3 | 仕様確認 |
| 3  | Unicode XID | 🟡 Medium | 未対応 | Phase 2-3 | ライブラリ選定 |
| 4  | AST Printer 改善 | 🟡 Medium | 未対応 | Phase 2 W8 | Pretty Print |
| 5  | 性能測定 | 🟢 Low | 未対応 | Phase 3 | ベンチマーク |
| 6  | エラー回復強化 | 🟢 Low | 未対応 | Phase 3 | 診断改善 |

---

## 更新履歴

- **2025-10-06**: 初版作成（Phase 1 完了時）
  - レコードパターン問題を記録
  - Handler パース問題を記録
  - Unicode XID 未対応を記録

---

**次回更新予定**: Phase 2 Week 4（中間レビュー時）
