# Phase4: DSL Test Kit 計画（Core.Test.Dsl）

## 背景と決定事項
- `docs/notes/dsl/dsl-enhancement-proposal.md` の提案「3.1 DSL Test Kit (`Core.Test.Dsl`)」を Phase 4 の実装・回帰計画へ落とし込む。
- `Core.Test` の現行 API では DSL の AST 構造検証やエラー位置検証が冗長で、試作速度と回帰の再現性に課題がある。
- `docs/spec/0-1-project-purpose.md` の安全性・診断明瞭性の原則を維持しつつ、テスト記述量を削減する。

## 目的
1. DSL パーサー向けのテスト DSL (`Core.Test.Dsl`) の仕様を定義し、`Core.Test` に統合する。
2. AST Matcher / Error Expectation / Golden File の 3 軸で、回帰と実用テストの記述を簡潔化する。
3. Phase 4 のシナリオマトリクスと回帰ログに DSL テストの標準パターンを登録し、KPI 計測に接続する。

## スコープ
- **含む**: `Core.Test.Dsl` の構文設計、診断コード連携、ゴールデンファイル運用フロー、サンプル/期待出力の追加。
- **含まない**: LSP/Visualizer 連携、プラグイン配布、実行パイプライン自動化（`4-2-practical-execution-pipeline-plan.md` に委譲）。

## 成果物
- `docs/spec/3-11-core-test.md` への DSL Test Kit 仕様追記（構文・Matcher・Error 期待値）。
- `docs/guides/tooling/testing.md` への運用ガイド追記（ゴールデンファイル、更新手順、診断の安定化）。
- `examples/` と `expected/` の DSL テストサンプル、および Phase 4 シナリオ登録。

## 仕様ドラフト（最小構成）

### テスト DSL の最小構文
```reml
use Core.Test.Dsl

test_parser(my_parser) {
  case "1 + 2" => Add(Int(1), Int(2))
  case "1 + " => Error(code="parser.unexpected_eof", at=4)
  case "fn main() {}" => Func(name="main", ...)
}
```

### 対応する Matcher 仕様（案）
- `...` による構造的部分一致（未指定フィールドは無視）。
- `List`/`Record` での順序一致・キー一致の明記。
- `Option`/`Result` は `Some(...)` / `Ok(...)` を明示する簡略記法を提供。

### エラー期待値の標準キー
- `code`: 診断コード（`docs/spec/2-5-error.md` と整合）
- `at`: 文字位置または `line:col` 指定
- `message`: 部分一致（全文一致ではない）

## 作業ステップ

### フェーズA: 仕様整理
1. [x] `Core.Test.Dsl` の構文定義（`test_parser`/`case`/`Error`/`...`）を `docs/spec/3-11-core-test.md` に追記する。
2. [x] 既存の `Core.Test` API と整合する型シグネチャ（`Parser<T>` の戻り値/診断）を整理する。
3. [x] `docs/spec/2-5-error.md` と診断コードの粒度・命名規則をクロスチェックし、Test DSL で参照可能な最小セットを明記する。

### フェーズB: ゴールデンファイル運用
1. [x] `golden_case` の入出力命名規則（`*.input`/`*.ast`/`*.error`）を定義する。
2. [x] `Core.Test` のスナップショット更新ポリシーと統合し、差分更新の手順を `docs/guides/tooling/testing.md` に記載する。
3. [x] `AuditEnvelope` に記録するイベント名とキー（例: `snapshot.updated`）を整理する。

### フェーズC: サンプルと回帰接続
1. [x] `examples/practical/` に DSL テストサンプルを追加し、`expected/` に期待出力を固定する。
2. [x] `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に新規シナリオを登録する。
3. [x] `reports/spec-audit/ch5/logs/` へのログ保存テンプレートを用意する。
4. [x] `CH3-TEST-410/411/412` を `ok` に更新し、`reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md` に実行ログを記録する。
   - **補足**: `test_parser { case ... }` の糖衣構文で `CH3-TEST-410/411` を更新し、診断 0 件で確認した。

### フェーズD: Rust 実装追加
1. [x] `compiler/runtime/src/test/` に `Core.Test.Dsl` のエントリポイント（`test_parser`/`case`/`Error`）を追加する。
2. [x] AST Matcher と Error Expectation の最小ロジックを実装し、`TestError`/`Diagnostic` への橋渡しを統一する。
3. [x] ゴールデンファイル読み込み/比較の経路を `assert_snapshot` と揃え、`snapshot.updated` の監査イベントを記録する。
4. [x] CLI で DSL サンプルを実行し、`reports/spec-audit/ch5/logs/stdlib-test-dsl-template.md` に実行ログを追記する。

#### フェーズD 残タスク（優先順）
1. [x] ゴールデンファイル経路の実装
   - `examples/**/golden/{case_id}.input` と `expected/**/golden/{case_id}.ast|error` を読み込み、`GoldenCase` から `assert_snapshot_with` 経由で比較する。
   - `snapshot.name` と `case_id` の整合を必須とし、`snapshot.updated` に `snapshot.mode` / `snapshot.bytes` を記録する。
2. [x] Error Matcher の拡張
   - `Diagnostic.codes` 相当のエイリアス一致を追加する。
   - `AtSpec::LineCol` の一致判定を有効化し、`parser.position` の line/column と整合する。
   - エラー複数件時の優先順（最遠/先頭）と期待値無し時の許容規則を実装に合わせて明文化する。
3. [x] AST Matcher の部分一致
   - `...` の部分一致、`List`/`Record` の順序/キー一致ルールを実装する。
4. [x] DSL 糖衣構文の復帰
   - `test_parser { case ... }` の構文を Rust 側で受理し、`DslCase` へ展開する。

## Rust 実装の現状と API 追加案

### 既存実装の範囲（`compiler/runtime/src/test/mod.rs`）
- `assert_eq`/`assert_snapshot`/`test`/`table_test`/`fuzz_bytes` の最小 API を提供。
- スナップショットはプロセス内メモリ保持で、`snapshot.updated` の監査イベントを記録。
- `test.failed` 診断の生成と `AuditEnvelope` への橋渡しを実装済み。

### Core.Test.Dsl の追加 API 案（Rust 側）
- `test_parser(parser, cases) -> TestResult` の導入（`test_with` と同等の診断収集経路を使用）。
- `DslCase` 型（`source: Text` + `expectation` を保持）と `DslExpectation` enum。
- `DslExpectation::Ast(AstMatcher)` / `DslExpectation::Error(ErrorExpectation)` / `DslExpectation::Golden(GoldenCase)` を用意。
- `AstMatcher` の最小比較器（`...` による部分一致、Record/List の順序一致）を提供。
- `ErrorExpectation` の最小比較器（`code`/`at`/`message`）を提供。
- `GoldenCase` で `assert_snapshot_with` を流用し、`snapshot.name` と `scenario_id` を一致させる。

### モジュール分割案（`compiler/runtime/src/test/`）
- `mod.rs`: 既存の `Core.Test` API と共通型を保持し、`dsl` サブモジュールを公開する。
- `dsl/mod.rs`: `test_parser`/`DslCase`/`DslExpectation` のエントリポイントを提供する。
- `dsl/matcher.rs`: `AstMatcher` と `ErrorExpectation` の最小比較器を実装する。
- `dsl/golden.rs`: `GoldenCase` とスナップショット連携（`assert_snapshot_with` 経由）を集約する。
- `dsl/render.rs`: AST レンダリングの補助（`Str` へ正規化する関数群）を置く。

## 依存関係
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md` の Core.Test 実装ロードマップに依存。
- `docs/plans/bootstrap-roadmap/4-1-core-parse-combinator-plan-v2.md` の `Parser<T>` 仕様と整合させる。

## リスクと緩和策
| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| Matcher 記法が複雑化する | 学習コスト増大 | 最小記法に限定し、拡張は Phase 5 へ移管 |
| 診断位置の揺れ | 回帰が不安定 | `at` の解釈ルールを `Span` 起点で統一し、テスト入力を短く保つ |
| ゴールデン差分の増加 | レビュー負担増 | 更新手順と差分レビュー基準を `docs/guides/tooling/testing.md` に明記 |

## 参照
- `docs/notes/dsl/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/2-5-error.md`
- `docs/spec/3-11-core-test.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/guides/tooling/testing.md`
- `docs/plans/bootstrap-roadmap/4-1-stdlib-improvement-implementation-plan.md`
