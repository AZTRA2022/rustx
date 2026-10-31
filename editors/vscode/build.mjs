// One file in, one file out.
//
// `vscode` is provided by the editor at runtime and must stay external,
// or the bundle tries to carry the editor inside itself and nothing
// loads.

import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));

await build({
  entryPoints: [join(here, "src/extension.ts")],
  outfile: join(here, "dist/extension.js"),
  bundle: true,
  platform: "node",
  target: "node20",
  format: "cjs",
  external: ["vscode"],
  sourcemap: true,
  minify: process.argv.includes("--release"),
  logLevel: "info",
});
