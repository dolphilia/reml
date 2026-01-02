# フェーズ 5: 高度な機能と仕様準拠

このフェーズでは、Reml の複雑な機能であるパターンマッチング、Algebraic Effects (代数的効果)、および任意精度演算に取り組みます。

## 5.1 BigInt 統合
- **ライブラリ**: `libtommath` (必要に応じて `gmp` ラッパー)。
- **タスク**:
  1.  `deps/` へのライブラリ統合。
  2.  `Core.Numeric` プリミティブバインディングの実装。
  3.  大きな整数リテラルの解析サポート (例: `12345678901234567890`)。
  4.  仕様で要求される場合 (または個別の型として)、標準 `Int` (64-bit) のオーバーフローを `BigInt` に昇格させて処理する。

## 5.2 パターンマッチングのコンパイル
- **目標**: 効率的な決定木 (decision tree) 生成。
- **仕様**: `docs/plans/pattern-matching-improvement/`。
- **アプローチ**:
  - `match` 式を `switch` と `if` チェックの列にコンパイルする。
  - ヒューリスティック: 判別式 (Enums) での switch、ガードのカスケード。
- **タスク**:
  1.  `DecisionTree` ビルダーの実装。
  2.  網羅性と冗長性のチェック。
  3.  Codegen: `DecisionTree` を LLVM IR (BasicBlocks, Br) に下降させる。

## 5.3 Algebraic Effects (ランタイムサポート)
- **目標**: `perform`, `resume`, `handle` のサポート。
- **戦略**: 
  - 初期段階: 完全な限定継続 (delimited continuations) が C 言語 v1 で複雑すぎる場合、ワンショット継続 (one-shot continuations) または単純なスタックコピー。
  - または: CPS が好ましい場合、ステートマシン (Async/Await スタイル) へのコンパイル。
  - 実装の選択: LLVM で `jumptable` ステートマシンを使用するか、慎重にラップされた `makecontext`/`swapcontext` (非ポータブル) を使用する。
  - *推奨*: 単純な効果 (State, Reader) に対しては **Typed State Passing** スタイルから始め、完全な効果については後で CPS 変換を研究する。

## 5.4 文字列と Unicode
- **ライブラリ**: `utf8proc` + `libgrapheme`。
- **タスク**:
  1.  `String` を `struct { char* ptr; size_t len; }` (UTF-8) として実装。
  2.  `Core.Text` 関数の実装 (長さ, スライス, 検証)。
  3.  正しい「文字」カウントのための書記素クラスタ (Grapheme cluster) イテレーション。

## チェックリスト
- [ ] `BigInt` 演算が動作する。
- [ ] Enum と Integer に対してパターンマッチングがコンパイルされる。
- [ ] 文字列リテラルと基本的な文字列操作が Unicode で正しく動作する。
- [ ] 基本的な Effect Handler が動作する (少なくとも State/Exception に対して)。
