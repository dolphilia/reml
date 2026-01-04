# プログラミング言語 Reml

Reml (Readable & Expressive Meta Language) はパーサーコンビネーターと静的保証に重点を置いたプログラミング言語です。

## Reml の特徴

```reml
module Spec.Core.Chapter1.Match.BindingAsOk

use Core.Prelude

fn describe(value: Option<Int>) -> Str {
  match value with
    | Some(v) as whole -> "v=" + v.to_string()
    | None -> "none"
}

fn main() -> Str = describe(Some(42))
```

## リポジトリの役割

このリポジトリでは Reml 言語の実装やエコシステム、各種ドキュメントの整備を進めています。

- 言語仕様の策定と更新（`docs/spec/`）
- 言語実装の開発（`compiler/`、`tooling/`）
- 実務ガイド、調査ノート、計画書の整理（`docs/`）
- サンプルとテストケースの集約（`examples/`、`tests/`）

## クイックスタート

Rust 版 Remlコンパイラは `compiler/` 配下で開発しています。ルートには `Cargo.toml.ws` のみがあるため、基本的に `--manifest-path` 指定で実行します。

```bash
cargo build --manifest-path compiler/frontend/Cargo.toml
cargo test --manifest-path compiler/frontend/Cargo.toml
cargo fmt --manifest-path compiler/frontend/Cargo.toml
cargo clippy --manifest-path compiler/frontend/Cargo.toml
```

macOS でリンクが不安定な場合は `lld` を利用できます。
```bash
RUSTFLAGS="-C link-arg=-fuse-ld=lld" cargo test --manifest-path compiler/frontend/Cargo.toml
```

> macOS の LLVM バックエンドを含むビルドやツールチェーンの詳細は `compiler/README.md` を参照してください。

## ドキュメント

- [docs/README.md](`docs/README.md`): ドキュメントルート
- [docs/spec/README.md)](docs/spec/README.md): 仕様詳細
- [docs/guides/README.md](docs/guides/README.md): 実務ガイド
- [docs/notes/README.md](docs/notes/README.md): 調査ノート
- [docs/plans/README.md](docs/plans/README.md): 計画書
- [docs/schemas/](docs/schemas/): JSON Schema

## ディレクトリ構成

- [compiler/](compiler/): コンパイラ/ランタイム実装（Rust）
- [tooling/](tooling/): LSP サーバー、CLI、CI 支援などの周辺ツール
- [docs/](docs/): 仕様、ガイド、ノート、計画書、スキーマ
- [examples/](examples/): Reml サンプルコードと検証ケース
- [tests/](tests/): 統合テスト
- [reports/](reports/): 監査ログやメトリクスレポート

## サンプルの実行

- [examples/README.md](examples/README.md): サンプルについて
- スイート実行:
  - `tooling/examples/run_examples.sh --suite spec_core`
  - `tooling/examples/run_examples.sh --suite practical`
- 単体実行（cargo 経由）:
  - `cargo run --manifest-path compiler/frontend/Cargo.toml --bin reml_frontend -- --output json examples/.../*.reml`

## ライセンスとクレジット

Reml のライセンス整備は進行中です。

再配布しているデータのライセンスは `THIRD_PARTY_LICENSES.md` と `docs/THIRD_PARTY_NOTICES.md` を参照してください。

各仕様書・計画書のライセンス欄に補足がある場合は併せて確認してください。
