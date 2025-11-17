# Parser Trace Events

| # | kind | trace_id | span | label |
|---|------|----------|------|-------|
| 1 | module_header_accepted | syntax:module-header | 0..32 | docs.spec.syntax.examples |
| 2 | use_decl_accepted | syntax:use | 34..88 | ::Core.Parse.{Lex, Op.{Infix, Prefix as PrefixOp}} |
| 3 | use_decl_accepted | syntax:use | 89..150 | ::Core.Diagnostics.{Report as DiagnosticReport, Severity} |
| 4 | use_decl_accepted | syntax:use | 151..201 | self.helpers.{TokenBuffer, build_pretty_trace} |
