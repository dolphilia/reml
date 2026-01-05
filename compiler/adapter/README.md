# adapter

プラットフォーム差異を吸収する Rust 側アダプタ層です。Env/FS/Network/Time/Random/Process/Target の各サブシステムに対して Capability/監査の共通抽象化を提供します。

## 主要モジュール
- `capability`: Capability 設定と監査用の共通モデル
- `env`: 環境変数・実行環境の抽象化
- `fs`: ファイルシステム操作のラッパ
- `network`: TCP などネットワーク関連のラッパ
- `process`: プロセス起動・終了コードの取得
- `random`: 乱数生成の抽象化
- `target`: 実行ターゲット/プラットフォーム判定
- `time`: 時刻取得とタイムゾーンの補助

## ビルド/テスト
```
cargo test --manifest-path compiler/adapter/Cargo.toml
```

ネットワーク試験は `127.0.0.1:0` への bind 権限が必要です。権限を付与できない環境では次の変数を使って制御します。

- `REML_ADAPTER_SKIP_NETWORK_TESTS=1`（スキップ）
- `REML_ADAPTER_FORCE_NETWORK_TESTS=1`（スキップ設定の上書き）

## 関連ドキュメント
- `docs/plans/rust-migration/2-2-adapter-layer-guidelines.md`
