import { spawn } from "node:child_process";
import type { GatewayConfig } from "./config.js";

export interface MolAdtApiResult {
  sku_id: string;
  service_kind: string;
  price_usdc_micros: number;
  price_display: string;
  settlement_method: string;
  smiles: string;
  molecule: unknown;
  artifact: {
    id: { "0"?: string } | string;
    content_hash?: string;
    schema_tags?: unknown;
    [key: string]: unknown;
  };
  provider_agent: string;
  notes: string[];
}

function artifactId(result: MolAdtApiResult): string {
  const id = result.artifact?.id;
  if (typeof id === "string") {
    return id;
  }
  if (id && typeof id === "object" && typeof id["0"] === "string") {
    return id["0"];
  }
  // serde transparent string often deserializes as plain string; fallback:
  if (id && typeof id === "object") {
    const values = Object.values(id);
    if (typeof values[0] === "string") {
      return values[0];
    }
  }
  return "unknown";
}

export function extractArtifactId(result: MolAdtApiResult): string {
  return artifactId(result);
}

/**
 * Invoke `chimiaclaw-cli moladt-api --smiles ...` and parse the JSON body.
 */
export function runMoladtApi(
  config: GatewayConfig,
  smiles: string,
  opts: { noWorker?: boolean } = {},
): Promise<MolAdtApiResult> {
  return new Promise((resolve, reject) => {
    const args = ["moladt-api", "--smiles", smiles];
    if (opts.noWorker) {
      args.push("--no-worker");
    }

    const child = spawn(config.cliPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
      env: process.env,
    });

    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk: Buffer) => {
      stdout += chunk.toString("utf8");
    });
    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString("utf8");
    });

    child.on("error", (error) => {
      reject(
        new Error(
          `failed to spawn chimiaclaw-cli at ${config.cliPath}: ${error.message}`,
        ),
      );
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(
          new Error(
            `chimiaclaw-cli moladt-api exited ${code}: ${stderr.trim() || stdout.trim()}`,
          ),
        );
        return;
      }
      try {
        const parsed = JSON.parse(stdout) as MolAdtApiResult;
        resolve(parsed);
      } catch (error) {
        reject(
          new Error(
            `invalid JSON from moladt-api: ${error instanceof Error ? error.message : String(error)}`,
          ),
        );
      }
    });
  });
}
