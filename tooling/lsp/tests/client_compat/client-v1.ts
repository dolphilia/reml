/**
 * LSP V1 互換クライアント雛形
 *
 * 既存 CLI JSON 出力から LSP 診断へ変換する際に、
 * V2 フィールドを無視してもクラッシュしないことを確認する。
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

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
  relatedInformation?: Array<{
    location: { uri: string; range: Range };
    message: string;
  }>;
};

/**
 * CLI が生成した JSON を読み込み、V1 互換形式へ変換する。
 * 実装は Phase 2-4 で `tooling/lsp/diagnostic_transport.ml` に準拠。
 */
export function convertToV1(documentRoot: string, fixture: string): DiagnosticV1[] {
  const raw = readFileSync(join(documentRoot, fixture), "utf8");
  const payload = JSON.parse(raw) as Record<string, unknown>[];
  return payload.map((entry) => {
    const range = extractRange(entry);
    return {
      message: String(entry["message"] ?? ""),
      severity: Number(entry["severity"] ?? 1),
      code: typeof entry["code"] === "string" ? entry["code"] : undefined,
      range,
      relatedInformation: extractRelated(entry),
    };
  });
}

function extractRange(entry: Record<string, unknown>): Range {
  const primary = entry["primary"] as Record<string, unknown> | undefined;
  if (!primary) {
    return {
      start: { line: 0, character: 0 },
      end: { line: 0, character: 0 },
    };
  }
  return {
    start: {
      line: Number(primary["start_line"] ?? 0),
      character: Number(primary["start_col"] ?? 0),
    },
    end: {
      line: Number(primary["end_line"] ?? 0),
      character: Number(primary["end_col"] ?? 0),
    },
  };
}

function extractRelated(entry: Record<string, unknown>): DiagnosticV1["relatedInformation"] {
  const notes = entry["notes"];
  if (!Array.isArray(notes)) return undefined;
  return notes
    .map((note) => {
      if (
        !note ||
        typeof note !== "object" ||
        typeof (note as { message?: unknown }).message !== "string"
      ) {
        return null;
      }
      const span = (note as { span?: Record<string, unknown> }).span;
      if (!span) return null;
      const uri = String(span["file"] ?? "file://unknown");
      return {
        location: {
          uri,
          range: {
            start: {
              line: Number(span["start_line"] ?? 0),
              character: Number(span["start_col"] ?? 0),
            },
            end: {
              line: Number(span["end_line"] ?? 0),
              character: Number(span["end_col"] ?? 0),
            },
          },
        },
        message: (note as { message: string }).message,
      };
    })
    .filter((value): value is NonNullable<typeof value> => value !== null);
}

if (import.meta.url === (process.argv[1] ? new URL(process.argv[1], "file://").href : "")) {
  const currentDir = dirname(fileURLToPath(import.meta.url));
  const diagnostics = convertToV1(currentDir, "fixtures/diagnostic-sample.json");
  console.log("[client-v1] diagnostics", diagnostics.length);
}
