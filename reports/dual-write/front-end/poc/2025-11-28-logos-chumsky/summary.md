| case | ocaml_ast | ocaml_diag_count | rust_status |
| --- | --- | --- | --- |
| empty_uses | fn answer() = int(42:base10) | 0 | 未実行 (依存取得不可) |
| multiple_functions | fn log(x) = var(x)<br>fn log_twice(x) = call(var(log))[call(var(log))[var(x)]] | 0 | 未実行 (依存取得不可) |
| addition | fn add(x, y) = binary(var(x) + var(y)) | 0 | 未実行 (依存取得不可) |
| missing_paren | (解析失敗) | 1 | 未実行 (依存取得不可) |
