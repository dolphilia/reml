# Reml 改善提案反映サマリー

このドキュメントは、言語実装比較の調査結果をもとに仕様へ反映した更新点を整理したものです。対象コミットでは、Reml の書きやすさ・読みやすさ・DSL 支援を強化するために以下の変更を行いました。

## 1. Core Prelude & Iteration の拡張

- `Option.ok_or` を追加し、`Option` から `Result` への昇格を 1 行で記述可能にしました（3-1-core-prelude-iteration.md）。
- `Map.get` 等で頻出する「値が存在しない」ケースに対し、遅延評価されるエラー生成を利用できることを説明に追記しました。

## 2. Lexer レイヤの糖衣追加

- `leading` / `trim` を新設し、空白処理を `skipL`/`skipR` へ頼らずに書けるよう仕様化しました（2-3-lexer.md）。
- JSON・PL/0 のサンプルで確認された空白処理のボイラープレート削減を目的とした注記を追加しています。

## 3. パーサーコンビネーター派生関数

- `expect_keyword` / `expect_symbol` を派生 API として明示し、キーワード・記号の欠落時に統一的な診断を提供できるようにしました（2-2-core-combinator.md）。
- DSL 実装時に `label+cut` の記述を短縮することで、診断の一貫性とコーディング速度を両立させます。

## 4. Map 初期化支援

- `Map.from_pairs` を永続コレクションのユーティリティとして追加し、標準環境などの初期マップを安全に構築できる仕様を定義しました（3-2-core-collections.md）。
- 重複キー検出は `CollectError::DuplicateKey` で扱う旨を明記し、Lisp の標準環境初期化など実装例を参照先として追記しました。

## 5. パターン束縛仕様の明文化

- `let` が `match` と同等のパターン構文を受け付けることを仕様に追記しました（1-1-syntax.md）。
- 残余束縛や列挙分解を含む例を追加し、コンパイル時網羅性チェックが行われることを記載しています。

## 6. コードスタイルガイドの更新

- `List.fold` や `Iter.try_fold` を利用して蓄積目的の `var` を避ける方針を明文化しました（0-3-code-style-guide.md）。
- 仕様上のサンプルが宣言的スタイルへ寄せられるよう推奨事項を補強しています。

## 7. 影響ファイル一覧

- `0-3-code-style-guide.md`
- `1-1-syntax.md`
- `2-2-core-combinator.md`
- `2-3-lexer.md`
- `3-1-core-prelude-iteration.md`
- `3-2-core-collections.md`

## 8. フォローアップ案

- Core ライブラリ実装体へ今回追加した API（`leading`、`trim`、`Map.from_pairs` など）を実装し、単体テストを整備する。
- `Option.ok_or` を活用したサンプルコードを Chapter 3 全体で横展開し、仕様通読時に記述スタイルの改善が明確に伝わるようにする。

