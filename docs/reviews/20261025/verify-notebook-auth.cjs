#!/usr/bin/env node
// Applies the patch only to disposable route copies; no original data is read.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const vm = require('node:vm');
const { execFileSync } = require('node:child_process');
const { createRequire } = require('node:module');
const { createHash } = require('node:crypto');

if (!process.argv[2]) throw new Error('Usage: node verify-notebook-auth.cjs /path/to/original/SciCrucible_v1 [dependency-root]');
const original = path.resolve(process.argv[2]);
const dependencyRoot = path.resolve(process.argv[3] || path.join(__dirname, '../../../SciCrucible_v1'));
const dependencyRequire = createRequire(path.join(dependencyRoot, 'package.json'));
const ts = dependencyRequire('typescript');
const routes = ['app/api/notebooks/corrections/next-task/route.ts', 'app/api/notebooks/corrections/confirm/route.ts', 'app/api/evidence/reviews/route.ts'];
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'notebook-auth-review-'));
const root = path.join(scratch, 'SciCrucible_v1');
const hashes = {};
const authPath = 'lib/notebook-service-runtime.ts';
for (const relative of [...routes, authPath]) {
  const bytes = fs.readFileSync(path.join(original, relative));
  hashes[relative] = createHash('sha256').update(bytes).digest('hex');
  const target = path.join(root, relative);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, bytes);
}
const patch = path.join(__dirname, 'notebook-auth.patch');
let checks = 0;

function route(relative, session, files = {}, tasks = []) {
  let storageCalls = 0;
  const fakeFs = {
    readFile: async (file) => { storageCalls++; if (Object.hasOwn(files, path.basename(file))) return JSON.stringify(files[path.basename(file)]); throw Object.assign(new Error('missing'), { code: 'ENOENT' }); },
    mkdir: async () => { storageCalls++; },
    writeFile: async () => { storageCalls++; },
    rename: async () => { storageCalls++; },
    readdir: async () => { storageCalls++; return Object.keys(files); },
  };
  let auth;
  function load(file) {
    const source = fs.readFileSync(path.join(root, file), 'utf8');
    const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
    const exports = {};
    const context = {
      exports,
      Buffer, URL,
      process: { env: { NODE_ENV: 'production' }, cwd: () => scratch },
      require: (id) => {
        if (id === 'server-only') return {};
        if (id === '@/lib/session') return { getSession: async () => session };
        if (id === '@/lib/notebook-service-runtime') return auth;
        if (id === 'node:fs/promises') return fakeFs;
        if (id === 'next/server') return { NextResponse: { json: (body, options = {}) => ({ body, status: options.status || 200 }) } };
        if (id === '@/lib/notebook-memory-data') return { NOTEBOOK_CORRECTION_MARKET_TASKS: tasks };
        if (id === '@/lib/evidence-lab-data') return { ELECTOR_EVIDENCE_EXAMPLES: [{ id: 'known-evidence' }] };
        if (id.startsWith('@/lib/')) return {};
        return id.startsWith('node:') ? require(id) : dependencyRequire(id);
      },
    };
    vm.runInNewContext(js, context, { filename: file, timeout: 1000 });
    return exports;
  }
  auth = load(authPath);
  return { handler: load(relative).POST, storageCalls: () => storageCalls };
}

async function denied(relative, session, body, status) {
  const loaded = route(relative, session);
  const response = await loaded.handler({ json: async () => body });
  assert.equal(response.status, status, relative);
  assert.equal(loaded.storageCalls(), 0, 'Denied request touched storage');
  checks++;
}

(async () => {
  try {
    execFileSync('git', ['apply', '--check', patch], { cwd: scratch });
    execFileSync('git', ['apply', patch], { cwd: scratch });
    const researcher = { sub: 'reviewer-a', role: 'researcher', verified: true, orcid: 'test-orcid' };
    const operator = { ...researcher, role: 'operator' };
    for (const relative of routes) {
      await denied(relative, null, {}, 401);
      await denied(relative, { ...researcher, verified: false }, {}, 403);
    }
    await denied(routes[1], researcher, {}, 403);
    await denied(routes[0], researcher, { assigneeId: 'someone-else' }, 403);
    await denied(routes[0], researcher, { assigneeId: 'reviewer-a', walletAddress: 'unlinked-wallet' }, 400);
    await denied(routes[1], operator, { taskId: 'task', artifactId: 'art_abcdef', reviewStatus: 'accepted', reviewerId: 'someone-else' }, 403);
    const evidence = route(routes[2], researcher);
    const response = await evidence.handler({ json: async () => ({ id: 'known-evidence', reviewStatus: 'unreviewed', cropOk: 'unknown', ocrOk: 'unknown', correctedValue: '', correctedText: '', annotationTags: [], reviewer: 'someone-else', notes: '' }) });
    assert.equal(response.status, 200);
    assert.equal(response.body.review.reviewer, 'reviewer-a');
    assert.ok(evidence.storageCalls() > 0);
    checks++;
    // Record a remaining defect, not a successful integrity/security test.
    const payload = { task_id: 'synthetic-task' };
    const fakeArtifact = { id: 'art_abcdef', schema_tags: ['science.notebook.correction'], topic: 'synthetic', parent_artifact_ids: [], content_hash: 'INVALID', signature: 'INVALID', signing_public_key: 'INVALID', payload: { hash: 'INVALID', location: { Inline: { bytes_hex: Buffer.from(JSON.stringify(payload)).toString('hex') } } } };
    const confirmation = route(routes[1], operator, {
      'science_notebook_correction.synthetic.json': fakeArtifact,
      'notebook-correction-sign-requests.json': { requests: { 'synthetic-task': { task_id: 'synthetic-task', payload_path: 'synthetic-payload.json' } } },
      'synthetic-payload.json': payload,
    }, [{ taskId: 'synthetic-task', bountyAmount: 1, bountyCurrency: 'USDC' }]);
    const remainingGap = await confirmation.handler({ json: async () => ({ taskId: 'synthetic-task', artifactId: 'art_abcdef', reviewStatus: 'accepted', reviewerId: 'reviewer-a' }) });
    assert.equal(remainingGap.status, 200);
    assert.equal(remainingGap.body.confirmation_binding_state.payout_eligible, true);
    const signatureGap = { reproduced: true, synthetic_invalid_signature_accepted: true, status: remainingGap.status, review_status: remainingGap.body.confirmation.review_status, payout_status: remainingGap.body.confirmation.payout_status, actual_payout: false };
    for (const [relative, hash] of Object.entries(hashes)) {
      assert.equal(createHash('sha256').update(fs.readFileSync(path.join(original, relative))).digest('hex'), hash, 'Original source changed');
    }
    console.log(JSON.stringify({ ok: true, checks, patch_sha256: createHash('sha256').update(fs.readFileSync(patch)).digest('hex'), original_source_sha256: hashes, unresolved_signature_verification: signatureGap, scope: 'Session and role denial paths, identity binding, isolated fake storage. Separate synthetic reproduction demonstrates unresolved signature verification; no live route or payout.' }, null, 2));
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
