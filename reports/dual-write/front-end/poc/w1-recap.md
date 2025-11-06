# W1 Packrat / SpanTrace Recap (2025-11-28 run)

## Packrat 統計 (dual-write)

```json
{
  "w1_dualwrite": [
    {
      "case": "addition",
      "ocaml_diag": 0,
      "rust_diag": 0,
      "ocaml_packrat": "0/0",
      "rust_packrat": "3/2"
    },
    {
      "case": "empty_uses",
      "ocaml_diag": 0,
      "rust_diag": 0,
      "ocaml_packrat": "0/0",
      "rust_packrat": "3/2"
    },
    {
      "case": "missing_paren",
      "ocaml_diag": 1,
      "rust_diag": 1,
      "ocaml_packrat": "0/0",
      "rust_packrat": "7/6"
    },
    {
      "case": "multiple_functions",
      "ocaml_diag": 0,
      "rust_diag": 0,
      "ocaml_packrat": "0/0",
      "rust_packrat": "6/4"
    }
  ]
}
```

## parse-debug サンプル (Rust)

```json
[
  {
    "case": "addition",
    "packrat": {
      "approx_bytes": 118,
      "budget_drops": 0,
      "entries": 1,
      "evictions": 0,
      "hits": 2,
      "pruned": 0,
      "queries": 3
    },
    "span_trace": [
      {
        "label": "parser.success",
        "span": {
          "end": 20,
          "start": 0
        }
      }
    ]
  },
  {
    "case": "empty_uses",
    "packrat": {
      "approx_bytes": 118,
      "budget_drops": 0,
      "entries": 1,
      "evictions": 0,
      "hits": 2,
      "pruned": 0,
      "queries": 3
    },
    "span_trace": [
      {
        "label": "parser.success",
        "span": {
          "end": 16,
          "start": 0
        }
      }
    ]
  },
  {
    "case": "multiple_functions",
    "packrat": {
      "approx_bytes": 236,
      "budget_drops": 0,
      "entries": 2,
      "evictions": 0,
      "hits": 4,
      "pruned": 0,
      "queries": 6
    },
    "span_trace": [
      {
        "label": "parser.success",
        "span": {
          "end": 13,
          "start": 0
        }
      },
      {
        "label": "parser.success",
        "span": {
          "end": 41,
          "start": 14
        }
      }
    ]
  },
  {
    "case": "missing_paren",
    "packrat": {
      "approx_bytes": 363,
      "budget_drops": 0,
      "entries": 1,
      "evictions": 0,
      "hits": 6,
      "pruned": 0,
      "queries": 7
    },
    "span_trace": [
      {
        "label": "parser.simple_error",
        "span": {
          "end": 14,
          "start": 13
        }
      }
    ]
  }
]
```
