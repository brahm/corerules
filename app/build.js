/**
 * Three bundles: the main process, the preload, and the renderer.
 *
 * esbuild rather than a framework toolchain, for the reason the engine has no runtime
 * dependencies at all — everything this ships should be something you can read. The engine is
 * bundled IN rather than linked, because there is nothing to publish and a second package.json
 * would only be a thing to keep in step.
 */
import { build } from "esbuild";
import { cpSync, rmSync } from "node:fs";

rmSync("dist", { recursive: true, force: true });

const common = { bundle: true, format: "esm", target: "node22", logLevel: "info" };

await build({
  ...common,
  entryPoints: ["src/main/main.ts"],
  outfile: "dist/main.js",
  platform: "node",
  // Electron supplies these at run time and bundling them would produce a second copy that
  // is not the one running.
  external: ["electron"],
});

await build({
  ...common,
  entryPoints: ["src/main/preload.ts"],
  outfile: "dist/preload.cjs",
  platform: "node",
  format: "cjs",     // a sandboxed preload is not an ES module
  external: ["electron"],
});

await build({
  ...common,
  entryPoints: ["src/renderer/index.tsx"],
  outfile: "dist/renderer.js",
  platform: "browser",
  target: "es2022",
  // The renderer gets no Node, so nothing that reaches for it may end up here. If this build
  // ever fails on `node:fs`, the answer is that something crossed the line, not that the
  // config is wrong.
  jsx: "automatic",
});

// The window's own two files are copied rather than bundled: there is nothing to compile in
// either, and a build step that rewrote them would only be a place for them to drift.
cpSync("src/renderer/index.html", "dist/index.html");
cpSync("src/renderer/renderer.css", "dist/renderer.css");
