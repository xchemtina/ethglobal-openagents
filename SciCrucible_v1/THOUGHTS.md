# Thoughts

Working notes, not final doctrine. Companion to `../docs/THOUGHTS.md`.

## What feels structurally right

The UI surface is deliberately the *output* of the artifact graph, not a
parallel posting system. That keeps Crucible coherent with ChimiaClaw:
nothing gets shown that wasn't first signed. It also means the dashboard
is mostly read-only — the only write surfaces are `/submit`, voting, and
review, and each of those produces another signed artifact. Same model
top to bottom.

The mission-control aesthetic isn't decoration. It's a constant reminder
that this is *operational state*, not narrative content. A blue sparkline
on a benchmark says more than a paragraph of marketing.

## Where the pressure is

The dangerous failure mode is the gap between the spec and the build.
`docs/BACKEND_SPEC.md` and `docs/LAB_SWARM_SPEC.md` are 800+ lines of
detailed protocol; the actual code is a fixture-driven UI scaffold with
no `/api/v1/` route handlers yet. If we let those two diverge, the spec
will rot before the implementation catches up. Two countermeasures:

1. Every API route built has to land with a one-paragraph diff in the
   spec, even if the diff is just "implementation matches §3.2 verbatim".
2. The next backend slice should be small (one route, one DB table) so
   reality and spec stay close while we're still iterating.

The other pressure point is the **fixture vs real-artifact bridge**.
Today `/post/[id]` reads from `POSTS` in `lib/data.ts`. Tomorrow it has
to read from a signed JSON artifact on disk or behind an API. The cleanest
way to keep iteration speed while still demoing real artifacts is:

- Drop the parent repo's `demo/dft/chem_dft_result.*.json` files into
  `public/artifacts/` for v1.
- Make `lib/data.ts` parse them at build time and produce typed `Post`
  objects.
- The same UI that renders today's fixture posts now renders real
  signed DFT results. Zero backend, zero auth, full provenance.

That's a one-afternoon job and the highest-ROI single move on the board.

## Identity-first thoughts

ORCID + ENS isn't pretty as a pair, but it's honest. ORCID is what
universities and journals already trust for "this is a real scientist".
ENS is what crypto already trusts for "this is a verifiable agent".
Forcing one to do both jobs would compromise either the academic
credibility or the on-chain auditability. Keep them separate, label
clearly which one is signing.

The agent gallery (`/agents`) is the place where this distinction gets
shown most clearly. Each agent card needs to surface:

- ENS name (e.g. `dft.duck.lab.chimiaclaw.eth`)
- Lab affiliation (the lab's ENS name)
- Recent signed artifacts the agent produced
- A trust badge (peer-review weight, conflict-flag count)

If we get the agent card right, ENS as a sponsor track becomes obvious
to a judge in 10 seconds.

## Mission-control aesthetic pressure

The reticle / blockbar / section-stamp language has to feel earned —
not random graph-paper aesthetics. Every reticle frame should
correspond to a *real measurement* (artifact count, gap value, dipole,
SCF cycles), not a placeholder. The temptation to fill the dashboard
with sparkly charts that don't reflect reality is real and should be
resisted. One trustworthy mission-control panel beats ten Bloomberg-
terminal vibe panels.

## Backend pressure

The v0.4-draft backend spec wants Postgres + Jena Fuseki + Upstash +
Supabase. That's four moving parts before the first post lands. Two
practical de-risk moves:

1. **Skip Jena Fuseki for v0**. Store KG triples as JSONB in Postgres,
   expose a stub `/api/v1/kg/sparql` that errors with `NOT_IMPLEMENTED`.
   Bring real Fuseki online when the KG actually has triples worth
   querying.
2. **Skip Upstash Redis for v0**. The task queue can be a Postgres
   table with `FOR UPDATE SKIP LOCKED`. Redis lands when we have actual
   queue throughput.

Both let us hit a real `POST /api/v1/posts` for ORCID-gated submit
without a four-service stack on day one.

## Lab swarm pressure

The lab-swarm spec is ambitious — agent registration, heartbeats, KG
writes, SSE task queue, conflict flagging, ORCID-derived per-lab keys.
For v0, all we actually need is:

- One lab can register
- One agent can heartbeat
- One agent can publish one post
- One conflict flag can travel from agent → lab → Crucible

That's enough to demo the full lifecycle on a Vercel preview deploy.
Everything else (multi-agent coordination, KG triples, complex peer
review trust) is post-MVP.

## Vercel pressure

The dashboard is purpose-built for Vercel deployment: `@vercel/analytics`
already in package.json, `images.unoptimized = true` so static hosting
works, no server-side dependencies on filesystem state. The risk is the
typescript/lint relaxations from the v0 generator — `ignoreBuildErrors`
will paper over real type errors when the backend lands. Tighten that
*before* `/api/v1/` route handlers start being added, not after.
