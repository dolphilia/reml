/**
 * LSP V1 互換クライアント雛形
 *
 * 既存 CLI JSON 出力から LSP 診断へ変換する際に、
 * V2 フィールドを無視してもクラッシュしないことを確認する。
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

type Span = {
  file: string;
  start_line: number;
  start_col: number;
  end_line: number;
  end_col: number;
};

type Position = {
  line: number;
  character: number;
};

type Range = {
  start: Position;
  end: Position;
};

type DiagnosticV1 = {
  message: string;
  severity: number;
  code?: string;
  range: Range;
  timestamp?: string | null;
  auditId?: string | null;
  relatedInformation?: Array<{
    location: { uri: string; range: Range };
    message: string;
  }>;
};

type SecondaryEntry = {
  span?: Span | null;
  message?: string | null;
};

type AuditPayload =
  | {
      audit_id?: string | null;
    }
  | null
  | undefined;

type DiagnosticV2 = {
  id?: string | null;
  message: string;
  severity: number;
  codes?: string[] | null;
  primary: Span;
  secondary?: SecondaryEntry[] | null;
  timestamp?: string | null;
  audit?: AuditPayload;
};

/**
 * CLI が生成した JSON を読み込み、V1 互換形式へ変換する。
 * 実装は Phase 2-4 で `tooling/lsp/diagnostic_transport.ml` に準拠。
 */
export function convertToV1(documentRoot: string, fixture: string): DiagnosticV1[] {
  const raw = readFileSync(join(documentRoot, fixture), "utf8");
  const payload = JSON.parse(raw) as DiagnosticV2[];
  return payload.map((entry) => {
    const range = extractRange(entry);
    const code = Array.isArray(entry.codes) && entry.codes.length > 0 ? entry.codes[0] : undefined;
    const audit = (entry.audit ?? null) as AuditPayload;
    return {
      message: entry.message,
      severity: Number(entry.severity ?? 1),
      code,
      range,
      relatedInformation: extractRelated(entry),
      timestamp: entry.timestamp ?? null,
      auditId: typeof audit?.audit_id === "string" ? audit?.audit_id : null,
    };
  });
}

function extractRange(entry: DiagnosticV2): Range {
  const primary = entry.primary;
  return {
    start: {
      line: Number(primary.start_line ?? 0),
      character: Number(primary.start_col ?? 0),
    },
    end: {
      line: Number(primary.end_line ?? 0),
      character: Number(primary.end_col ?? 0),
    },
  };
}

function extractRelated(entry: DiagnosticV2): DiagnosticV1["relatedInformation"] {
  const secondary = entry.secondary;
  if (!Array.isArray(secondary) || secondary.length === 0) return undefined;
  return secondary
    .map((note) => {
      if (!note || typeof note !== "object" || typeof note.message !== "string" || !note.span) {
        return null;
      }
      const span = note.span;
      const uri = span.file ? span.file : "file://unknown";
      return {
        location: {
          uri,
          range: {
            start: {
              line: Number(span.start_line ?? 0),
              character: Number(span.start_col ?? 0),
            },
            end: {
              line: Number(span.end_line ?? 0),
              character: Number(span.end_col ?? 0),
            },
          },
        },
        message: note.message,
      };
    })
    .filter((value): value is NonNullable<typeof value> => value !== null);
}

if (import.meta.url === (process.argv[1] ? new URL(process.argv[1], "file://").href : "")) {
  const currentDir = dirname(fileURLToPath(import.meta.url));
  const diagnostics = convertToV1(currentDir, "fixtures/diagnostic-sample.json");
  console.log("[client-v1] diagnostics", diagnostics.length);
}
