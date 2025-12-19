# 3.12 Core Cli

> 目的：DSL 向け CLI を宣言的に構築し、ヘルプ生成・診断出力・型安全な解析を標準化する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | ドラフト（Phase 4 対象） |
| 効果タグ | `effect {io}` |
| 依存モジュール | `Core.Prelude`, `Core.Env`, `Core.Diagnostics` |
| 相互参照 | [3-10 Core Env](3-10-core-env.md), [3-6 Core Diagnostics & Audit](3-6-core-diagnostics-audit.md), Guides: [cli-authoring](../guides/cli-authoring.md) |

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

## 3. ヘルプ生成

- `CliSpec` から使用法を自動生成し、未知のフラグや引数不足の際は `CliError.hint` へ提案文を入れる。
- CLI 出力は `Core.Diagnostics` の CLI フォーマット規約に従う。

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

  match Cli.parse(spec, ["--verbose", "input.reml"]) {
    Ok(values) => {
      let input = Cli.get_arg(values, "input")
      match input {
        Some(path) => if Cli.get_flag(values, "verbose") { "verbose:" + path } else { "quiet:" + path }
        None => "missing"
      }
    }
    Err(_) => "cli:error"
  }
}
```
