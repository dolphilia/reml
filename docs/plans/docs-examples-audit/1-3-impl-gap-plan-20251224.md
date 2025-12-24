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
  - ブロック文解析（`Block`/`Stmt` 相当）の分岐に `defer` キーワードを追加する。
  - `defer` の直後に必ず式が続く前提で、`Expr` パーサへ委譲する構文を追加する。
  - `defer` 後に式が欠落した場合、`parser.syntax.expected_tokens` に `Expr` 系の期待を含める。
  - `defer` をトップレベルや宣言位置で検出した場合、`parser.top_level_expr.disallowed` と同等の回復診断を出す。
  - `defer` を `return` / `let` / `var` と同等のステートメント優先順位で解釈する。

### フェーズ 2: AST/実行時セマンティクスの整合
#### 設計方針（MIR/IR 低レイヤ）
- **MIR に `block` と `defer` を保持する**: Typed AST に `Block` / `Stmt` を追加し、`MirExprKind::Block { statements, defers }` を新設する（`defers` は出現順保持）。
- **LIFO 実行は lowering で明示化**: `return` / `?` / `panic` / ループ脱出の各パスに対し、スコープ終端で `defers` を逆順に展開して実行する。
- **互換性**: フロントエンド MIR スキーマに `block` を追加するため、`compiler/rust/backend/llvm` の JSON ローダは未対応期間中 `block` を `unknown` として受理するガードを入れる。
- **スコープ単位**: `if` / `match` / `loop` の各ブロックに `defer` を束縛し、親ブロックへは持ち上げない。

#### 展開ポイント（実行系）
- **採用方針**: `MirExprKind::Block.defer_lifo` を「バックエンド/実行系の lowering」で展開する。
- **理由**: 実行順序の保証は IR 変換時点で明示化するほうが、`return` / `?` / `panic` の早期脱出経路に一貫して挿入しやすい。
- **具体化**: `compiler/rust/backend/llvm` が `block` を受理する段階で、`return`/`break`/`propagate` の直前に `defer_lifo` を展開する（フロントエンド MIR では順序のみ保持）。
- **補足**: 既存の簡易 runtime フェーズは診断生成用途のため、`defer` 実行の責務は持たせない。

#### バックエンド対応タスク（起票）
- **対象**: `compiler/rust/backend/llvm/src/codegen.rs`
- **タスク1**: `MirExprKind::Block` を受理するケースを追加し、`defer_lifo` を順に評価する lowering を実装する。
- **タスク2**: `return` / `propagate(? 相当)` / `panic` の分岐生成直前に `defer_lifo` 展開を挿入する。
- **タスク3**: `block` 未対応時の fallback を `unknown` に落とすガードを削除し、実行パスへ統合する。

#### 早期脱出の挿入位置（設計メモ）
- **return**: return 直前に `defer_lifo` を順に評価してから戻る。
- **?（propagate）**: エラー分岐へ飛ぶ前に `defer_lifo` を評価し、成功パスは従来通り継続。
- **panic**: panic 呼び出し直前に `defer_lifo` を評価し、以降は abort/ unwind の想定に従う。

1) `defer` の実行保証
- 目的: `return` / `?` / 例外的終了で `defer` が必ず実行されることを保証する。
- 成果物: ブロック終了時の `defer` 実行がテストで確認できる。
- 作業ステップ:
  - ブロック AST に `defer` リスト（式の配列）を保持し、構文解析時に順序を保存する。
  - MIR/IR 生成でブロック終了時に `defer` を LIFO で実行するパスを追加する。
  - `return` / `?` / `panic` による早期脱出経路に `defer` 実行を挿入する。
  - `match` / `if` / `loop` の入れ子ブロックに対して、スコープ終端単位で `defer` が閉じることを確認する。
  - `unsafe` ブロック内の `defer` について、外側へエフェクトを持ち上げる既存規則と衝突しないことを確認する。

### フェーズ 3: サンプル復元と再検証
1) サンプルを正準例へ戻す
- `sec_g.reml` の `defer` コメントを正規の `defer` へ戻す。
- `docs/spec/1-3-effects-safety.md` の G 節コードブロックを正準例へ戻す。

2) 再検証
- `cargo build --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend` で再ビルドする。
- `compiler/rust/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` を実行し、診断 JSON を再生成する。
- `reports/spec-audit/ch1/1-3-effects-safety__sec_g-YYYYMMDD-diagnostics.json` を最新に置き換える。
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
