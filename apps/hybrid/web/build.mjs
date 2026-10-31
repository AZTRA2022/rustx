// The bundle the page loads, built from `src/` and whatever is in
// `node_modules`.
//
// A script rather than a line of flags, because the one thing that must
// be right is the quoting of `process.env.NODE_ENV`: npm hands a script
// to the shell, the shell eats the quotes, and React then reads an
// undefined identifier called `production` on the bundle's first line.
import { build, context } from "esbuild";

const options = {
  entryPoints: ["src/main.jsx"],
  bundle: true,
  format: "iife",
  target: "chrome120",
  minify: true,
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  outfile: "../assets/components.js",
  logLevel: "info",
};

if (process.argv.includes("--watch")) {
  const watcher = await context(options);
  await watcher.watch();
} else {
  await build(options);
}
