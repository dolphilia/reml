# LSP / IDE 連携ガイド（Draft）

> 目的：Kestrel で生成した構文ハイライト・補完・診断情報を、Language Server Protocol (LSP) を通じて IDE に提供するための実装指針を整理する。

## 1. サービス構成

1. `kestrel-run lsp --stdio` を起動し、LSP サーバとして動作させる。
2. エディタは `initialize` → `initialized` → `textDocument/didOpen` の順にメッセージを送信。
3. サーバ側は `RunConfig` に `lsp` オプションを含めた状態で `run_with_lsp` を呼び出し、構文情報/診断を生成する。

### 1.1 RunConfig 例

```kestrel
let cfg = {
  lsp = {
    highlight = true,
    completion = true,
    codeActions = true,
    semanticTokens = true
  },
  log_format = "json",
  audit = Some(|event| audit_log(event))
}
```

- `highlight` : トークン種別を LSP の `SemanticTokens` へ変換。
- `completion` : プラグイン capability (`parser.requires`) を参照し DSL 別の補完候補を生成。
- `codeActions` : FixIt 情報を LSP CodeAction として返す。
- `semanticTokens` : 拡張（Draft）。`guides/runtime-bridges.md` と同様に構造化ログと連動。

## 2. メッセージマッピング

| LSP メッセージ | Kestrel 側処理 | 備考 |
| --- | --- | --- |
| `textDocument/didOpen` | `run_with_lsp` の初回実行。構文ハイライトと初期診断を送信 | `to_lsp_diagnostics` を使用 |
| `textDocument/didChange` | 増分差分 (`diff`) を `run_stream`/`resume` へ渡し再解析 | `Continuation` を活用 |
| `textDocument/completion` | `completionProvider` から DSL 固有候補を生成 | capability に応じて候補をフィルタ |
| `textDocument/codeAction` | FixIt テンプレートを `CodeAction` に変換 | `audit_id` を `data` フィールドへ埋め込み |
| `workspace/didChangeConfiguration` | `Config.compare` を利用して設定差分を検証 | 監査ログへ `change_set` を記録 |

## 3. 診断と監査の連携

```kestrel
fn to_lsp_diagnostics(diags: List<Diagnostic>) -> List<LspDiagnostic> =
  diags.map(|d| LspDiagnostic {
    range = span_to_range(d.at),
    severity = map_severity(d.severity),
    message = d.message,
    code = d.code,
    data = Some(json!({
      "domain": d.domain,
      "audit_id": d.audit_id,
      "change_set": d.change_set
    }))
  })
```

- `audit_id` を IDE 側で保持し、承認フローや差分レビューに利用。
- `change_set` を JSON として埋め込むことで、設定差分を IDE 上で表示可能。

## 4. ハイライト・補完ツールチェーン

1. `SemanticTokensLegend` を capability ごとに登録（例: `template.directive`, `config.field`）。
2. エディタが `semanticTokens/full` を要求 -> パーサがトークン種別を返却。
3. プラグイン側で追加トークンを登録する場合、`register_capability` で `syntax.highlight` を宣言。

## 5. CLI との一貫性

```bash
kestrel-run lint config.ks --format json --domain config   | jq '.diagnostics[] | {code, message, audit_id}'
```

- CLI と LSP の診断結果は同じ JSON フォーマットを共有し、CI/CD と IDE の結果照合が容易。
- `audit_id` をキーにランタイムガイド（`guides/runtime-bridges.md`）で差分適用ログと結合する。

## 6. TODO / 制限事項

- `semanticTokens` 対応は Draft。トークン種別の正規化が必要。
- CodeAction で複数 FixIt を提示する際、ユーザ承認フローを定義する必要がある。
- `workspace/applyEdit` など LSP 拡張は未定義。プラグインごとの要件を整理予定。

> 本ガイドはフェーズ3でさらに事例と図解を追加する予定です。
