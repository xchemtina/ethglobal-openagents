import { appendFile, mkdir } from "node:fs/promises";
import path from "node:path";

export interface RevenueEvent {
  at_unix: number;
  sku_id: string;
  mode: string;
  amount_usdc_micros: number;
  price_display: string;
  pay_to: string;
  network: string;
  result_artifact_id: string;
  smiles?: string;
  /** Optional human label for cached DFT lookups. */
  label?: string;
  payment_signature_ref?: string | null;
}

export async function appendRevenueEvent(
  logPath: string,
  event: RevenueEvent,
): Promise<void> {
  const dir = path.dirname(logPath);
  await mkdir(dir, { recursive: true });
  await appendFile(logPath, `${JSON.stringify(event)}\n`, "utf8");
}
