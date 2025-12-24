# 1.2 実装ギャップ対応計画（Rust Frontend / 2025-12-24）

`docs/spec/1-2-types-Inference.md` のサンプル修正で判明した **仕様と実装のギャップ** を整理し、Rust Frontend 側で追随するための対応計画を定義する。

## 目的
- 仕様に合わせて Rust Frontend の受理範囲を拡張する。
- 仕様サンプルの簡略化を段階的に解消し、正準例へ戻す。

## 対象範囲
- 仕様章: `docs/spec/1-2-types-Inference.md`, `docs/spec/2-1-parser-type.md`
- サンプル: `examples/docs-examples/spec/1-2-types-Inference/*.reml`
- 監査ログ: `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md`

## ギャップ一覧（簡略化／回避済み）

### 1. スライス型 `[T]` の型注釈が未対応
- 影響: 仕様で定義済みのスライス型 `[T]` を Rust Frontend が型注釈として受理できない。
- 該当サンプル:
  - `examples/docs-examples/spec/1-2-types-Inference/sec_b_3.reml`
  - `examples/docs-examples/spec/1-2-types-Inference/sec_h_2-a.reml`
- 現状の回避: `List<T>` へ置換。

### 2. 参照型 `&` / `&mut` が未対応
- 影響: `&mut State` を含む関数型シグネチャが構文エラーになる。
- 該当サンプル:
  - `examples/docs-examples/spec/1-2-types-Inference/sec_f.reml`
  - `docs/spec/2-1-parser-type.md` の `Parser<T>` 定義
- 現状の回避: `fn(State) -> Reply<T>` に簡略化。

## 実装修正計画（Rust Frontend）

### フェーズ 1: 構文受理（最小限のパース対応）
進捗: 完了（型パーサ/トークン/構文ASTの追加まで実施）
1) スライス型 `[T]` のパーサ追加
- 目的: 型注釈として `[T]` を受理し、AST に保持できるようにする。
- 成果物: `[T]` を含むサンプルが `--emit-diagnostics` で 0 件になる。
- 対象サンプル: `sec_b_3`, `sec_h_2-a`
- 作業ステップ:
  - 型パース処理で「型原子」に `[` 型 `]` の構文を追加する。
  - `]` 欠落などの代表的な構文エラーに対して、期待トークンを明示する診断を追加する。
  - `List<T>` との優先順位が衝突しないよう、ジェネリクス/型引数の解析順を確認する。

2) 参照型 `&` / `&mut` のパーサ追加
- 目的: `&T` / `&mut T` を型として受理する。
- 成果物: `Parser<T>` の型注釈が構文エラーにならない。
- 対象サンプル: `sec_f`（および `docs/spec/2-1-parser-type.md` の該当サンプル）
- 作業ステップ:
  - 型パース処理に `&` 前置演算子を追加し、`&mut` を可変参照として分岐させる。
  - `&` が付いた型注釈が関数型（`fn(...) -> ...`）より強く結合するよう優先順位を整理する。
  - `&` 単独や `&mut` 後に型が続かないケースで、回復可能な診断を用意する。

### フェーズ 2: AST/型表現の整合
進捗: 完了（型ASTの追加と型推論/型検査への接続まで対応）
1) 型 AST へのスライス/参照ノード追加
- 目的: 型推論/型検査において `[T]` と `&` を識別できるようにする。
- 成果物: 型出力（`--emit-typed-ast`）にスライス/参照情報が含まれる。
- 作業ステップ:
  - 型 AST に `Slice` / `Ref`（可変フラグ付き）を追加し、パーサ出力から接続する。
  - 既存の型フォーマッタ・デバッグ表示・JSON 出力などがある場合は新ノードを反映する。
  - `--emit-typed-ast` の出力サンプルで、`[T]` と `&mut T` が判別できることを確認する。
  - 対応状況: 完了（`typeck` 型表現で `Slice`/`Ref` を追加し、単一化・出力へ反映済み）。

2) 既存型との互換ルール整理
- 目的: `List<T>` と `[T]` の役割差分、参照型の不変/可変制約を仕様と整合させる。
- 成果物: `docs/spec/1-2-types-Inference.md` / `docs/spec/2-1-parser-type.md` で差分注記が不要になる。
- 作業ステップ:
  - `List<T>` と `[T]` の相互変換可否（暗黙変換の有無、型等価性）を仕様に合わせて整理する。
  - `&T` と `&mut T` の代入/引数受け渡し制約を既存の型ルールに組み込む。
  - 既存の型チェックやユニフィケーション処理が参照型を見落とさないようケースを追加する。
  - 参照箇所: `docs/spec/1-2-types-Inference.md` §A.2「合成」（配列/スライス）、`docs/spec/3-2-core-collections.md` §2.1「List<T>」、`docs/spec/2-1-parser-type.md` §A「主要型」（Parser/State/Reply）
  - 対応状況: 完了（`typeck` の単一化で `&`/`&mut` を区別し、`[T]` と反復可能型の扱いを更新済み）。

### フェーズ 3: サンプル復元と再検証
1) サンプルを仕様寄りに戻す
- `[T]` を用いた例、`&mut State` の表記を復元。
- 作業ステップ:
  - `examples/docs-examples/spec/1-2-types-Inference/` の該当ファイルを正準例へ戻す。
  - 仕様内の説明文や注釈が `List<T>` 前提になっていないか再確認する。

2) 監査ログ更新
- `reports/spec-audit/summary.md` に再検証結果を記録。
- `reports/spec-audit/ch1/docs-examples-fix-notes-YYYYMMDD.md` を更新。
- 作業ステップ:
  - 監査ログに「受理可否」「残課題」「差分理由」を箇条書きで追記する。
  - 追記した日付のファイル名に合わせて相互リンクを更新する。

## 進捗管理
- 本計画書作成日: 2025-12-24
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [ ] フェーズ 3 完了

## 関連リンク
- `docs/spec/1-2-types-Inference.md`
- `docs/spec/2-1-parser-type.md`
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-plan.md`
- `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md`
