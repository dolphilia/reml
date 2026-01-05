# クロスコンパイル実務ガイド

> 目的：Reml プロジェクトでクロスコンパイルを行う際に必要な CLI 操作、ツールチェーン管理、レジストリ連携、CI 構築の流れを一箇所で把握できるようにする。本ガイドは `../runtimeportability.md` と `../toolingci-strategy.md` を補完し、日常運用の手引きを提供する。

## 1. クイックスタート

1. **プロファイル確認**: `reml target list` で利用可能な `TargetProfile` を確認。
2. **ツールチェーン取得**: `reml toolchain install desktop-x86_64`（必要に応じてキャッシュを利用）。
3. **ビルド**: `reml build --target desktop-x86_64 --emit-metadata build/target.json`。
4. **テスト**: `reml test --target desktop-x86_64 --runtime smoke`。
5. **検証**: `reml target validate desktop-x86_64 && reml toolchain verify desktop-x86_64`。
6. **公開準備**: `build/target.json` をレジストリ用メタデータ（`targets` 配列）にマージし、`reml publish --targets listed` を実行。

### 1.1 ターゲット別ビルド例

Reml コンパイラ `remlc` は `RunConfig.extensions["target"]` に整形済みターゲット情報を渡す。クロスビルド時は以下のスニペットを基準として、`@cfg` と標準ライブラリのプラットフォーム抽象（[3-5](../../spec/3-5-core-io-path.md)、[3-10](../../spec/3-10-core-env.md)）を同期させる。事前に `reml target list` / `reml toolchain install <profile>` で必要なプロファイルと標準ライブラリを取得し、本ガイドの残りの節で整合性チェックを進める。

```bash
# Windows 用バイナリを Linux ホストで生成
remlc --target x86_64-pc-windows-msvc src/main.reml

# Apple Silicon 向けビルド
remlc --target aarch64-apple-darwin src/main.reml
```

ターゲット指定に合わせて `RunConfig.extensions["target"]` を初期化することで、`@cfg` の条件分岐や FFI 呼出規約（[3-9](../../spec/3-9-core-async-ffi-unsafe.md)）が一貫した状態で評価される。CI/CD では `REML_TARGET_PROFILE`, `REML_TARGET_TRIPLE`, `REML_TARGET_CAPABILITIES`, `REML_TARGET_FEATURES`, `REML_STD_VERSION`, `REML_RUNTIME_REVISION` などの環境変数を設定し、`Core.Env.infer_target_from_env()` が期待通りに解決したか `Diagnostic.domain = Target` のメッセージで確認する。

## 2. ターゲットプロファイルのライフサイクル

### 2.1 プロファイル作成

```bash
reml target scaffold mobile-arm64 --output profiles/mobile-arm64.toml
```

生成ファイル例：

```toml
[profile]
id = "mobile-arm64"
triple = "aarch64-unknown-linux-gnu"
runtime_revision = "rc-2024-09"
stdlib_version = "1.0.0"
capabilities = [
  "unicode.nfc",
  "fs.case_sensitive",
  "ffi.callconv.c"
]
```

カスタム Capability を追加する場合は `CapabilityRegistry::register_custom_target_capability` を実装し、プロファイルにも同名の文字列を記載してください。

### 2.2 プロファイル検証

```bash
reml target validate mobile-arm64 --output json > reports/target-validate.json
```

- `target.profile.missing`: `REML_TARGET_PROFILE` が未指定。`--target` か環境変数で補う。
- `target.capability.unknown`: 文字列が `TargetCapability` に存在しない。スペルまたは登録処理を見直す。

### 2.3 プロファイル同期

```bash
reml target sync --write-cache
```

環境差分を `~/.reml/targets/cache.json` に記録し、次回の `reml build --target` で `target.config.mismatch` を素早く検出できます。

## 3. ツールチェーン管理

| コマンド | 説明 |
| --- | --- |
| `reml toolchain list` | インストール済みプロファイルの `runtime_revision` とハッシュを表示。 |
| `reml toolchain install <profile>` | 標準ライブラリとランタイムを取得し、`toolchain-manifest.toml` を更新。 |
| `reml toolchain update <profile>` | 新しい `hash` が利用可能な場合のみ再ダウンロード。 |
| `reml toolchain verify <profile>` | `RunArtifactMetadata.hash`・署名・Capability を検証。 |
| `reml toolchain prune` | 使われていないハッシュを削除し、ディスク使用量を抑制。 |

推奨環境変数：

| 変数 | 既定値 | 用途 |
| --- | --- | --- |
| `REML_TOOLCHAIN_HOME` | `$HOME/.reml/toolchains` | ツールチェーンのルート。CI ではキャッシュディレクトリを指定。 |
| `REML_TOOLCHAIN_CACHE` | `$REML_TOOLCHAIN_HOME/cache` | ダウンロードキャッシュ。`reml toolchain prune` が参照。 |
| `REML_TARGET_PROFILE_PATH` | `$WORKSPACE/.reml/targets` | プロファイル定義の検索パス。 |

## 4. ビルド & テストフロー

### 4.1 標準的なシェルスクリプト

```bash
set -euo pipefail
PROFILE=${PROFILE:-desktop-x86_64}

reml target validate "$PROFILE"
reml toolchain install "$PROFILE" --auto-approve
reml build --target "$PROFILE" --emit-metadata build/target.json
reml test --target "$PROFILE" --runtime smoke --output json > reports/tests.json
```

`reports/tests.json` 内の `CliDiagnosticEnvelope` から `Diagnostic.domain = "Target"` の件数を抽出し、ポータビリティ回帰を検知します。

### 4.2 メタデータ活用

- `build/target.json` をレジストリの `targets` 配列にそのまま変換可能。
- `hash` / `runtime_revision` を CI アーティファクトに保存しておくと、ナイトリービルドで比較が容易です。

## 5. CI テンプレート（GitHub Actions例）

```yaml
jobs:
  build:
    strategy:
      matrix:
        include:
          - profile: desktop-x86_64
            triple: x86_64-unknown-linux-gnu
            capabilities: unicode.nfc,fs.case_sensitive
          - profile: mac-arm64
            triple: aarch64-apple-darwin
            capabilities: unicode.nfc,fs.case_preserving
    runs-on: ubuntu-latest
    env:
      REML_TARGET_PROFILE: ${{ matrix.profile }}
      REML_TARGET_TRIPLE: ${{ matrix.triple }}
      REML_TARGET_CAPABILITIES: ${{ matrix.capabilities }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/cache@v4
        with:
          path: ~/.reml/toolchains
          key: reml-toolchain-${{ matrix.profile }}-${{ hashFiles('profiles/**/*.toml') }}
      - run: reml target sync --write-cache
      - run: reml toolchain install ${{ matrix.profile }}
      - run: reml toolchain verify ${{ matrix.profile }}
      - run: reml build --target ${{ matrix.profile }} --emit-metadata build/target.json
      - run: reml test --target ${{ matrix.profile }} --runtime smoke --output json > reports/tests.json
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.profile }}-diagnostics
          path: |
            build/target.json
            reports/tests.json
```

CI 全体の運用は `../toolingci-strategy.md` のマトリクスと併せて調整してください。

## 6. レジストリ公開チェック

1. `reml build --emit-metadata` で得た JSON を `targets` 配列へ変換。
2. `reml publish --targets listed` を実行し、`400 target.capability.unknown` や `target.abi.mismatch` が返らないことを確認。
3. 署名を利用する場合は `reml toolchain verify` の結果を添付し、レジストリの監査ログに `signature` 情報を記録します。

## 7. トラブルシューティング

| 診断コード | 意味 | 対処 |
| --- | --- | --- |
| `target.profile.missing` | `profile_id` が空または未解決 | `REML_TARGET_PROFILE` を設定、`reml target validate` を再実行 |
| `target.capability.unknown` | Capability 名が未登録 | `capability_name(TargetCapability::…)` に合わせてスペル修正、またはカスタム Capability を登録 |
| `target.abi.mismatch` | ランタイム/stdlib のバージョン不一致 | `reml toolchain install --force` で最新を取得し、再ビルド |
| `target.config.mismatch` | 実行環境とプロファイルが異なる | `reml target sync` 実行後にプロファイル更新、CI では警告をエラーへ昇格 |

---

クロスコンパイルはプロジェクト規模・ターゲット数に応じて段階的に導入することを推奨します。まずは CLI/Toolchain の自動検証を整備し、次に CI マトリクスとレジストリ運用を拡張してください。詳細な背景や設計判断は `../../notes/backend/cross-compilation-spec-intro.md` を参照してください。
