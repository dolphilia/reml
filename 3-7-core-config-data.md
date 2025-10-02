# 3.7 Core Config & Data

> 目的：設定スキーマ (`Core.Config`) とデータモデリング (`Core.Data`) を Chapter 3 の標準ライブラリ体系へ統合し、差分管理・監査・CLI ツールとの連携を明文化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {diagnostic}`, `effect {config}`, `effect {audit}`, `effect {io}`, `effect {migration}` |
| 依存モジュール | `Core.Prelude`, `Core.Collections`, `Core.Diagnostics`, `Core.IO`, `Core.Numeric & Time` |
| 相互参照 | [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.2 Core Collections](3-2-core-collections.md) |

> **移行メモ**: Chapter 2 に残る 2.7/2.8 は参照用として維持されるが、本章で標準ライブラリ視点の API 契約と監査統合を再整理する。将来的に Chapter 2 版は概要＋互換ノートへ縮約する計画。
>
> **段階的導入ポリシー**: マニフェストやスキーマに新しい効果タグや Capability を追加する場合は、`Manifest.expect_effects_stage` や `Schema.metadata.stage` へ `Experimental` / `Beta` / `Stable` を記録し、`reml config lint` が未承認のステージを警告できるようにする。実験機能を本番へ昇格させる際は `notes/algebraic-effects-implementation-roadmap-revised.md` のチェックリストに従い、`@dsl_export` とマニフェスト値が一致することを確認する。

## 1. Core.Config.Manifest — `reml.toml` スキーマ {#manifest}

Reml のプロジェクトマニフェスト `reml.toml` は `Core.Config.Manifest` 名前空間で扱う。言語仕様（Chapter 1）と連携する DSL メタデータ、依存関係、ビルド構成を一元管理する。

### 1.1 構造定義

```reml
type Manifest = {
  project: ProjectSection,
  dependencies: Map<PackageName, DependencySpec>,
  dsl: Map<Str, DslEntry>,
  build: BuildSection,
  registry: RegistrySection,
}

type ProjectSection = {
  name: PackageName,
  version: SemVer,
  authors: List<Contact>,
  license: Option<LicenseId>,
  description: Option<Str>,
}

type DslEntry = {
  entry: Path,
  exports: List<DslExportRef>,
  kind: DslCategory,
  expect_effects: Set<EffectTag>,
  allow_prerelease: Bool,
  capabilities: List<CapabilityId>,
  summary: Option<Str>,
}

type DslExportRef = {
  name: Str,
  signature: Option<DslExportSignature<Json>>,  // Chapter 1.2 で定義
}

type BuildSection = {
  target: TargetTriple,
  features: Set<Str>,
  optimize: OptimizeLevel,
  warnings_as_errors: Bool,
  profiles: Map<Str, BuildProfile>,
}

type RegistrySection = {
  upstream: Url,
  mirrors: List<Url>,
  auth: Option<AuthConfig>,
}
```

- `DslExportRef.signature` はコンパイラが `@dsl_export` から抽出した `DslExportSignature` を JSON にシリアライズして格納する（未解析時は `None`）。
- `expect_effects` は 1.3 §I.1 の効果境界と突き合わせるための期待集合。CI などではこれを上限として用いる。
- `allow_prerelease` が `true` の場合、互換判定で pre-release バージョンを許容する（1.2 §G 参照）。

### 1.2 API

```reml
fn load_manifest(path: Path) -> Result<Manifest, Diagnostic>             // `effect {io, config}`
fn validate_manifest(manifest: Manifest) -> Result<(), Diagnostic>      // `@pure`
fn declared_effects(manifest: Manifest, dsl: Str) -> Result<Set<EffectTag>, Diagnostic> // `@pure`
fn update_dsl_signature(manifest: Manifest, dsl: Str, signature: DslExportSignature<Json>) -> Manifest // `@pure`
fn iter_dsl(manifest: Manifest) -> Iter<(Str, DslEntry)>                // `@pure`
```

- `load_manifest` は TOML を解析し、`DslEntry.entry` の相対パスを `Path` に正規化する。存在しないファイルは `diagnostic("manifest.entry.missing")` で報告。
- `validate_manifest` は必須フィールド、バージョン範囲、Capability と効果境界を点検し、`expect_effects` に存在しないタグが記述されていれば `diagnostic("manifest.dsl.unknown_effect")` を返す。
- `declared_effects` は CLI が `@dsl_export(allows_effects=...)` との差異を比較するために利用し、`update_dsl_signature` はコンパイラが型検査後にマニフェストへ署名情報を書き戻す際に使用する。

### 1.3 DSL セクションと型システム連携
### 1.4 効果宣言との連動（`@dsl_export`）

- `@dsl_export(allows_effects=...)` と `reml.toml` の `dsl.<name>.expect_effects` は、効果宣言 `effect Foo : ...` およびステージ管理 (`Stage = Experimental | Beta | Stable`) と同期する。
- 型検査時に得られた `DslExportSignature.allows_effects`、Stage 情報、Capability 要求をマニフェストへ書き戻し、`declared_effects` と比較する。差分がある場合は `manifest.dsl.effect_mismatch` を発生させる。
- `expect_effects_stage`（オプション）をマニフェストに追加すると、`Stage` が昇格した際に CLI が未更新のエントリを警告する。例:

```toml
[dsl.example]
entry = "src/example.reml"
exports = ["ExampleDSL"]
expect_effects = ["io.console"]
expect_effects_stage = "experimental"
```

- Stage を `beta`/`stable` へ更新した場合は、マニフェストと `@requires_capability(stage=...)` の値を同時に更新し、`effects.stage.promote_without_checks` 診断が出ないことを確認する。


1. `load_manifest` で DSL エントリを収集し、`entry` ごとに `exports[*].name` を記録。
2. コンパイラが `@dsl_export` を処理して `DslExportSignature` を生成したら、`update_dsl_signature` によって対応する `exports[*]` へ埋め込む。
3. `declared_effects` と `signature.allows_effects` を比較し、一致しない場合は `diagnostic("manifest.dsl.effect_mismatch")` を生成（Chapter 3.6 §9 で CLI へ伝播）。
4. `kind` と `signature.category` が一致しない場合は型検査を中断し、`diagnostic("manifest.dsl.category_mismatch")` を返す。

### 1.5 互換モード（`ConfigCompatibility`）

JSON5 や TOML など、現実の構成ファイルは標準仕様にない拡張（コメント、トレーリングカンマ、bare key 等）を持つ。Reml ではこれらを Stage/監査ポリシーに沿って制御するため、互換モードを以下の構造で表現する。

```reml
type ConfigCompatibility = {
  trivia: ConfigTriviaProfile = ConfigTriviaProfile::strict_json,
  trailing_comma: TrailingCommaMode = TrailingCommaMode::Forbid,
  unquoted_key: KeyPolicy = KeyPolicy::Forbid,
  duplicate_key: DuplicateKeyPolicy = DuplicateKeyPolicy::Error,
  number: NumberCompatibility = NumberCompatibility::Strict,
  feature_guard: Set<Str> = {},
}

enum TrailingCommaMode = Forbid | Arrays | Objects | ArraysAndObjects
enum KeyPolicy = Forbid | AllowAlpha | AllowAlphaNumeric
enum DuplicateKeyPolicy = Error | LastWriteWins | CollectAll
enum NumberCompatibility = Strict | AllowLeadingPlus | AllowHexFloat

fn ConfigCompatibility::default() -> ConfigCompatibility
fn ConfigCompatibility::stable(format: ConfigFormat) -> ConfigCompatibility
fn compatibility_profile(format: ConfigFormat, stage: Stage) -> ConfigCompatibility
fn resolve_compat(cfg: RunConfig, format: ConfigFormat) -> ConfigCompatibility
fn with_compat(cfg: RunConfig, compat: ConfigCompatibility) -> RunConfig
```

- `trivia` は 2-3 §G-1 の `ConfigTriviaProfile` を参照し、字句レベルの互換挙動（コメント・shebang 等）を共有する。CLI/LSP は `RunConfig.extensions["config"].trivia` を設定して一貫性を確保する。
- `trailing_comma` は寛容モードを選択しても `Diagnostic.notes += { label = "config.compat", value = "trailing_comma" }` を追加し、監査ログが互換拡張の使用箇所を追跡できる（3-6 §2.4）。
- `unquoted_key` は bare key の許容範囲を制御する。禁止状態で検出した場合は `Diagnostic.code = "config.key.unquoted"` を報告し、`AllowAlpha` は `[A-Za-z_]`、`AllowAlphaNumeric` は `[A-Za-z0-9_\-]` まで許容する。
- `duplicate_key` が `CollectAll` の場合、CLI は `ChangeSet.duplicates` に値を追加し、レビューで衝突解消を促す。`LastWriteWins` は JSON5 互換挙動だが、`RunConfig.extensions["config"].policy = "warn"` を既定にする。
- `number=AllowHexFloat` は HPC 系設定で使われるが、0-1 §1.1 の性能要求を満たすために `Diagnostic.extensions["config"].number_mode` に正規化済み桁情報を保持する。
- `feature_guard` は `RunConfig.extensions["config"].features` と一致させ、`compatibility_profile` が Stage ごとに推奨値（`Experimental` は最も寛容）を返す。未承認の機能を本番で有効化しようとすると `Diagnostic.code = "config.feature.unapproved"` を返し、3-6 §3.2 の監査ポリシーに従って拒否する。

#### 1.5.1 既定プロファイルと整合性

- `ConfigCompatibility::default()` は JSON/TOML を問わず Stage::Stable を前提にした厳格プロファイルを返し、上記のフィールド既定値と完全に一致する。0-1 §1.2 の安全性原則を満たすため、コメントや曖昧な数値表現を許容しない設定を明示的に採用する。
- `ConfigCompatibility::stable(format)` はフォーマットごとの列挙値（例: TOML は bare key を `AllowAlphaNumeric` まで許可）を Stage::Stable 基準で返す。`compatibility_profile(format, Stage::Stable)` の返り値と等価であり、CLI が厳格プロファイルを初期化する際の省略形として利用する。
- 仕様変更で既定値を緩和する場合は `AuditEvent::ConfigCompatChanged` を必須とし、マニフェストの `config.compatibility.<format>` に対して自動的に `compatibility=relaxed` といったタグを追加して履歴を残す。

#### 1.5.2 解決順序と責務

`resolve_compat` は以下の優先順位でプロファイルを決定する：

1. CLI パラメータ（`RunConfig.cli_overrides.compat`）が存在すればそれを採用する。`reml config lint --compat relaxed-json` などのオプションはここへ反映され、監査ログに `source = "cli"` を必ず残す。
2. 環境変数による上書き（3-10 §2.1）を `RunConfig.extensions["config"].env_compat` として取り込み、CLI 指定がない場合にのみ適用する。互換プロファイル名と feature guard の双方が対象であり、未知値は `Diagnostic.code = "config.compat.env_invalid"` で拒否する。
3. `reml.toml` の `config.compatibility.<format>` を Stage と feature guard を検証した上でマージし、欠落フィールドは厳格プロファイル側を優先する。
4. それ以外は `ConfigCompatibility::stable(format)`（=`compatibility_profile(format, Stage::Stable)`）へフォールバックする。

互換モードを切り替えた際は `Core.Diagnostics` が `AuditEvent::ConfigCompatChanged` を記録し、`Diagnostic.severity` を `Warning` 以上に設定することで 0-1 §1.2 の安全性を維持する。CLI/LSP/ビルドは `resolve_compat` の結果を共通で使用し、環境差異による設定解析の不一致を防止する。また CLI/CI は優先順位が実装どおりであることを保証するための準拠テスト（CLI 指定 > 環境変数 > マニフェスト > 既定値）を `Core.TestKit::config_compat_order` で提供し、逆順の上書きが発生した場合はビルドを失敗させる。

このワークフローにより、マニフェスト・言語仕様・CLI が同じ DSL メタデータと互換モードを共有できる。詳細な記述ガイドは `guides/manifest-authoring.md` と `guides/config-compatibility.md`（新規作成予定）で扱う。

#### 1.5.3 診断生成と効果タグ

- `load_manifest` や `validate_manifest` など `@pure` 指定の API は診断レコードを構築するだけに留め、`Diag.new_uuid()` や `Core.Numeric.now()` などの効果を直接呼び出してはならない。`AuditEnvelope` やコード、Severity は引数や戻り値を通じて受け渡し、発行タイミングを制御する。
- CLI/ランタイムは `effect {diagnostic, audit}` の文脈で `Core.Diagnostics.emit` を呼び出し、`id` や `timestamp` が未設定の診断に自動付番を行う。これにより 0-1 §1.2 の安全性と §2.2 の分かりやすいエラーメッセージ指針を両立させられる。
- 推奨パターン：`Result<(), Diagnostic>` を返す純粋な検証と、`Result<(), Diagnostic>` を監査付きで送出する関数を分離し、以下のように `AuditSink` へ委譲する。

```reml
fn check_manifest(manifest: Manifest) -> Result<(), Diagnostic> =
  ensure(manifest.project.name.is_valid(), || diagnostic("manifest.project.invalid"))?;
  Ok(());

fn report_manifest(manifest: Manifest, audit: AuditSink) -> Result<(), Diagnostic> =
  check_manifest(manifest).tap_diag(|diag| { emit(diag, audit).ok(); })
```

- 上記の `tap_diag` は 3-6 §2 で定義された `effect {diagnostic, audit}` を用い、複数の検証結果を一括で記録できる。`feature_guard` や `compatibility_profile` の検証も同一パターンで扱い、診断の再現性と監査ログの連携を保証する。

## 2. Config スキーマ API（再整理）

`Core.Config.schema` を中心に、差分・監査・CLI 連携を明記する。

```reml
fn schema<T>(name: Str, build: (SchemaBuilder<T>) -> ()) -> Schema<T>         // `@pure`

struct SchemaBuilder<T> {
  fields: Map<Str, Field<T>>,
}

impl<T> SchemaBuilder<T> {
  fn field<U>(self, name: Str, ty: TypeRef<U>, default: Option<U>) -> Self;   // `@pure`
  fn optional<U>(self, name: Str, ty: TypeRef<U>) -> Self;                    // `@pure`
  fn compute<U>(self, name: Str, f: (T) -> U) -> Self;                        // `@pure`
  fn when(self, pred: (T) -> Bool, patch: Patch<T>) -> Self;                  // `@pure`
  fn finalize(self) -> Schema<T>;                                            // `@pure`
}
```

- `Patch<T>` は条件付き更新ルール。`when` と組み合わせて宣言的バリデーションを構築する。
- `TypeRef<U>` は `Core.Data` の型リファレンスと統一され、列定義と再利用できる。

### 2.1 スキーマ差分

```reml
pub type SchemaDiff<T> = {
  added: List<Field<T>>,
  removed: List<Field<T>>,
  modified: List<FieldChange<T>>,
}

fn diff<T>(old: Schema<T>, new: Schema<T>) -> SchemaDiff<T>                    // `@pure`
fn apply_patch<T>(schema: Schema<T>, patch: Patch<T>) -> Schema<T>            // `@pure`
fn plan<T>(old: Schema<T>, new: Schema<T>) -> ChangeSet                       // `@pure`
fn validate_migration<T>(old: Schema<T>, new: Schema<T>) -> Result<MigrationPlan, MigrationError> // `@pure`
fn estimate_migration_cost<T>(plan: MigrationPlan) -> MigrationCost           // `@pure`

pub type MigrationPlan = {
  steps: List<MigrationStep>,
  estimated_duration: Duration,
  requires_downtime: Bool,
  data_loss_risk: RiskLevel,
}

pub enum MigrationStep = {
  AddField { name: Str, field: Field<T>, default: Option<T> },
  RemoveField { name: Str, backup_location: Option<Path> },
  RenameField { old_name: Str, new_name: Str },
  ChangeType { name: Str, old_type: TypeRef<T>, new_type: TypeRef<U>, converter: Option<(T) -> Result<U, ConversionError>> },
  ReorganizeData { strategy: ReorganizationStrategy },
}

pub enum RiskLevel = None | Low | Medium | High | Critical
```

- `ChangeSet` は監査ログ（4.7）で利用する差分形式。`plan` は CLI/CI でレビュー可能なパッチを生成する。

## 3. Config 実行 API

```reml
fn load(path: Path, schema: Schema<T>) -> Result<T, Diagnostic>                // `effect {io, config}`
fn validate<T>(value: T, schema: Schema<T>) -> Result<(), Diagnostic>          // `@pure`
fn compare<T>(old: T, new: T, schema: Schema<T>) -> Result<(), ChangeSet>     // `@pure`
fn apply_diff<T>(value: T, diff: ChangeSet) -> Result<T, Diagnostic>           // `effect {config}`
```

- `load` は 4.6 の IO API と連携。`Diagnostic` には `audit_id` と `change_set` が付与される。
- `compare` は差分が発生した場合 `Err(ChangeSet)` を返し、`ChangeSet` を監査へ送る想定。
- マイグレーションはデータ失失リスクを最小化するため、バックアップとロールバック機能を標準で提供。

### 3.1 マイグレーション安全性

```reml
fn backup_before_migration<T>(schema: Schema<T>, data: T, backup_path: Path) -> Result<BackupHandle, MigrationError>
fn rollback_migration<T>(backup: BackupHandle) -> Result<T, MigrationError>
fn verify_migration<T>(old_data: T, new_data: T, schema: Schema<T>) -> Result<(), ValidationError>
```

### 3.2 パーサ診断連携

- `load` / `validate` / `apply_diff` は 3.6 §2.2 の `from_parse_error` を利用して `Diagnostic` を生成する。`RunConfig.locale` が指定されていれば、パーサ段階と同じロケールでエラーメッセージを整形する。
- コンフィグ差分の解析で `Parse.fail` が発生した場合、監査用の `ChangeSet` と `AuditEnvelope` を `Diagnostic.audit` へ転写し、`extensions["config"].diff` に差分の概略（例: `{"missing": ["host"], "unexpected": ["timeout"]}`）を格納する。
- CLI では `RunConfig.extensions["audit"].policy` を参照し、構成ファイルの読み込み時に自動的に §3.2 の `apply_policy` を適用する。これにより、本番環境で求められる監査証跡と 0-1 §2.2 の「分かりやすいエラーメッセージ」の両立が保証される。
- `compare` / `plan` が返す `ChangeSet` に由来する警告は、同じ `AuditEnvelope` を共有することでレビュー履歴とリンクできる。`Diagnostic.code` を `config.diff.*` 名前空間で登録しておくと、差分種別ごとの追跡が容易になる。

## 4. Data モデリング API（再整理）

```reml
pub type Column<T, Meta> = {
  dtype: TypeRef<T>,
  constraints: List<Constraint<T>>,
  meta: Meta,
}

pub type SchemaRecord<T> = Map<Str, Column<T, ColumnMeta>>

fn column<T>(dtype: TypeRef<T>, constraints: List<Constraint<T>>) -> Column<T, ColumnMeta> // `@pure`
fn resource<P, K>(prefix: P, key: K) -> ResourceId<P, K>                                   // `@pure`
fn infer_schema<T>(samples: Iter<Json>) -> Result<SchemaRecord<T>, Diagnostic>             // `effect {audit}`
```

- `infer_schema` はサンプル JSON を解析し、`Diagnostic` に推論根拠を保持。`effect {audit}` を付与し、推論経路を監査ログに残す。

### 4.1 データ品質検証

```reml
pub type DataQualityRule<T> = {
  name: Str,
  description: Str,
  validator: (T) -> Result<(), QualityViolation>,
  severity: QualitySeverity,
}

pub enum QualitySeverity = Info | Warning | Error | Critical

fn validate_data_quality<T>(data: Iter<T>, rules: List<DataQualityRule<T>>) -> QualityReport
fn auto_fix_quality_issues<T>(data: T, rules: List<DataQualityRule<T>>) -> Result<T, QualityError>
```

### 4.2 統計との連携

```reml
fn update_stats(column: ColumnStats, values: Iter<Json>) -> Result<ColumnStats, Diagnostic> // `@pure`
fn merge_stats(left: ColumnStats, right: ColumnStats) -> ColumnStats                        // `@pure`
fn as_metric(points: ColumnStats) -> List<MetricPoint<Float>>                               // `@pure`
```

- `MetricPoint` は [3.4](3-4-core-numeric-time.md) で定義。データ品質監査へ転送するためのラッパ。

## 5. CLI / ツール連携

設定 CLI や LSP から利用するユーティリティを明示する。

```reml
fn diff_to_table(diff: ChangeSet) -> Table<Str, Json>                      // `effect {mut}`
fn render_summary(diff: ChangeSet, fmt: OutputFormat) -> String            // `effect {mem}`
fn attach_exit_code(diag: Diagnostic) -> ExitCode                          // `@pure`
```

- `Table` は 3.2 の可変コレクション。CLI 表形式へ変換する際に使用。
- `OutputFormat` は CLI/JSON/Markdown 等に対応。
- `ExitCode` は CLI ツールが戻す整数コード。

## 6. 使用例（差分レビュー）

```reml
use Core;
use Core.Config;
use Core.Diagnostics;

fn review_config(old: AppConfig, new: AppConfig, schema: Schema<AppConfig>, audit: AuditSink) -> Result<(), Diagnostic> =
  match compare(old, new, schema) with
  | Ok(()) => Ok(())
  | Err(diff) => {
      let envelope = from_change(Change::Config(diff.clone()));
      let table = diff_to_table(diff.clone());
      emit(
        diagnostic("config changes detected")
          .with_severity(Severity::Warning)
          .attach_audit(envelope)
          .finish(),
        audit,
      )?;
      println(render_summary(diff, OutputFormat::Markdown));
      Err(Diagnostic::manual_review_required(table))
    }
```

- `compare` により差分検出。`from_change`（4.7）で監査情報を生成。
- CLI では `render_summary` を表示し、`manual_review_required` 診断で手動承認を促す。

## 7. 高度なスキーマ操作

### 7.1 スキーマバージョニング

```reml
pub type SchemaVersion = {
  major: u32,
  minor: u32,
  patch: u32,
  compatibility: CompatibilityLevel,
}

pub enum CompatibilityLevel = {
  FullyCompatible,
  BackwardCompatible,
  ForwardCompatible,
  BreakingChange,
}

fn check_compatibility(old: SchemaVersion, new: SchemaVersion) -> CompatibilityResult
fn auto_version_schema<T>(old: Schema<T>, new: Schema<T>) -> SchemaVersion
```

### 7.2 動的スキーマ生成

```reml
fn generate_from_sample<T>(samples: Iter<Json>, confidence: Float) -> Result<Schema<T>, InferenceError>
fn merge_schemas<T>(schemas: List<Schema<T>>) -> Result<Schema<T>, MergeError>
fn optimize_schema<T>(schema: Schema<T>) -> Schema<T>  // 冗長フィールドの統合、型の簡略化
```

### 7.3 スキーマ演算

```reml
// スキーマ間のマッピング
fn map_schema<T, U>(from: Schema<T>, to: Schema<U>, mapping: FieldMapping) -> Result<U, MappingError>
fn transform_data<T, U>(data: T, from_schema: Schema<T>, to_schema: Schema<U>) -> Result<U, TransformError>

// スキーマの結合と分解
fn union_schemas<T>(schemas: List<Schema<T>>) -> Schema<T>
fn intersect_schemas<T>(schemas: List<Schema<T>>) -> Option<Schema<T>>
fn project_schema<T>(schema: Schema<T>, fields: List<Str>) -> Schema<T>  // フィールドサブセットの抽出
```

> 関連: [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.2 Core Collections](3-2-core-collections.md)

> 注意: 本章は 2.7 設定スキーマ API と 2.8 データモデリング API の内容を Chapter 3 に移行統合したものです。
