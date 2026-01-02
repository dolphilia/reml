# WS6: 左再帰対処（Left Recursion）計画

## 背景と狙い
式文法など多くの言語文法は左再帰を含むが、一般的な PEG/コンビネーターは左再帰を素直には扱えない。
調査メモ `docs/notes/parser/core-parse-improvement-survey.md` でも「左再帰ガード」の可能性に言及している。

Reml では `chainl1` などの典型回避策に加え、OpBuilder/優先度ビルダー（`docs/spec/2-4-op-builder.md`）との整合も重要となる。

## 目標
- 左再帰を含む文法を「落とし穴を避けて」書くための実用ガイドがある
- 代表的な式文法が、優先度/結合性を含めて安定に実装できる
- 退行（無限再帰/極端な遅さ）を回帰で検出できる

## 方針
- **方針A: ガイドライン重視**（推奨）
  - 左再帰は `chainl1/chainr1` または `expr_builder` 相当へ変換して書く
  - 必要なら「左再帰ガード」を補助として提供（仕様外の補助でも可）
- **方針B: 自動左再帰対応**（保留）
  - Packrat と組み合わせて自動化する案は強力だが、仕様・実装の複雑度が高い

## タスク分割
### Step 0: “左再帰はどう扱うか” を仕様上の言葉で固定する
左再帰はアルゴリズムの話である一方、DSL 作者にとっては「書ける/書けない/書き方がある」が最重要である。
まず仕様・ガイドでの立場を一貫させる。

- 参照
  - `docs/spec/2-6-execution-strategy.md`（Packrat/左再帰の位置づけ）
  - `docs/spec/2-4-op-builder.md`（優先度・結合性を “左再帰なし” で表現する道）
  - `docs/spec/2-2-core-combinator.md`（`chainl1/chainr1` と `expr_builder`）
- 確定したい結論
  - 仕様としては「左再帰をそのまま書くことは想定しない」
  - 実用解として「優先度ビルダー/chain 系への変換」を第一選択にする
  - 左再帰ガードは “補助（安全策）” として扱い、無限ループをユーザーが踏みにくくする

### Step 1: 実用ガイド（Do/Don’t）を作る
左再帰は「回避のレシピ」を提示しないと、学習コストが高くなる。

- Do（推奨）
  - 式: `expr_builder` または `chainl1/chainr1` で書く
  - 優先度・結合性は `docs/spec/2-4-op-builder.md` のモデルに寄せる
  - 分岐確定点（演算子/括弧）で WS1（Cut）を併用し、期待を自然にする
- Don’t（避ける）
  - `expr = expr "+" term | term` のような左再帰をそのまま書く（無限再帰/退行の原因）
  - `many` に空成功パーサを渡す（無限ループ）
- 追記先
  - `docs/spec/2-2-core-combinator.md` または `docs/spec/2-6-execution-strategy.md` に短いガイドを追加する案を作る

### Step 2: 式サンプルを “回帰に耐える形” で確定する
- サンプル
  - `examples/spec_core/chapter2/parser_core/` に、優先度・結合性・括弧・単項/二項を含む最小式 DSL を追加
  - 既存の優先度ビルダー系サンプル（bootstrap-roadmap 側に存在する場合）と重複しないよう、ここでは「左再帰回避の解説」と「失敗時の期待品質（Label/Cut）」に焦点を当てる
- 進捗メモ（2025-xx-xx）
  - 追加済み: `examples/spec_core/chapter2/parser_core/core-parse-left-recursion-avoid.reml`
    - `expr_builder` + `label/cut` で左再帰回避と期待集合の品質を同時に示す構成
    - 入力例: `1 + (2 * )`（演算子直後の失敗品質を想定）
- 品質チェック（WS1/WS2 連動）
  - `+` の直後に式がないケースで、期待が `expression` になる（WS2）
  - 括弧の内側で失敗したときに、別分岐へ逃げない（WS1）

### Step 3: “踏み抜き” の検出を回帰へ入れる（無限再帰・極端な遅さ）
左再帰は「失敗として扱える」こと自体が価値になるため、まずは “安全策” を固定する。

- 回帰
  - 計画起点 ID: `CP-WS6-001`（左再帰を含む定義を禁止/検出できる）
  - 計画起点 ID: `CP-WS6-002`（極端な遅さを profile 指標で検知する）
- 期待出力で固定する方針
  - 初期は診断の **存在** と **位置** を固定し、詳細メッセージは段階導入する
  - 性能面は絶対値ではなく profile 指標（backtracks/memo_entries など）の異常増加を監視する（WS5 と整合）

#### Step 3 補足メモ（ドラフト）

- `CP-WS6-001`
  - 目的: `RunConfig.left_recursion="off"` 時に左再帰（自己呼出）が検出されることを回帰で固定
  - 想定資産:
    - サンプル（追加予定）: `examples/spec_core/chapter2/parser_core/core-parse-left-recursion-direct.reml`
    - 期待出力（追加予定）: `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-direct.expected.md`
  - 期待:
    - 診断キー `E4001`（2-5 §D-6）を含む
    - 位置は左再帰定義の開始位置を指す

- `CP-WS6-002`
  - 目的: 左再帰ガードが有効でも性能退行が見えるケースを profile 指標で検知
  - 想定資産:
    - サンプル（追加予定）: `examples/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.reml`
    - 期待出力（追加予定）: `expected/spec_core/chapter2/parser_core/core-parse-left-recursion-slow.expected.md`
  - 期待:
    - `ParseResult.profile` に `left_recursion_guard_hits` と `memo_entries` が出力される
    - しきい値は Phase4 の計測実績で調整し、初期は「0 ではない」ことのみ固定

### Step 4: 既存計画（bootstrap-roadmap）との住み分けを明文化する
- Phase8（優先度ビルダー）・Phase10（プロファイル）の成果と衝突しないように、本 WS は
  - “書き方ガイド”
  - “回帰（安全策と検知）”
  - “WS1/WS2 と一緒に期待品質を上げる”
 へ焦点を当てることを `docs/plans/core-parse-improvement/2-0-integration-with-regression.md` に追記する（必要なら）

## リスクと緩和
- 自動左再帰対応は実装負債になりやすい  
  → まずはガイドライン + 優先度ビルダー（既存仕様）で実用域へ到達させる
