# Phase4: Reml Lite プロファイルとテンプレート計画

## 背景と決定事項
- `docs/notes/dsl-enhancement-proposal.md` の提案「3.4 "Reml Lite" プロファイルとテンプレート」を具体化する。
- `reml new --template` は `docs/spec/5-1-package-manager-cli.md` と `docs/spec/5-4-community-content.md` に既に言及があるが、Lite 向けの仕様とテンプレート定義は未整備。
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
- `AuditPolicy::None` を既定とするが、診断出力は必ず有効化する（監査ログのみ省略）。
- `SecurityPolicy::Permissive` を既定とし、ローカル実行と試作用途に限定する。
- `dsl.lite.expect_effects = []` / `dsl.lite.capabilities = []` を既定とし、追加は明示的に宣言する。
- `config.compatibility.json.profile = "json-relaxed"` を既定とし、後から `stable` へ移行可能にする。
- `RunConfig` は複雑なチューニングを要求しない簡易セットを採用し、デバッグ負荷を抑える。

**安全性の境界（必須条件）**
- 0-1 §1.2 の安全性方針は Lite でも緩和しない（型安全性、`Result` による例外防止を維持）。
- 監査ログを省略しても `Core.Diagnostics` のエラー/警告は必ず出力する。
- Capability は既定で空とし、Lite では暗黙追加しない（必要時に明示宣言が必須）。
- 効果タグは Lite でも明示する方針とし、未宣言の効果を隠蔽しない。
- Lite プロファイルの範囲外（外部配布・運用）では `project.stage` の昇格と監査ログ有効化を必須とする。

### フェーズB: テンプレート設計
1. Lite テンプレートの最小ファイル構成を決める。
2. テンプレート内に含める DSL サンプル（簡易パーサ、テスト、最小 conductor）を定義する。
3. `reml new --template` の既存テンプレート体系と整合する名称・説明文を確定する。

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

### フェーズD: 仕様・ガイドの更新計画
1. `docs/spec/5-1-package-manager-cli.md` に Lite テンプレートを追記する。
2. `docs/spec/5-4-community-content.md` に Lite テンプレートの紹介と用途を追記する。
3. `docs/guides/manifest-authoring.md` に Lite テンプレートの最小マニフェスト例を追加する。

**追記案の具体テキスト（草案）**

`docs/spec/5-1-package-manager-cli.md` 追記候補（`3.1 reml new` セクション）:
```
- `--template lite`: 学習/試作向けの最小構成テンプレートを生成する。監査/Capability は既定で簡略化されるが、`README.md` に標準プロファイルへの移行手順を含める。
```

`docs/spec/5-4-community-content.md` 追記候補（テンプレート一覧）:
```
- `lite`: 学習・試作向けの最小テンプレート。`Core.Parse` と `Core.Test` の最小例を含み、監査と Capability は最小設定で開始する。
```

`docs/guides/manifest-authoring.md` 追記候補（最小マニフェスト例）:
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

## 参照
- `docs/notes/dsl-enhancement-proposal.md`
- `docs/spec/0-1-project-purpose.md`
- `docs/spec/5-1-package-manager-cli.md`
- `docs/spec/5-4-community-content.md`
- `docs/spec/3-6-core-diagnostics-audit.md`
- `docs/spec/3-8-core-runtime-capability.md`
- `docs/guides/manifest-authoring.md`

## タイムライン（目安）

| 週 | タスク |
| --- | --- |
| 72 週 | フェーズA: Lite プロファイル定義 |
| 73 週 | フェーズB: テンプレート設計 |
| 74 週 | フェーズC: 監査/Capability 導線整理 |
| 75 週 | フェーズD: 仕様・ガイド更新 |
| 76 週 | フェーズE: 回帰/サンプル接続判断 |

## リスクと緩和策

| リスク | 影響 | 緩和策 |
| --- | --- | --- |
| Lite が安全性方針と衝突する | 仕様の整合性低下 | 0-1 §1.2 の必須事項を Lite でも維持し、緩和点を明示する |
| テンプレートが増えすぎる | 学習導線の混乱 | Lite と標準テンプレートの違いを明確にし、用途別に分離する |
| Lite から本番への移行が不明瞭 | 実運用移行の遅延 | 移行チェックリストと再設定手順をテンプレート内に明記する |

## 進捗状況
- 2025-12-20: フェーズA（Lite プロファイル定義案）を追記。
