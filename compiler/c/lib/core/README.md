# Reml C 標準ライブラリ (Core) 移植メモ

このディレクトリは C 実装向けの Core 標準ライブラリの Reml ソースを配置するための領域です。
Phase 6 の 6.2 で必要な最小 API を順次移植し、C ランタイム ABI と接続します。

## 構成

- `Core.reml`: `Core` ルートの再エクスポート。
- `Core/Prelude.reml`: 基本型と頻出モジュールの再エクスポート。
- `Core/Option.reml` / `Core/Result.reml`: 合成型と補助関数。
- `Core/String.reml` / `Core/Bytes.reml`: 文字列・バイト列の最小 API。
- `Core/Collections.reml` / `Core/Collections/*`: List/Map/Set の最小 API。
- `Core/Math.reml` / `Core/Int.reml` / `Core/Float.reml` / `Core/BigInt.reml`: 数値 API の入口。
- `Core/IO.reml` / `Core/Env.reml` / `Core/Time.reml`: ランタイム ABI と結合する API。

## Rust 実装の参照元

C 版の API は Rust 実装を参照しつつ最小限から合わせる。

- `compiler/rust/runtime/src/prelude` -> `Core.Prelude`
- `compiler/rust/runtime/src/collections` -> `Core.Collections`
- `compiler/rust/runtime/src/text` -> `Core.String` / `Core.Bytes`
- `compiler/rust/runtime/src/numeric` -> `Core.Math` / `Core.Int` / `Core.Float` / `Core.BigInt`
- `compiler/rust/runtime/src/io` -> `Core.IO`
- `compiler/rust/runtime/src/env.rs` -> `Core.Env`
- `compiler/rust/runtime/src/time` -> `Core.Time`

## @intrinsic マッピング (暫定)

Reml の `@intrinsic` 文字列は C 側シンボル名に合わせ、C ランタイムへ委譲する。

| Reml API | `@intrinsic` | C 実装 |
| --- | --- | --- |
| `Core.IO.print` | `reml_print` | `compiler/c/lib/runtime/runtime.c` |
| `Core.IO.eprint` | `reml_eprint` | `compiler/c/lib/runtime/runtime.c` |
| `Core.IO.read_file` | `reml_read_file` | `compiler/c/lib/runtime/runtime.c` |
| `Core.IO.write_file` | `reml_write_file` | `compiler/c/lib/runtime/runtime.c` |
| `Core.Time.now_unix_ms` | `reml_time_now` | `compiler/c/lib/runtime/runtime.c` |
| `Core.Env.get` | `reml_env_get` | `compiler/c/lib/runtime/runtime.c` |
| `Core.Env.args` | `reml_args` | `compiler/c/lib/runtime/runtime.c` |
| `Core.Env.cwd` | `reml_cwd` | `compiler/c/lib/runtime/runtime.c` |
| `Core.String.len_bytes` | `reml_str_len_bytes` | `compiler/c/src/text/string.c` |
| `Core.String.len_graphemes` | `reml_str_len_graphemes` | `compiler/c/src/text/string.c` |

## Unicode 連携

- UTF-8 検証と NFC 正規化: `compiler/c/src/text/unicode.c` (`reml_unicode_*`).
- 書記素クラスタ分割: `compiler/c/src/text/grapheme.c` (`reml_grapheme_*`).
- Reml 側は `Core.String` / `Core.Unicode` で橋渡しし、UTF-8 の検証責務を `Core` に置く。

## 診断 ID 方針 (暫定)

C ランタイムの診断 ID は 3-6 章の診断分類に合わせて文字列コードへ寄せる。
現在の数値 ID は `Core.IO` / `Core.Env` / `Core.Time` 側で `Diagnostic.code` を付与する際の基準とする。

| C 診断 ID | 暫定コード | 対応カテゴリ |
| --- | --- | --- |
| `REML_DIAG_RUNTIME_OOM` | `runtime.oom` | Runtime / Memory |
| `REML_DIAG_RUNTIME_IO` | `runtime.io` | Runtime / IO |
| `REML_DIAG_RUNTIME_ENV` | `runtime.env` | Runtime / Env |
| `REML_DIAG_RUNTIME_ARGS` | `runtime.args` | Runtime / Args |
| `REML_DIAG_RUNTIME_CWD` | `runtime.cwd` | Runtime / IO |
| `REML_DIAG_RUNTIME_TIME` | `runtime.time` | Runtime / Time |

正式なコード名は `docs/spec/3-6-core-diagnostics-audit.md` の診断カタログに合わせて更新する。
