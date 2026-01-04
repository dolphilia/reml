# Core.Parse 強化計画: 追跡と作業分割

## 目的
Core.Parse の強化作業を「追跡可能な成果物」に分割し、`docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の回帰運用へ **安全に接続**するためのルールを定義する。

## 前提
- 仕様の正本は `docs/spec/2-x` であり、本ディレクトリは計画・分割を扱う。
- 既存の Phase 4.1 計画（例: `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md`）と重複しうるが、ここでは **「メモ由来の優先順位」**（Cut/Label/Lex/Zero-copy）を軸に再整理する。

## ワークストリーム一覧
各ストリームは、仕様・サンプル・回帰（シナリオ）・診断キーの 4 点セットで完了を判断する。

### WS1: Cut/Commit（バックトラック制御）
- 計画: `1-0-cut-commit-plan.md`
- 成果物:
  - 仕様: `docs/spec/2-1-parser-type.md`, `docs/spec/2-5-error.md` の整合確認（必要なら追記）
  - サンプル: Cut の有無で診断が改善する入力例
  - 回帰: 失敗位置が「最も近い分岐点」へ固定されること
  - 既存シナリオ: `CH2-PARSE-101`（`core-parse-or-commit-ok`）
  - 追加シナリオ: `CH2-PARSE-102`（`core-parse-cut-branch-mislead` / 演算子右項欠落）
  - 追加シナリオ: `CH2-PARSE-103`（`core-parse-cut-unclosed-paren` / 括弧閉じ忘れ）
  - 比較対象（Cut 無し相当）:
    - `core-parse-cut-branch-mislead-no-cut`（誤誘導版の期待集合を保持）
    - `core-parse-cut-unclosed-paren-no-cut`（括弧ペア未完の巻き戻りを保持）

### WS2: Error Labeling（文脈・期待集合）
- 計画: `1-1-error-labeling-plan.md`
- 成果物:
  - 仕様: `label`/`rule` の期待統合ルール（期待集合と文脈スタック）
  - サンプル: `<?>` 相当のラベル付けで期待が読めること
  - 回帰: 期待集合がトークン列ではなく「概念名」になること

### WS3: Lex Helpers（scannerless ヘルパ）
- 計画: `1-2-lex-helpers-plan.md`
- 成果物:
  - 仕様: `docs/spec/2-3-lexer.md` と API 名の揃え（`lexeme/symbol` 等）
  - サンプル: リテラル/識別子/コメントの最小 DSL
  - 回帰: whitespace/comment を含む入力での安定挙動
  - 既存シナリオ: `CH2-PARSE-901`（autoWhitespace/Layout）, `CH2-PARSE-902`（profile_output）
  - 実装計画: `docs/plans/bootstrap-roadmap/4-1-core-parse-lex-helpers-impl-plan.md`（WS3 Step3 の Phase4 反映）

### WS4: Error Recovery（複数エラー・IDE）
- 計画: `1-3-error-recovery-plan.md`
- 成果物:
  - 仕様: `recover` 系 API と診断蓄積・再開点の契約
  - サンプル: 1 ファイルに複数エラーを仕込んだ例
  - 回帰: 「1 つ目のエラーで停止しない」ことを期待出力で固定
  - 既存シナリオ: `CH2-PARSE-201`（`core-parse-recover-diagnostic`）
  - 運用: `RunConfig.extensions["recover"].mode = "collect"|"off"` により IDE/LSP と Build/CI を切り替えられること（WS4 Step0）
  - 追加シナリオ（計画起点 ID）:
    - `CP-WS4-001`（`core-parse-recover-multiple-errors-semicolon` / 文末 `;` 同期で複数診断を固定 / Phase4: `CH2-PARSE-202`）

### WS5: Input/Zero-copy（入力抽象と性能）
- 計画: `1-4-input-zero-copy-plan.md`
- 成果物:
  - 仕様: `docs/spec/2-1-parser-type.md#B-入力モデル-Input` の前提を満たす実装方針
  - チェックリスト: `docs/plans/bootstrap-roadmap/checklists/core-parse-input-invariants.md`（WS5 Step0、監査/回帰の入口）
  - メトリクス: 10MB 級入力での線形特性（`docs/spec/0-1-project-purpose.md`）
  - 回帰: 期待位置（行/列/Span）が Unicode モデルと一致すること

### WS6: Left Recursion（左再帰対処）
- 計画: `1-5-left-recursion-plan.md`
- 成果物:
  - 仕様: 左再帰が存在する文法の記述ガイド（変換/ビルダー/ガード）
  - サンプル: 式文法の代表例（優先度/結合性）
  - 回帰: 「落ちない」「極端に遅くならない」を最低条件として固定

## 回帰計画への接続ルール
- 追加するシナリオは、まず本ディレクトリ側で ID を付けて整理し、後で `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に転写する。
  - 例: `CP-WS1-001` のように **計画起点 ID** を付け、転写後に `CH2-PARSE-xxx` 等へ割り当てる。
- 診断キーの追加・変更がある場合は、`docs/spec/2-5-error.md` と `docs/spec/3-6-core-diagnostics-audit.md`（監査/診断キー運用）を参照し、キー名と Stage 影響（error/warn/info）を明示する。

## 次のアクション
- まず WS1/WS2（Cut/Label）を先行し、エラー品質の底上げを回帰計画へ反映する。
- WS3/WS4（Lex/Recovery）は DSL 実装コスト削減に直結するため、並行でサンプルと回帰を整備する。
- WS5/WS6（Zero-copy/Left Recursion）は性能・アルゴリズム要素が強いので、測定・段階導入（opt-in）を前提に進める。
