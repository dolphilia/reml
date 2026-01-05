# frontend

Reml の Rust フロントエンド実装です。字句解析・構文解析・型検査・診断出力・ストリーミング実行の土台を提供します。

## 主要モジュール
- `lexer/` / `token.rs` / `unicode.rs`: 字句解析とトークン定義
- `parser/`: AST と標準パーサ API
- `semantics/`: Typed AST と MIR の骨格
- `typeck/`: 型推論・制約生成・テレメトリ
- `diagnostic/` / `output/`: 診断モデルと CLI 出力
- `pipeline/` / `streaming/`: 実行パイプラインとストリーミング実行

## CLI
- `reml_frontend`: 入力ソースを解析し JSON を出力する CLI
- `remlc`: マニフェスト/設定の検証やテンプレート作成を行う CLI

## ビルド/テスト
```
cargo build --manifest-path compiler/frontend/Cargo.toml
cargo test --manifest-path compiler/frontend/Cargo.toml
```

### オプション
- `--features schema`: JSON Schema を出力するための補助を有効化
