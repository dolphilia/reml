# 5.3 Developer Toolchain（ドラフト）

> 目的：IDE/LSP、フォーマッタ、リンター、デバッガー、プロファイラーなどの開発者ツールチェーン仕様を統合し、Reml 言語・標準ライブラリとの結節点を示す。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 草案（Draft） |
| 参照文書 | [guides/lsp-integration.md](guides/lsp-integration.md), [guides/config-cli.md](guides/config-cli.md), [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md) §4.1, §4.2 |
| 関連章 | 3-6, 3-7, 3-8, 4-1 |

## 1. LSP 実装ロードマップ

- サーバー機能（補完、定義ジャンプ、ドキュメント、コードアクション）。
- `DslExportSignature` を活用した DSL 特化ヒント。
- 監査/トレースとの連携（3-6 §8.2）。

## 2. フォーマッタ & リンター

- `reml fmt` / `reml check` に対応するライブラリ API と CLI.
- スタイルガイド、設定ファイル形式（`reml-format.toml` 仮称）。
- 効果タグや DSL メタデータに基づく静的解析規則。

## 3. Toolchain 配布とターゲット管理

- 標準ライブラリ (`Core.*`) とランタイムアーティファクトをターゲット単位で事前ビルドし、`artifact/std/<triple>/<hash>` ディレクトリに配置する。`hash` は `RunArtifactMetadata.hash` を採用し、CLI/レジストリと同一値で検証可能にする。
- ランタイム (`runtime/<profile_id>/libremlrt.*` 等) は `runtime_revision` ごとに区分し、`TargetProfile` が要求する `runtime_revision` と一致しない場合はロードを拒否する。
- `toolchain-manifest.toml` を導入し、インストール済みプロファイルのメタデータ (`profile_id`, `triple`, `runtime_revision`, `stdlib_version`, `capabilities`, `installed_at`) を記録。CLI/IDE はこのマニフェストを参照してローカル状態を判定する。

### 3.1 CLI サブコマンド (`reml toolchain`)

| コマンド | 説明 |
| --- | --- |
| `reml toolchain list` | ローカルにインストール済みターゲットプロファイルと `runtime_revision` を一覧表示。`--json` で `toolchain-manifest` をそのまま出力。 |
| `reml toolchain install <profile>` | レジストリまたは指定 URL から標準ライブラリとランタイムを取得。`--artifact-url` / `--runtime-url` で手動指定可。`target.abi.mismatch` が発生した場合はロールバック。 |
| `reml toolchain update <profile>` | `hash` が異なる場合のみ再ダウンロードし、旧バージョンは `toolchain/archive/` に退避。 |
| `reml toolchain prune` | 未使用ハッシュを削除し、ディスク占有を削減。`--keep-latest <n>` で保持数を指定。 |
| `reml toolchain verify <profile>` | `hash` / `signature` / `capabilities` を再検証し、`DiagnosticDomain::Target` の結果を報告。 |

- `reml toolchain install` は `TargetProfile` を取得後、`Core.Env.resolve_run_config_target` と `merge_runtime_target` を使用して整合性をチェック。失敗時は `target.capability.unknown` を報告し、インストールを中断する。
- すべてのサブコマンドは `CliDiagnosticEnvelope.summary.stats` に `profiles_installed`, `bytes_downloaded`, `verification_time` を記録し、性能指標 (0-2 §1.1) を可視化する。

### 3.2 環境変数とディレクトリ構成

| 環境変数 | 既定値 | 用途 |
| --- | --- | --- |
| `REML_TOOLCHAIN_HOME` | `$HOME/.reml/toolchains` | インストール先ルート。CI では writable キャッシュを指定。 |
| `REML_TOOLCHAIN_CACHE` | `$REML_TOOLCHAIN_HOME/cache` | ダウンロードキャッシュ。`reml toolchain prune` の対象。 |
| `REML_TARGET_PROFILE_PATH` | `$WORKSPACE/.reml/targets` | プロファイル定義のローカル検索パス。`reml target scaffold` と連携。 |

- ディレクトリ例：

```
$REML_TOOLCHAIN_HOME/
  profiles/
    desktop-x86_64/
      toolchain-manifest.toml
      std/
        x86_64-unknown-linux-gnu/
          <hash>/libcore.a
      runtime/
        rc-2024-09/libremlrt.a
```

- IDE/LSP は `REML_TOOLCHAIN_HOME/profiles/<profile_id>` を監視し、変更検知時に `RunConfigTarget` の再初期化を行う。`toolchain-manifest` の `installed_at` との差分でキャッシュ無効化を制御する。

### 3.3 キャッシュ・検証ポリシー

- すべてのアーティファクトは `sha256` もしくは `blake3` のダブルハッシュで検証し、`hash` が一致しない場合は即削除・再取得を行う。
- `signature` が存在する場合は Ed25519 で検証し、成功した証跡を `toolchain-manifest.toml` の `verify.signature = true` に記録。
- CI では `reml toolchain verify --all --output json` を実行し、`DiagnosticDomain::Target` の結果を `guides/ci-strategy.md` に準じて収集する。
- `reml build --target` 実行時、要求された `profile_id` がインストール済みでない場合は自動的に `toolchain install` を提案し、`--auto-install` が有効なら即時実行する。

## 4. テストランナー

- `Core.Test`（計画中）および `reml test` の相互作用。
- 並列化戦略とEffect制約。
- スナップショットテストや DSL 専用マクロの扱い。

## 5. デバッガー

- DSL 境界を跨ぐステップ実行、逆方向トレース。
- `TraceSink`（3-8 §7）および `AuditEnvelope` を用いた履歴管理。
- CLI 連携 (`reml debug` コマンド案)。

## 6. プロファイラー

- `benchmark_dsl` との整合性、サンプリング/計測の要件。
- ホットスポット抽出、DSL パイプライン単位の計測。

## 7. AI 支援ツール

- `reml ai-*` コマンド（4-0, reml-ecosystem-analysis §4.3）に対応する API。
- LLM 連携時の安全ガードライン（guides/ai-integration.md 参照）。

## 8. 今後の作業

- 既存ガイドの統合（lsp, config-cli, dsl-performance など）。
- 参考実装（VS Code 拡張、Neovim プラグイン、CLI ツール）の執筆計画。
- テストベンチ / CI 戦略の取りまとめ。

> メモ: 本章はツールチェーンの包括的な仕様をまとめるための骨組みであり、個別節の詳細は今後のドラフトで充実させる。
