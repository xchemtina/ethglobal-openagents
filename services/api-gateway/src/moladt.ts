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

export class MoladtExecutionError extends Error {
  constructor(public readonly code: string, message: string, public readonly status: number) {
    super(message);
    this.name = "MoladtExecutionError";
  }
}

let activeProcesses = 0;

/** Invoke a bounded CLI process group; no shell interpolation is used. */
export function runMoladtApi(
  config: GatewayConfig,
  smiles: string,
  opts: { noWorker?: boolean } = {},
): Promise<MolAdtApiResult> {
  if (activeProcesses >= config.moladtMaxConcurrent) {
    return Promise.reject(new MoladtExecutionError("moladt_busy", "MolADT worker capacity is full; retry later.", 503));
  }
  activeProcesses += 1;
  return new Promise<MolAdtApiResult>((resolve, reject) => {
    const args = ["moladt-api", "--smiles", smiles];
    if (opts.noWorker) args.push("--no-worker");
    const grouped = process.platform !== "win32";
    const child = spawn(config.cliPath, args, {
      stdio: ["ignore", "pipe", "pipe"],
      env: process.env,
      detached: grouped,
    });
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    let outputBytes = 0;
    let failure: Error | null = null;
    let settled = false;

    function finish(error?: Error, result?: MolAdtApiResult) {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      activeProcesses -= 1;
      if (error) reject(error);
      else resolve(result!);
    }

    function stop(error: Error) {
      if (failure || settled) return;
      failure = error;
      try {
        if (grouped && child.pid) process.kill(-child.pid, "SIGKILL");
        else child.kill("SIGKILL");
      } catch (killError) {
        // ESRCH means the group already exited. Keep the original failure.
        if ((killError as NodeJS.ErrnoException).code !== "ESRCH") child.kill("SIGKILL");
      }
      // Descendants cannot hold the output pipes open beyond the deadline.
      child.stdout.destroy();
      child.stderr.destroy();
    }

    const timer = setTimeout(() => stop(new MoladtExecutionError(
      "moladt_timeout", "MolADT worker exceeded its time limit.", 504,
    )), config.moladtTimeoutMs);

    function collect(target: Buffer[], chunk: Buffer) {
      if (failure || settled) return;
      outputBytes += chunk.length;
      if (outputBytes > config.moladtMaxOutputBytes) {
        stop(new MoladtExecutionError("moladt_output_limit", "MolADT worker exceeded its output limit.", 502));
        return;
      }
      target.push(chunk);
    }
    child.stdout.on("data", (chunk: Buffer) => collect(stdout, chunk));
    child.stderr.on("data", (chunk: Buffer) => collect(stderr, chunk));
    child.on("error", () => finish(new MoladtExecutionError(
      "moladt_spawn_failed", "Unable to start the configured MolADT worker.", 503,
    )));
    child.on("close", (code) => {
      // A child that exits after spawning a background descendant must not leave
      // work running after its concurrency slot has been returned.
      if (grouped && child.pid) {
        try { process.kill(-child.pid, "SIGKILL"); } catch { /* group already exited */ }
      }
      if (failure) return finish(failure);
      if (code !== 0) {
        const diagnostic = Buffer.concat(stderr).toString("utf8");
        return finish(new MoladtExecutionError(
          "moladt_worker_failed",
          diagnostic.includes("not in curated library") ? "Molecule not in curated library." : `MolADT worker exited with code ${code}.`,
          diagnostic.includes("not in curated library") ? 422 : 502,
        ));
      }
      try {
        const parsed = JSON.parse(Buffer.concat(stdout).toString("utf8")) as MolAdtApiResult;
        if (!parsed || typeof parsed !== "object" || extractArtifactId(parsed) === "unknown") {
          throw new Error("missing artifact identity");
        }
        finish(undefined, parsed);
      } catch {
        finish(new MoladtExecutionError("moladt_invalid_output", "MolADT worker did not return a JSON artifact.", 502));
      }
    });
  });
}
