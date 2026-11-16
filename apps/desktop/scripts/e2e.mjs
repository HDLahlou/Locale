// Runs Playwright, or exits 0 with a message when there are no specs yet.
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";

const specs = readdirSync("e2e").filter((f) => /\.spec\.ts$/.test(f));
if (specs.length === 0) {
  console.log("e2e: no specs yet");
  process.exit(0);
}
const r = spawnSync("playwright", ["test", ...process.argv.slice(2)], {
  stdio: "inherit",
  shell: true,
});
process.exit(r.status ?? 1);
