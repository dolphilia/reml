# 4.3 Developer Toolchain

> 目的：IDE/LSP、フォーマッタ、リンター、デバッガー、プロファイラーなど Reml 開発者ツールチェーンの仕様を統合し、言語仕様・標準ライブラリとの結節点を明確化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 執筆中（Working Draft） |
| 参照文書 | [guides/lsp-integration.md](../guides/lsp/lsp-integration.md), [guides/config-cli.md](../guides/tooling/config-cli.md), [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md) §4.1, §4.2 |
| 関連章 | [3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md), [3-7-core-config-data.md](3-7-core-config-data.md), [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md), [4-1-package-manager-cli.md](4-1-package-manager-cli.md) |

## 1. LSP 実装ロードマップ

### 1.1 サーバー責務
- `RemlLanguageServer` は LSP 3.17 をターゲットとし、`textDocument/*`, `workspace/*`, `window/*` の標準メソッドを実装する。
- `Initialize` 応答には `capabilities.experimental.dsl` を含め、DSL 固有情報の交換を行う。`DslExportSignature` を IDE へ提供し、補完・シグネチャヘルプを強化する。
- `workspace/didChangeConfiguration` を受け取った際は `reml.toml` の再読込を行い、`ClientCapabilities` の `configuration` が無効な場合は `Warning` を返す。

### 1.2 コードインテリジェンス
- 補完：標準構文、DSL エクスポート、ターゲット capability を含めた候補を提示する。候補には `kind`, `detail`, `data` を含め、`data` に DSL ID を格納する。
- 定義ジャンプ：ソース内定義のほか、DSL エクスポートであれば `dsl://` スキームの仮想 URI を返し、`../guides/dsl-gallery.md` の参照を含める。
- ドキュメント：`hover` 応答に `Markdown` 形式で例と effect 情報を併記し、`Core.Docs`（計画中 API）と連携する。
- コードアクション：`reml fmt`/`reml check` の提案結果を利用して自動修正を提示する。結果は `command = reml.applyFix` を介して CLI サブプロセスを呼び出す。

### 1.3 診断連携
- LSP サーバーは CLI の `--output lsp` を内部的に呼び出し、`CliDiagnosticEnvelope` を `PublishDiagnostics` に変換する。`severity` は `Error=1`, `Warning=2`, `Information=3`, `Hint=4` にマッピングする。
- `Diagnostic.tags` に `Unnecessary`, `Deprecated` を追加し、`build`/`check` の情報を IDE 上で区別できるようにする。

## 2. フォーマッタ & リンター

### 2.1 フォーマッタ
- `reml fmt` はライブラリ `Core.Format` を利用し、言語仕様 ([1-1](1-1-syntax.md)) の構文木から `RewritePlan` を生成する。フォーマットは**安定性**（同じ入力に対して常に同じ出力）と**コメント保持**（`DocComment` の位置保持）を保証する。
- 設定ファイル `reml-format.toml` の主なキー：
  - `line_width`（既定 100）
  - `indent_style`（`space` or `tab`）
  - `dsl_section.ordering`（DSL セクション優先順位）
- CLI オプション `--config <path>`、`--stdin`, `--check` を提供し、`--emit-changes` で差分パッチを JSON 形式で返す。

### 2.2 リンター
- `reml check` は `Core.Lint` モジュールを使用して静的解析と規約チェックを実行する。規則は `lint.toml` の `rules.<name>.level` で制御し、`allow`, `warn`, `deny` をサポートする。
- 効果システムに基づく解析（[1-3](1-3-effects-safety.md)）を利用し、`unsafe` な DSL との境界を警告する。
- LSP 連携時には `CodeAction` として修正提案を返し、CI 用には `--report sarif` オプションで SARIF を出力する。

## 3. Toolchain 配布とターゲット管理

### 3.1 物理配置
- 標準ライブラリ (`Core.*`) とランタイムはターゲット単位で事前ビルドし、`$REML_TOOLCHAIN_HOME/profiles/<profile_id>` 配下に格納する。`std/<triple>/<hash>` と `runtime/<runtime_revision>` を分離し、更新の粒度を最適化する。
- `toolchain-manifest.toml` は以下のキーを持つ：
  - `profile_id`（必須）
  - `runtime_revision`
  - `stdlib_version`
  - `capabilities`
  - `artifacts`（ハッシュとファイル名のマッピング）
  - `installed_at`, `last_verified_at`, `signature`

### 3.2 CLI サブコマンド

| コマンド | 説明 |
| --- | --- |
| `reml toolchain list` | インストール済みプロファイルを表示。`--json` でマニフェストをそのまま出力。 |
| `reml toolchain install <profile>` | レジストリまたは URL から `std`/`runtime` を取得。`target.capability.unknown` 検出時はロールバック。 |
| `reml toolchain update <profile>` | 新しい `hash` が提供された場合のみ差分更新。旧版は `archive/` に退避。 |
| `reml toolchain prune` | 未使用ハッシュを削除。`--keep-latest <n>` で保持数を指定。 |
| `reml toolchain verify <profile>` | ハッシュ・署名・ capability の整合を検証し、`CliDiagnosticEnvelope` に `domain = Target` を設定。 |

- `install`/`update` は `download -> verify -> unpack -> register` の 4 段を踏む。各段の所要時間を `summary.stats` へ記録し、性能回帰を検知する。

### 3.3 キャッシュと検証
- ダウンロードキャッシュは `REML_TOOLCHAIN_CACHE` に配置し、ハッシュ一致時のみ再利用。破損検知時は自動削除。
- 署名が存在する場合は Ed25519 で検証し、`toolchain-manifest.toml` に `verify.signature = true` を記録する。
- CI 環境では `reml toolchain verify --all --output json` を推奨し、結果を `../guides/ci-strategy.md` に沿って収集する。

### 3.4 IDE 連携
- IDE は `toolchain-manifest.toml` の `installed_at` を監視し、更新検知でインデックスを再生成する。
- `REML_TOOLCHAIN_HOME` の変更は LSP の `workspace/didChangeConfiguration` 経由で通知する。未通知状態で CLI 経由の変更があった場合はサーバーが `window/showMessage` で警告する。

## 4. テストランナー

### 4.1 `Core.Test` 連携
- テスト DSL は `describe`/`it` 構文と `async` 実行をサポートする予定で、`effect` 制約を `TestPlan` にエンコードする。
- `reml test` は `Core.Test.Runner` を呼び出し、`TestResult` を `CliDiagnosticEnvelope.summary.tests` に集計する。

### 4.2 並列・シリアル実行
- デフォルトはテストケース単位の並列実行。`--parallel (auto|n)` でスレッド数を制御し、`effect` に `RequiresIsolation` が含まれる場合は自動的にシリアルキューへ移動する。
- スナップショットテストは `snapshots/` ディレクトリに保存し、`--update-snapshots` を指定しない限り変更を拒否する。

### 4.3 レポート
- `--report junit` で JUnit XML を出力。`--report html` は Phase 2 の追加予定。
- テスト失敗は `domain = Test` の診断として IDE に通知し、`failures[*].target_profile` でターゲット情報を提供する。

## 5. デバッガー

### 5.1 CLI 連携
- `reml debug`（計画中）は DSL 境界を跨いだステップ実行を提供し、`TraceSink`（[3-8 §7](3-8-core-runtime-capability.md#dsl-capability-utility)）に計測情報を流す。
- ブレークポイントは `SourceBreakpoint` と `DslBreakpoint` の 2 種類を持ち、後者は DSL 名とエクスポートシンボルで指定する。

### 5.2 逆方向トレース
- 実行履歴を `TraceEnvelope` に蓄積し、最大 10,000 ステップ（構成可能）を保持する。逆方向ステップ時は `effect` の巻き戻しが安全かを検証し、危険な場合は `Warning` を表示する。

## 6. プロファイラー

### 6.1 計測モデル
- `reml profile` はサンプリング（既定）と計測（instrumentation）の 2 モードを提供する。
- サンプリング間隔は 1ms〜10ms の範囲で指定でき、計測対象 DSL の `PerformanceHint`（[guides/dsl-performance-playbook.md](../guides/dsl/dsl-performance-playbook.md)）を尊重する。

### 6.2 ホットスポット可視化
- 出力は `profile.json`（火炎グラフ互換）と `profile.html`（Phase 2 で追加）を生成する。
- DSL パイプライン単位の計測結果を `dsl_segments[*]` に格納し、パフォーマンス回帰を検知する。

## 7. AI 支援ツール

### 7.1 コマンド体系
- `reml ai-suggest`：コード補完・リファクタ提案を取得し、`AuditCapability` を用いて利用ログを記録する。
- `reml ai-review`：指定コミットまたは差分を解析し、潜在的な互換性問題を `Diagnostic.domain = Review` として出力する。

### 7.2 安全ガードライン
- AI 連携時は `../guides/ai-integration.md` のポリシーに従い、個人情報・シークレットのリークを防ぐフィルタリングを適用する。
- すべての AI 出力は人間による確認が前提であり、`--auto-apply` は提供しない。CI では `--summary-only` モードを推奨する。

## 8. 今後の作業
- LSP の JSON-RPC トランスクリプト例とテストケースを追補し、`../guides/lsp/lsp-integration.md` と同期させる。
- `Core.Test` API のドラフトを Chapter 3 に追加し、本章のテストランナー節と往復参照する。
- プロファイラーの HTML レポート仕様および AI ツールの利用制限フローを付録として整理する。

> メモ: 本章は Working Draft として、主要ツールの責務と CLI との統合ポイントを明示した。詳細 API リファレンスは個別ガイドおよび Chapter 3 の更新に合わせて拡張する。
