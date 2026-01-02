# フェーズ 5: 高度な機能と仕様準拠

このフェーズでは、Reml の複雑な機能であるパターンマッチング、Algebraic Effects (代数的効果)、および任意精度演算に取り組みます。

## 5.0 前提と範囲
- **前提**: フェーズ 4 までのコード生成・基本型・AST/型検査の基盤が動作していること。
- **対象**: BigInt / パターンマッチ / 文字列と Unicode / ADT・レコード / 参照型 / トレイト・型クラス / 効果行 / 型推論の拡張 / Effect の初期ランタイム。
- **成果物**: 仕様準拠の型・効果・文字列処理と、主要な高機能構文が C 実装で動作すること。
- **非対象**: すべての最適化・完全な効果システムの最終形・全プラグイン連携・GC。

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

## 5.5 ADT とレコード型
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/1-5-formal-grammar-bnf.md`。
- **タスク**:
  1.  AST に ADT/コンストラクタ/レコード型とリテラルを追加。
  2.  パーサーで `type` 宣言、コンストラクタ呼び出し、レコードリテラル/更新を解析。
  3.  型チェック: 型引数、フィールド集合の一致、コンストラクタの引数型検査。
  4.  レイアウト: レコードのフィールド順序を仕様（正規化順）に固定。
  5.  Codegen: ADT タグ/ペイロードの表現とアクセスを実装。
  6.  パターンマッチングと連携する診断（不足/余剰フィールド、未知コンストラクタ）。
  7.  テスト: `Option`/`Result`、レコードの構築・参照・更新。

## 5.6 参照型 (`&T`, `&mut T`)
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/1-3-effects-safety.md`。
- **タスク**:
  1.  AST と型表現に参照型を追加し、`&`/`&mut` の構文を解析。
  2.  型チェックで不変/可変参照の整合性と再代入の制約を検証。
  3.  `mut` 効果との整合（可変参照の導入時に効果タグを付与）。
  4.  Codegen: 参照をポインタとして表現し、読み書きの命令列を定義。
  5.  テスト: `&T` の読み出し、`&mut T` の更新、参照の別名衝突診断。

## 5.7 トレイト/型クラス（演算子解決の一般化）
- **仕様参照**: `docs/spec/1-2-types-Inference.md`。
- **タスク**:
  1.  組み込みトレイト (`Add`, `Sub`, `Eq` など) の定義を型システムに統合。
  2.  演算子をトレイト解決にマッピングし、型推論と連携させる。
  3.  MVP の範囲で対象型の `impl` を固定テーブル化。
  4.  失敗時の診断（未解決/曖昧/重複）を整備。
  5.  テスト: `Int`/`Float`/`String` の演算子解決。

## 5.8 効果行 (`! Σ`)
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/1-3-effects-safety.md`。
- **タスク**:
  1.  関数型に効果行を保持する型表現を追加。
  2.  型推論で効果集合の合成と制約伝搬を実装。
  3.  `@pure` / `@no_panic` 等の属性と効果行の整合チェック。
  4.  診断: 効果契約違反、効果不一致のコードを確定。
  5.  テスト: 効果注釈付き関数と伝搬の挙動。

## 5.9 型推論の拡張
- **仕様参照**: `docs/spec/1-2-types-Inference.md`。
- **タスク**:
  1.  レコード/ADT/参照型を含む単一化と推論ルールの拡張。
  2.  トレイト制約を統合し、制約解決の失敗時に明確な診断を出す。
  3.  数値リテラルの既定解決と `BigInt` への昇格ルールを統一。
  4.  効果行と値制限の統合（効果がある `let` を単相化）。
  5.  テスト: 型注釈なしの推論、曖昧性診断、レコード/ADT の推論。

## 5.10 検証と完了条件
- **テスト**: `tests/unit` と `tests/integration` に追加。
- **実行確認**: `examples/spec_core` の文字列/パターン/効果を含む例の実行。
- **診断**: JSON 診断（位置情報・修正案）の出力が整合。

## チェックリスト
- [ ] `BigInt` 演算が動作する。
- [ ] Enum と Integer に対してパターンマッチングがコンパイルされる。
- [ ] 文字列リテラルと基本的な文字列操作が Unicode で正しく動作する。
- [ ] 基本的な Effect Handler が動作する (少なくとも State/Exception に対して)。
- [ ] ADT/レコード型がパース・型検査・コード生成まで通る。
- [ ] 参照型 (`&T`, `&mut T`) の型規則とコード生成が動作する。
- [ ] 組み込みトレイト解決が演算子に適用される。
- [ ] 効果行 (`! Σ`) が型推論と診断に反映される。
- [ ] 型推論がレコード/ADT/参照型/BigIntを含む式で成立する。
- [ ] 主要ケースの診断 ID とエラーメッセージが整備される。
