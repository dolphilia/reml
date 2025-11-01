# プラグイン開発ガイド

Reml プラグイン（DSL テンプレート、ネイティブ UI、通知、監視ツールなど）を開発・配布するための基本手順をまとめます。Phase 3 のポータビリティ拡張に合わせ、ターゲット差異への配慮と監査ポリシー遵守を重視してください。

## 1. プロジェクト構成

```
my-plugin/
  plugin.toml              # メタデータ（id/version/capabilities/targets）
  src/
    lib.reml
    native/                # OS 別ネイティブコード（必要なら）
  guides/
    README.md              # 導入手順と設定例
  tests/
    integration.reml
```

`plugin.toml` では以下のキーを必須とします。

| キー | 説明 |
| --- | --- |
| `id` | 一意なプラグイン ID（例: `reml.native-ui`） |
| `version` | Semantic Versioning |
| `capabilities` | 要求する Capability ID のセット |
| `targets` | 対応 OS/アーキテクチャ一覧（`target_os`, `target_family`, `feature`） |

## 2. Capability と `@cfg`

### 2.1 必須設定

```reml
@cfg(target_os = "windows")
use ::Core.Platform.Windows.UI

@cfg(target_os = "macos")
use ::Core.Platform.Mac.UI

@cfg(target_family = "unix")
use ::Core.Platform.Posix.Notify
```

- `RunConfig.extensions["target"]` を参照し、利用可能なプラットフォーム以外ではプラグインを no-op にする。
- Capability Registry (`3-8-core-runtime-capability.md`) に対して `register("native_ui", handle)` のように識別子を明示する。

### 2.2 監査・診断

- `Diagnostic.extensions["cfg"]` を利用してターゲット差異を報告し、CI で `target_config_errors` を集計できるようにする。
- `AuditEnvelope.metadata` にプラグイン固有の設定（通知エンドポイント等）を記録し、`Core.Diagnostics` の保持方針に従う。
- Phase 2-5 ERR-001 で `Diagnostic.expected` に `ExpectationSummary` が常時含まれるようになったため、プラグイン側でも CLI/LSP と同じ期待集合を利用できる。`Core.Diagnostics.humanize_expected` を呼び出して候補一覧を提示し、DSL 独自の補完や自動修復に活用する。

## 3. 配布と署名

1. `reml plugin package` でアーカイブを生成。
2. `reml plugin sign --cert cert.pem` で署名し、`plugin.toml` に署名情報を記録。
3. 公式 `reml-plugins` レジストリへアップロードし、互換テストを登録。

## 4. テスト戦略

- `docs/guides/ci-strategy.md` のマトリクスに従い、対象ターゲットで `reml plugin test` を実行。
- ネイティブ UI プラグインでは OS ごとにダミー通知／ダイアログテストを用意し、CI と手動確認を組み合わせる。
- `Core.Env` を通じて環境変数を注入し、秘密情報を扱う際は監査ログで追跡できるようにする。

## 5. ドキュメント化

- `docs/guides/runtime-bridges.md` や `docs/guides/portability.md` からリンクされる README を用意し、導入手順/設定例/制限事項を明記。
- 動作確認済みターゲットと既知の制約（例: WASM では一部機能が無効）を表形式でまとめる。

## 6. Core.Parse コンビネーターと RunConfig 共有（Phase 2-5 Step6）

- `PARSER-003` Step6 で OCaml 実装に導入された `Core_parse` モジュールは、`rule`/`label`/`cut`/`recover` など仕様コアと同名のコンビネーターを提供する。プラグインは Capability 宣言に加え、RunConfig 拡張を経由して字句プロフィールや回復トークンを共有することで CLI/LSP/ストリーミングと同一の実行条件を再現できる。
- `RunConfig.extensions["lex"]`・`["recover"]`・`["effects"]` を明示し、`Core.Parse.Plugin` が提供する `with_capabilities` と併用することで、複数 Capability が要求された際にも Packrat/Recover/Telemetry の監査メタデータが欠落しない。詳細は `docs/notes/core-parse-api-evolution.md` Phase 2-5 Step6 と `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2025-12-24 エントリを参照。
- コンビネーター層を利用する導線を README やリリースノートに記録し、Phase 2-7 で予定されているテレメトリ統合・Menhir 置換判断へ引き継ぐ。

```reml
use Core.Parse
use Core.Parse.Plugin

let build_run_config() =
  RunConfig::builder()
    .with_extension("lex", {
      profile: "dsl.block",
      line: ["//"],
      block: { start: "(*", end: "*)", nested: true },
      space_id: ParserId::DslBlock,
    })
    .with_extension("recover", {
      sync_tokens: [";"],
      notes: true,
    })
    .with_extension("effects", {
      required_capabilities: ["parser.recover", "parser.trace"],
    })
    .finish()

let space =
  Lex.spaceOrTabsOrNewlines |> Lex.skipMany

let recover_clause =
  Core.Parse.recover(
    until = Core.Parse.symbol(space, ";"),
    with = |_| default_block()
  )

let entry =
  Core.Parse.rule("dsl.entry",
    Core.Parse.symbol(space, "construct")
      .then(Core.Parse.cut(Core.Parse.label("value", value_expr())))
      .then(Core.Parse.symbol(space, "end"))
      |> recover_clause
  )

let register =
  |reg| {
    let cfg = build_run_config()
    let parser =
      entry
        |> Core.Parse.Plugin.with_capabilities({"parser.recover", "parser.trace"})
    reg.register_parser("dsl.entry", || {
      parser = parser,
      config = cfg,
    })
  }
```

上記のように RunConfig と Capability の両方を明示することで、CLI／LSP／CI が同じ `space_id`・同期トークン・効果メタデータを取得し、`collect-iterator-audit-metrics.py --require-success` で監査欠落が検出された際も再現性を確保できる。

---

さらなる詳細は `docs/notes/dsl-plugin-roadmap.md` の各プランに従って拡張してください。
