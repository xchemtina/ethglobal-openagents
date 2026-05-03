import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(new URL(".", import.meta.url).pathname, "..");
const target = "SciCrucible_v1/package.json";

console.log("[restore] cwd:", repoRoot);

try {
  // Restore the file from the latest commit on the current branch
  execSync(`git checkout HEAD -- "${target}"`, { cwd: repoRoot, stdio: "inherit" });
} catch (e) {
  console.error("[restore] git checkout HEAD failed, trying origin/master...");
  try {
    execSync(`git checkout origin/master -- "${target}"`, { cwd: repoRoot, stdio: "inherit" });
  } catch (e2) {
    console.error("[restore] origin/master failed too, trying log to find any commit with this file...");
    const sha = execSync(`git log --all --pretty=format:%H -n 1 -- "${target}"`, { cwd: repoRoot })
      .toString()
      .trim();
    if (!sha) throw new Error("Could not find any commit containing " + target);
    console.error("[restore] using sha:", sha);
    execSync(`git checkout ${sha} -- "${target}"`, { cwd: repoRoot, stdio: "inherit" });
  }
}

if (!existsSync(path.join(repoRoot, target))) {
  throw new Error("[restore] file still missing after checkout");
}
console.log("[restore] OK:", target);
