# Archive: github-ready-20260503

EthGlobal OpenAgents submission snapshot (May 2026).

- Git branch (in ~/OpenAgents repo): `github-ready-20260503`
- Former worktree path: `~/OpenAgents-github-ready` (safe to remove after this archive)
- Live development: `~/OpenAgents` on `master`

Do not dual-develop against the old worktree. Use:

```bash
cd ~/OpenAgents
git branch -v | grep github-ready
# remove worktree when clean:
git worktree remove ~/OpenAgents-github-ready
# or force if only junk dirty state remains after archive:
# git worktree remove --force ~/OpenAgents-github-ready
```

Branch tip at archive time was approximately `4720498` (see `git log github-ready-20260503`).
