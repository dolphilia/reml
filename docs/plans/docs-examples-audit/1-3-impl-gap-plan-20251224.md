# 1.3 実装ギャップ対応計画（Rust Frontend / 2025-12-24）

`docs/spec/1-3-effects-safety.md` のサンプル修正で判明した **仕様と実装のギャップ** を整理し、Rust Frontend 側で追随するための対応計画を定義する。

## 目的
- 仕様に合わせて Rust Frontend の受理範囲を拡張する。
- フォールバックで回避したサンプルを正準例へ戻す。

## 対象範囲
- 仕様章: `docs/spec/1-3-effects-safety.md`
- サンプル: `examples/docs-examples/spec/1-3-effects-safety/sec_g.reml`
- 監査ログ: `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md`

## ギャップ一覧（簡略化／回避済み）

### 1. `defer` 構文の未対応
- 影響: `defer expr` を含むブロックが `parser.syntax.expected_tokens` で失敗する。
- 該当サンプル:
  - `examples/docs-examples/spec/1-3-effects-safety/sec_g.reml`
- 現状の回避: `defer` をコメント化し、`f.close()` を明示的に呼ぶフォールバックへ置換。

## 実装修正計画（Rust Frontend）

### フェーズ 1: 構文受理（最小限のパース対応）
1) `defer` 文のパーサ追加
- 目的: ブロック内で `defer expr` を受理し、AST に記録できるようにする。
- 成果物: `sec_g.reml` が `--emit-diagnostics` で診断 0 件になる。
- 作業ステップ:
  - ブロック内ステートメントの分岐に `defer` を追加する。
  - `defer` 後に式が欠落した場合の診断（期待トークン）を追加する。
  - `defer` がトップレベルで出現した際の回復診断を定義する。

### フェーズ 2: AST/実行時セマンティクスの整合
1) `defer` の実行保証
- 目的: `return` / `?` / 例外的終了で `defer` が必ず実行されることを保証する。
- 成果物: ブロック終了時の `defer` 実行がテストで確認できる。
- 作業ステップ:
  - ブロック AST に `defer` リストを保持し、コード生成で LIFO で実行する。
  - `return` / `?` での早期脱出経路に `defer` 実行を挿入する。
  - `unsafe` ブロックや `match` 内の `defer` に対する評価順を明確化する。

### フェーズ 3: サンプル復元と再検証
1) サンプルを正準例へ戻す
- `sec_g.reml` の `defer` コメントを正規の `defer` へ戻す。
- `docs/spec/1-3-effects-safety.md` の G 節コードブロックを正準例へ戻す。

2) 再検証
- `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` を実行し、診断 JSON を再生成する。
- `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の `validation` 欄を更新する。

## 進捗管理
- 本計画書作成日: 2025-12-24
- 進捗欄（運用用）:
  - [ ] フェーズ 1 完了
  - [ ] フェーズ 2 完了
  - [ ] フェーズ 3 完了

## 関連リンク
- `docs/spec/1-3-effects-safety.md`
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-plan.md`
- `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md`
