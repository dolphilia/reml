# 4.1 Package Manager & CLI

> 目的：Reml エコシステムの基盤となる公式 CLI (`reml`) およびパッケージマネージャーの仕様を定義し、Chapter 1-3 で規定された言語・標準 API・ランタイム機能と統合する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 執筆中（Working Draft） |
| 参照文書 | [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md), [4-0-ecosystem-integration-plan.md](4-0-ecosystem-integration-plan.md) |
| 主要関連章 | [1-1-syntax.md](1-1-syntax.md), [1-2-types-Inference.md](1-2-types-Inference.md), [1-3-effects-safety.md](1-3-effects-safety.md), [3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md), [3-7-core-config-data.md](3-7-core-config-data.md), [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) |

## 1. CLI アーキテクチャ

### 1.1 設計原則
- `0-2-project-purpose.md` で提示された性能・安全性・段階的習得性を満たすよう、すべてのサブコマンドは**構造化診断**と**線形時間性能**を基準とする。
- CLI が出力する診断は `CliDiagnosticEnvelope`（[3-6 §10](3-6-core-diagnostics-audit.md#cli-protocol)）を共通で用いる。エラーは `domain` と `phase` を明示し、`extensions` にターゲットや DSL 情報を付与する。
- 実行パイプラインは `Input → Analyzer → Planner → Executor → Reporter` の 5 段で構成され、それぞれが idempotent に設計される。`Planner` が `RunConfigTarget` を解決し `Executor` がフェーズごとの `CliPhase` を供給する。

### 1.2 サブコマンド体系

| コマンド | 主要責務 | 関連章 |
| --- | --- | --- |
| `new` | プロジェクト雛形生成、初期マニフェスト作成 | [3-7](3-7-core-config-data.md) |
| `add` | 依存追加とロックファイル更新、バージョン解決 | [4-2](4-2-registry-distribution.md) |
| `build` | 解析→型推論→効果検査→コード生成を実行し、成果物とメタデータを生成 | [1-1](1-1-syntax.md), [1-2](1-2-types-Inference.md), [1-3](1-3-effects-safety.md), [2-6](2-6-execution-strategy.md) |
| `test` | テストランナー呼び出し、ターゲット互換性検証、結果集計 | `3-10-core-test-support.md`（計画中） |
| `fmt` | フォーマット適用、差分出力、終了コード管理 | [3-3](3-3-core-text-unicode.md), [3-6](3-6-core-diagnostics-audit.md) |
| `check` | リンター実行、静的解析、CI 統合 | [3-6](3-6-core-diagnostics-audit.md) |
| `publish` | レジストリ連携、成果物署名、互換性チェック | [4-2](4-2-registry-distribution.md) |
| `registry` | レジストリの設定・同期・ステータス確認 | [4-2](4-2-registry-distribution.md) |
| `target` | `TargetProfile` の管理、検証、キャッシュ制御 | [3-8](3-8-core-runtime-capability.md) |
| `toolchain` | ランタイム/標準ライブラリ配布の取得・整合性確認 | [4-3](4-3-developer-toolchain.md) |

すべてのサブコマンドは共通の初期化ルーチンで `RunContext` を構築し、`reml.toml` の整合性を確認した後に個別ロジックへ進む。初期化の失敗は `cli.init.failed` として一貫化し、フェイズ `Init` で終了する。

### 1.3 クロスコンパイル支援
- プロジェクト内/ユーザーの `profiles/` ディレクトリを探索し `TargetProfile` を合成する。探索順は `--target-json` 明示 → `--target` 指定 → `REML_TARGET_PROFILE_PATH` → `REML_TOOLCHAIN_HOME` → バンドル済みプロファイル。
- プロファイル解決に失敗した場合、即座に `target.profile.missing` を `phase = ResolveTarget` で報告し、後続フェーズを実行しない。
- `TargetProfile.merge_runtime_target` を用い、CLI レベルで `runtime_revision` と `capabilities` を勘案した互換性判定を行う。失敗時は `target.abi.mismatch` あるいは `target.capability.unknown` を返す。

## 2. マニフェスト統合

### 2.1 ロードシーケンス
1. `reml.toml` を `Core.Config.Manifest.load` で読み込み、`ManifestEnvelope` を生成する。
2. `manifest.validate()` が `Warning` を含む場合でもデフォルトで継続するが、`--fail-on-warning` 指定時は `cli.manifest.warning` を `severity = Error` として扱う。
3. `DslCapabilityProfile.sync` によって DSL 能力の宣言とインポートを整合させ、結果を `.extensions["dsl"]` に格納する。
4. `manifest.lock`（`reml.lock` 仮称）が存在する場合は依存解決をスキップし、`ResolvedDependencyGraph` を復元する。ロック不整合が検出された場合は `manifest.lock.outdated` を報告し `phase = Manifest` で終了。

### 2.2 キャッシュ管理
- `ManifestCache` は `~/.reml/cache/manifest/<hash>.json` に配置し、`hash = blake3(reml.toml + toolchain-revision)` を基準とする。
- `reml manifest sync`（将来追加）を想定し、キャッシュを手動で更新できるよう CLI 側に `--refresh-cache` オプションを提供する。キャッシュ利用時は `CliDiagnosticEnvelope.summary.stats.manifest_cache_hit` を更新する。

### 2.3 DSL プロファイル連携
- `[dsl]` セクションに記載された `capability` と `exports` を `DslCapabilityProfile` と突き合わせる。未登録 capability は `dsl.capability.unknown` として記録し、`--allow-unknown-capability` が無い限りエラー扱いとする。
- DSL コンパイル時に `DslExportSignature` を参照し、`reml new` が出力するテンプレートに署名のスタブを含める。

## 3. サブコマンド仕様

各サブコマンドは `--output (human|json|ndjson|lsp)` と `--log-path <path>` を共通で受け付け、終了コードは `0 = success`, `1 = diagnostics-error`, `2 = infrastructure-error` を厳守する。

### 3.1 `reml new`
- 入力
  - `reml new <path>`：指定パスに新規プロジェクトを生成。
  - `--template <name>`：`../guides/dsl-gallery.md` に列挙されたテンプレートを選択（`lite`, `core-app`, `pipeline`, `conductor` など）。
  - `--template lite`：学習/試作向けの最小構成テンプレートを生成する。CLI ヘルプには用途と `project.stage` 昇格の導線を含める。
  - `--dsl-entry <module::name>`：メイン DSL エクスポートを上書き。
- 処理
  - 雛形の `reml.toml` と `src/main.reml` を生成し、`[dsl.capabilities]` はテンプレート定義を初期値とする。
  - `lite` テンプレートは `config.compatibility.json.profile = "json-relaxed"` を既定とし、`dsl.lite.capabilities = []` で開始する。監査は `audit = none` を既定とし、必要時に `--audit-log <path>` で有効化する。
  - `lite` テンプレートの `README.md` には、監査ログ省略の前提と `project.stage` 昇格（`beta`/`stable`）を含む移行手順を記載する。
  - `.reml/targets` に既定プロファイル `desktop-x86_64` を配置し、`CliDiagnosticEnvelope.summary.stats.templates_used` を更新する。
- 出力
  - 成功時は `cli.new.success` 診断を `severity = Info` で出力。
  - 既存ディレクトリが空でない場合は `cli.new.workspace_conflict` を `phase = Init` で報告。

### 3.2 `reml add`
- 入力
  - `reml add <package>[@<constraint>]`：SemVer 互換の制約を受け付け（例：`~1.2`, `^0.5`, `>=1.0,<2.0`）。
  - `--git <url>` / `--rev <sha>`：Git 由来の依存を追加。`hash` は取得後に `reml.lock` へ保存。
  - `--registry <name>`：複数レジストリ環境向け。`registry` セクションのエイリアスを参照。
- 処理
  - `Resolver` が依存グラフを再構築し、競合がある場合は `resolver.conflict` を生成。提案解決策として `alternatives` を `extensions` に格納。
  - 依存インストールは `downloads/` ディレクトリのキャッシュを利用し、`hash` が一致しない場合は再取得。
- 出力
  - 成功時は `reml.lock` を更新し、差分を `CliDiagnosticEnvelope.summary.changes.dependencies_added` に列挙。
  - ソースビルドが必要なパッケージを追加する場合、`requires_source_build = true` を検知して警告を表示し、ユーザーには `--allow-source-build` の利用を促す。

### 3.3 `reml build`
- 入力
  - `--target <profile>` または `--target-json <path>` を指定でき、互いに排他。
  - `--emit-metadata <path>`：`RunArtifactMetadata` を JSON で書き出す。
  - `--profile (dev|release|custom)`：ビルド最適化レベルを指定。
- 処理
  - フェーズ別に `phase = Parse → TypeCheck → Effect → Codegen → Artifact` を割り当て、各フェーズの診断数・所要時間を `summary.stats` に記録。
  - `TargetProfile` と `StdlibVersion` の整合性をチェックし、齟齬があれば即時停止。
  - 成果物には `targets = [...]` メタデータを付与し、`publish` が再利用できるよう `artifact` ディレクトリを整備。
- 出力
  - 正常終了時は `cli.build.success` を `severity = Info` で報告し、`summary.stats.artifacts` に生成物一覧を格納。
  - `--allow-abi-drift` が未指定で ABI 不一致が発生した場合は `Error` として終了コード 1 を返す。

### 3.4 `reml test`
- 入力
  - `--filter <pattern>`：テスト名フィルタ。`pattern` は glob 表記。
  - `--target` 系オプションは `build` と同一仕様。
  - `--runtime smoke` / `--runtime emulator=<name>`：実行環境の切り替え。
- 処理
  - `Core.Test` ランタイム（予定）と連携し、テストプランを構築。
  - エミュレーションを使用する場合、`TargetCapability.coverage` を収集し `summary.stats.emulator_runs` に記録。
  - テスト結果は `CliDiagnosticEnvelope.summary.tests` に `passed`, `failed`, `skipped`, `target_failures` を格納。
- 出力
  - 失敗テストは `domain = Test` の診断として列挙し、最初の 50 件をデフォルト表示。`--output json` の場合はすべて返却。

### 3.5 `reml fmt` / `reml check`
- `fmt` は整形結果をワークスペースへ書き戻し、`--check` 指定時は書き戻しを抑制し差分の有無のみを報告。
- `check` はスタイル違反と静的解析結果を `severity = Warning` 以上で出力。`AuditEnvelope` との連携により、CI で `--output ndjson --audit-log <path>` を使用すると各違反をイベントとして蓄積できる。
- 両コマンドは `formatting.changed_files` と `linting.violations` を `summary.stats` へ格納し、ダッシュボードで可視化しやすくする。

### 3.6 `reml publish`
- 入力：`--registry <alias>`、`--dry-run`、`--signing-key <path>`、`--include-sources` など。
- 処理：
  1. `build` フェーズを再実行または既存成果物を検証。
  2. `RunArtifactMetadata` から `targets` 配列を収集し、`hash` と `signature` を検証。
  3. レジストリ（[4-2](4-2-registry-distribution.md)）へメタデータを送信し、応答の `publish_receipt` を保存。
- 出力：成功時は `cli.publish.success` を出力し、`summary.stats.targets_uploaded` にアップロード済みターゲットを列挙。失敗時は HTTP ステータスを `extensions["registry"]` に含める。

### 3.7 `reml target`
- `list` はローカル・リモートの `TargetProfile` を列挙し、`source`, `runtime_revision`, `capabilities` を表形式で表示。`--json` で構造化出力。
- `show <id>` は `TargetProfile` の全属性と関連する `toolchain-manifest` の要約を表示。
- `scaffold <id>` はテンプレート TOML を生成し、`capabilities` セクションに `capability_name(TargetCapability::…)` のコメントを挿入。
- `validate <path|id>` は `Core.Env.resolve_run_config_target` を使用して完全性を検証し、未登録 capability を即時に `Error` として返す。`--warn-only` で降格可能。
- `sync` は環境変数とローカル設定を比較し、差分を `target.config.mismatch` として報告。`--write-cache` で `~/.reml/targets/cache.json` を更新。

### 3.8 `reml toolchain`
- `list` / `install` / `update` / `prune` / `verify` の詳細は [4-3](4-3-developer-toolchain.md) を参照し、CLI 側では `toolchain-manifest.toml` の変化を検出して `summary.stats.bytes_downloaded` などを更新する。

### 3.9 `reml registry`
- `reml registry list`：利用可能なレジストリを表示し、`default`, `auth`, `mirror` 情報を提供。
- `reml registry login <alias>`：OAuth/OIDC を介したトークン取得。`AuditEnvelope` に `login` イベントを記録。
- `reml registry sync`：レジストリ側のメタデータとローカルキャッシュを同期。結果を `summary.stats.registry_cache_hit` として出力。

## 4. 出力形式と UX

### 4.1 出力モード
- `--output human`（既定）はカラーリングを行い、TTY では 256 色まで利用。非 TTY では自動的にカラー無効。
- `--output json` は `CliDiagnosticEnvelope` を単一 JSON として出力。
- `--output ndjson` は診断ごとに 1 行の JSON を出力し、ストリーミング処理に適合。
- `--output lsp` は Language Server Protocol 互換の通知を生成し、IDE へのパイプラインに使用。

### 4.2 フラグと環境変数

| オプション | 説明 |
| --- | --- |
| `--summary (auto|always|never)` | サマリ行の出力制御。`auto` は TTY のみ表示。 |
| `--fail-on-warning` | `Warning` 診断が存在する場合でも終了コード 1 を返す。 |
| `--fail-on-performance` | `Diagnostic.domain = Performance` が検出された際に失敗扱いとする。 |
| `--color (auto|always|never)` | カラーリング制御。 |
| `--locale <tag>` | メッセージのロケール選択。既定はシステムロケール。 |

環境変数：
- `REML_CLI_LOG`：ログ出力先ファイル。未設定時は標準エラー。
- `REML_CACHE_DIR`：CLI キャッシュルート。デフォルトは `~/.reml/cache`。

### 4.3 アクセシビリティ
- エラーメッセージは番号付きリストを併記し、スクリーンリーダーが認識しやすい構造を保つ。
- ロケール未対応の場合でも英語フォールバックを保証し、`--locale` 指定時の検証を行う。

## 5. セキュリティ・監査要件

### 5.1 Capability 要求
- `publish`, `registry`, `toolchain` サブコマンドは `SecurityCapability::NetworkAccess` を要求し、マニフェスト上で `capabilities.network = true` が確認できない場合は実行前に警告を表示。
- `add` が外部リソースをダウンロードする際は `SecurityCapability::FilesystemWrite` を確認し、CI モード（`REML_CI=1`）で未許可の場合は即時失敗する。

### 5.2 監査ログ
- すべてのサブコマンドで `AuditEnvelope` を任意指定できるよう `--audit-log <path>` を受け付ける。ログは NDJSON 形式で `run_id`, `timestamp`, `command`, `phase`, `diagnostic_id` を含む。
- `publish` と `registry login` は常に監査ログを要求し、未指定の場合は `audit.log.required` を警告として出力。`--allow-missing-audit` でのみ無効化可能。

### 5.3 署名と検証
- `publish` 時には成果物とマニフェスト双方に署名を付与可能。CLI は `signing-key` から秘密鍵を読み取り、`targets[*].signature` に格納。
- `add`/`build` は取得したアーティファクトを検証し、署名が無い場合は `Warning` を通知。`--enforce-signature` を指定すると未署名パッケージを拒否する。

## 6. 今後の作業
- CLI オプションの正式なリファレンス表を `Appendix` として追加し、短縮形・環境変数とのマッピングを整理する。
- `CliDiagnosticEnvelope` の JSON スキーマを付録化し、`../guides/cli-workflow.md` と同期させる。
- 性能ベンチマーク計測のためのテストシナリオとしきい値を策定し、`build`/`test` の `--fail-on-performance` が参照する基準値を別途定義する。

> メモ: 本章は Draft から Working Draft へ移行した。個別サブコマンドの詳細実装例および JSON スキーマは次版で追加予定。
