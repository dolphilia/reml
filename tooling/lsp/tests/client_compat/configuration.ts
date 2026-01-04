import { readFileSync } from "node:fs";
import { join } from "node:path";
import Ajv from "ajv";
import addFormats from "ajv-formats";

export type WorkspaceConfiguration = {
  diagnostics?: {
    filter?: {
      severity?: string;
      include?: string[] | string;
      exclude?: string[] | string;
    };
  };
  audit?: {
    policy?: {
      level?: string;
      include_patterns?: string[] | string;
      exclude_patterns?: string[] | string;
      retention_days?: number;
      anonymize_pii?: boolean;
    };
  };
};

const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);

const patternCollectionSchema = {
  anyOf: [
    { type: "string" },
    { type: "array", items: { type: "string" } },
  ],
};

const workspaceConfigurationSchema = {
  type: "object",
  properties: {
    diagnostics: {
      type: "object",
      properties: {
        filter: {
          type: "object",
          properties: {
            severity: { enum: ["error", "warning", "info", "hint"] },
            include: patternCollectionSchema,
            exclude: patternCollectionSchema,
          },
          additionalProperties: false,
        },
      },
      additionalProperties: false,
    },
    audit: {
      type: "object",
      properties: {
        policy: {
          type: "object",
          properties: {
            level: { enum: ["off", "error", "warning", "info", "debug", "trace"] },
            include_patterns: patternCollectionSchema,
            exclude_patterns: patternCollectionSchema,
            retention_days: { type: "integer", minimum: 0 },
            anonymize_pii: { type: "boolean" },
          },
          additionalProperties: false,
        },
      },
      additionalProperties: false,
    },
  },
  additionalProperties: true,
};

const validateConfiguration = ajv.compile<WorkspaceConfiguration>(workspaceConfigurationSchema);

export function readWorkspaceConfiguration(root: string, fixture: string): WorkspaceConfiguration {
  const raw = readFileSync(join(root, fixture), "utf8");
  const payload = JSON.parse(raw) as WorkspaceConfiguration;
  if (!validateConfiguration(payload)) {
    throw new Error(
      `[workspace-configuration] ${ajv.errorsText(validateConfiguration.errors ?? [], { separator: ", " })}`,
    );
  }
  return payload;
}
