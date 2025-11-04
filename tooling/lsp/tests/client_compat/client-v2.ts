/**
 * LSP V2 互換クライアント雛形
 *
 * V2 で追加される `codes[]` や `structured_hints` を読み取り、
 * 既存クライアントが拡張フィールドを活用できるかどうかを検証する。
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import Ajv from "ajv";
import addFormats from "ajv-formats";

type StructuredHint = {
  id?: string;
  title?: string;
  kind: string;
  span?: {
    file?: string;
    start_line?: number;
    start_col?: number;
    end_line?: number;
    end_col?: number;
  };
  payload: Record<string, unknown>;
  actions?: unknown[];
};

type Span = {
  file: string;
  start_line: number;
  start_col: number;
  end_line: number;
  end_col: number;
};

type SecondaryEntry = {
  span?: Span | null;
  message?: string | null;
};

type HintAction = {
  kind: string;
  range: Span;
  text?: string | null;
};

type Hint = {
  message?: string | null;
  actions?: HintAction[];
};

type DiagnosticV2 = {
  message: string;
  severity: number;
  codes: string[];
  primary: {
    file: string;
    start_line: number;
    start_col: number;
    end_line: number;
    end_col: number;
  };
  id?: string | null;
  domain?: string | null;
  secondary?: SecondaryEntry[];
  hints?: Hint[];
  structured_hints?: StructuredHint[];
  extensions?: Record<string, unknown>;
  audit_metadata?: Record<string, unknown>;
  audit?: Record<string, unknown> | null;
  timestamp?: string | null;
  stream_meta?: {
    bytes_consumed?: number;
    chunks_consumed?: number;
    await_count?: number;
    resume_count?: number;
    last_reason?: string | null;
    memo_bytes?: number | null;
    backpressure_policy?: string | null;
    backpressure_events?: number;
  } | null;
};

const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);

/**
 * JSON スキーマ検証用の初期化。Phase 2-4 完了時に schema を確定させる。
 */
const currentDir = dirname(fileURLToPath(import.meta.url));
const schemaPath = join(currentDir, "..", "..", "..", "json-schema", "diagnostic-v2.schema.json");

let validateSchema:
  | ((value: unknown) => value is DiagnosticV2)
  | undefined = undefined;

try {
  const schemaRaw = readFileSync(schemaPath, "utf8");
  const schema = JSON.parse(schemaRaw);
  validateSchema = ajv.compile(schema) as typeof validateSchema;
} catch (error) {
  // まだ schema が完成していない場合は警告のみ表示する。
  console.warn("[client-v2] schema 未設定のためバリデーションをスキップします", error);
}

export function readDiagnostics(documentRoot: string, fixture: string): DiagnosticV2[] {
  const raw = readFileSync(join(documentRoot, fixture), "utf8");
  const payload = JSON.parse(raw) as DiagnosticV2[];
  if (validateSchema) {
    payload.forEach((entry) => {
      if (!validateSchema?.(entry)) {
        throw new Error(`[client-v2] スキーマ違反: ${ajv.errorsText(validateSchema.errors)}`);
      }
    });
  }
  return payload;
}

export function collectCodes(diagnostics: DiagnosticV2[]): Set<string> {
  return diagnostics.reduce((acc, diag) => {
    diag.codes?.forEach((code) => acc.add(code));
    return acc;
  }, new Set<string>());
}

export function collectStructuredHints(diagnostics: DiagnosticV2[]): StructuredHint[] {
  return diagnostics.flatMap((diag) => diag.structured_hints ?? []);
}

export function collectTimestamps(diagnostics: DiagnosticV2[]): (string | null | undefined)[] {
  return diagnostics.map((diag) => diag.timestamp);
}

export function collectAuditSnapshots(diagnostics: DiagnosticV2[]): (Record<string, unknown> | null | undefined)[] {
  return diagnostics.map((diag) => diag.audit);
}

if (import.meta.url === (process.argv[1] ? new URL(process.argv[1], "file://").href : "")) {
  const samplesDir = currentDir;
  const diagnostics = readDiagnostics(samplesDir, "fixtures/diagnostic-v2-sample.json");
  const codes = Array.from(collectCodes(diagnostics));
  console.log("[client-v2] codes", codes);
  console.log("[client-v2] structured_hints", collectStructuredHints(diagnostics).length);
  console.log(
    "[client-v2] timestamps",
    collectTimestamps(diagnostics)
      .filter((value): value is string => typeof value === "string")
      .length,
  );
}
