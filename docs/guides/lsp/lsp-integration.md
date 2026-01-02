# LSP / IDE 連携ガイド

> 目的：Reml で生成した構文ハイライト・補完・診断情報を、Language Server Protocol (LSP) を通じて IDE に提供するための実装指針を整理する。

## 1. サービス構成

1. `reml-run lsp --stdio` を起動し、LSP サーバとして動作させる。
2. エディタは `initialize` → `initialized` → `textDocument/didOpen` の順にメッセージを送信。
3. サーバ側は `RunConfig.extensions["lsp"]` を設定した状態で `run_with_lsp` を呼び出し、構文情報/診断を生成する。

### 1.1 RunConfig 例

```reml
let cfg = {
  trace = false,
  locale = Some(Locale::new("ja-JP")),
  extensions = {
    lsp = {
      highlight = true,
      completion = true,
      codeActions = true,
      semanticTokens = true,
      syntaxHighlight = true
    },
    i18n = {
      sources = ["workspace://l10n/ja"],
      fallback = "en-US"
    },
    logging = { format = "json" },
    audit = Some(|event| audit_log(event))
  }
}
```

- `extensions.lsp.highlight` : トークン種別を LSP の `SemanticTokens` へ変換。
- `extensions.lsp.completion` : プラグイン capability (`parser.requires`) を参照し DSL 別の補完候補を生成。
- `extensions.lsp.codeActions` : FixIt 情報を LSP CodeAction として返す。
- `extensions.lsp.semanticTokens` : 拡張モジュール。`../runtimeruntime-bridges.md` と同様に構造化ログと連動。
- `locale` が `Some` のときは `RunConfig` から `PrettyOptions` へロケールを伝搬し、診断メッセージと期待テンプレートの両方を同じ言語で整形する。
- `extensions.i18n` は LSP 側の翻訳カタログの監視・ホットリロード設定を保持し、`workspace/didChangeWatchedFiles` で更新を拾う。

### 1.2 ロケールネゴシエーション

1. クライアントは `initialize` の `params.locale`、または `initializationOptions.i18n.locale` に優先ロケールを指定する。サーバはこの
   値を `RunConfig.locale` に設定し、`PrettyOptions` の既定値と `extensions["i18n"].active_locale` に反映する。
2. `initialize` でロケールが指定されなかった場合は `REML_LOCALE` → `LANG` を参照し、いずれも無ければ `Locale::EN_US` を選択す
   る。フォールバック時は `window/showMessage` (`Warning`) で一度だけ通知し、以降は `RunConfig.extensions["i18n"].fallback_hits`
   をインクリメントするだけに留める。
3. `workspace/didChangeConfiguration` で `reml.language.locale` が更新されたら、差分検出後に `RunConfig.locale` と
   `PrettyOptions` を再構築し、既存セッションの診断を `textDocument/publishDiagnostics` で再送する。未指定 (`null`) に戻された場
   合は環境変数→既定値の順で再解決する。
4. LSP 側で翻訳カタログをホットリロードしたら `extensions["i18n"].catalog_version` を更新し、次回診断整形時に `message_key` と
   `locale` の組でキャッシュを引き直す。

## 2. メッセージマッピング

| LSP メッセージ | Reml 側処理 | 備考 |
| --- | --- | --- |
| `textDocument/didOpen` | `run_with_lsp` の初回実行。構文ハイライトと初期診断を送信 | `to_lsp_diagnostics` を使用 |
| `textDocument/didChange` | 増分差分 (`diff`) を `run_stream`/`resume` へ渡し再解析 | `Continuation` を活用 |
| `textDocument/completion` | `completionProvider` から DSL 固有候補を生成 | capability に応じて候補をフィルタ |
| `textDocument/codeAction` | FixIt テンプレートを `CodeAction` に変換 | `audit_id` を `data` フィールドへ埋め込み |
| `workspace/didChangeConfiguration` | `Config.compare` を利用して設定差分を検証 | 監査ログへ `change_set` を記録 |

### 2.1 モジュール解決支援

- `textDocument/completion` で `use` 文や `module` ヘッダ内にカーソルがある場合、名前解決の探索順（同一モジュール → 親 → ルート → プレリュード）を反映した候補リストを返します。最上位候補として `self`, `super`, `::` を提示し、`super` は連続入力に応じて `super.super` などを生成します。
- `use` の別名指定（`as`）はドキュメントシンボルの別名テーブルに記録し、衝突検知で既存シンボルと重複した場合に `Diagnostic` (`namespace.conflict`) を返します。再エクスポート（`pub use`）で公開名が変わる際は、モジュールシグネチャへ `exported=true` フラグを付与して IDE 側の API ビューと同期します。

## 3. 診断と監査の連携

```reml
fn to_lsp_diagnostics(diags: List<Diagnostic>) -> List<LspDiagnostic> =
  diags.map(|d| LspDiagnostic {
    range = span_to_range(d.at),
    severity = map_severity(d.severity),
    message = d.message,
    code = d.code,
    data = Some(json!({
      "message_key": d.message_key,
      "locale": d.locale,
      "locale_args": d.locale_args,
      "domain": d.domain,
      "audit_id": d.audit_id,
      "change_set": d.change_set,
      "severity_hint": d.severity_hint,
      "stream_meta": d.stream_meta,
      "quality_report_id": d.quality_report_id
    }))
  })
```

### 3.1 `display_width` を利用した列同期

- `span_to_range` では 1-4 §G.1 / 2-5 §B-11 / 3-3 §5.1 の規約に従い、行頭から対象スパンまでを `Core.Text.slice_graphemes` で抽出し、`Core.Text.display_width` で列オフセットと長さを算出する。ASCII 長に基づく `len()` や手動の `String.grapheme_at` 反復は禁止。
- LSP の `Range` と CLI 抜粋表示を一致させるため、`RunConfig` が提供する `Input.g_index` / `cp_index` キャッシュを再利用し、`span_to_range` 内で O(1) の列計算を行う。必要な補助データ（グラフェム幅ベクトルなど）が無い場合は、`Diagnostic.extensions["lsp"].display_cache` に格納してクライアントと共有する。
- 未更新 IDE の互換性を保つには、`Diagnostic.extensions["lsp"].utf16_len` といった UTF-16 コード単位の補助情報を保持しつつ、下線描画自体は display width 基準で行う。
- CI テストでは結合文字列（例: `"Å"`, `"👨‍👩‍👧"`）を含むスナップショットを追加し、CLI (`reml-run lint --format text`) と LSP (`textDocument/publishDiagnostics`) が同一の列開始/終了値を出力することを検証する。

- `audit_id` を IDE 側で保持し、承認フローや差分レビューに利用。
- `change_set` を JSON として埋め込むことで、設定差分を IDE 上で表示可能。
- `severity_hint` に基づき、IDE 側で「ロールバック推奨」「再試行可」などのガイドを提示できる。
- `stream_meta` フィールドは `StreamEvent`/`ContinuationMeta` のサマリを格納し、ライブ補完やバックプレッシャ指標をステータスバーへ表示できる。
- `quality_report_id` には `QualityReport.audit_id` の参照を保持し、データ品質診断を IDE から直接リンクできる。

### 3.2 多言語診断の運用モデル

1. サーバは `PrettyOptions.locale` に基づく整形済み文字列を `message` へ格納しつつ、`data.message_key` と `data.locale` を併記
   する。クライアントは `message_key` で翻訳テーブルを引き、ユーザ設定ロケールと異なる場合は `locale_args` を用いて再整形できる。
2. `workspace/didChangeConfiguration` でクライアント側ロケールが変わった際は、既存診断の `data.message_key` + `data.locale_args`
   を使って即座に再翻訳し、`message` がサーバロケールであっても UI 上ではクライアントロケールへ差し替えられる。
3. `Diagnostic.data.locale` と `extensions["i18n"].catalog_version` を突合することで、IDE は翻訳カタログが古い場合に差分取得を促
   すトースト通知を表示できる。
4. **未対応クライアントへのフォールバック例**：`data` を無視する LSP クライアントではサーバ側で整形した `message`（環境から解決
   したロケール、既定は `en-US`）がそのまま表示される。`RunConfig.locale` を `None` のままにすれば英語 UI を前提とした互換モード
   を維持できる。

## 4. ハイライト・補完ツールチェーン

1. `SemanticTokensLegend` を capability ごとに登録（例: `template.directive`, `config.field`）。下表に標準トークンとカスタムトークンの分類例を示す。
2. エディタが `semanticTokens/full` を要求 -> パーサがトークン種別を返却。
3. プラグイン側で追加トークンを登録する場合、`register_capability` で `parser.syntax.highlight` を宣言。

| Semantic Token | 対応 capability | 用途 |
| --- | --- | --- |
| `keyword.control.reml` | (標準) | `if` / `match` など言語キーワード |
| `type.schema` | `"config"` | `schema` DSL で宣言された型 |
| `property.template` | `"template"` | テンプレート DSL のディレクティブ |
| `function.dsl` | プラグイン登録 | DSL が提供する関数名 |
| `modifier.syntax-highlight` | `"parser.syntax.highlight"` | syntax highlight 拡張で追加されるトークン |


### 4.1 SemanticTokensLegend の標準分類

| Legend | scope | 説明 |
| --- | --- | --- |
| `namespace.dsl` | Core | `use` 宣言で導入される DSL 名前空間 |
| `type.config.required` | Config | 必須フィールドを表す型シンボル（`Core.Config` の `requires` でマーク） |
| `type.config.optional` | Config | 任意フィールド。`optional` で生成され、警告時に別カラーで表示 |
| `parameter.template.slot` | Template | テンプレート DSL のスロット名（`{{slot}}`） |
| `property.data.metric` | Data | `Core.Data` の統計値（`mean`, `stddev` 等） |
| `modifier.audit` | Audit | `audit` 効果で追跡される領域（`audit_id` を埋め込む） |

Legend の拡張はプラグインが `register_capability` で `semantic_tokens.legend` を追加し、IDE 初期化時に `workspace/semanticTokens/refresh` を送信して再配布する。



## 5. CLI との一貫性

```bash
reml-run lint config.ks --format json --domain config   | jq '.diagnostics[] | {code, message, audit_id}'
```

- CLI と LSP の診断結果は同じ JSON フォーマットを共有し、CI/CD と IDE の結果照合が容易。
- `audit_id` をキーにランタイムガイド（`../runtimeruntime-bridges.md`）で差分適用ログと結合する。

## 6. 運用メモ

- **semanticTokens**: 上記表に基づき、標準トークンと capability 別トークンを組み合わせて `SemanticTokensLegend` を構築する。未定義のトークンは `namespace` や `type` にフォールバック。
- **CodeAction 承認フロー**: FixIt 適用時は `audit_id` と関連する変更をプレビューし、ユーザが確認後に `workspace/applyEdit` を実行する。承認済みアクションは `PluginCapability` の `traits`（例: `"auto-fix"`）で識別。
  1. サーバが `textDocument/codeAction` で `data.audit_id` を返却。
  2. クライアントは承認ダイアログを表示し、承認後に `workspace/executeCommand`(`reml.applyFix`) を送信。
  3. サーバは `audit.log("lsp.codeAction", { audit_id, command })` を記録し、`workspace/applyEdit` を発行。

> 本ガイドはフェーズ3でさらに事例と図解を追加する予定です。

## 7. 互換性チェックリスト

| 項目 | 内容 | 参照 |
| --- | --- | --- |
| `Diagnostic.data.stream` | `StreamDriver` ベースの差分解析をサポートするクライアントは `ContinuationMeta` のサマリを解釈できるか | 2-6 実行戦略 §F |
| `Diagnostic.data.quality_report_id` | データ品質診断リンクが `docs/guides/ecosystem/data-model-reference.md#quality-report-schema` に従うか | データモデルリファレンス |
| `gc.stats` メッセージ | IDE が GC メトリクスを取得する場合 `../runtimeruntime-bridges.md#10-2` のスキーマを処理できるか | Runtime Bridges |
| CLI / LSP 診断整合 | `reml-run lint --format json` と LSP 診断が同一 `audit_id` / `code` を保持しているか | 本ガイド §5 |
| レガシークライアント | `Diagnostic.data` を無視しても従来どおり動作できるか（フォールバックポリシーの確認） | 互換性ポリシー |

互換性テストは LSP サーバの CI で `lsp-sample-client`（仮称）によるスナップショット比較を行い、差分はチーム内のバックログ（Issue トラッカー等）にまとめて共有する。
