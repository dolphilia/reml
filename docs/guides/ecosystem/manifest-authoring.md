# Reml マニフェスト記述ガイド（Draft）

> `reml.toml` の書き方、検証、運用ベストプラクティスをまとめる。Chapter 3.7 と 4.x 章の執筆準備ドキュメント。

## 1. 基礎構造
- `[project]`, `[dependencies]`, `[dsl]`, `[build]`, `[registry]` セクションの概要。
- 必須フィールドと推奨フィールド一覧。

## 2. DSL セクション
- `exports`, `kind`, `expect_effects`, `capabilities` の指定方法。
- `@dsl_export` との対応付け、`DslCapabilityProfile` との同期手順。

## 3. 依存関係管理
- バージョン指定、Git/ローカルパス参照。
- 今後の中央レジストリ対応（4-2 参照）。

## 4. ビルド & プロファイル設定
- `build.target`, `profiles` の使い分け。
- CI 向け設定 (`warnings_as_errors`, `optimize`) の推奨値。

## 5. バリデーションフロー
- `reml manifest validate`（案）と `validate_manifest` API の使用例。
- エラーメッセージ読み解きガイド。

## 6. テンプレート
- `reml new` が生成する既定テンプレート。
- DSL タイプ別テンプレート例（config, template, query）。

### 6.1 Lite テンプレート最小例（草案）

Lite テンプレートは学習/試作向けに最小構成で開始し、`config.compatibility.json` を緩和した状態で提供する。

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

Lite では監査ログ出力を省略する前提だが、`project.stage` を `beta`/`stable` に昇格する場合は `--audit-log <path>` の指定と `capabilities`/`expect_effects` の明示を推奨する。

> 本ガイドはドラフト。Chapter 4 完成に合わせて詳細化する。
