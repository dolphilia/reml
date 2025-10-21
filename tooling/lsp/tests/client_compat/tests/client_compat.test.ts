import { describe, expect, it } from "vitest";
import {
  collectAuditSnapshots,
  collectCodes,
  collectStructuredHints,
  collectTimestamps,
  readDiagnostics,
} from "../client-v2.js";
import { convertToV1 } from "../client-v1.js";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const currentDir = dirname(fileURLToPath(import.meta.url));
const fixturesDir = join(currentDir, "..", "fixtures");

describe("client compatibility scaffolding", () => {
  it("loads V1 diagnostics without throwing", () => {
    const diagnostics = convertToV1(fixturesDir, "diagnostic-sample.json");
    expect(diagnostics).toBeInstanceOf(Array);
    expect(diagnostics[0]?.timestamp).toBeTypeOf("string");
  });

  it("loads V2 diagnostics and extracts codes", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-sample.json");
    const codes = collectCodes(diagnostics);
    expect(codes).toBeInstanceOf(Set);
  });

  it("collects structured hints array", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-sample.json");
    const hints = collectStructuredHints(diagnostics);
    expect(hints).toBeInstanceOf(Array);
  });

  it("retains timestamp and audit metadata in V2 payload", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-sample.json");
    const timestamps = collectTimestamps(diagnostics);
    expect(timestamps.filter((value) => typeof value === "string").length).toBeGreaterThan(0);
    const audits = collectAuditSnapshots(diagnostics);
    expect(audits.filter((value) => value && typeof value === "object").length).toBeGreaterThan(0);
  });

  it("parses FFI stage mismatch fixture with structured hints", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-ffi-sample.json");
    expect(diagnostics).toHaveLength(1);

    const [diagnostic] = diagnostics;
    const codes = Array.from(collectCodes(diagnostics));
    expect(codes).toContain("ffi.bridge.stage_mismatch");

    const hints = collectStructuredHints(diagnostics);
    expect(Array.isArray(hints)).toBe(true);
    expect(hints.flat().length).toBeGreaterThan(0);

    const audits = collectAuditSnapshots(diagnostics);
    expect(audits.some((value) => value && value.metadata?.["bridge.return.status"] === "unsafe")).toBe(true);
  });
});
