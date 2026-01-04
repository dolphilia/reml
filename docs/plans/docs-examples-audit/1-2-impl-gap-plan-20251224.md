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
  - 型パース処理の「型原子」分岐に `[` 型 `]` を追加し、AST へ `Slice` 型として格納する。
  - `[` 開始後に型が無い／` ]` 欠落のケースで、期待トークン（型/`]`）を明示する診断を定義する。
  - `List<T>` と同一の型引数構文を併用できるよう、`[` を見たらスライス優先で確定する順序に整える。
  - `examples/docs-examples/spec/1-2-types-Inference/sec_b_3.reml` / `sec_h_2-a.reml` を入力とした診断ログ取得の導線を用意する。

2) 参照型 `&` / `&mut` のパーサ追加
- 目的: `&T` / `&mut T` を型として受理する。
- 成果物: `Parser<T>` の型注釈が構文エラーにならない。
- 対象サンプル: `sec_f`（および `docs/spec/2-1-parser-type.md` の該当サンプル）
- 作業ステップ:
  - 型パース処理に `&` 前置演算子を追加し、直後に `mut` が続いた場合は可変参照として記録する。
  - 参照型が関数型より強く結合するよう、`&` の結合順位と型原子の境界を調整する。
  - `&` 単独や `&mut` 後に型が無いケースで、回復可能な診断（期待トークン/推奨例）を追加する。
  - `docs/spec/2-1-parser-type.md` の `Parser<T>` 定義と衝突しない型表記になっているか確認する。

### フェーズ 2: AST/型表現の整合
進捗: 完了（型ASTの追加と型推論/型検査への接続まで対応）
1) 型 AST へのスライス/参照ノード追加
- 目的: 型推論/型検査において `[T]` と `&` を識別できるようにする。
- 成果物: 型出力（`--emit-typed-ast`）にスライス/参照情報が含まれる。
- 作業ステップ:
  - 型 AST に `Slice` / `Ref`（可変フラグ付き）を追加し、パーサ出力から変換する。
  - 型表示（デバッグ/整形）と JSON 出力に `Slice` / `Ref` の表記を加え、`[T]` と `&mut T` の違いが表示に残るようにする。
  - `--emit-typed-ast` の出力で `Slice` / `Ref` が区別されることを確認し、必要なら出力サンプルを更新する。
  - 対応状況: 完了（`typeck` 型表現で `Slice`/`Ref` を追加し、単一化・出力へ反映済み）。

2) 既存型との互換ルール整理
- 目的: `List<T>` と `[T]` の役割差分、参照型の不変/可変制約を仕様と整合させる。
- 成果物: `docs/spec/1-2-types-Inference.md` / `docs/spec/2-1-parser-type.md` で差分注記が不要になる。
- 作業ステップ:
  - `List<T>` と `[T]` の役割差分（暗黙変換の有無、型等価性）を仕様記述と照合し、型等価判定に反映する。
  - `&T` / `&mut T` の代入互換性・引数受け渡し制約を型検査ルールへ組み込み、ミューテーション制約を明示する。
  - 既存の単一化/制約解決で参照型が抜けないよう分岐を追加し、エラーメッセージの型表示を更新する。
  - 参照箇所: `docs/spec/1-2-types-Inference.md` §A.2「合成」（配列/スライス）、`docs/spec/3-2-core-collections.md` §2.1「List<T>」、`docs/spec/2-1-parser-type.md` §A「主要型」（Parser/State/Reply）
  - 対応状況: 完了（`typeck` の単一化で `&`/`&mut` を区別し、`[T]` と反復可能型の扱いを更新済み）。

### フェーズ 3: サンプル復元と再検証
1) サンプルを仕様寄りに戻す
- `[T]` を用いた例、`&mut State` の表記を復元。
- 作業ステップ:
  - `examples/docs-examples/spec/1-2-types-Inference/` の `sec_b_3` / `sec_h_2-a` / `sec_h_2-b` を `[T]` へ戻す。
  - `examples/docs-examples/spec/1-2-types-Inference/sec_f.reml` を `&mut State` を含む定義へ復元する。
  - `docs/spec/1-2-types-Inference.md` の B.3 / F / H.2 のコードブロックと注釈を正準例へ戻す。
  - `docs/spec/2-1-parser-type.md` の `Parser<T>` 定義と表記が食い違わないか目視確認する。

2) 再検証
- `compiler/frontend/target/debug/reml_frontend --emit-diagnostics <sample>` で診断を再取得。
- 作業ステップ:
  - `sec_b_3` / `sec_f` / `sec_h_2-a` / `sec_h_2-b` を対象に再実行する。
  - 出力 JSON を `reports/spec-audit/ch1/1-2-types-Inference__<file>-YYYYMMDD-diagnostics.json` として保存する。
  - 既存の診断 JSON が旧サンプル由来の場合は置き換え、差分メモを添える。
  - `docs/plans/docs-examples-audit/1-1-spec-code-block-inventory.md` の `validation` 欄を更新する。

3) 監査ログ更新
- `reports/spec-audit/summary.md` に再検証結果を記録。
- `reports/spec-audit/ch1/docs-examples-fix-notes-YYYYMMDD.md` を更新。
- 作業ステップ:
  - 監査ログに「受理可否」「残課題」「差分理由」を箇条書きで追記する。
  - 診断ログが未取得の場合は「未再検証」を明記し、再検証予定コマンドを添える。
  - 追記した日付のファイル名に合わせて相互リンクを更新する。
  - 既存の診断 JSON が旧サンプル由来である場合、再生成が必要である旨を明示する。

## Backend / Runtime 影響の再確認（2025-12-24 追記）
- `[T]` と `&` / `&mut` は **Frontend の型表現に追加済み**だが、`compiler/backend/llvm/src/integration.rs` の `parse_reml_type` は文字列トークンを `pointer` にフォールバックするため、バックエンドでは型情報が失われる。
- `compiler/backend/llvm/src/type_mapping.rs` の `RemlType` は `Slice` / `Ref` を持たないため、**コード生成を前提にする場合は型列挙とレイアウト計算を追加**する必要がある（`[T]` の `{ptr,len}` 表現、`&T` / `&mut T` の ABI 整理）。
- Runtime 側は `List<T>` の実装はあるが、`[T]` に対応する **明示的な ABI/FFI 表現が未整理**のため、実行系で扱う場合は `docs/spec/1-2-types-Inference.md` と `docs/spec/3-2-core-collections.md` を参照して整合方針を確定する。
- ドキュメント監査・サンプル復元の範囲では **Backend / Runtime 追加対応は不要**（型注釈の受理と診断整合が主目的）。

## 進捗管理
- 本計画書作成日: 2025-12-24
- 進捗欄（運用用）:
  - [x] フェーズ 1 完了
  - [x] フェーズ 2 完了
  - [x] フェーズ 3 完了

## 関連リンク
- `docs/spec/1-2-types-Inference.md`
- `docs/spec/2-1-parser-type.md`
- `docs/plans/docs-examples-audit/1-2-spec-sample-fix-plan.md`
- `reports/spec-audit/ch1/docs-examples-fix-notes-20251224.md`
