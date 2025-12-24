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

#### 早期脱出の IR 形（panic / propagate）
- **panic を終端命令へ落とす**: `MirExprKind::Panic` は `call @reml_panic`（仮）後に `LlvmTerminator::Unreachable` を置く設計とする。値は返さず、`return` と同列の終端として扱う。`LlvmTerminator` に `Unreachable` を追加し、`panic` の lowering は必ず終端ブロックを生成する。
- **propagate の分岐形**: `Result<T,E>` と `Option<T>` を想定し、`Ok/Err` または `Some/None` で分岐する IR に落とす。
  - `Result`: `intrinsic_is_ctor("Ok")` で成功分岐、成功側は `intrinsic_ctor_payload("Ok")` で値を抽出、失敗側は元の `Result` 値を `ret` で即時返す（`defer_lifo` は先行評価済み）。
  - `Option`: `null` 判定で分岐し、非 `null` 側は `intrinsic_ctor_payload("Some")` で値を抽出、`null` 側は `ret null` で即時返す。
  - どちらも成功側は後続ブロックへ合流させ、必要なら `phi` で値を引き継ぐ（`propagate` が式位置で使われるため）。

#### match 以外の式コンテキストでの `propagate` ブロック化（設計案）
- **対象**: `if/else`、`binary`、`call` 引数、`let` 右辺など、`match` 以外で `propagate` が式として現れるケース。
- **方針**: `emit_value_expr` は「値を返す」関数であり分岐ブロック生成に向かないため、`propagate` を含む式は **外側の lowering でブロックへ持ち上げる**。
  - `if/else`: cond 評価ブロック → then/else 評価ブロック（各ブロックで `propagate` を展開）→ end ブロックに合流し `phi` を構成。
  - `call`/`binary`: 左右（または各引数）を順に評価するブロックを作り、途中で `propagate` が出たら早期 `ret` するエラーブロックへ分岐。
  - `let` 右辺: 右辺評価ブロック内で `propagate` を展開し、成功時のみ束縛を継続、失敗時は `ret` で関数終端。
- **導入方法**: `emit_value_expr` ではなく `emit_value_expr_to_blocks`（新設予定）で、`propagate` を含む式をブロック列に変換する。
  - 最小実装は `if/else` から着手し、`match` と同様の `phi` 合流パターンを流用する。

#### panic 引数型の IR 仕様合わせ（runtime 整合）
- **現状**: runtime 側の `panic(const char*)` は `NULL` 終端文字列を受け取る設計で、`reml_string_t` ではない。
- **方針**:
  - LLVM IR の panic 呼び出しは **`@panic(ptr)` を正準**とする（`@reml_panic` の別名は廃止）。
  - Reml 側の `Str` は `{ptr, i64}` を前提とするため、`panic` の引数は **`Str` → `ptr` への変換**を lowering 時に行う。
  - `Str` 以外の引数は `@reml_value`/`@reml_call` 経由で `Str` に整形する既存規約に合わせる（未整備なら TODO 化）。
- **TODO**:
  - `runtime/native/include/reml_runtime.h` のコメントにある「LLVM IR 側では panic(ptr, i64)」の記述を現在の IR と一致させる。
  - `@panic` の宣言形式（引数数・型）を `compiler/rust/backend/llvm` 側で固定化し、テストで IR 断片を確認する。

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
