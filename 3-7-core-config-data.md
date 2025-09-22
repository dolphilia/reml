# 3.7 Core Config & Data（フェーズ3 ドラフト）

Status: Draft（内部レビュー中）

> 目的：設定スキーマ (`Core.Config`) とデータモデリング (`Core.Data`) を Chapter 3 の標準ライブラリ体系へ統合し、差分管理・監査・CLI ツールとの連携を明文化する。

## 0. ドラフトメタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | Draft（フェーズ3） |
| 効果タグ | `@pure`, `effect {config}`, `effect {audit}`, `effect {io}` |
| 依存モジュール | `Core.Prelude`, `Core.Collections`, `Core.Diagnostics`, `Core.IO`, `Core.Numeric & Time` |
| 相互参照 | [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md) |

> **移行メモ**: Chapter 2 に残る 2.7/2.8 は参照用として維持されるが、本章で標準ライブラリ視点の API 契約と監査統合を再整理する。将来的に Chapter 2 版は概要＋互換ノートへ縮約する計画。

## 1. Config スキーマ API（再整理）

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

### 1.1 スキーマ差分

```reml
pub type SchemaDiff<T> = {
  added: List<Field<T>>,
  removed: List<Field<T>>,
  modified: List<FieldChange<T>>,
}

fn diff<T>(old: Schema<T>, new: Schema<T>) -> SchemaDiff<T>                    // `@pure`
fn apply_patch<T>(schema: Schema<T>, patch: Patch<T>) -> Schema<T>            // `@pure`
fn plan<T>(old: Schema<T>, new: Schema<T>) -> ChangeSet                       // `@pure`
```

- `ChangeSet` は監査ログ（4.7）で利用する差分形式。`plan` は CLI/CI でレビュー可能なパッチを生成する。

## 2. Config 実行 API

```reml
fn load(path: Path, schema: Schema<T>) -> Result<T, Diagnostic>                // `effect {io, config}`
fn validate<T>(value: T, schema: Schema<T>) -> Result<(), Diagnostic>          // `@pure`
fn compare<T>(old: T, new: T, schema: Schema<T>) -> Result<(), ChangeSet>     // `@pure`
fn apply_diff<T>(value: T, diff: ChangeSet) -> Result<T, Diagnostic>           // `effect {config}`
```

- `load` は 4.6 の IO API と連携。`Diagnostic` には `audit_id` と `change_set` が付与される。
- `compare` は差分が発生した場合 `Err(ChangeSet)` を返し、`ChangeSet` を監査へ送る想定。

## 3. Data モデリング API（再整理）

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

### 3.1 統計との連携

```reml
fn update_stats(column: ColumnStats, values: Iter<Json>) -> Result<ColumnStats, Diagnostic> // `@pure`
fn merge_stats(left: ColumnStats, right: ColumnStats) -> ColumnStats                        // `@pure`
fn as_metric(points: ColumnStats) -> List<MetricPoint<Float>>                               // `@pure`
```

- `MetricPoint` は [3.4](3-4-core-numeric-time.md) で定義。データ品質監査へ転送するためのラッパ。

## 4. CLI / ツール連携

設定 CLI や LSP から利用するユーティリティを明示する。

```reml
fn diff_to_table(diff: ChangeSet) -> Table<Str, Json>                      // `effect {mut}`
fn render_summary(diff: ChangeSet, fmt: OutputFormat) -> String            // `effect {mem}`
fn attach_exit_code(diag: Diagnostic) -> ExitCode                          // `@pure`
```

- `Table` は 3.2 の可変コレクション。CLI 表形式へ変換する際に使用。
- `OutputFormat` は CLI/JSON/Markdown 等に対応。
- `ExitCode` は CLI ツールが戻す整数コード。

## 5. 使用例（差分レビュー）

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

> 関連: [3.6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md)

> 注意: 本章は 2.7 設定スキーマ API と 2.8 データモデリング API の内容を Chapter 3 に移行統合したものです。
