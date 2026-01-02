# プログラミング言語 Reml

Reml (Readable & Expressive Meta Language) はパーサーコンビネーターと静的保証に重点を置いたプログラミング言語です。

## サンプル

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

## 目的

Reml 言語の実装とエコシステム、各種ドキュメントの整備を進めています。

- 言語仕様の策定と更新（`docs/spec/`）
- 言語実装の開発（`compiler/rust/`、`runtime/`、`tooling/`）
- 実務ガイド、調査ノート、計画書の整理（`docs/`）
- サンプルとテストケースの集約（`examples/`、`tests/`）
- 参照実装としての OCaml 版の保持（`compiler/ocaml/`）

## クイックスタート

Rust 版 Remlコンパイラは `compiler/rust/` 配下で開発しています。ルートには `Cargo.toml.ws` のみがあるため、`--manifest-path` 指定で実行します。

```bash
cargo build --manifest-path compiler/rust/frontend/Cargo.toml
cargo test --manifest-path compiler/rust/frontend/Cargo.toml
cargo fmt --manifest-path compiler/rust/frontend/Cargo.toml
cargo clippy --manifest-path compiler/rust/frontend/Cargo.toml
```

> macOS で LLVM バックエンドを含むビルドやツールチェーンの詳細は `compiler/rust/README.md` を参照してください。

## ドキュメント

- [docs/README.md](`docs/README.md`): ドキュメントルート
- [docs/spec/README.md)](docs/spec/README.md): 仕様詳細
- [docs/guides/README.md](docs/guides/README.md): 実務ガイド
- [docs/notes/README.md](docs/notes/README.md): 調査ノート
- [docs/plans/README.md](docs/plans/README.md): 計画書
- [docs/schemas/](docs/schemas/): JSON Schema

## ディレクトリ構成

- `compiler/`: コンパイラ実装（Rust が主、OCaml は参照）
- `runtime/`: ランタイムライブラリ
- `tooling/`: LSP サーバー、CLI、CI 支援などの周辺ツール
- `docs/`: 仕様、ガイド、ノート、計画書、スキーマ
- `examples/`: Reml サンプルコードと検証ケース
- `tests/`: 統合テスト
- `reports/`: 監査ログやメトリクスレポート

## 例と実行

- サンプルの整理: `examples/README.md`
- スイート実行:
  - `tooling/examples/run_examples.sh --suite spec_core`
  - `tooling/examples/run_examples.sh --suite practical`
- 単体実行（cargo 経由）:
  - `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin reml_frontend -- --output json examples/.../*.reml`

## ライセンスとクレジット

Reml のライセンス整備は進行中です。再配布しているデータのライセンスは `THIRD_PARTY_LICENSES.md` と `docs/THIRD_PARTY_NOTICES.md` を参照してください。各仕様書・計画書のライセンス欄に補足がある場合は併せて確認してください。
