# プラグイン開発ガイド

Reml プラグイン（DSL テンプレート、ネイティブ UI、通知、監視ツールなど）を開発・配布するための基本手順をまとめます。Phase 3 のポータビリティ拡張に合わせ、ターゲット差異への配慮と監査ポリシー遵守を重視してください。

## Rust 実装対応状況

- Rust フロントエンド/ランタイムは 4.1 フェーズで `Core.Parse` コンビネーター層を導入済みだが、`Core.Parse.Plugin` の Capability 連携や Lex プロファイル共有、Streaming Runner との統合は未実装。プラグイン側の RunConfig 拡張を Rust CLI/LSP に反映するブリッジは今後の追補となる。
- Manifest (`reml.toml`) と DSL 署名同期は Rust 版でも動作するものの、パーサープラグイン登録 API（`register_parser` + Capability 公開）は未整備のため、Rust で動作確認する場合はバッチ実行のみを想定し、Streaming/Recover の挙動差分を `../../notes/parser/core-parse-api-evolution.md#todo-rust-lex-streaming-plugin` へ記録しておく。
- Capability/Stage 監査メタデータは Rust でも生成されるが、`with_capabilities` などプラグイン側の糖衣が欠けている点に注意。必要な場合は CLI から `RunConfig.extensions["effects"]` を直接指定して暫定対応する。

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

> Phase 2-7 で Unicode 識別子プロファイルが既定化され、`identifier_profile=unicode` が CLI/LSP/Streaming すべての標準設定になりました。互換運用が必要な場合のみ `identifier_profile=ascii-compat` を指定し、`plugin.toml` や README で ASCII 固定が必要な理由と復帰条件を明示してください。[^plugin-lexer001]

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

#### 2.2.1 Capability 情報の検証

```bash
reml_capability list --format json > tmp/capabilities.json
python3 scripts/capability/generate_md.py --json tmp/capabilities.json
reml_capability describe io.fs.read --output human
```

- `reml_capability describe <id>` を利用すると `stage`・`effect_scope`・`provider` を CLI で確認でき、`plugin-capability-matrix.csv`（`../../plans/bootstrap-roadmap/assets/plugin-capability-matrix.csv`）と突合しやすくなる。CI では `reml_capability list --format json` を `reports/spec-audit/ch3/capability_list-YYYYMMDD.json` に記録し、`scripts/capability/generate_md.py` で仕様のテーブルと同期させる。
- Stage 要件や監査ログの実例は `examples/core_diagnostics/pipeline_branch.reml`（`effects.contract.stage_mismatch`）と `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` に保存している。プラグイン側で Stage mismatch をテストする場合は `tooling/examples/run_examples.sh --suite core_diagnostics --with-audit` を流用すると、`capability.id` / `effect.stage.*` / `bridge.stage.*` の必須キーをまとめて検証できる。

### 2.3 Core.IO / Core.Path の活用

- ファイル／パスを扱うプラグインは `Core.IO` の `with_reader` / `with_writer` / `copy` と `Core.Path` の `validate_path` / `sandbox_path` を標準化された順番で呼び出し、`IoContext.operation`・`metadata.io.helper` を必ず設定する。サンプルは [examples/practical/core_io/file_copy/canonical.reml](../../../examples/practical/core_io/file_copy/canonical.reml) と [examples/practical/core_path/security_check/relative_denied.reml](../../../examples/practical/core_path/security_check/relative_denied.reml)（旧 `examples/core_io` / `examples/core_path`）を参照。
- Capability 検証（`io.fs.*`, `security.fs.policy`, `memory.buffered_io` など）は `FsAdapter::ensure_*` や `SecurityPolicy::enforce` を経由し、ステージ不一致時は `effects.contract.stage_mismatch` を発火させる。`tooling/examples/run_examples.sh --suite core_io` を CI で呼び出すと `core_io.example_suite_pass_rate` が 0.0 になり、欠落した監査キーを早期に検出できる。
- `IoContext` の内容 (`path`, `capability`, `buffer`, `glob`, `watch`) は `IoError::into_diagnostic()` が自動で診断へ転写するため、プラグイン固有の監査情報は `context.helper = "plugin.<name>"` のように付与すると追跡が容易になる。Runtime Bridge と同じ構造に揃えると `../../notes/runtime/runtime-bridges-roadmap.md` で管理する CI KPI と突合しやすい。

### 2.4 Manifest と `@dsl_export` の同期

- `@dsl_export` で宣言した `allows_effects`・`requires_capabilities`・`stage_bounds` は `update_dsl_signature` API を通じて `reml.toml` の `dsl.<name>.exports[*].signature` に書き戻す。Rust 実装では `compiler/runtime/src/config/manifest.rs` が `signature.stage_bounds.current` を `expect_effects_stage` へ反映するため、Stage 情報は Manifest と DSL の両方で一貫して参照できる。
- [examples/core_config/README.md](../../../examples/core_config/README.md) と [examples/practical/core_config/audit_bridge/audit_bridge.reml](../../../examples/practical/core_config/audit_bridge/audit_bridge.reml) のサンプルを実行すると `dsl.audit_bridge` の Capability/Stage/Effect の同期手順を確認できる。`cargo run --manifest-path compiler/frontend/Cargo.toml --bin remlc -- manifest dump --manifest examples/core_config/reml.toml` を実行し、`dsl.audit_bridge.exports[0].signature.stage_bounds` や `capabilities` が Practical スイートの `@dsl_export` と一致することを確かめる。
- Manifest API の単体テストは `cargo test manifest --test manifest --manifest-path compiler/runtime/Cargo.toml` で実行できる（`toml v0.5.11` の checksum 問題が発生した場合は `compiler/runtime/Cargo.lock` の再取得が必要）。`update_dsl_signature_records_stage_bounds` が Stage プロジェクションを保証しており、`DslExportSignature` が持つ `requires_capabilities`・`stage_bounds` の JSON を `serde_json` 経由で固定している。
- `RuntimeBridgeAuditSpec` で要求される `bridge.stage.required`/`bridge.stage.actual` と Capability 情報は、現状 `collector.stage.*` として `reports/spec-audit/ch1/core_iter_collectors.audit.jsonl` に出力されている。`cargo test --manifest-path compiler/frontend/Cargo.toml core_iter_collectors -- --nocapture` を実行すると同ファイルの最新ログを取得でき、Stage プロファイルが `../../spec/3-8-core-runtime-capability.md` §10 の要件（`stage.mode`, `stage.kind`, `stage.capability` など）を満たしているか確認できる。Runtime Bridge 連携ではこの JSON キーが `bridge.stage.*` としてそのまま流用されるため、DSL/Manifest の Stage 情報が欠落していないかを本テストで継続的に検証する。
- DSL プラグインを公開する際は Manifest が最新の `DslExportSignature` に追従しているか手動または CI で確認し、差分があった場合は `../../spec/3-7-core-config-data.md#14` で定義される `manifest.dsl.effect_mismatch`/`manifest.dsl.stage_mismatch` 診断の発火条件を参照して修正する。

## 3. 配布と署名

1. `reml plugin package` でアーカイブを生成。
2. `reml plugin sign --cert cert.pem` で署名し、`plugin.toml` に署名情報を記録。
3. 公式 `reml-plugins` レジストリへアップロードし、互換テストを登録。

### 3.1 バンドル JSON 形式（CLI 連携）

CLI が `reml plugin install --bundle <path>` で読み込む JSON は以下の形式を基準とする。

```json
{
  "bundle_id": "bundle.demo",
  "bundle_version": "0.1.0",
  "plugins": [
    { "manifest_path": "plugins/demo/reml.toml" }
  ],
  "signature": {
    "algorithm": "ed25519",
    "certificate": "base64-cert",
    "issued_to": "bundle.demo",
    "valid_until": "2027-01-01T00:00:00Z",
    "bundle_hash": "sha256:<hex>"
  }
}
```

- `plugins[*].manifest_path` はバンドル JSON からの相対パスで解釈する。
- `bundle_hash` は `bundle_id` / `bundle_version` と各 `manifest_path` の内容を連結した入力から算出する。
- 署名が無い場合は `VerificationPolicy::Permissive` で警告のみ、`Strict` ではインストールを失敗させる。
- `--policy strict` は署名必須、`--policy permissive` は警告のみで続行する。CI では `strict` を既定とする。
- JSON 出力のスキーマは `../../schemas/plugin-bundle-registration.schema.json` を参照する。

## 4. テスト戦略

- `../toolingci-strategy.md` のマトリクスに従い、対象ターゲットで `reml plugin test` を実行。
- ネイティブ UI プラグインでは OS ごとにダミー通知／ダイアログテストを用意し、CI と手動確認を組み合わせる。
- `Core.Env` を通じて環境変数を注入し、秘密情報を扱う際は監査ログで追跡できるようにする。

## 5. ドキュメント化

- `../runtimeruntime-bridges.md` や `../runtimeportability.md` からリンクされる README を用意し、導入手順/設定例/制限事項を明記。
- 動作確認済みターゲットと既知の制約（例: WASM では一部機能が無効）を表形式でまとめる。

## 6. Core.Parse コンビネーターと RunConfig 共有（Phase 2-5 Step6）

- `PARSER-003` Step6 で整理された `Core.Parse` モジュールは、`rule`/`label`/`cut`/`recover` など仕様コアと同名のコンビネーターを提供する。プラグインは Capability 宣言に加え、RunConfig 拡張を経由して字句プロフィールや回復トークンを共有することで CLI/LSP/ストリーミングと同一の実行条件を再現できる。
- `RunConfig.extensions["lex"]`・`["recover"]`・`["effects"]` を明示し、`Core.Parse.Plugin` が提供する `with_capabilities` と併用することで、複数 Capability が要求された際にも Packrat/Recover/Telemetry の監査メタデータが欠落しない。詳細は `../../notes/parser/core-parse-api-evolution.md` Phase 2-5 Step6 と `../../plans/bootstrap-roadmap/2-5-review-log.md` 2025-12-24 エントリを参照。
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

さらなる詳細は `../../notes/dsl/dsl-plugin-roadmap.md` の各プランに従って拡張してください。

## 7. Core.Parse プラグイン連携チェックリスト（Phase 11）

- **署名検証**: `@dsl_export` が出力する `signature.stage_bounds` / `requires_capabilities` と `plugin.toml` の `dsl.<name>.exports[*].signature` を必ず同期し、`reml plugin verify`（署名検証）または CI の Manifest チェックで乖離がないか確認する。Stage 不一致は `../../spec/3-8-core-runtime-capability.md` の `effects.contract.stage_mismatch` として報告されるため、公開前に `verify_capability_stage`（ランタイム Bridge 経由）で検査する。
- **RunConfig 共有**: Core.Parse をプラグイン内で呼び出す際は、`RunConfig` に `extensions["lex"]`（トリビア/レイアウトプロファイル）、`extensions["recover"]`（同期トークン）、`extensions["parse"].operator_table`（演算子優先度ビルダーで上書きする場合）を明示し、CLI/LSP/Streaming と同じ期待集合・空白処理を再現する。`Core.Parse.Plugin.with_capabilities` は Capability だけでなく RunConfig を併せて渡す前提で設計する。
- **空白/観測フラグ**: autoWhitespace/Layout を利用するパーサーは `RunConfig.extensions["lex"].profile/layout_profile` を伝搬し、プラグイン側で未指定の場合は既定の簡易空白が選ばれる点を README に記載する。性能計測が必要な場合のみ `RunConfig.profile` / `extensions["parse"].profile_output` を有効化し、書き込み失敗が診断に影響しない best-effort であることを利用者に周知する。
- **Streaming 互換**: ストリーミング実行を前提にするプラグインは、`ContinuationMeta.resume_hint` と `DemandHint` を `RunConfig.extensions["stream"]` から受け取り、バッチとストリーミングで同じ復旧フックを提供する。Rust 版では Streaming Runner が未実装のため、バッチ経路での再現手順を README に併記し、将来の実装時に互換性を検証できるよう `../../notes/parser/core-parse-api-evolution.md#todo-rust-lex-streaming-plugin` へ差分を記録する。
- **監査メタデータ**: `register_parser` 時に `RuntimeBridgeAuditSpec` で要求される `bridge.stage.*` / `effect.capabilities[*]` / `parser.runconfig.*` を埋め込み、Packrat/Recover/autoWhitespace の各経路で診断と同じキーが出力されるか確認する。`reports/spec-audit/ch3` 系ログと突合して欠落がないかをチェックする。
- **オペレーター宣言**: OpBuilder DSL や新しい演算子優先度ビルダーを使う場合は、`RunConfig.extensions["parse"].operator_table` で CLI/OpBuilder からの上書きに耐えるようにし、プラグイン内の DSL 設定と衝突しないかを `phase4-scenario-matrix.csv` の該当シナリオで回帰確認する。

[^plugin-lexer001]: `../../plans/bootstrap-roadmap/2-7-deferred-remediation.md` §7 および `../../plans/bootstrap-roadmap/2-5-proposals/SYNTAX-001-proposal.md` Step5/6 実施記録を参照。Phase 2-7 で Unicode プロファイルが既定化され、ASCII 互換モードは移行期間中のフォールバック手段としてのみ利用する。
