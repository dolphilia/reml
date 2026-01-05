# ベンチマーク運用ガイド

Reml のベンチマークを安定的に実行し、結果を比較・共有するための手順をまとめます。

## 対象ディレクトリ

- `benchmarks/`: Criterion ベースのベンチマーク群
- `benchmarks/text/`: テキスト処理ベンチマーク
- `benchmarks/parse/`: パーサー計測ベンチマーク

## 実行手順

ベンチマークは `benchmarks/Cargo.toml` を起点に実行します。

```bash
cargo bench --manifest-path benchmarks/Cargo.toml
```

個別のベンチマークを指定する場合:

```bash
cargo bench --manifest-path benchmarks/Cargo.toml --bench text_builder
```

Criterion のフィルタを使って対象を絞り込む場合:

```bash
cargo bench --manifest-path benchmarks/Cargo.toml --bench text_builder -- text::builder/push_grapheme_finish
```

## 出力と成果物

- 計測結果: `benchmarks/target/criterion/`
- 実行バイナリ: `benchmarks/target/release/deps/`

## 運用上の注意

- 計測はリリースビルドで実行されます。ローカル環境の負荷や CPU 周波数の変動で結果が揺れるため、同一条件で比較してください。
- `benchmarks/` は独立した `Cargo.toml` を持つため、ルートの `Cargo.toml.ws` を移動する必要はありません。
- 失敗時は `benchmarks/` 配下の対象ファイルを確認し、`--bench` 指定で原因を切り分けてください。
