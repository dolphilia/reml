# 1.6 開発者体験整備計画

## 進捗状況（2025-10-10 更新）

| フェーズ | タスク | ステータス | 完了率 |
|---------|--------|-----------|--------|
| Week 14 | 診断出力システム | ✅ 完了 | 100% |
| Week 15 | トレース・ログ | ✅ 完了 | 100% |
| Week 16 | ヘルプ・ドキュメント & サンプル | ⏸️ 進行中 | 50% |

**全体進捗**: 63% (5/8タスク完了)

### 完了した項目（Week 14）
- ✅ ソースコードスニペット表示実装（`diagnostic_formatter.ml`）
- ✅ カラーコード対応実装（`color.ml`）
- ✅ JSON出力フォーマット実装（`json_formatter.ml`）
- ✅ CLIオプション管理のモジュール化（`options.ml`）
- ✅ 診断フォーマット仕様書作成（`docs/guides/diagnostic-format.md`）
- ✅ CLI診断テスト修正完了（`test_cli_diagnostics.ml`）

### 完了した項目（Week 15）
- ✅ `--trace` 実装とフェーズ統合（`cli/trace.ml`, `src/main.ml`）
- ✅ GC 統計に基づくメモリアロケーション計測とトレースサマリー出力
- ✅ 統計カウンタ実装（`cli/stats.ml`）と CLI オプション `--stats`
- ✅ CLI トレース/統計テスト追加（`tests/test_cli_trace.ml`）
- ✅ トレース出力ガイド作成（`docs/guides/trace-output.md`）

### 次のアクション

**Week 16 フォーカス（優先度: High）**:
1. **ヘルプ・ドキュメント整備**:
   - ✅ `--help` 出力のセクション化・使用例追加（`options.ml`）
   - ✅ `docs/guides/cli-workflow.md` の初稿整備とトレースガイドへの相互リンク
   - ⏳ `examples/cli/` サンプル整備（ベース追加済み、フォローアップ継続）
   - ❌ man ページ生成と同期テンプレート整備
2. **統計機能の拡張計画**:
   - フェーズ別時間比率やピークメモリなど残タスクの洗い出し
   - JSON/CSV など外部連携フォーマットの要件整理

## 目的
- `remlc-ocaml` CLI を Phase 1 で整備し、開発者が解析結果・IR・診断を観測できる開発体験を提供する。
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) のフィールド定義に従い、出力フォーマットを将来のセルフホスト版と一致させる。

## スコープ
- **含む**: CLI インタフェース設計、サブコマンド実装、出力フォーマット整備、診断メッセージの国際化方針（日本語 + 原語併記）、ドキュメント整備。
- **含まない**: GUI、LSP サーバ、IDE プラグイン。これらは Phase 2 以降に計画。
- **前提**: Parser/TypeChecker/Core IR/LLVM が CLI から呼び出せる状態であること。

## 作業ディレクトリ
- `tooling/cli` : CLI 実装、オプション処理、ヘルプ出力
- `tooling/cli/tests`（想定）: CLI スナップショット・メトリクス収集テスト
- `compiler/ocaml/src` : CLI から呼び出すパイプライン統合ポイント
- `docs/guides` : CLI 使⽤ガイドの更新。とくに `docs/guides/cli-workflow.md`
- `tooling/ci` : CLI を exercise する CI ジョブ

## 作業ブレークダウン

### 1. CLIアーキテクチャ設計（9-13週目並行）
**担当領域**: コマンドラインインタフェース基盤

1.1. **エントリポイント設計**
- `remlc-ocaml` メインモジュールの作成
- コマンドライン引数パーサ（`Cmdliner` または手動実装）
- サブコマンド構造の検討（`compile`, `check`, `emit` 等）

1.2. **オプション体系設計**
- 入力: `<file.reml>` または stdin (`-`)
- 出力制御: `--emit-ast`, `--emit-tast`, `--emit-core`, `--emit-ir`
- コンパイルオプション: `-O0`/`-O1`, `--target`, `--link-runtime`
- 診断オプション: `--trace`, `--verbose`, `--color=auto|always|never`
- 出力先: `--out-dir`, `--out-file`

1.3. **設定ファイル対応**
- `reml.toml` 形式の検討（Phase 2で本格化）
- CLI優先度の明確化（CLI > 設定ファイル > デフォルト）

**成果物**: `cli/main.ml`, CLI設計ドキュメント

### 2. パイプライン統合（13-14週目）
**担当領域**: コンパイルフロー統合

2.1. **フェーズオーケストレーション**
- Parser → Typer → Core IR → LLVM の順次実行
- 各フェーズの入出力整合性チェック
- エラー時の早期終了と診断出力

2.2. **中間生成物の管理**
- `--emit-*` 時のファイル出力処理
- 一時ディレクトリの管理と削除
- デバッグモードでの中間ファイル保持

2.3. **並行処理準備**
- 複数ファイル入力の対応（Phase 2で本格化）
- 依存解析の基礎設計

**成果物**: `cli/pipeline.ml`, パイプライン統合

### 3. 診断出力システム（14週目）✅ 完了
**担当領域**: エラー・警告の表示

3.1. **Diagnostic構造体実装** ✅
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) 準拠のフィールド定義
- Span情報との統合
- 重大度レベル（error, warning, info, hint）

3.2. **テキスト出力フォーマット** ✅
- ✅ ソースコードスニペット表示（前後1行）
- ✅ カラーコード対応（エラー=赤、警告=黄等）
- ✅ ポインタ表示（`^^^` での位置指示）
- ✅ 日本語メッセージ + 英語キーワード併記

3.3. **JSON出力フォーマット** ✅
- ✅ JSON出力実装（`json_formatter.ml`）
- ✅ 機械判読可能な構造化出力
- ⏳ JSONスキーマ定義（Phase 2 で正式化）
- ⏳ LSP互換性の準備（Phase 2）

**成果物**: ✅ `cli/diagnostic_formatter.ml`, `cli/json_formatter.ml`, `cli/color.ml`, 診断出力実装完了

### 4. トレース・ログ機能（14-15週目）✅ 完了 / ⏸️ 一部保留
**担当領域**: デバッグ支援とオブザーバビリティ

4.1. **`--trace` 実装** ✅
- 各フェーズの開始・終了ログ（`src/main.ml` へ統合済み）
- 時間計測（経過時間、累積時間）および GC 統計ベースのメモリアロケーション計測
- トレースサマリー出力（`Cli.Trace.print_summary`）

4.2. **ログレベル管理** ⏸️
- ✅ `--verbose` オプション定義（`options.ml`）・環境変数 `REMLC_LOG` の初期化
- ❌ ログレベルに応じた出力量制御（Phase 2 で実装予定）
- ❌ デバッグ情報カテゴリー分離（TODO 継続）

4.3. **統計情報収集** ✅
- パースしたトークン数、AST ノード数、unify 呼び出し回数などをカウンタ実装
- `Cli.Stats.print_stats`・`Cli.Stats.to_json` を提供
- テスト `test_cli_trace.ml` でカウンタ挙動を検証

**成果物**: `cli/trace.ml`, `cli/stats.ml`, `docs/guides/trace-output.md`, `tests/test_cli_trace.ml`

### 5. 統計サマリ出力（15週目）⏸️ 部分完了
**担当領域**: パフォーマンス可視化

5.1. **時間計測** ⏸️
- ✅ フェーズ別実行時間の取得（`Cli.Trace.print_summary`）
- ❌ パーセンテージ表示やフェーズ別ランキング
- ❌ トレースと統計の統合ビュー（Phase 2 へ繰越）

5.2. **メモリ使用量** ❌
- プロセスメモリ使用量のサンプリング
- ピークメモリの記録
- GC統計（OCaml GC）

5.3. **メトリクス出力** ⏸️
- ✅ `Cli.Stats.to_json` による JSON 取得
- ❌ `--metrics` フラグおよびファイル出力
- ❌ `0-3-audit-and-metrics.md` への自動書き出し
- ❌ CSV 等のバルクエクスポート

**成果物**: ❌ `cli/metrics.ml`, メトリクス機能（未実装）

### 6. ヘルプ・ドキュメント生成（15週目）⏸️ 進行中
**担当領域**: ユーザー支援

6.1. **`--help` 実装** ✅
- ✅ セクション化されたヘルプと使用例（`options.ml` 内 `print_full_help`）
- ✅ `--help` / `-help` での詳細ヘルプ表示
- ⏳ ログレベル別の説明・環境変数一覧の追記

6.2. **manページ生成** ❌
- `remlc-ocaml.1` マニュアルページの作成
- インストール時の配置（`/usr/local/share/man`）
- 🔁 **同期ポリシー**: `print_full_help`（`options.ml`）を一次ソースとし、man ページは同一内容をテンプレート経由で生成する。Phase 1-7 で `docs/guides/cli-help-template.md`（仮）を追加し、ヘルプ更新時は (1) `options.ml` → (2) テンプレート → (3) man ページの順に反映させるチェックリストを運用する。

6.3. **ユーザーガイド作成** ⏸️
- ✅ `docs/guides/cli-workflow.md` 初稿（ワークフロー・トレース活用・CI 連携）
- ❌ サンプルプロジェクトとの連動、トラブルシューティング詳細
- ❌ CLI テンプレート（`examples/cli/`）の整備

**成果物**: `options.ml` 拡張済みヘルプ、`docs/guides/cli-workflow.md` 初稿、man ページ・サンプル整備は未了

### 7. サンプルプロジェクト整備（15-16週目）⏸️ 部分完了
**担当領域**: 実践的な使用例

7.1. **サンプルコード拡充** ⏸️
- ✅ `examples/cli/` ディレクトリ新設、`add.reml`/`type_error.reml`/`trace_sample.reml` 追加
- ⏳ Hello World 以外の段階的サンプル（ベンチマーク・診断応用）の拡張
- ❌ CLI サンプル README の自動生成（現状は手動）

7.2. **ワークフロー実演** ⏸️
- ✅ `examples/cli/README.md` で基本手順と関連コマンドを提示
- ❌ 各 `--emit-*` オプションの出力例（スクリーンショット・ログ）の整備
- ❌ CI での実行例スクリプト化

7.3. **ベンチマークスイート準備** ❌
- パフォーマンス計測用のサンプルセット
- 回帰テストの基準値設定

**成果物**: `examples/cli/README.md`、`examples/cli/*.reml`（ベースセット完了）、追加サンプル & 自動化フローは継続タスク

### 8. CI統合とテスト（16週目）⏸️ 部分完了
**担当領域**: CLI品質保証

8.1. **CLI統合テスト** ⏸️
- ✅ 診断出力テスト（`test_cli_diagnostics.ml`）
- ❌ 各オプションの網羅的動作検証
- ❌ エラー時の正しい終了コード確認

8.2. **スモークテスト** ❌
- 基本的なコンパイルフローの自動テスト
- `--emit-*` オプション全パターンの検証
- CI での定期実行

8.3. **ドキュメント整備** ⏸️
- ✅ 診断フォーマット仕様（`docs/guides/diagnostic-format.md`）
- ❌ CLI アーキテクチャの技術文書
- ❌ Phase 2への引き継ぎ（LSP、設定ファイル）

**成果物**: ⏸️ 診断テストのみ完了、❌ 完全なCLIテストスイート（未完）

## 成果物と検証
- CLI コマンドが `dune exec remlc-ocaml -- --help` で利用可能になり、各オプションが CI のスモークテストで網羅される。
- `Diagnostic` JSON のスキーマを `jsonschema` 形式で管理し、CI で検証。
- ドキュメント (`docs/guides/llvm-integration-notes.md` または新規ガイド) に CLI 利用手順が掲載される。

## リスクとフォローアップ
- 出力フォーマットの変更がフェーズ間で発生しやすいため、バージョンタグを付与し後方互換性を `0-4-risk-handling.md` で管理。
- CLI のオプションが増えすぎると UX が低下するため、Phase 2 で LSP 計画へ引き継ぎ、現段階では観測用途に絞る。
- 多言語対応は Phase 4 のエコシステム移行で本格化するため、日本語テキストに英語キーワードを括弧書きで併記する程度に留める。

## 参考資料
- [1-0-phase1-bootstrap.md](1-0-phase1-bootstrap.md)
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [guides/llvm-integration-notes.md](../../guides/llvm-integration-notes.md)
