# Numeric Basic Extended (median/mode/range)

- **実行日時**: 2025-12-09 (ローカル開発機 `cargo test --manifest-path compiler/rust/runtime/Cargo.toml --features core-numeric numeric::tests::median_mode_and_range_cover_basic_cases`)
- **対象 API**: `median` / `mode` / `range` / `IterNumericExt`
- **結果**: 3 ケースすべて PASS。偶数件の中央値は lower median として `values[len/2 - 1]` を返し、最頻値は出現順（tie-breaker に `first_seen` を採用）で決定。`range` は最初のサンプルをベースに逐次比較して `(min, max)` を返す。
- **サンプル入力**:
  ```text
  data = [8, 2, 3, 2, 7, 9]
  median(data) = 3
  mode(data) = 2
  range(data) = (2, 9)
  ```
- **備考**: `Iter::from_list` コピーでは内部ステートが共有されるため、テストではデータ列を `Vec` に保持し呼び出し毎に `Iter::from_list` を生成。`HashMap` ベースのカウントは `first_seen` インデックスを保持して tie を解消した。
