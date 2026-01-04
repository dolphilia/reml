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
import { readWorkspaceConfiguration } from "../configuration.js";

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

  it("parses effect stage mismatch fixture", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-effects-sample.json");
    expect(diagnostics).toHaveLength(1);

    const codes = Array.from(collectCodes(diagnostics));
    expect(codes).toContain("effects.contract.stage_mismatch");

    const hints = collectStructuredHints(diagnostics);
    expect(hints.length).toBeGreaterThan(0);

    const audits = collectAuditSnapshots(diagnostics);
    expect(
      audits.some((value) => {
        if (!value || typeof value !== "object") return false;
        const audit = value as { metadata?: Record<string, unknown> };
        return audit.metadata?.["effect.stage.actual"] === "preview";
      }),
    ).toBe(true);
  });

  it("parses metrics stage mismatch fixture", () => {
    const diagnostics = readDiagnostics(fixturesDir, "metrics-stage.json");
    expect(diagnostics).toHaveLength(1);

    const codes = Array.from(collectCodes(diagnostics));
    expect(codes).toContain("effects.contract.stage_mismatch");

    const [diag] = diagnostics;
    expect(diag.extensions?.["effects.contract.capability"]).toBe("metrics.emit");
    expect(diag.audit_metadata?.["effect.stage.required"]).toBe("beta");
  });

  it("parses macOS-specific bridge fixture", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-ffi-macos-sample.json");
    expect(diagnostics).toHaveLength(1);

    const [diag] = diagnostics;
    expect(diag.extensions?.["bridge.platform"]).toBe("macos-arm64");
    const audits = collectAuditSnapshots(diagnostics);
    expect(
      audits.some((value) => {
        if (!value || typeof value !== "object") return false;
        const audit = value as { metadata?: Record<string, unknown> };
        return audit.metadata?.["bridge.platform"] === "macos-arm64";
      }),
    ).toBe(true);
  });

  it("preserves streaming metadata", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-streaming-meta.json");
    expect(diagnostics).toHaveLength(1);
    const [diag] = diagnostics;
    expect(diag.stream_meta).toStrictEqual({
      bytes_consumed: 128,
      chunks_consumed: 4,
      await_count: 2,
      resume_count: 2,
      last_reason: "pending.backpressure",
      memo_bytes: 64,
      backpressure_policy: "auto",
      backpressure_events: 1,
    });
  });

  it("emits LSP severity values for info/hint diagnostics", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-info-hint.json");
    expect(diagnostics).toHaveLength(2);
    const severities = diagnostics.map((diag) => diag.severity);
    expect(severities).toStrictEqual([3, 4]);
    const labels = diagnostics.map((diag) => diag.codes?.[0]);
    expect(labels).toStrictEqual(["demo.info.sample", "demo.hint.sample"]);
  });

  it("roundtrips diagnostics JSON through stringify/parse", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostics_roundtrip.json");
    const cloned = JSON.parse(JSON.stringify(diagnostics));
    expect(cloned).toStrictEqual(diagnostics);
  });

  it("validates workspace configuration fixtures", () => {
    const configuration = readWorkspaceConfiguration(fixturesDir, "workspace-configuration.json");
    expect(configuration.diagnostics?.filter?.severity).toBe("warning");
    expect(configuration.audit?.policy?.level).toBe("info");
    expect(configuration.audit?.policy?.anonymize_pii).toBe(true);
  });

  it("parses pattern diagnostics fixture and collects codes", () => {
    const diagnostics = readDiagnostics(fixturesDir, "diagnostic-v2-pattern-sample.json");
    const codes = Array.from(collectCodes(diagnostics));
    expect(codes).toEqual(
      expect.arrayContaining([
        "pattern.slice.multiple_rest",
        "pattern.binding.duplicate_name",
        "pattern.regex.unsupported_target",
        "pattern.range.bound_inverted",
        "pattern.active.effect_violation",
      ]),
    );
  });
});
