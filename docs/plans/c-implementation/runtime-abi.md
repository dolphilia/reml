# Reml C Runtime ABI 一覧 (Phase 6)

本書は Phase 6 の `@intrinsic` で公開する C ランタイム ABI を整理したものです。

## 共通構造

### reml_result
- 目的: C 側の成功/失敗を Reml へ返す共通戻り値。
- 形式: `ok`/`err`、診断 ID、メッセージ。

```
typedef struct {
  bool ok;
  int32_t diagnostic_id;
  const char *message;
  size_t message_len;
} reml_result;
```

### reml_string / reml_str / reml_bytes
- `reml_string`: 所有する UTF-8 文字列 (ptr + len)。
- `reml_str`: 参照ビュー (ptr + len)。
- `reml_bytes`: バイト列 (ptr + len)。
- UTF-8 検証は Core 側で行い、Runtime はバイト列として扱う。

## エラーとパニック

- `reml_panic` は診断 ID を出力し `REML_PANIC_EXIT_CODE=70` で終了。
- OOM は `REML_DIAG_RUNTIME_OOM=1000` を返すかパニック。

## ABI: メモリ

| 関数 | 引数 | 戻り値 | 説明 | エラー |
| --- | --- | --- | --- | --- |
| `reml_alloc` | `size_t size` | `void *` | ヒープ確保。OOM でパニック。 | OOM | 
| `reml_free` | `void *ptr` | `void` | ヒープ解放。 | - |
| `reml_arena_init` | `reml_arena *arena, size_t capacity` | `bool` | アリーナ初期化。 | OOM |
| `reml_arena_alloc` | `reml_arena *arena, size_t size, size_t alignment` | `void *` | アリーナから確保。 | OOM |
| `reml_arena_reset` | `reml_arena *arena` | `void` | アリーナを再利用可能にする。 | - |
| `reml_arena_deinit` | `reml_arena *arena` | `void` | アリーナ解放。 | - |

## ABI: IO

| 関数 | 引数 | 戻り値 | 説明 | エラー |
| --- | --- | --- | --- | --- |
| `reml_print` | `reml_str message` | `reml_result` | stdout へ出力。 | IO | 
| `reml_eprint` | `reml_str message` | `reml_result` | stderr へ出力。 | IO |
| `reml_read_file` | `reml_str path, reml_bytes *out_bytes` | `reml_result` | ファイル読込。 | IO/OOM |
| `reml_write_file` | `reml_str path, reml_bytes data` | `reml_result` | ファイル書込。 | IO |
| `reml_bytes_free` | `reml_bytes bytes` | `void` | `reml_read_file` の解放。 | - |

## ABI: システム

| 関数 | 引数 | 戻り値 | 説明 | エラー |
| --- | --- | --- | --- | --- |
| `reml_time_now` | `int64_t *out_unix_ms` | `reml_result` | UNIX epoch ms を返す。 | TIME |
| `reml_env_get` | `reml_str key, reml_str *out_value` | `reml_result` | 環境変数を取得。 | ENV |
| `reml_runtime_set_args` | `int argc, const char **argv` | `void` | 引数配列を登録。 | - |
| `reml_args` | `reml_str **out_args, size_t *out_len` | `reml_result` | 引数配列を返す。 | ARGS |
| `reml_cwd` | `reml_string *out_cwd` | `reml_result` | カレントディレクトリ取得。 | CWD |

## 公開場所

- 実装: `compiler/c/lib/runtime/runtime.c`
- ヘッダ: `compiler/c/lib/runtime/runtime.h`
