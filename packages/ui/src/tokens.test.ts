import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import tokens from "../tokens.json";
import labelsJson from "../labels.json";
import { labels, roleLabel } from "./labels";

const pkg = resolve(__dirname, "..");
const read = (rel: string) => readFileSync(resolve(pkg, rel), "utf8");

// Digest section 9, written out literally. Where the digest gives no dark value
// the entry is absent here and covered by the contrast tests instead.
const DIGEST_LIGHT = {
  page: "#F5F3EE",
  surface: "#FFFFFF",
  text: "#16171A",
  muted: "#5B5F66",
  line: "#E2DFD8",
  primary: "#A5B4FC",
  primaryHover: "#818CF8",
  onPrimary: "#1E1B4B",
  link: "#4F46E5",
  tabSelected: "#6366F1",
  badgeTint: "#E6E9FE",
  badgeInk: "#3730A3",
  okFill: "#DCFCE7",
  okInk: "#14532D",
  warnFill: "#FEF3C7",
  warnInk: "#78350F",
  errorFill: "#FEE4E2",
  errorInk: "#912018",
};
const DIGEST_DARK = {
  page: "#131516",
  surface: "#1C1F21",
  text: "#E9E7E2",
  muted: "#A9ADB2",
  line: "#2E3235",
  primary: "#A5B4FC",
  primaryHover: "#818CF8",
  onPrimary: "#131516",
  link: "#A5B4FC",
};

describe("tokens.json matches digest section 9", () => {
  for (const [k, v] of Object.entries(DIGEST_LIGHT)) {
    it(`light ${k} is ${v}`, () => {
      expect((tokens.colors.light as Record<string, string>)[k]).toBe(v);
    });
  }
  for (const [k, v] of Object.entries(DIGEST_DARK)) {
    it(`dark ${k} is ${v}`, () => {
      expect((tokens.colors.dark as Record<string, string>)[k]).toBe(v);
    });
  }
  it("dark has the same keys as light", () => {
    expect(Object.keys(tokens.colors.dark).sort()).toEqual(Object.keys(tokens.colors.light).sort());
  });
  it("has spacing, radius and type scales", () => {
    expect(Object.keys(tokens.spacing).length).toBeGreaterThan(3);
    expect(Object.keys(tokens.radius).length).toBeGreaterThan(2);
    expect(Object.keys(tokens.type.sizes).length).toBeGreaterThan(3);
  });
  it("uses a system font stack with no web fonts", () => {
    expect(tokens.type.family).toMatch(/^system-ui/);
    expect(read("src/tokens.css")).not.toMatch(/@font-face|url\(/);
    expect(read("src/components.css")).not.toMatch(/@font-face|url\(|https?:/);
  });
});

describe("generated files", () => {
  it("are up to date with tokens.json and labels.json", () => {
    const run = spawnSync("node", ["scripts/generate.mjs", "--check"], {
      cwd: pkg,
      encoding: "utf8",
    });
    expect(run.stderr, "run: pnpm -C packages/ui generate").toBe("");
    expect(run.status).toBe(0);
  });

  it("the stale check fails and names the regenerate command", () => {
    // Run the real generator against a copy whose tokens differ.
    const dir = mkdtempSync(resolve(tmpdir(), "lc-stale-"));
    try {
      const copy = JSON.parse(read("tokens.json"));
      copy.colors.light.page = "#000000";
      // Copy the generator and its inputs into a throwaway package layout.
      const script = read("scripts/generate.mjs");
      execFileSync("mkdir", ["-p", resolve(dir, "pkg/scripts"), resolve(dir, "pkg/src")]);
      writeFileSync(resolve(dir, "pkg/scripts/generate.mjs"), script);
      writeFileSync(resolve(dir, "pkg/tokens.json"), JSON.stringify(copy));
      writeFileSync(resolve(dir, "pkg/labels.json"), read("labels.json"));
      const run = spawnSync("node", ["scripts/generate.mjs", "--check"], {
        cwd: resolve(dir, "pkg"),
        encoding: "utf8",
      });
      expect(run.status).toBe(1);
      expect(run.stderr).toContain("pnpm -C packages/ui generate");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it("tokens.css has light, system dark, forced dark and forced light", () => {
    const css = read("src/tokens.css");
    expect(css).toContain('[data-theme="light"]');
    expect(css).toContain("@media (prefers-color-scheme: dark)");
    expect(css).toContain(':root:not([data-theme="light"])');
    expect(css).toContain('[data-theme="dark"]');
    expect(css).toContain("--lc-page: #F5F3EE;");
    expect(css).toContain("--lc-page: #131516;");
  });
});

// WCAG 2.x relative luminance and contrast ratio.
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

const PAIRS: [string, string][] = [
  ["text", "page"],
  ["text", "surface"],
  ["muted", "page"],
  ["muted", "surface"],
  ["link", "page"],
  ["link", "surface"],
  ["onPrimary", "primary"],
  ["onPrimary", "primaryHover"],
  ["badgeInk", "badgeTint"],
  ["okInk", "okFill"],
  ["warnInk", "warnFill"],
  ["errorInk", "errorFill"],
];

describe("text contrast is at least 4.5:1 (WCAG)", () => {
  for (const theme of ["light", "dark"] as const) {
    for (const [fg, bg] of PAIRS) {
      it(`${theme}: ${fg} on ${bg}`, () => {
        const c = tokens.colors[theme] as Record<string, string>;
        const ratio = contrast(c[fg] as string, c[bg] as string);
        expect(
          ratio,
          `${theme} ${fg} on ${bg} measured ${ratio.toFixed(2)}:1`,
        ).toBeGreaterThanOrEqual(4.5);
      });
    }
  }
});

describe("labels", () => {
  it("maps roles to the decided words", () => {
    expect(roleLabel("host")).toBe("Host");
    expect(roleLabel("uploader")).toBe("Uploader");
    expect(roleLabel("viewer")).toBe("Guest");
    expect(labels.joinAsUploader).toBe("Join as Uploader");
    expect(labels.joinAsGuest).toBe("Join as Guest");
    expect(labels.guestBrowser).toBe("Guest · browser");
  });
  it("the word Viewer appears in no generated label", () => {
    const files = [
      read("src/labels.ts").replace(/roleViewer|"viewer"/g, ""),
      readFileSync(resolve(pkg, "../../apps/mobile/lib/l10n/labels.g.dart"), "utf8").replace(
        /roleViewer/g,
        "",
      ),
    ];
    for (const text of files) expect(text).not.toMatch(/viewer/i);
    for (const v of Object.values(labelsJson)) expect(v).not.toMatch(/viewer/i);
    for (const v of Object.values(labels)) expect(v).not.toMatch(/viewer/i);
  });
});
