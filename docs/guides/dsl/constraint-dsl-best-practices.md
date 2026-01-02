# 制約DSL・ポリシー検証ベストプラクティス

## 1. 目的と適用範囲
- `Core.Config` のスキーマ宣言と `Core.Data` の検証APIを組み合わせ、設定・データ・ポリシーを統一的に扱う指針を提供する。
- 標準APIでサポートするべき機能と、プラグイン/外部DSLへ委ねる機能の境界を明確化する。
- 対象読者はポリシーDSL設計者、運用チーム、監査担当。

## 2. 標準APIで提供するべき機能
### 2.1 Core.Config における制約記述
- `schema` ビルダの `requires`, `when`, `compute` を活用し、設定整合性を静的に検証 `3-7-core-config-data.md:6`
- `Change` と `SchemaDiff` を監査ログに出力し、`audit_id` をCLIと共有 `3-7-core-config-data.md:24`
- 条件付きパッチ `apply_when` を使用し、環境差分を宣言的に扱う。

### 2.2 Core.Data における制約検証
- `Constraint<T>` トレイトで列・レコードレベルの制約を表現 `3-7-core-config-data.md:38`
- `validate_with_profile` を用いて環境別プロファイルを適用 `3-7-core-config-data.md:48`
- `ValidationReport` の `diagnostics` と `stats` を監査フローへ統合。

### 2.3 共通ガイドライン
1. **静的宣言優先**: DSL内に可能な限り制約情報を埋め込み、ランタイム検証は一貫したログとともに実行。
2. **監査整合性**: `audit.log("config.diff", ...)` と `audit.log("data.validation", ...)` の `audit_id` を揃える。
3. **診断品質**: `2-5-error.md:12` の `Diagnostic` フォーマットを準拠し、`code`, `span`, `expected`, `context` を付加。

## 3. プラグインに委ねる領域
- **外部サービス連携**: Cloud API との接続、権限確認など効果 `io` を伴う処理は `Capability` プラグインに分離。
- **高コスト推論**: SAT/SMT ソルバや機械学習モデルを用いた推論は実行コストが高いため、拡張モジュールで提供。
- **領域固有DSL**: セキュリティポリシー、ネットワークACL、RBACなどドメイン固有言語はプラグイン側で定義し、標準APIは`Constraint` 登録ポイントのみ提供。

## 4. 制約DSL設計ワークフロー
1. `schema` or `Schema` で型とフィールドを宣言。
2. 標準制約を `requires` / `Constraint<T>` として登録。
3. プロファイル別閾値を `Config`/`QualityProfile` で管理。
4. 検証実行時に `ValidationReport` / `QualityReport` を生成し、監査ログ保存。
5. プラグイン制約が必要な場合は Capability を介して `ConstraintContext` の `profile` や `audit_id` を受け渡す `3-7-core-config-data.md:43`。

## 5. 運用パターン
| パターン | 標準API利用範囲 | プラグイン範囲 | 備考 |
| --- | --- | --- | --- |
| CIでの設定検証 | `Config.compare`, `requires` | なし | `reml-config validate` の既定使用例 |
| データ品質ゲート | `run_quality`, `Constraint` | 統計集計器 | `QualityReport` をCIに連携 |
| ポリシー監査 | `schema` + `validate_with_profile` | IAM, Network API | `audit_id` で運用記録を統合 |

## 6. 診断メッセージ設計
- `code` は `domain.category.detail` 形式とし、例: `config.requires.env_missing`
- `severity` は `Info`/`Warn`/`Error` を使用し、`QualityRule` と整合。
- 推奨修正 (`fixit`) は `2-5-error.md:65` のフォーマットに従い、CLIで `--apply-fix` オプションを提供可能にする。

## 7. 今後の拡張タスク
1. `Constraint` 登録 API に `capability` メタデータを追加し、プラグイン依存の明示化を検討。
2. `../runtimeruntime-bridges.md` に制約評価時の監査ログ例を追加。
3. 品質DSLに関する用語と優先度を `3-7-core-config-data.md` §4 と同期し、重複表現を整理。
4. 標準APIとプラグインの境界チェックリストを本ガイドの付録として追加し、更新履歴を README に反映。

## 8. 参考資料
- `3-7-core-config-data.md`
- `2-5-error.md`
- `3-7-core-config-data.md`
- `../runtimeruntime-bridges.md`
