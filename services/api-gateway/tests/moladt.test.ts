import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { loadConfig } from "../src/config.js";
import { runMoladtApi, MoladtExecutionError } from "../src/moladt.js";

test("MolADT process limits and error handling use disposable fake workers", async t => {
  const dir = await mkdtemp(path.join(os.tmpdir(), "gateway-worker-"));
  const cliPath = path.join(dir, "fake-cli");
  await writeFile(cliPath, `#!${process.execPath}
const input = process.argv[process.argv.indexOf('--smiles') + 1];
if (input === 'descendant') {
  const { spawn } = require('node:child_process');
  spawn(process.execPath, ['-e', 'setInterval(() => require("node:fs").appendFileSync(process.argv[1], "."), 10)', ${JSON.stringify(path.join(dir, 'heartbeat'))}], { stdio: ['ignore', 'inherit', 'inherit'] });
  setInterval(() => {}, 1000);
}
else if (input === 'hang') setInterval(() => {}, 1000);
else if (input === 'large') process.stdout.write('x'.repeat(10000));
else if (input === 'stderr') process.stderr.write('x'.repeat(10000));
else if (input === 'fail') { process.stderr.write('private diagnostic'); process.exit(9); }
else if (input === 'bad') process.stdout.write('not JSON');
else if (input === 'empty') process.stdout.write('{}');
else process.stdout.write(JSON.stringify({artifact: {id:'art_0123456789abcdef'}, smiles:input}));
`, { mode: 0o700 });
  const config = { ...loadConfig(), cliPath, moladtTimeoutMs: 3000, moladtMaxOutputBytes: 1000, moladtMaxConcurrent: 1 };
  try {
    await t.test("successful JSON result preserves a literal SMILES argument", async () => {
      const result = await runMoladtApi(config, "C; echo should-never-run", { noWorker: true });
      assert.equal(result.smiles, "C; echo should-never-run");
    });
    for (const [input, code] of [["hang", "moladt_timeout"], ["large", "moladt_output_limit"], ["stderr", "moladt_output_limit"], ["fail", "moladt_worker_failed"], ["bad", "moladt_invalid_output"], ["empty", "moladt_invalid_output"]]) {
      await t.test(input, async () => {
        await assert.rejects(runMoladtApi(input === "hang" ? { ...config, moladtTimeoutMs: 300 } : config, input), (error: unknown) => error instanceof MoladtExecutionError && error.code === code);
        assert.equal((await runMoladtApi(config, "O")).smiles, "O", "slot must be released after failure");
      });
    }
    await t.test("concurrency cap and timeout release", async () => {
      const hanging = assert.rejects(runMoladtApi({ ...config, moladtTimeoutMs: 300 }, "hang"), /time limit/);
      await assert.rejects(runMoladtApi(config, "O"), (error: unknown) => error instanceof MoladtExecutionError && error.code === "moladt_busy");
      await hanging;
      assert.equal((await runMoladtApi(config, "O")).smiles, "O");
    });
    await t.test("timeout stops descendant work in the same process group", { skip: process.platform === "win32" }, async () => {
      await assert.rejects(runMoladtApi({ ...config, moladtTimeoutMs: 1500 }, "descendant"), /time limit/);
      const heartbeat = await readFile(path.join(dir, "heartbeat"), "utf8");
      assert.ok(heartbeat.length > 0, "descendant must actually have started");
      await delay(100);
      assert.equal(await readFile(path.join(dir, "heartbeat"), "utf8"), heartbeat);
    });
    await t.test("missing executable releases slot without exposing path", async () => {
      await assert.rejects(runMoladtApi({ ...config, cliPath: path.join(dir, "missing") }, "O"), (error: unknown) => error instanceof MoladtExecutionError && error.code === "moladt_spawn_failed" && !error.message.includes(dir));
      assert.equal((await runMoladtApi(config, "O")).smiles, "O");
    });
  } finally { await rm(dir, { recursive: true, force: true }); }
});
