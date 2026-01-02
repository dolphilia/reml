# 3.12 Core Cli

> 目的：DSL 向け CLI を宣言的に構築し、ヘルプ生成・診断出力・型安全な解析を標準化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}` |
| 依存モジュール | `Core.Prelude`, `Core.Env`, `Core.Diagnostics` |
| 相互参照 | [3-10 Core Env](3-10-core-env.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [cli-authoring](../guides/tooling/cli-authoring.md) |

## 1. 基本概念

`Core.Cli` は CLI 仕様 (`CliSpec`) と実行時値 (`CliValues`) を分離し、宣言的なビルダーで構築した仕様を `parse` で解決する。解析失敗は `CliError` で返す。

## 2. 型と API

```reml
pub type CliSpec = {
  name: Str,
  version: Option<Str>,
  description: Option<Str>,
  entries: List<CliEntry>,
}

pub enum CliEntry =
  | Flag { name: Str, help: Str }
  | Arg { name: Str, help: Str }
  | Command { name: Str, help: Str, spec: CliSpec }

pub type CliValues = {
  flags: Set<Str>,
  args: Map<Str, Str>,
  command: Option<Str>,
}

pub type CliBuilder

pub type CliError = {
  kind: CliErrorKind,
  message: Str,
  hint: Option<Str>,
}

pub enum CliErrorKind =
  | MissingArgument
  | UnknownFlag
  | UnknownCommand
  | ValueParseFailed

fn builder() -> CliBuilder
fn flag(builder: CliBuilder, name: Str, help: Str) -> CliBuilder
fn arg(builder: CliBuilder, name: Str, help: Str) -> CliBuilder
fn command(builder: CliBuilder, name: Str, help: Str, spec: CliSpec) -> CliBuilder
fn build(builder: CliBuilder) -> CliSpec
fn parse(spec: CliSpec, argv: List<Str>) -> Result<CliValues, CliError>
fn get_flag(values: CliValues, name: Str) -> Bool
fn get_arg(values: CliValues, name: Str) -> Option<Str>
```

### 2.1 Core.Env との役割分担
- `Core.Cli` は `argv`（引数配列）の解析とヘルプ/診断整形を担当し、環境変数・実行中プラットフォーム情報の取得は `Core.Env` に委ねる。
- `Core.Env` で取得した設定値（互換プロファイル、ターゲット情報など）は CLI 側が優先されるため、CLI オプションの解決後に `RunConfig` へ反映する。
- `Core.Cli` は環境変数を直接読み込まず、必要な値は `Core.Env` に委譲することで責務を明確にする。

## 3. ヘルプ生成

- `CliSpec` から使用法を自動生成し、未知のフラグや引数不足の際は `CliError.hint` へ提案文を入れる。
- CLI 出力は `Core.Diagnostics` の CLI フォーマット規約に従い、`CliError` 由来の診断は `cli.parse.failed` を既定とする。

### 3.1 ヘルプ整形規則
- **出力構造**: `Usage` → `Summary` → `Commands` → `Arguments` → `Flags` → `Examples` の順で出力する。
- **Usage 行**: `name` が指定されている場合は `name` を使用し、未指定の場合は `cli` を表示する。
- **Summary**: `description` があれば 1 行で表示し、無い場合は省略する。
- **Commands**: `CliEntry::Command` を宣言順で列挙し、`name` と `help` を表示する。
- **Arguments**: `CliEntry::Arg` を宣言順で列挙し、`name` と `help` を表示する。
- **Flags**: `CliEntry::Flag` を宣言順で列挙し、`--` 付きの `name` と `help` を表示する。
- **Examples**: 仕様例の `argv` をそのまま使える形で 1 行ずつ表示する。例が無い場合は省略する。

#### ヘルプ出力の最小形（例）
```
Usage:
  reml-dsl <command> [args]

Summary:
  DSL ツールの実行エントリ

Commands:
  parse     parse input
  validate  validate input
  format    format input

Arguments:
  input     input file

Flags:
  --verbose verbose log
```

### 3.2 エラー整形規則
- **先頭行**: `error: <CliError.message>` を 1 行で出力する。
- **ヒント行**: `CliError.hint` がある場合は `hint: <hint>` を 1 行で出力する。
- **使用法**: `Usage:` を短縮形で 1 行表示し、最小引数・フラグを示す（`CliSpec` の宣言順）。
- **ヘルプ誘導**: `help: --help を参照` の固定文言を末尾に出力する。
- **診断連携**: 上記の文字列は `Core.Diagnostics` の CLI 表示に統合され、`cli.parse.failed` の `Diagnostic.message` と `notes` に反映される。

## 4. 診断と監査

- 解析失敗は `Diagnostic.code = "cli.parse.failed"` を既定とする。
- `AuditEvent::CliParse` に `cli.command` / `cli.flags` / `cli.args` を記録する。

## 5. 例

```reml
use Core.Cli

fn main() -> Str {
  let spec = Cli.builder()
    .flag("verbose", help = "verbose log")
    .arg("input", help = "input file")
    .build()

  match Cli.parse(spec, ["--verbose", "input.reml"]) with
  | Ok(values) -> {
      let input = Cli.get_arg(values, "input")
      match input with
      | Some(path) ->
          if Cli.get_flag(values, "verbose") then "verbose:" + path else "quiet:" + path
      | None -> "missing"
    }
  | Err(_) -> "cli:error"
}
```
