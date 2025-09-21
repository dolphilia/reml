# LSP / IDE 連携ガイド

> 目的：Reml で生成した構文ハイライト・補完・診断情報を、Language Server Protocol (LSP) を通じて IDE に提供するための実装指針を整理する。

## 1. サービス構成

1. `reml-run lsp --stdio` を起動し、LSP サーバとして動作させる。
2. エディタは `initialize` → `initialized` → `textDocument/didOpen` の順にメッセージを送信。
3. サーバ側は `RunConfig` に `lsp` オプションを含めた状態で `run_with_lsp` を呼び出し、構文情報/診断を生成する。

### 1.1 RunConfig 例

```reml
let cfg = {
  lsp = {
    highlight = true,
    completion = true,
    codeActions = true,
    semanticTokens = true,
    syntaxHighlight = true
  },
  log_format = "json",
  audit = Some(|event| audit_log(event))
}
```

- `highlight` : トークン種別を LSP の `SemanticTokens` へ変換。
- `completion` : プラグイン capability (`parser.requires`) を参照し DSL 別の補完候補を生成。
- `codeActions` : FixIt 情報を LSP CodeAction として返す。
- `semanticTokens` : 拡張モジュール。`guides/runtime-bridges.md` と同様に構造化ログと連動。

## 2. メッセージマッピング

| LSP メッセージ | Reml 側処理 | 備考 |
| --- | --- | --- |
| `textDocument/didOpen` | `run_with_lsp` の初回実行。構文ハイライトと初期診断を送信 | `to_lsp_diagnostics` を使用 |
| `textDocument/didChange` | 増分差分 (`diff`) を `run_stream`/`resume` へ渡し再解析 | `Continuation` を活用 |
| `textDocument/completion` | `completionProvider` から DSL 固有候補を生成 | capability に応じて候補をフィルタ |
| `textDocument/codeAction` | FixIt テンプレートを `CodeAction` に変換 | `audit_id` を `data` フィールドへ埋め込み |
| `workspace/didChangeConfiguration` | `Config.compare` を利用して設定差分を検証 | 監査ログへ `change_set` を記録 |

## 3. 診断と監査の連携

```reml
fn to_lsp_diagnostics(diags: List<Diagnostic>) -> List<LspDiagnostic> =
  diags.map(|d| LspDiagnostic {
    range = span_to_range(d.at),
    severity = map_severity(d.severity),
    message = d.message,
    code = d.code,
    data = Some(json!({
      "domain": d.domain,
      "audit_id": d.audit_id,
      "change_set": d.change_set,
      "severity_hint": d.severity_hint,
      "stream": d.stream_meta,
      "quality_report": d.quality_report_id
    }))
  })
```

- `audit_id` を IDE 側で保持し、承認フローや差分レビューに利用。
- `change_set` を JSON として埋め込むことで、設定差分を IDE 上で表示可能。
- `severity_hint` に基づき、IDE 側で「ロールバック推奨」「再試行可」などのガイドを提示できる。
- `stream` フィールドは `StreamEvent`/`ContinuationMeta` のサマリを格納し、ライブ補完やバックプレッシャ指標をステータスバーへ表示できる。
- `quality_report` には `QualityReport.audit_id` の参照を保持し、データ品質診断を IDE から直接リンクできる。

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
- `audit_id` をキーにランタイムガイド（`guides/runtime-bridges.md`）で差分適用ログと結合する。

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
| `Diagnostic.data.quality_report` | データ品質診断リンクが `guides/data-model-reference.md#quality-report-schema` に従うか | データモデルリファレンス |
| `gc.stats` メッセージ | IDE が GC メトリクスを取得する場合 `guides/runtime-bridges.md#10-2` のスキーマを処理できるか | Runtime Bridges |
| CLI / LSP 診断整合 | `reml-run lint --format json` と LSP 診断が同一 `audit_id` / `code` を保持しているか | 本ガイド §5 |
| レガシークライアント | `Diagnostic.data` を無視しても従来どおり動作できるか（フォールバックポリシーの確認） | 互換性ポリシー |

互換性テストは LSP サーバの CI で `lsp-sample-client`（仮称）によるスナップショット比較を行い、差分は `reml-backlog.md` の「LSP/CLI 互換性」セクションに記録する。
