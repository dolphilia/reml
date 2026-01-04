# 3.7 Core Config & Data

> 目的：設定スキーマ (`Core.Config`) とデータモデリング (`Core.Data`) を Chapter 3 の標準ライブラリ体系へ統合し、差分管理・監査・CLI ツールとの連携を明文化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 正式仕様 |
| 効果タグ | `@pure`, `effect {diagnostic}`, `effect {config}`, `effect {audit}`, `effect {io}`, `effect {migration}` |
| 依存モジュール | `Core.Prelude`, `Core.Collections`, `Core.Diagnostics`, `Core.IO`, `Core.Numeric & Time` |
| 相互参照 | [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), [3.5 Core IO & Path](3-5-core-io-path.md), [3.2 Core Collections](3-2-core-collections.md) |

> **移行メモ**: 旧仕様で Chapter 2 に置かれていた設定スキーマ（2.7）とデータモデリング（2.8）の記述はアーカイブ済みで、最新仕様では本章に統合されています。古いドラフトを参照する場合は `../notes/` 配下の移行ノートを確認してください。
>
> **段階的導入ポリシー**: マニフェストやスキーマに新しい効果タグや Capability を追加する場合は、`Manifest.expect_effects_stage` や `Schema.metadata.stage` へ `Experimental` / `Beta` / `Stable` を記録し、`reml config lint` が未承認のステージを警告できるようにする。実験機能を本番へ昇格させる際は `../notes/algebraic-effects-implementation-roadmap-revised.md` のチェックリストに従い、`@dsl_export` とマニフェスト値が一致することを確認する。

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

> **NOTE**: 本章で利用する `Set<T>` 型は [3.2 Core Collections](3-2-core-collections.md) の定義に従う。実行時表現の概要は [3.2 §2.2.1](3-2-core-collections.md#set-runtime-abi) を参照。

- `DslExportRef.signature` はコンパイラが `@dsl_export` から抽出した `DslExportSignature` を JSON にシリアライズして格納する（未解析時は `None`）。
- `expect_effects` は 1.3 §I.1 の効果境界と突き合わせるための期待集合。CI などではこれを上限として用いる。
- `allow_prerelease` が `true` の場合、互換判定で pre-release バージョンを許容する（1.2 §G 参照）。
- `signature.requires_capabilities` は Capability Registry で検証済みの Stage 要件と効果スコープを保持し、`capabilities` フィールドはその ID を投影した派生値として扱う。`transform_capability_manifest_to_reml`（3-8 §7.4）で外部マニフェストと同期する際は、この集合に `StageRequirement` が落とし込まれる。
- `signature.stage_bounds` は DSL エクスポートの公開ステージ・下限・上限を記録し、`expect_effects_stage` や `reml capability stage promote` の結果と突き合わせる。`maximum = None` の場合は上限を未設定として扱い、Capability 側の判定を優先する。

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

#### 1.4.1 Capability マニフェストとの同期

1. 外部システムから提供された Capability マニフェスト（GraphQL/JSON Schema 等）は `Core.Runtime.DslCapability.transform_capability_manifest_to_reml`（3-8 §7.4）で `CapabilityBridgeSnapshot`（以下 `snapshot`）として取り込む。
2. `update_dsl_signature` は `snapshot.requirements` の各要素について `StageRequirement`・`effect_scope` を再構成し、`capabilities` に派生値を設定する。Stage 要件がマニフェスト側の上限 (`stage_bounds.maximum`) を超える場合は `diagnostic("manifest.dsl.stage_mismatch")` を生成する。
3. `validate_manifest` は DSL ごとに `requires_capabilities` と `dsl.<name>.capabilities` の差分を検証し、未宣言の Capability を検出した場合は `diagnostic("manifest.dsl.capability_missing")` を返す。Stage 下限 (`snapshot.stage_bounds.minimum`) を満たさない環境構成が見つかった場合は `ConfigDiagnosticExtension.capability` に詳細を格納して監査ログへ送る。
4. 0-1 §1.2 の安全性指針に基づき、Stage 範囲が不一致のまま `allow_prerelease=true` を利用することは禁止とし、`snapshot.stage_bounds.maximum` が設定されている場合は必ず優先し、該当ケースでは `diagnostic("manifest.dsl.stage_prerelease_conflict")` を返す。

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
- 仕様変更で既定値を緩和する場合は `AuditEvent::ConfigCompatChanged`（3-6 §1.1.1）を必須とし、`AuditEnvelope.metadata` へ `config.source` / `config.format` / `config.profile` / `config.compatibility` を必ず記録する。マニフェストの `config.compatibility.<format>` には `compatibility=relaxed` などのタグを追加し、履歴を追跡可能にする。

```toml
[config.compatibility.json]
profile = "json-relaxed"
trailing_comma = "arrays"
feature_guard = ["json5", "bare_keys"]
```

上記のように `config.compatibility.<format>` テーブルへ列挙体の文字列表現を記述すると、`Manifest::compatibility_layer` と CLI が Stage 情報に応じて `ConfigCompatibility` を再構築する。

#### 1.5.2 解決順序と責務

`resolve_compat` は以下の優先順位でプロファイルを決定する：

1. CLI パラメータ（`RunConfig.cli_overrides.compat`）が存在すればそれを採用する。`reml config lint --compat relaxed-json` などのオプションはここへ反映され、監査ログでは `AuditEvent::ConfigCompatChanged` の `config.source` を `"cli"` として保存する。
2. 環境変数による上書き（3-10 §2.1）を `RunConfig.extensions["config"].env_compat` として取り込み、CLI 指定がない場合にのみ適用する。互換プロファイル名と feature guard の双方が対象であり、未知値は `Diagnostic.code = "config.compat.env_invalid"` で拒否する。
3. `reml.toml` の `config.compatibility.<format>` を Stage と feature guard を検証した上でマージし、欠落フィールドは厳格プロファイル側を優先する。
4. それ以外は `ConfigCompatibility::stable(format)`（=`compatibility_profile(format, Stage::Stable)`）へフォールバックする。

互換モードを切り替えた際は `Core.Diagnostics` が `AuditEvent::ConfigCompatChanged` を記録し、`Diagnostic.severity` を `Warning` 以上に設定することで 0-1 §1.2 の安全性を維持する。CLI/LSP/ビルドは `resolve_compat` の結果を共通で使用し、環境差異による設定解析の不一致を防止する。また CLI/CI は優先順位が実装どおりであることを保証するための準拠テスト（CLI 指定 > 環境変数 > マニフェスト > 既定値）を `Core.TestKit::config_compat_order` で提供し、逆順の上書きが発生した場合はビルドを失敗させる。Rust 実装の CLI (`reml_frontend --config-compat <profile>`) はこの優先順位の第 1 層を直接操作するエントリポイントとして実装されており、互換プロファイル名を解析できなかった場合は CLI で即座にエラーを返す。

このワークフローにより、マニフェスト・言語仕様・CLI が同じ DSL メタデータと互換モードを共有できる。詳細な記述ガイドは `../guides/manifest-authoring.md` と `../guides/config-compatibility.md`（新規作成予定）で扱う。

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

#### 1.5.4 Config 診断拡張の適用

- Config 関連 API が診断を生成する際は、3-6 §6.1.3 で定義した `ConfigDiagnosticExtension` を `Diagnostic.extensions["config"]` に格納する。これにより CLI/LSP/監査ログが同じ情報粒度で設定問題を提示できる。
- `source` は `resolve_compat` の優先順位に沿って決定する。CLI で明示された場合は常に `ConfigSource::Cli` とし、環境変数からの上書き時は `ConfigSource::Env` を設定して `AuditEnvelope.metadata["config.source"] = "env"` を記録する。
- `key_path` は設定ファイルのルートからの完全パスを表し、TOML のテーブルや配列を `ConfigKeySegment` のリストとして保持する。`manifest_path` が `Some(path)` の場合、LSP はこの情報を利用して直接該当位置をハイライトする。
- `compatibility` と `feature_guard` は `ConfigCompatibility`／`feature_guard` の検証結果を反映させる。たとえば Stage ミスマッチを検出した場合は `FeatureGuardDigest.actual_stage = Stage::Experimental`、`expected_stage = Stage::Stable` とし、`cfg_condition` に `@cfg(target = "prod")` のような実際の条件を記録する。
- 差分検証（`compare`, `plan`, `apply_diff` 等）では `diff` を必ず埋め、`missing` や `unexpected` のキーは `ConfigKeySegment` と同じ順序で再構築できるよう `Str` ベースで表現する。非公開データが含まれる場合は `snapshot` を空にし、`AuditPolicy.anonymize_pii = true` のときのみダンプを許可する。

```reml
fn config_extension_from_ctx(ctx: ConfigLintContext, issue: ConfigIssue) -> ConfigDiagnosticExtension =
  ConfigDiagnosticExtension {
    source: ctx.source,
    manifest_path: ctx.manifest_path,
    key_path: issue.key_path,
    profile: ctx.profile,
    compatibility: ctx.compatibility.map(|compat|
      ConfigCompatibilityDigest {
        format: compat.format,
        profile: compat.profile,
        stage: compat.stage,
      }),
    feature_guard: issue.feature_guard,
    schema: issue.schema_id,
    diff: issue.diff_summary,
    snapshot: ctx.snapshot,
  }
```

- 上記ユーティリティは `ConfigLintContext`（CLI 側で収集したメタデータ）と `ConfigIssue`（検証結果）を組み合わせ、`ConfigDiagnosticExtension` を組み立てる例である。`feature_guard` が `None` の場合でも構造は維持されるため、後続ツールは空値を検知して「同期済み」と判断できる。
- `AuditEnvelope.metadata` には `config_extension_from_ctx` の結果をコピーし、`config.diff` や `config.feature_guard` を JSON 化したうえで保存する。監査ログの再解析時に元の診断を再構築できることが 0-1 §1.2 の安全性要件を満たす条件となる。

#### 1.5.5 feature_guard と `@cfg` の同期検証

- `feature_guard` は設定ファイルが前提とする互換機能の **ソースオブトゥルース** であり、`RunConfig.extensions["config"].features` と `RunConfigTarget.features` の双方を通じてコンパイラへ共有される。CLI/ビルドはこの集合を `ResolveFeatureGuard` フェーズで確定し、以降の `@cfg` 評価と診断生成で再利用する。
- 構文解析は `@cfg(feature = "...")`、`@cfg(any(...))`、`@cfg(all(...))` で参照された機能名を `CfgFeatureSet` として収集する。`CfgFeatureSet` はマクロ展開・DSL 展開後の AST 単位で重複を排除し、`RunConfig.extensions["target"].feature_requirements` に保存する。
- 検証アルゴリズム：
  1. `compat_declared = ConfigCompatibility.feature_guard`
  2. `target_active = RunConfigTarget.features`
  3. `cfg_required = RunConfig.extensions["target"].feature_requirements`
  4. 差集合を求める：
     - `missing_in_compat = cfg_required \ compat_declared`
     - `missing_in_target = cfg_required \ target_active`
     - `extra_in_compat = compat_declared \ target_active`
  5. いずれかが非空の場合、診断 `config.feature.mismatch` を生成する。
- `config.feature.mismatch` は `Diagnostic.extensions["config"].feature_guard = Some(FeatureGuardDigest)` を要求し、`FeatureGuardDigest.feature` には差集合ごとに検出した機能名を格納する。`cfg_condition` には該当する `@cfg` 条件式（例：`any(feature = "json5", feature = "experimental_syntax")`）を文字列化して保存し、再現性を保証する。
- `missing_in_target` が非空の場合は Stage に関わらず `Severity::Error` を既定とし、0-1 §1.2 の安全性原則を満たすためにビルドを失敗させる。`extra_in_compat` のみが非空の場合は `Severity::Warning` を推奨し、CLI は自動修正 (`--fix`) で `feature_guard` をターゲット値へ同期できるようにする。`missing_in_compat` は Stage::Stable では `Severity::Error`、Stage::Experimental では `Severity::Warning` を推奨する。
- CLI/LSP は `config.feature_guard` の差分を UI 上で強調表示し、`resolve_compat` 実行後に `RunConfig.extensions["config"].features = compat_declared` を再設定する。これにより `feature_guard` が同期した状態で `@cfg` 判定が行われ、実行時挙動の差異を抑止できる。

### 1.6 Manifest と Schema のバージョン互換

`project.version`（SemVer 文字列）と `Schema.version`（`SchemaVersion { major, minor, patch }`）は同じ互換境界で管理する。互換条件は以下の通り。

1. `major` が一致していること。Schema の major が `project.version.major` と異なる場合、互換性なしとして CLI/CI を停止する。
2. `(major, minor, patch)` のタプル比較で `project.version >= Schema.version` になること。Schema 側の minor または patch が先行している場合も互換不可とみなす。
3. `Schema.version = None` の場合のみチェックをスキップし、移行プランで version を確定させる。

Rust 実装では `ensure_schema_version_compatibility(manifest: &Manifest, schema: &Schema)` を提供し、CLI/RunConfig/CI から同じ判定を利用できる。診断コードと必須メタデータは以下。

| コード | 条件 | メタデータ（extensions / audit） |
| --- | --- | --- |
| `config.project.version_invalid` | `project.version` を SemVer として解析できない | `manifest_version`, `schema_name`, `version_mismatch = "parse_error"` / `config.version_reason = "parse_error"` |
| `config.schema.version_incompatible` | major 不一致 or Schema 側が新しい | `manifest_version`, `schema_version`, `schema_name`, `version_mismatch ∈ {"major","schema_ahead"}` / `config.version_reason` 同値 |

監査ログでは `config.schema_name` / `config.schema_version` / `config.version_reason` を KPI として扱い、`MIGRATION-BLOCKER-*` を登録する際の根拠とする。

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

> **実装メモ（Phase 3-7）**: Rust 実装では `reml_runtime::config::migration` モジュール（`compiler/runtime/src/config/migration.rs`）に `MigrationPlan`/`MigrationStep`/`RiskLevel` を実装し、`Cargo` フィーチャ `experimental-migration` 有効時のみ公開している。CLI からプランを生成する段階では `effect {migration}` を付与し、監査ログに `config.migration.*` メタデータを残すことを前提にする。【P:docs/plans/bootstrap-roadmap/3-7-core-config-data-plan.md#5.1】

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

### 4.1 Nest.Data スキーマ構築 {#nest-data-schema}

```reml
use Nest.Data

let userSchema = Schema.build("User", |s| {
  s.field("id", Column<Guid, { nullable = false }>)
   .field("email", Column<Text, { nullable = false, description = "連絡先" }>)
   .field("signup_at", Column<DateTime, { nullable = false }>)
   .field("score", Column<f64, { nullable = true, stats = Some({ mean = 0.0, stddev = 1.0, ..ColumnStats::zero }) }))
   .index("pk_user", columns = ["id"], unique = true)
})
```

- `Schema.build` は `SchemaBuilder` のチェーン API を利用して列・制約・インデックスを宣言する。`ColumnMeta.stats` を指定すると `reml-data validate --stats` と監査ログに同じ統計が出力される。
- `Column<Text, ColumnMeta>` の `description` は CLI/LSP の診断やドキュメント生成に利用される。
- インデックス定義は `SchemaDiff`（§2.1）で追跡され、破壊的変更は `ChangeSet` を通じてレビューされる。

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

- `infer_schema` はサンプル JSON から推論したカラム統計を返し、`Diagnostic.extensions["data"].inference` に推論根拠を格納する。`effect {audit}` を伴い、推論経路を監査ログへ残す。

### 4.2 制約とプロファイル検証

```reml
struct EmailFormat;
impl Constraint<Text> for EmailFormat {
  fn id() -> Str = "constraint.email.format"
  fn check(value: &Text, ctx: ConstraintContext) -> Result<(), Diagnostic> {
    if Regex::is_match("^[^@]+@[^@]+$", value) {
      Ok(())
    } else {
      Err(Diagnostic::error(ctx.path, "メールアドレス形式が不正です")
            .with_code("data.email.format")
            .finish())
    }
  }
}

let hardenedSchema = userSchema.with(|s| {
  s.constraint("email", EmailFormat)
   .constraint("score", Range::new(-10.0, 10.0))
})
```

- `ConstraintContext.profile` は `prod`/`staging` などのプロファイル識別子を提供し、`Profile::overrides()` で閾値を差し替える。診断には `domain = "schema"` と `code` を必ず付与する。
- `Constraint::id()` は CLI/監査ログで利用される識別子であり、値が変更された場合は `../guides/data-model-reference.md` の用語集と同期する。

### 4.3 プロファイル別評価とメトリクス

```reml
struct ProdProfile;
impl Profile for ProdProfile {
  fn id(&self) -> ProfileId = ProfileId::new("prod")
  fn overrides(&self) -> Map<Str, Any> = map!{ "score.max" => 5.0 }
}

let report = validate_with_profile(hardenedSchema, incoming, &ProdProfile)?;
if !report.diagnostics.is_empty() {
  emit_metrics("data.validation", {
    latency_ms = 12.4,
    throughput_per_min = 320.0,
    error_rate = report.diagnostics.len() as f64 / incoming.len() as f64,
    last_audit_id = report.audit_id,
    custom = map!{ "profile" => "prod" }
  })
}
```

- `ValidationReport.audit_id` は `reml-data` の JSON と監査イベント `reml.data.validate` で共有される。`emit_metrics` は 0-1 §1.1 の性能指標を同時に送信する。
- `custom.profile` を利用してダッシュボード上でプロファイル別の品質傾向を追跡する。

### 4.4 QualityReport スキーマ {#quality-report-schema}

Quality レポートは JSON スキーマで公開され、CLI/LSP/監査で共通に利用する。

```json
{
  "$id": "https://spec.reml.dev/schema/quality-report.json",
  "type": "object",
  "required": ["profile", "findings", "stats", "generated_at"],
  "properties": {
    "profile": {"type": "string"},
    "audit_id": {"type": ["string", "null"], "format": "uuid"},
    "generated_at": {"type": "string", "format": "date-time"},
    "severity_max": {"enum": ["None", "Warn", "Error"]},
    "stats": {
      "type": "object",
      "additionalProperties": {"$ref": "#/definitions/columnStats"}
    },
    "findings": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["rule", "scope", "severity", "diagnostic", "auto_fixed"],
        "properties": {
          "rule": {"type": "string"},
          "scope": {"$ref": "#/definitions/qualityScope"},
          "severity": {"enum": ["Warn", "Error"]},
          "diagnostic": {"$ref": "https://spec.reml.dev/schema/diagnostic.json"},
          "auto_fixed": {"type": "boolean"}
        }
      }
    }
  },
  "definitions": {
    "qualityScope": {
      "oneOf": [
        {"type": "object", "required": ["Column"], "properties": {"Column": {"type": "object", "required": ["name"], "properties": {"name": {"type": "string"}}}}},
        {"const": "Dataset"},
        {"type": "object", "required": ["Relation"], "properties": {"Relation": {"type": "object", "required": ["columns"], "properties": {"columns": {"type": "array", "items": {"type": "string"}}}}}}
      ]
    },
    "columnStats": {
      "type": "object",
      "required": ["count"],
      "properties": {
        "count": {"type": "integer", "minimum": 0},
        "distinct": {"type": ["integer", "null"], "minimum": 0},
        "min": {"type": ["number", "null"]},
        "max": {"type": ["number", "null"]},
        "mean": {"type": ["number", "null"]},
        "stddev": {"type": ["number", "null"]},
        "percentiles": {
          "type": ["object", "null"],
          "additionalProperties": {"type": "number"}
        },
        "histogram": {
          "type": ["array", "null"],
          "items": {
            "type": "object",
            "required": ["bucket", "count"],
            "properties": {
              "bucket": {
                "type": "object",
                "required": ["label", "min", "max"],
                "properties": {
                  "label": {"type": "string"},
                  "min": {"type": "number"},
                  "max": {"type": "number"}
                }
              },
              "count": {"type": "integer", "minimum": 0}
            }
          }
        },
        "last_updated": {"type": ["string", "null"], "format": "date-time"}
      }
    }
  }
}
```

- `severity_max` は CLI / 監査ログの両方で整合させる。スキーマ違反を検出するテストケースは `qualityReportSchema` JSON を利用し、CI で `jsonschema` 検証を行う。
- `columnStats.histogram` に重複バケットが存在する場合、`Diagnostic.code = "data.stats.invalid_bucket"` を返す。

### 4.5 監査ログ統合

| イベント ID | 出所 | 主なフィールド |
| --- | --- | --- |
| `reml.data.validate` | `reml-data validate` | `audit_id`, `diagnostics`, `profile`, `stats` |
| `reml.data.migrate` | `reml-data migrate` | `audit_id`, `changes`, `duration_ms`, `status` |
| `reml.data.rollback` | `reml-data migrate --rollback` | `audit_id`, `actions`, `reason` |
| `reml.data.quality` | `reml-data quality run` | `audit_id`, `profile`, `findings`, `severity_max`, `stats` |
| `reml.data.quality.rule` | `register_quality_rule`, `reml-data quality rules list` | `rule_id`, `scope`, `severity`, `owner` |

- 監査イベントは [3.6](3-6-core-diagnostics-audit.md) の JSON 契約に従い、`audit_id` をもとに CLI 出力と監査ログを照合する。`severity_max` の不一致は `AuditEvent::DataQualityMismatch` を生成して即時に失敗させる。

### 4.6 CLI / ツール連携

代表的な `reml-data` コマンドは以下の通りで、すべて `QualityReport` スキーマと互換である。

- `reml-data validate data/users.json --schema schemas/user.ks --profile prod --format json --locale ja-JP`
- `reml-data diff --schema-old schemas/user_v1.ks --schema-new schemas/user_v2.ks --format json --locale en-US`
- `reml-data migrate --diff diff.json --input data/import.parquet --output data/output.parquet --locale en-US`
- `reml-data quality run data/users.json --schema schemas/user.ks --profile staging --format json --locale ja-JP`
- `reml-data stats collect --schema schemas/user.ks --provider warehouse --format json`

各コマンドは `audit_id` と `diagnostics[].locale` を含む JSON を出力し、LSP/CI が翻訳カタログと統計を共有できる。

### 4.7 データ品質検証 API

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

- `QualityReport` の `findings[].auto_fixed` は `auto_fix_quality_issues` の結果と同期し、監査ログの `reml.data.quality` イベントで `auto_fix=true` と一致しなければならない。
- `validator` が `Err` を返した場合、`QualityViolation` は `Diagnostic` を内包し、CLI は `severity` に応じて Exit Code を決定する。

### 4.8 統計との連携

```reml
fn update_stats(column: ColumnStats, values: Iter<Json>) -> Result<ColumnStats, Diagnostic> // `@pure`
fn merge_stats(left: ColumnStats, right: ColumnStats) -> ColumnStats                        // `@pure`
fn as_metric(points: ColumnStats) -> List<MetricPoint<Float>>                               // `@pure`
```

- `ColumnStats` は `count`・`distinct`・`percentiles` などの指標を保持する。`update_stats` は重複したヒストグラム区間を検出すると `Diagnostic.code = "data.stats.invalid_bucket"` を返す。
- `as_metric` は [3.4](3-4-core-numeric-time.md) の `MetricPoint` を用いて監視基盤へ送信する値を生成する。

### 4.9 ベストプラクティス

1. スキーマとコードを単一リポジトリで管理し、CI で常に `reml-data validate` を実行する。
2. バッチ処理後に `ColumnStats.last_updated` を更新し、監査ログへ出力する。
3. `MigrationStep.breaking=true` を含む差分はプラグイン承認フローと同等のレビューを要求する。
4. 監視メトリクスと監査ログで `audit_id` を共有し、品質逸脱と統計ドリフトを同じダッシュボードで追跡する。

## 5. CLI / ツール連携

CLI や LSP から利用するユーティリティを明示する。代表的な `reml-data` コマンドは §4.6 を参照。

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

> 注意: 本章は Chapter 2 初期ドラフトで扱っていた設定スキーマ API とデータモデリング API の内容を統合したものです。
