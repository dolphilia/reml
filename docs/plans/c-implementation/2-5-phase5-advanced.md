# フェーズ 5: 高度な機能と仕様準拠

このフェーズでは、Reml の複雑な機能であるパターンマッチング、Algebraic Effects (代数的効果)、および任意精度演算に取り組みます。

## 5.0 前提と範囲
- **前提**: フェーズ 4 までのコード生成・基本型・AST/型検査の基盤が動作していること。
- **対象**: BigInt / パターンマッチ / 文字列と Unicode / Effect の初期ランタイム。
- **非対象**: すべての最適化・完全な効果システムの最終形・全プラグイン連携。

## 5.1 BigInt 統合
- **ライブラリ**: `libtommath` (必要に応じて `gmp` ラッパー)。
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/3-4-core-numeric-time.md`。
- **タスク**:
  1.  `deps/` へのライブラリ統合。
  2.  `Core.Numeric` プリミティブバインディングの実装（加減乗除・比較・符号）。
  3.  大きな整数リテラルの解析サポート (例: `12345678901234567890`) とリテラル種別の確定。
  4.  `Int` (64-bit) のオーバーフロー時の扱いを明文化（`BigInt` 昇格 / 例外 / 診断）。
  5.  文字列変換・表示・パース（`to_string` / `parse`）の基本 API。
  6.  診断 ID とエラーメッセージの整備（桁あふれ/無効リテラル）。
  7.  テスト: 算術演算、境界値、文字列変換、リテラル解析。

## 5.2 パターンマッチングのコンパイル
- **目標**: 効率的な決定木 (decision tree) 生成。
- **仕様**: `docs/plans/pattern-matching-improvement/`。
- **仕様参照**: `docs/spec/1-5-formal-grammar-bnf.md`（構文）、`docs/spec/1-2-types-Inference.md`（パターンの型制約）。
- **採用方針（決定）**:
  - **アルゴリズム**: パターン行列から決定木を構築する方式（Maranget 系の列選択 + 分割）。
  - **参照計画**: `docs/plans/pattern-matching-improvement/1-2-match-ir-lowering-plan.md` と `docs/plans/pattern-matching-improvement/1-1-pattern-surface-plan.md` を基準とする。
- **アプローチ**:
  - `match` 式を `switch` と `if` チェックの列にコンパイルする。
  - ヒューリスティック: 判別式 (Enums/ADT) を優先し `switch` 化、Range/Slice/Or は行列分割で段階的に展開、ガードは最後に評価。
- **タスク**:
  1.  `DecisionTree` ビルダーの実装。
  2.  網羅性と冗長性のチェック（診断 ID の確定）。
  3.  ガードやネストパターンの優先順位ルールを固定。
  4.  Codegen: `DecisionTree` を LLVM IR (BasicBlocks, Br) に下降させる。
  5.  テスト: Enum/整数/タプル/リテラル/ガードの組み合わせ。

## 5.3 Algebraic Effects (ランタイムサポート)
- **目標**: `perform`, `resume`, `handle` のサポート。
- **仕様参照**: `docs/spec/1-3-effects-safety.md`、`docs/spec/3-8-core-runtime-capability.md`。
- **戦略（決定）**:
  - **CPS 変換 + ステートマシン化**を採用（LLVM の `switch`/`br` でジャンプテーブル生成）。
  - **非ポータブル API (`makecontext`/`swapcontext`) とスタックコピー方式は Phase 5 では採用しない**。
  - State/Reader などの単純な効果は **Typed State Passing** の糖衣として扱い、CPS 変換の同一ランタイム基盤に合流させる。
- **タスク**:
  1.  効果ハンドラの最小ランタイム API 定義（C ABI、リソース解放規約、one-shot 保証）。
  2.  `perform` / `resume` / `handle` の MIR/IR ノード定義と CPS 変換パスの追加。
  3.  生成されるステートマシンのランタイム実行器（trampoline）実装。
  4.  `resume` の再入禁止（one-shot）検査と診断を実装。
  5.  例外/State を代表ケースとして実装し、テストで動作保証。

## 5.4 文字列と Unicode
- **ライブラリ**: `utf8proc` + `libgrapheme`。
- **仕様参照**: `docs/spec/1-4-test-unicode-model.md`、`docs/spec/3-3-core-text-unicode.md`。
- **タスク**:
  1.  `String` を `struct { char* ptr; size_t len; }` (UTF-8) として実装。
  2.  `Core.Text` 関数の実装 (長さ, スライス, 検証) と境界条件の整理。
  3.  正しい「文字」カウントのための書記素クラスタ (Grapheme cluster) イテレーション。
  4.  正規化 (NFC) と無効 UTF-8 の扱いを仕様に合わせて固定。
  5.  テスト: 絵文字/結合文字/幅計算/無効列の診断。

## 5.5 検証と完了条件
- **テスト**: `tests/unit` と `tests/integration` に追加。
- **実行確認**: `examples/spec_core` の文字列/パターン/効果を含む例の実行。
- **診断**: JSON 診断（位置情報・修正案）の出力が整合。

## チェックリスト
- [ ] `BigInt` 演算が動作する。
- [ ] Enum と Integer に対してパターンマッチングがコンパイルされる。
- [ ] 文字列リテラルと基本的な文字列操作が Unicode で正しく動作する。
- [ ] 基本的な Effect Handler が動作する (少なくとも State/Exception に対して)。
- [ ] 主要ケースの診断 ID とエラーメッセージが整備される。
