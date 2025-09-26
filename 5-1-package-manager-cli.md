# 5.1 Package Manager & CLI（ドラフト）

> 目的：Reml エコシステムの基盤となる公式 CLI (`reml`) およびパッケージマネージャーの仕様を定義し、Chapter 1-3 で規定された言語・標準API・ランタイム機能と統合する。現段階では 4-0 の統合計画に基づくアウトラインであり、今後の章立て作業の指針を提供する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 草案（Draft） |
| 参照文書 | [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md), [5-0-ecosystem-integration-plan.md](5-0-ecosystem-integration-plan.md) |
| 主要関連章 | 1-1, 1-2, 1-3, 3-6, 3-7, 3-8 |

## 1. CLI 全体像

- `reml` コマンドの責務、サブコマンド一覧（`new`, `add`, `build`, `test`, `fmt`, `check`, `publish`, `registry`, `target`, `toolchain` など）。
- 0-2 指針の性能・安全性原則との関連付け（構造化エラー、線形性能）。
- `CliDiagnosticEnvelope`（3-6 §9）との統合ポイント。
- クロスコンパイル支援：`TargetProfile`/`RunConfigTarget` を CLI で管理し、`DiagnosticDomain::Target` を通じてフェイルセーフな運用を実現する。

## 2. マニフェスト統合

- `reml.toml` のロード/検証（3-7 §1）を CLI がどのように利用するか。
- `dsl` セクションと `DslCapabilityProfile` の同期フロー（3-8 §7）。
- プロファイルキャッシュ、`reml manifest sync` 等の運用コマンド草案。

## 3. サブコマンド仕様（アウトライン）

### 3.1 `reml new`
- プロジェクトひな形生成、DSL エントリーポイントの初期配置。
- テンプレート選択（config/template/query 等）と `guides/dsl-gallery.md` の連携。

### 3.2 `reml add`
- 依存解決アルゴリズムの要件（Git/Tarball、将来の中央レジストリ）。
- 競合解決ポリシー、バージョン範囲の記法。

### 3.3 `reml build`
- ビルドステージ（解析→型付け→効果検査→コード生成）。
- `CliPhase`（3-6 §9）を使用した診断フェーズの追跡。
- パフォーマンス・セーフティチェック（0-2 指針）を CLI レベルで担保する方法。
- `--target <profile>` / `--target-json <path>` を受け取り、`TargetProfile` を解決して `RunConfig.extensions["target"]` へ注入する。`profile_id` が未解決の場合は `target.profile.missing` を即時に発行し、ビルドを中断（1.1 性能のフェイルストップ）。
- ビルド成果物には `RunArtifactMetadata`（2-6 §B-2-1-a）を添付し、`--emit-metadata <path>` で JSON 出力を選択可能。`RuntimeRevision` や `StdlibVersion` の検証は `target.abi.mismatch` 診断で行い、`--allow-abi-drift` を指定しない限り失敗扱い。

### 3.4 `reml test`
- 標準テストランナー（計画中 `3-10-core-test-support.md`）との接続点。
- 並列・シリアル実行戦略、`effect` 制約の扱い。
- `--target` を `reml build` と同一仕様で受け入れ、テスト対象バイナリが指定ターゲット向けにビルドされたことを `TargetProfile` メタデータで検証する。
- `--runtime smoke` / `--runtime emulator=<name>` でエミュレーション実行を宣言し、`TargetCapability` のカバレッジをメトリクス化（3-6 §7.3）。
- テスト結果の `CliDiagnosticEnvelope.summary.stats` には `target_failures`, `emulator_runs`, `profile_id` を含め、ダッシュボード集計と整合させる。

### 3.5 `reml fmt` / `reml check`
- フォーマッタ・リンターの差分出力、`--check` モードの規定。
- `Diagnostic` と `AuditEnvelope` の使い分け。

### 3.6 `reml publish`
- 4.2 レジストリ仕様との整合性。
- 署名・検証・再現性（deterministic tarball）の要求事項。
- 添付するアーティファクトには `targets = [...]` メタデータを必須化し、`--targets all|listed` でクロスビルド済み成果物を同梱。`target.abi.mismatch` が存在する場合は publish を拒否する。

### 3.7 `reml target`
- `reml target list` — ローカルで利用可能な `TargetProfile` の一覧を表示。`source`（`workspace`, `registry`, `built-in` など）と `runtime_revision` を併記。
- `reml target show <id>` — 指定プロファイルの詳細（`triple`, `capabilities`, `stdlib_version`, `hash` 等）を JSON/人間向け表示で出力。
- `reml target scaffold <id>` — 新規プロファイル定義のテンプレートを `profiles/<id>.toml` として生成。`capabilities` は `capability_name(TargetCapability::...)` を参照して記述。
- `reml target validate <path|id>` — プロファイルの整合性チェック。`TargetCapability` が未登録の場合は即時 `target.capability.unknown` を報告し、`--warn-only` が無い限り失敗。
- `reml target sync` — `Core.Env.infer_target_from_env()` と比較し、`target.config.mismatch` の有無を表示。`--write-cache` で `~/.reml/targets/cache.json` に検出結果を保存し、CI 再実行時の線形性能を維持（0-2 §1.1）。
- すべてのサブコマンドは `CliDiagnosticEnvelope` を使用し、`Diagnostic.domain = Target` を明示する。`--output json` 利用時は `extensions.target` を `diagnostics[*]` に含める。

## 4. 出力形式と UX

- ヒューマンリーダブル出力と JSON/NDJSON/LSP モードの仕様詳細。
- `--summary` / `--fail-on-warning` / `--fail-on-performance` 等の制御フラグ。
- カラーリング、ロケール対応、アクセシビリティ指針。

## 5. セキュリティ・監査要件

- コマンドごとの Capability 要求（3-8 の Registry API、SecurityCapability 参照）。
- 監査ログ最小要件（`run_id`, `command`, `phase`）。
- 署名付きマニフェスト/アーティファクトの検証フロー。

## 6. 今後の執筆タスク

- サブコマンドごとの正式仕様策定。
- CLI オプションリファレンス（短形式/長形式、環境変数対応）。
- エラーコード/終了コードテーブルの整備（3-6 §9 と統一）。
- 性能要件テスト（ベンチマーク）と検証手順を付録として追加。

> メモ: 本書はドラフト段階であり、追って詳細仕様と例示を追加する。初稿作成時は 4-0 の進捗サマリを参照し、章間整合性を確認すること。
