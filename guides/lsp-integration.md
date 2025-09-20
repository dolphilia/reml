# LSP / IDE 連携ガイド（Draft）

> 目的：Kestrel の診断・補完・構文ハイライト情報を LSP 経由で IDE に提供するための手順をまとめる。

## 1. ランナー設定

```kestrel
let cfg = {
  lsp = {
    highlight = true,
    completion = true,
    codeActions = true
  },
  log_format = "json",
  audit = Some(|event| audit_log(event))
}
```

- `RunConfig.lsp` を有効化すると、`run_with_lsp` が構文情報・補完候補を生成。
- `log_format = "json"` により構造化ログを CLI から利用可能。

## 2. LSP サービス

- `kestrel-run lsp --stdio` を起動し、エディタから `initialize` / `textDocument/didChange` 等を受け付ける。
- 診断は `to_lsp_diagnostics`（2.5 節）で取得した `audit_id` / `change_set` を `data` に埋め込み、IDE 側で監査ビューへリンク。
- 補完候補はプラグイン capability を参照し、`requires({"template"})` のような DSL 特有の提案を返す。

## 3. CodeAction・FixIt

- `FixIt::AddMissing` などのテンプレートを LSP CodeAction へ変換し、ユーザに提案する。
- `audit_id` と紐付けることで、適用結果が監査ログへ自動記録される。

## 4. CLI 併用

```bash
kestrel-run lint config.ks --format json --domain config   | jq '.diagnostics[] | {code, message, audit_id}'
```

- CLI 出力と LSP の診断が同一フォーマットを共有するため、CI/CD と IDE の検出結果を一致させられる。

> 本ガイドはフェーズ3で詳細化予定。