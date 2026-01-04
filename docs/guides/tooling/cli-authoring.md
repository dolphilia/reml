# Reml CLI 作成ガイド（Core.Cli）

> DSL 向けの CLI を `Core.Cli` で宣言的に構築するための最小ガイド。

## 1. 基本フロー
1. `Cli.builder()` で仕様を構築
2. `Cli.parse` で引数を解決
3. `Cli.get_flag` / `Cli.get_arg` で値を取得

参照: [3-12 Core Cli](../../spec/3-12-core-cli.md)

## 1.1 Core.Env との分担
- `Core.Cli` は `argv` の解析とヘルプ/診断整形を担い、環境変数やプラットフォーム情報は `Core.Env` に委譲する。
- CLI オプションの決定後に `Core.Env` の設定解決を行い、`RunConfig` へ反映する。

## 2. 最小例

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

## 3. 診断と監査
- 解析失敗は `cli.parse.failed` を返し、`Core.Diagnostics` で整形する。
- `AuditEvent::CliParse` に `cli.flags` / `cli.args` を記録する。
