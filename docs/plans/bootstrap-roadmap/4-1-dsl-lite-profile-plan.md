# Phase4: Reml Lite プロファイルとテンプレート計画

## 背景と決定事項
- `docs/notes/dsl/dsl-enhancement-proposal.md` の提案「3.4 "Reml Lite" プロファイルとテンプレート」を具体化する。
- `reml new --template` は `docs/spec/4-1-package-manager-cli.md` と `docs/spec/4-4-community-content.md` に既に言及があるが、Lite 向けの仕様とテンプレート定義は未整備。
- 0-1 §1.2 の安全性方針を崩さず、学習コストを下げるための入口として設計する。

## 目的
1. Lite プロファイルの定義（既定値・制約・段階的有効化）を仕様として明文化する。
2. `reml new --template lite` の最小テンプレートを設計し、学習/試作の導線を用意する。
3. 監査/Capability/効果タグの最小設定を示し、Lite から標準プロファイルへ移行可能にする。

## スコープ
- **含む**: Lite プロファイルの仕様化、テンプレート構成、導入ガイド、相互参照の整備。
- **含まない**: 実装作業、CLI 実装、テンプレート配布インフラ。

## 成果物
- Lite プロファイルの仕様差分と既定設定を記述した計画書と更新案。
- `reml new --template lite` のテンプレート構成案（ファイル一覧/最小構成/導線）。
- 既存仕様・ガイドへのリンク更新案。

## 作業ステップ

### フェーズA: Lite プロファイル定義
1. Lite の対象ユーザー像と用途（学習/試作/小規模 DSL）を明記する。
2. 既定値の方針を列挙する（例: `AuditPolicy::None`、`SecurityPolicy::Permissive`、`RunConfig` の簡易セット）。
3. 安全性の境界を明記する（0-1 §1.2 との整合、Lite でも必須の診断/効果の扱い）。

#### Lite プロファイル定義（案）

**対象ユーザー像**
- Reml 初学者（学習用に最小構成で動作確認したい）
- 小規模 DSL を短期間で試作したい開発者（設定ファイル/簡易コマンドなど）
- `Core.Parse` と `Core.Test` の最小パターンを把握したい開発者

**用途と対象外**
- 主要用途: 学習、試作、プロトタイピング、小規模 DSL の構築
- 対象外: 監査/監視が必須の運用、外部配布前提の正式リリース、Capability を広く使う本番運用

**既定値の方針（Lite 既定）**
- `project.stage = "lite"` を明示し、正式運用向けの `beta`/`stable` とは区別する。
- `AuditPolicy::None` を既定とするが、`Diagnostic` と `AuditEnvelope` は生成し、監査ログの出力のみ省略する。
- `SecurityPolicy::Permissive` を既定とし、ローカル実行と試作用途に限定する。
- `dsl.lite.expect_effects = []` / `dsl.lite.capabilities = []` を既定とし、追加は明示的に宣言する。
- `config.compatibility.json.profile = "json-relaxed"` を既定とし、後から `stable` へ移行可能にする。
- `RunConfig` は複雑なチューニングを要求しない簡易セットを採用し、デバッグ負荷を抑える。

**安全性の境界（必須条件）**
- 0-1 §1.2 の安全性方針は Lite でも緩和しない（型安全性、`Result` による例外防止を維持）。
- 監査ログを省略しても `Core.Diagnostics` の `Diagnostic` は必ず出力し、`AuditEnvelope` と `audit_metadata` を欠落させない。
- Capability は既定で空とし、Lite では暗黙追加しない（必要時に明示宣言が必須）。
- 効果タグは Lite でも明示する方針とし、未宣言の効果を隠蔽しない。
- Capability を追加する場合は `StageRequirement`（`Exact`/`AtLeast`）を明示し、`verify_capability_stage` の検証対象とする。
- Lite プロファイルの範囲外（外部配布・運用）では `project.stage` の昇格と監査ログ有効化を必須とする。

### フェーズB: テンプレート設計
1. Lite テンプレートの最小ファイル構成を決める。
2. テンプレート内に含める DSL サンプル（簡易パーサ、テスト、最小 conductor）を定義する。
3. `reml new --template` の既存テンプレート体系と整合する名称・説明文を確定する。

#### Lite テンプレート設計詳細（案）

**設計方針**
- 初回起動までの手順を 3 ステップ以内に抑える（`reml new` → `reml run` → `reml test`）。
- `Core.Parse`/`Core.Test`/`Core.Diagnostics` の最小セットのみ使用し、他モジュールは README の「次の一歩」に送る。
- Lite 既定は「監査ログ省略・診断必須・Capability 空集合」の状態を明示し、`reml.toml` と README に同じ文言を置く。
- ファイル間の依存方向を固定する（`main.reml` → `parser.reml` / `dsl_test.reml`、逆依存を作らない）。

**ファイル別の内容設計（ドラフト）**
- `README.md`:
  - Lite の目的/制約/対象外（運用・配布は標準プロファイルへ移行）を冒頭に明記。
  - 実行手順（`reml run`/`reml test`）と期待出力の最小例を 1 画面以内に記載。
  - 監査ログは省略されるが `Diagnostic` は出力されることを明記。
  - 移行手順に `project.stage` 昇格と監査ログ有効化（`--audit-log <path>`）を含める。
- `reml.toml`:
  - `project.stage = "lite"` と `dsl.lite` ブロックを必須とし、`capabilities = []` を既定にする。
  - `config.compatibility.json.profile = "json-relaxed"` を設定し、`feature_guard` に移行対象を列挙する。
  - テンプレート内で `allow_prerelease = true` を付け、`beta`/`stable` への移行を README で示す。
- `src/main.reml`:
  - `Core.Parse` を呼び出し、成功時は AST を最小限に表示、失敗時は `Diagnostic` を出力する。
  - `AuditPolicy::None` の前提でも `Diagnostic` が生成されることを確認できる構成にする。
- `src/parser.reml`:
  - `lex_pack`/`with_space` を使った最小 DSL（例: `key = value` の設定 DSL）を実装。
  - 解析結果は小さな AST 型で表現し、テストとスナップショットに流用できる形にする。
- `src/dsl_test.reml`:
  - `table_test` による正常系・異常系の 2 パターンを含める。
  - `assert_snapshot` で AST 出力を固定化し、`templates/sample.*` と整合させる。
- `templates/sample.input` / `templates/sample.ast`:
  - `sample.input` は 3 行程度の最小 DSL 入力。
  - `sample.ast` はスナップショットの期待値を示し、差分確認に利用する。

**テンプレートに含める最小 DSL（案）**
- 形式: `key = value` の 1 行設定 DSL
- データ型: `Str` と `Int` のみ（解析失敗時に `Diagnostic.code` を出す）
- 余白: `with_space` による空白スキップ（コメントは含めない）

**最小 DSL の例（Reml 構文）**
```reml
config {
  port = 8080
  host = "localhost"
  mode = "lite"
}
```

**最小 AST 例（parser.reml 向け）**
```reml
type Config = {
  entries: List<Entry>
}

type Entry = {
  key: Str,
  value: Value
}

type Value =
  | StrValue(Str)
  | IntValue(Int)
```

**parser.reml のパース規則（擬似コード）**
```reml
use Core.Parse

let config_parser =
  lex_pack {
    let key = ident
    let value =
      choice [
        int.map(IntValue),
        string.map(StrValue)
      ]
    let entry =
      key
        .skip(symbol("="))
        .and_then(value)
        .map(|(k, v)| Entry { key: k, value: v })
    let entries =
      entry
        .sep_by(symbol("\n"))
        .map(|items| Config { entries: items })
    with_space(
      keyword("config")
        .skip(symbol("{"))
        .and_then(entries)
        .skip(symbol("}"))
    )
  }
```

**templates/sample.ast 期待出力フォーマット案**
```reml
Config {
  entries: [
    Entry { key: "port", value: IntValue(8080) },
    Entry { key: "host", value: StrValue("localhost") },
    Entry { key: "mode", value: StrValue("lite") }
  ]
}
```

**CLI 既定動作との整合**
- `reml new --template lite` の説明文は「学習/試作向け最小構成」を明示。
- `reml run` では `Diagnostic` を標準出力へ表示することを期待し、README に表記。
- `reml test` では `table_test`/`assert_snapshot` が動作する前提で記述する。

**テンプレート構成案（v0）**

```
reml-lite/
  README.md
  reml.toml
  src/
    main.reml
    parser.reml
    dsl_test.reml
  templates/
    sample.input
    sample.ast
```

- `README.md`: Lite の目的、制約、次の一歩（標準プロファイル移行）を記載。
- `reml.toml`: `project.stage = "lite"` を仮置きし、`compat`/`capability` の既定値を最小構成で示す。`audit` は Lite 既定値（`none`）を README と CLI 既定で適用する。
- `src/main.reml`: 入力読み込み→パース→診断の最小実行パイプライン。
- `src/parser.reml`: `Core.Parse` を用いた最小 DSL パーサ。
- `src/dsl_test.reml`: `Core.Test` のテーブル駆動例とゴールデン例。
- `templates/sample.*`: `golden_case` の入力/期待値サンプル。

**ファイル内容の要件（抜粋）**
- `src/main.reml` は `AuditPolicy::None` でも最低限の `Diagnostic` を出す構成にする。
- `src/parser.reml` は `lex_pack`/`with_space` の標準パターンを採用し、後から Profile を差し替え可能にする。
- `src/dsl_test.reml` は `table_test` と `assert_snapshot` を両方示す。

**README 文言案（Lite テンプレート）**
```
Reml Lite は学習/試作向けの最小構成テンプレートです。`Core.Parse` と `Core.Test` を最短距離で試せるよう、`config.compatibility.json = json-relaxed` から開始します。

既定ポリシー:
- 監査: `audit = none`（CLI 既定で監査ログ出力を省略）
- 互換性: `config.compatibility.json = json-relaxed`
- Capability: `dsl.lite.capabilities = []`（最小構成で開始）

標準プロファイルへの移行手順:
1. `config.compatibility.json` を `stable` へ戻し、`feature_guard` を削除する。
2. `dsl.lite.expect_effects` と `capabilities` を実際の DSL に合わせて宣言する。
3. 監査ログを有効化する場合は `--audit-log <path>` を指定し、差分を確認する（`publish` と `registry login` は監査ログ必須）。
4. `project.stage` を `beta` または `stable` に更新し、`reml test` で回帰を確認する。
```

**`reml.toml` 最小キーと具体値（Lite 版）**
```toml
[project]
name = "reml-lite-sample"
version = "0.1.0"
stage = "lite"

[dsl.lite]
entry = "src/main.reml"
exports = ["LiteDsl"]
kind = "config"
expect_effects = []
capabilities = []
allow_prerelease = true
summary = "Lite DSL template"

[config.compatibility.json]
profile = "json-relaxed"
trailing_comma = "arrays_and_objects"
unquoted_key = "allow_alpha_numeric"
duplicate_key = "last_write_wins"
feature_guard = ["json5", "bare_keys", "trailing_comma"]
```

### フェーズC: 監査・Capability・効果の導線
1. Lite で無効化/省略される項目と、後から有効化する導線を整理する。
2. Capability/Stage の最低限要件と、Lite から本番プロファイルへ移行するためのチェックリストを作成する。
3. `Core.Diagnostics` の最低限の診断出力が残ることを保証する。

#### 監査・Capability・効果の導線整理（案）

**Lite で省略される項目（明示）**
- 監査ログ出力（`AuditPolicy::None` によりログ出力は省略）
- Capability の事前登録（`dsl.lite.capabilities = []` のため既定では無効）
- Stage 検証の強制（`reml test` では簡易確認に留める）

**Lite でも維持する項目（必須）**
- `Diagnostic` と `AuditEnvelope` の生成（ログは省略しても構造体は生成）
- 効果タグの明示（未宣言効果は許容せず、診断対象とする）
- `verify_capability_stage` による Stage 整合（Capability 追加時は必須）

**移行導線（Lite → 標準プロファイル）**
1. `project.stage` を `beta` もしくは `stable` に更新する。
2. `--audit-log <path>` を指定して監査ログ出力を有効化する。
3. `dsl.lite.expect_effects` と `dsl.lite.capabilities` を DSL 実装に合わせて明示する。
4. Capability ごとに `StageRequirement`（`Exact`/`AtLeast`）を定義し、`verify_capability_stage` を通過させる。
5. `config.compatibility.json.profile` を `stable` へ戻し、`feature_guard` の依存を解消する。

**Capability/Stage の最低限要件（Lite からの差分）**
- Lite では `capabilities = []` を既定とし、追加する場合は必ず `CapabilityDescriptor.stage` を確認する。
- `StageRequirement::AtLeast` を基本とし、特定用途で `Exact` を採用する際は README に理由を記載する。
- `effects.contract.stage_mismatch` が発生した場合は Lite でも `Diagnostic` を `Error` として扱う。

**診断出力の最低限保証**
- `Diagnostic.severity = Error | Warning` は常に表示対象とする。
- `Diagnostic.audit_metadata` を空にしない（`schema.version` など最小キーは維持）。
- `AuditEnvelope.metadata` に `event.kind` など最低限のパイプライン情報を残す方針を維持する。

### フェーズD: 仕様・ガイドの更新計画
1. `docs/spec/4-1-package-manager-cli.md` に Lite テンプレートを追記する。
2. `docs/spec/4-4-community-content.md` に Lite テンプレートの紹介と用途を追記する。
3. `docs/guides/ecosystem/manifest-authoring.md` に Lite テンプレートの最小マニフェスト例を追加する。

#### 仕様・ガイド更新の詳細（案）

**更新対象と目的**
- `docs/spec/4-1-package-manager-cli.md`: `reml new --template lite` の目的、既定ポリシー、README への移行導線を明記する。
- `docs/spec/4-4-community-content.md`: テンプレート一覧に Lite を追加し、学習/試作向けであることを明示する。
- `docs/guides/ecosystem/manifest-authoring.md`: Lite 向け最小 `reml.toml` 例と `project.stage` の扱いを補足する。

**追記内容の要点（共通）**
- Lite は監査ログ省略・診断必須・Capability 空集合が既定であること。
- `project.stage = "lite"` の位置付けと、`beta`/`stable` への移行導線。
- `config.compatibility.json.profile = "json-relaxed"` の採用理由と、`stable` への戻し方。

**リンク整合の方針**
- 追記する各セクションから `docs/plans/bootstrap-roadmap/4-1-dsl-lite-profile-plan.md` を参照し、計画と仕様の差分を追跡できるようにする。
- `README.md` のテンプレート一覧に変更が出る場合は、フェーズD 完了時に追記する。

**更新時の確認項目**
- 用語表記（Lite/標準/Stage/Capability）が 5-1/5-4/manifest ガイド間で一致していること。
- `reml.toml` のキーが `docs/spec/3-8-core-runtime-capability.md` の Stage/Capability 表記と矛盾しないこと。
- `docs/spec/3-6-core-diagnostics-audit.md` の `Diagnostic`/`AuditEnvelope` 表現と整合すること。

**追記案の具体テキスト（草案）**

`docs/spec/4-1-package-manager-cli.md` 追記候補（`3.1 reml new` セクション）:
```
- `--template lite`: 学習/試作向けの最小構成テンプレートを生成する。監査/Capability は既定で簡略化されるが、`README.md` に標準プロファイルへの移行手順を含める。
```

`docs/spec/4-4-community-content.md` 追記候補（テンプレート一覧）:
```
- `lite`: 学習・試作向けの最小テンプレート。`Core.Parse` と `Core.Test` の最小例を含み、監査と Capability は最小設定で開始する。
```

`docs/guides/ecosystem/manifest-authoring.md` 追記候補（最小マニフェスト例）:
```toml
[project]
name = "reml-lite-sample"
version = "0.1.0"
stage = "lite"

[dsl.lite]
entry = "src/main.reml"
exports = ["LiteDsl"]
kind = "config"
expect_effects = []
capabilities = []
allow_prerelease = true
summary = "Lite DSL template"

[config.compatibility.json]
profile = "json-relaxed"
trailing_comma = "arrays_and_objects"
unquoted_key = "allow_alpha_numeric"
duplicate_key = "last_write_wins"
feature_guard = ["json5", "bare_keys", "trailing_comma"]
```

### フェーズE: 回帰/サンプル接続（ドキュメント前提）
1. Lite テンプレートのサンプルを `examples/` と `expected/` に置く場合の方針を記述する。
2. Phase 4 シナリオへの登録要否を判断し、必要なら `phase4-scenario-matrix.csv` への追加方針を定義する。

#### 回帰/サンプル接続の判断（案）

**サンプル配置の方針**
- Lite テンプレートのサンプルは `examples/` 直下に新規追加せず、テンプレート内 `templates/` のみに保持する。
- ただし回帰検証が必要になった場合は `examples/practical/lite_template/` と `expected/lite_template/` を追加し、入力/期待値を対で管理する。
- 監査ログを省略する Lite 既定に合わせ、`expected/` 側は `*.diagnostic.json` のみを必須とし、`*.audit.jsonl` は任意とする。

**回帰検証の最小対象**
- `templates/sample.input` → `templates/sample.ast` の変換結果が一致すること。
- `Diagnostic` の `severity` と `code` がスナップショットと一致すること（最低 1 つの異常系を含む）。

**回帰用サンプル構成（案）**
`examples/practical/lite_template/`:
- `README.md`（Lite テンプレート回帰の目的と実行手順）
- `reml.toml`（Lite 既定値を反映した最小マニフェスト）
- `src/main.reml`（テンプレートと同一のエントリ）
- `src/parser.reml`（最小 DSL パーサ）
- `src/dsl_test.reml`（テーブルテスト + スナップショット）
- `templates/sample.input`（正常系サンプル）
- `templates/sample.invalid`（異常系サンプル）

**expected 出力形式の命名規則**
`expected/lite_template/`:
- `sample.ast.expected`（AST スナップショット。Reml 表記で保持）
- `sample.invalid.diagnostic.json`（異常系の `Diagnostic` 期待値）
- `sample.invalid.audit.jsonl`（監査ログの任意出力。Lite 既定では省略可）

**sample.ast.expected フォーマット例**
```reml
Config {
  entries: [
    Entry { key: "port", value: IntValue(8080) },
    Entry { key: "host", value: StrValue("localhost") },
    Entry { key: "mode", value: StrValue("lite") }
  ]
}
```

**Phase 4 シナリオ登録の判断**
- Lite テンプレートの仕様が安定した時点で `phase4-scenario-matrix.csv` に追加する。
- 追加時のシナリオIDは `CH5-LITE-001` とし、用途は「テンプレート生成後の最短実行」を想定する。
- 監査ログ必須のシナリオ群とは分離し、`audit = none` の条件で実行できる行を用意する。

### フェーズF: 実装準備と実装計画
1. `reml new --template lite` の CLI 実装範囲を確定する。
2. Lite 既定値（監査/Capability/互換プロファイル）の実装ポイントを整理する。
3. テンプレート資産の配置と配布フローを定義する。
4. 回帰シナリオ（`CH5-LITE-001`）の自動実行条件を決める。

#### 実装準備と実装計画（案）

**CLI 実装範囲**
- `reml new --template lite` を CLI に追加し、テンプレート一覧とヘルプ文言を更新する。
- `reml new` 生成物に `reml.toml` と `README.md` の既定値を反映する。
- `--audit-log <path>` 指定時のみ監査ログを有効化し、未指定時は `AuditPolicy::None` を既定とする。
- 生成物は `README.md` / `reml.toml` / `src/main.reml` / `src/parser.reml` / `src/dsl_test.reml` / `templates/sample.input` / `templates/sample.ast` を最低限含む。
- `reml new --template lite --help` に Lite の用途（学習/試作）と移行導線（`project.stage` 昇格）を明記する。

**既定値の実装ポイント**
- `project.stage = "lite"` と `dsl.lite.*` を CLI 生成テンプレートに埋め込む。
- `config.compatibility.json.profile = "json-relaxed"` をテンプレート既定値として反映する。
- `dsl.lite.capabilities = []` を既定にし、Capability 追加は明示的に指定させる。

**テンプレート資産の配置**
- テンプレート本体は `tooling/templates/` に配置する（CLI 実装と同一リポジトリ内で管理）。
- Lite の配置パスは `tooling/templates/lite/` とし、CLI からは `lite` 名称で参照する。
- `docs/` は仕様/ガイドの参照に留め、生成物のソースは置かない。
- `examples/practical/lite_template/` は回帰用資産として固定し、CLI の生成元とは分離する。

**同期ルール（tooling/templates/lite/ と examples/practical/lite_template/）**
- 生成テンプレートのソース・オブ・トゥルースは `tooling/templates/lite/` とする。
- `examples/practical/lite_template/` は回帰用のミラーであり、内容は `tooling/templates/lite/` と同一に保つ。
- 回帰専用の補助ファイル（例: `src/main_io.reml`）は `examples/practical/lite_template/` のみに置き、テンプレート本体へは含めない。
- 同期は手動コピーで行い、更新時は README と `reml.toml` の差分が無いことを確認する。

**`tooling/templates/lite/` の確定構成**
```
tooling/templates/lite/
  README.md
  reml.toml
  src/
    main.reml
    parser.reml
    dsl_test.reml
  templates/
    sample.input
    sample.ast
```

**テンプレート内容の確定方針**
- `README.md` は Lite の目的/制約/移行導線（`project.stage` 昇格）を記載する。
- `reml.toml` は Lite 既定値（`project.stage = "lite"`, `dsl.lite.capabilities = []`, `config.compatibility.json = json-relaxed`）を反映する。
- `src/main.reml` は文字列入力の最短実行パス（IO 非使用）を維持する。
- `src/parser.reml` は `Core.Parse.Lex` の最小構成で `key = value` DSL を解析する。
- `src/dsl_test.reml` は `Core.Test` の `table_test`/`assert_snapshot` を併記する。
- `templates/sample.*` は `examples/practical/lite_template/templates/` と同内容で同期する。

**回帰シナリオ運用**
- `CH5-LITE-001` を CI の Phase 4 回帰に含めるかを判断し、含める場合は入力/期待値の同期手順を定義する。
- 監査ログを省略する Lite 既定では `*.audit.jsonl` を必須としない。

## 参照
- `docs/notes/dsl/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/4-1-package-manager-cli.md`
- `docs/spec/4-4-community-content.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/guides/ecosystem/manifest-authoring.md`

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 72 週 | フェーズA: Lite プロファイル定義 |
| 73 週 | フェーズB: テンプレート設計 |
| 74 週 | フェーズC: 監査/Capability 導線整理 |
| 75 週 | フェーズD: 仕様・ガイド更新 |
| 76 週 | フェーズE: 回帰/サンプル接続判断 |
| 77 週 | フェーズF: 実装準備と実装計画 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| Lite が安全性方針と衝突する | 仕様の整合性低下 | 0-1 §1.2 の必須事項を Lite でも維持し、緩和点を明示する |
| テンプレートが増えすぎる | 学習導線の混乱 | Lite と標準テンプレートの違いを明確にし、用途別に分離する |
| Lite から本番への移行が不明瞭 | 実運用移行の遅延 | 移行チェックリストと再設定手順をテンプレート内に明記する |

## 進捗状況
- 2025-12-20: フェーズA（Lite プロファイル定義案）を追記。
- 2025-12-20: フェーズB（テンプレート設計詳細案）を追記。
- 2025-12-20: フェーズC（監査/Capability/効果の導線整理）を追記。
- 2025-12-20: フェーズD（仕様・ガイド更新計画と反映）を追記。
- 2025-12-20: フェーズE（回帰/サンプル接続判断と構成案）を追記。
- 2025-12-20: 回帰資産（lite_template）とシナリオマトリクスの更新を完了。
- 2025-12-20: フェーズF（実装準備と実装計画）を追加。

## 作業ステータス（更新済み/残タスク）

**完了（計画更新済み）**
- フェーズA: Lite プロファイル定義（対象ユーザー/既定値/安全性境界の明文化）
- フェーズB: テンプレート設計詳細（構成/DSL例/AST/擬似パース/期待出力）
- フェーズC: 監査/Capability/効果の導線整理（Lite 省略/必須/移行導線）
- フェーズD: 仕様・ガイド更新案の整理
- フェーズE: 回帰/サンプル接続の判断と配置方針

**完了（仕様/ガイド反映）**
- `docs/spec/4-1-package-manager-cli.md` に Lite テンプレート追記（README 移行導線を含む）
- `docs/spec/4-4-community-content.md` に Lite テンプレート説明を反映
- `docs/guides/ecosystem/manifest-authoring.md` に Lite 最小例と移行補足を追記
- `docs/guides/dsl/dsl-gallery.md` に Lite テンプレートの既定値を反映

**完了（回帰資産）**
- `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に `CH5-LITE-001` を追加
- `examples/practical/lite_template/` と `expected/lite_template/` を追加（最小構成）

**未着手/残タスク**
- フェーズF: 実装準備と実装計画の実作業（CLI/テンプレート/回帰運用の確定）

**実行ログ（CLI 検証）**
- `remlc new tmp/remlc_new_test/empty --template lite` で生成に成功。
- `remlc new tmp/remlc_new_test/non_empty --template lite` で非空ディレクトリエラーを確認。
- `REML_TEMPLATE_ROOT=tooling/templates remlc new tmp/remlc_new_test/with_env --template lite` で環境変数指定時の生成を確認。
