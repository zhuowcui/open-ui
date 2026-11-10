// Build the shadcn/ui reference pages and the bundled assets.
//
//   node build.mjs           write pages/ and ../assets/ from the pinned sources
//   node build.mjs --check   rebuild in memory and fail if either differs
//
// Inputs are pinned: package-lock.json fixes React, Radix, Tailwind, the
// fonts, the icons and the other packages, and shadcn/ holds the shadcn/ui
// sources vendored from one commit (see shadcn/SOURCE.md). Each fixture is
// rendered with React's static renderer and wrapped in a page that links
// pages/reference.css, which Tailwind compiles from shadcn/globals.css plus
// the bundled fonts' @font-face rules, with source detection limited to the
// generated pages. The font
// and icon files are copied unmodified from their npm packages into
// ../assets/, which both the pages and the Rust components load.

import { spawnSync } from "node:child_process"
import { createHash } from "node:crypto"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

import * as esbuild from "esbuild"

const ROOT = path.dirname(fileURLToPath(import.meta.url))
const PAGES = path.join(ROOT, "pages")
const ASSETS = path.join(ROOT, "..", "assets")
const THEMES = ["light", "dark"]

// The Lucide icons the new-york-v4 UI components import from lucide-react,
// as lucide-static file names (shadcn 4.23.0, apps/v4/registry/new-york-v4/ui).
const LUCIDE_ICONS = [
  "arrow-down", "arrow-left", "arrow-right", "check", "chevron-down",
  "chevron-left", "chevron-right", "chevron-up", "circle", "circle-check",
  "grip-vertical", "info", "loader-2", "minus", "more-horizontal",
  "octagon-x", "panel-left", "search", "triangle-alert", "x",
]

// Each bundled file: its path under ../assets, its npm package and the path
// inside that package. Files are copied byte for byte.
const ASSET_FILES = [
  ["fonts/geist/Geist-Variable.woff2", "geist", "dist/fonts/geist-sans/Geist-Variable.woff2"],
  ["fonts/geist/GeistMono-Variable.woff2", "geist", "dist/fonts/geist-mono/GeistMono-Variable.woff2"],
  ["fonts/geist/OFL.txt", "geist", "LICENSE.txt"],
  ["fonts/inter/InterVariable.woff2", "inter-ui", "variable/InterVariable.woff2"],
  ["fonts/inter/OFL.txt", "inter-ui", "LICENSE.txt"],
  ["icons/lucide/LICENSE", "lucide-static", "LICENSE"],
  ...LUCIDE_ICONS.map((name) => [`icons/lucide/${name}.svg`, "lucide-static", `icons/${name}.svg`]),
]

// The faces fonts.css declares. Open UI registers the same files with the
// same descriptors (openui-components/src/fonts.rs checks this list).
const FONT_FACES = [
  { family: "Geist", file: "fonts/geist/Geist-Variable.woff2", weight: [100, 900], style: "normal" },
  { family: "Geist Mono", file: "fonts/geist/GeistMono-Variable.woff2", weight: [100, 900], style: "normal" },
  { family: "Inter", file: "fonts/inter/InterVariable.woff2", weight: [100, 900], style: "normal" },
]

// shadcn/globals.css stays byte-identical to the pinned source. These are
// the only lines the build replaces: the package imports become the
// vendored copy, and Tailwind scans only the generated pages.
const GLOBALS_IMPORTS = [
  '@import "tailwindcss";',
  '@import "tw-animate-css";',
  '@import "shadcn/tailwind.css";',
]
const ENTRY_IMPORTS = [
  '@import "tailwindcss" source(none);',
  '@source "./pages/*.html";',
  '@import "tw-animate-css";',
  '@import "./shadcn/tailwind.css";',
]

// Appended to the Tailwind entry after shadcn/globals.css. shadcn's docs
// load Geist and Geist Mono with next/font as --font-sans and --font-mono;
// these rules load the same families from the bundled files instead.
// `font-display: block` keeps a capture from painting a fallback face, and
// there is no fallback family: a missing glyph shows up as a mismatch.
function fontCss() {
  const faces = FONT_FACES.map(
    ({ family, file, weight, style }) =>
      "@font-face {\n" +
      `  font-family: "${family}";\n` +
      `  font-style: ${style};\n` +
      `  font-weight: ${weight[0]} ${weight[1]};\n` +
      "  font-display: block;\n" +
      `  src: url("../../assets/${file}") format("woff2");\n` +
      "}\n"
  )
  const theme =
    "@theme {\n" +
    '  --font-sans: "Geist";\n' +
    '  --font-mono: "Geist Mono";\n' +
    '  --font-inter: "Inter";\n' +
    "}\n"
  return "\n/* Bundled fonts (components/reference/build.mjs). */\n" + faces.join("") + theme
}

function packageVersion(name) {
  const manifest = path.join(ROOT, "node_modules", name, "package.json")
  return JSON.parse(fs.readFileSync(manifest, "utf8")).version
}

// The bundled files and ../assets/manifest.json, keyed by path under ../assets.
function buildAssets() {
  const files = new Map()
  const entries = []
  for (const [file, pkg, source] of ASSET_FILES) {
    const bytes = fs.readFileSync(path.join(ROOT, "node_modules", pkg, source))
    files.set(file, bytes)
    entries.push({
      file,
      package: `${pkg}@${packageVersion(pkg)}`,
      source,
      sha256: createHash("sha256").update(bytes).digest("hex"),
    })
  }
  const manifest = {
    schema: "openui-components-assets-v1",
    note: "Generated by components/reference/build.mjs; files are unmodified copies from the npm packages pinned by components/reference/package-lock.json.",
    font_faces: FONT_FACES,
    lucide_icons: LUCIDE_ICONS,
    files: entries,
  }
  files.set("manifest.json", Buffer.from(JSON.stringify(manifest, null, 2) + "\n"))
  return files
}

function page({ id, theme, title, body }) {
  const htmlClass = theme === "dark" ? ' class="dark"' : ""
  return (
    "<!DOCTYPE html>\n" +
    `<html lang="en"${htmlClass}>\n` +
    "<head>\n" +
    '<meta charset="utf-8">\n' +
    `<title>${title}</title>\n` +
    '<link rel="stylesheet" href="reference.css">\n' +
    "</head>\n" +
    "<body>\n" +
    `<main data-fixture="${id}" class="p-4">${body}</main>\n` +
    "</body>\n" +
    "</html>\n"
  )
}

function themeTokens(globals) {
  const root = globals.match(/^:root \{\n([\s\S]*?)^\}/m)
  if (!root) throw new Error("shadcn/globals.css has no :root block")
  const names = [...root[1].matchAll(/^\s+--([a-z0-9-]+):/gm)].map((m) => m[1])
  return names.filter((name) => name !== "radius")
}

// One page lists every color token in both themes. The harness reads
// Chromium's sRGB conversion of each from it (see harness/README.md).
function tokensPage(tokens) {
  const rows = (theme) =>
    tokens
      .map(
        (name) =>
          `<div data-theme="${theme}" data-token="${name}" ` +
          `style="color: var(--${name})"></div>`
      )
      .join("\n")
  return (
    "<!DOCTYPE html>\n" +
    '<html lang="en">\n' +
    "<head>\n" +
    '<meta charset="utf-8">\n' +
    "<title>theme-tokens</title>\n" +
    '<link rel="stylesheet" href="reference.css">\n' +
    "</head>\n" +
    "<body>\n" +
    `<section data-theme-scope="light">\n${rows("light")}\n</section>\n` +
    `<section class="dark" data-theme-scope="dark">\n${rows("dark")}\n</section>\n` +
    "</body>\n" +
    "</html>\n"
  )
}

async function loadFixtures(scratch) {
  const outfile = path.join(scratch, "fixtures.mjs")
  await esbuild.build({
    entryPoints: [path.join(ROOT, "fixtures.tsx")],
    outfile,
    bundle: true,
    format: "esm",
    platform: "node",
    jsx: "automatic",
    external: ["react", "react-dom", "react/jsx-runtime"],
    nodePaths: [path.join(ROOT, "node_modules")],
    logLevel: "error",
  })
  // The bundle imports React by name; resolve it from this package.
  const linked = path.join(scratch, "node_modules")
  fs.symlinkSync(path.join(ROOT, "node_modules"), linked, "junction")
  return (await import(pathToFileURL(outfile).href)).fixtures
}

function compileCss(scratch, pages) {
  const globals = fs.readFileSync(path.join(ROOT, "shadcn/globals.css"), "utf8")
  const lines = globals.split("\n")
  for (const [index, expected] of GLOBALS_IMPORTS.entries()) {
    if (lines[index] !== expected) {
      throw new Error(`shadcn/globals.css line ${index + 1} is not ${expected}`)
    }
  }
  const entry =
    [...ENTRY_IMPORTS, ...lines.slice(GLOBALS_IMPORTS.length)].join("\n") + fontCss()
  // Tailwind resolves the entry's relative paths from the entry file, so
  // stage the vendored CSS and the pages beside it.
  fs.mkdirSync(path.join(scratch, "shadcn"))
  fs.copyFileSync(
    path.join(ROOT, "shadcn/tailwind.css"),
    path.join(scratch, "shadcn/tailwind.css")
  )
  fs.mkdirSync(path.join(scratch, "pages"))
  for (const [name, text] of pages) {
    fs.writeFileSync(path.join(scratch, "pages", name), text)
  }
  const input = path.join(scratch, "reference.input.css")
  const output = path.join(scratch, "reference.css")
  fs.writeFileSync(input, entry)
  const cli = path.join(ROOT, "node_modules/@tailwindcss/cli/dist/index.mjs")
  const result = spawnSync(process.execPath, [cli, "-i", input, "-o", output], {
    cwd: scratch,
    encoding: "utf8",
  })
  if (result.status !== 0) {
    throw new Error(`tailwindcss failed:\n${result.stderr}`)
  }
  return { css: fs.readFileSync(output, "utf8"), tokens: themeTokens(globals) }
}

async function build() {
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "openui-reference-"))
  try {
    const { renderToStaticMarkup } = await import("react-dom/server")
    const fixtures = await loadFixtures(scratch)
    const ids = new Set()
    const pages = new Map()
    const manifest = []
    for (const fixture of fixtures) {
      if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(fixture.id)) {
        throw new Error(`invalid fixture id ${fixture.id}`)
      }
      const body = renderToStaticMarkup(fixture.render())
      for (const theme of THEMES) {
        const id = `${fixture.id}-${theme}`
        if (ids.has(id)) throw new Error(`duplicate fixture id ${id}`)
        ids.add(id)
        pages.set(`${id}.html`, page({ id, theme, title: id, body }))
        manifest.push({
          id,
          component: fixture.component,
          theme,
          description: fixture.description,
          page: `${id}.html`,
        })
      }
    }
    // Compile once with the fixture pages so the token page cannot add
    // utilities; it only uses the custom properties.
    const { css, tokens } = compileCss(scratch, pages)
    pages.set("theme-tokens.html", tokensPage(tokens))
    pages.set("reference.css", css)
    manifest.sort((a, b) => a.id.localeCompare(b.id))
    const files = {}
    for (const [name, text] of [...pages].sort(([a], [b]) => a.localeCompare(b))) {
      files[name] = createHash("sha256").update(text).digest("hex")
    }
    const lock = fs.readFileSync(path.join(ROOT, "package-lock.json"))
    pages.set(
      "manifest.json",
      JSON.stringify(
        {
          schema: "openui-components-reference-v1",
          package_lock_sha256: createHash("sha256").update(lock).digest("hex"),
          themes: THEMES,
          tokens,
          fixtures: manifest,
          files,
        },
        null,
        2
      ) + "\n"
    )
    return pages
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true })
  }
}

function listFiles(directory, prefix = "") {
  if (!fs.existsSync(directory)) return []
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory()
      ? listFiles(path.join(directory, entry.name), `${prefix}${entry.name}/`)
      : [`${prefix}${entry.name}`]
  )
}

// Names of files under `directory` that differ from `outputs`.
function staleFiles(directory, outputs, label) {
  const stale = []
  for (const name of listFiles(directory)) {
    if (!outputs.has(name)) stale.push(`extra ${label}/${name}`)
  }
  for (const [name, content] of outputs) {
    const file = path.join(directory, name)
    if (!fs.existsSync(file)) stale.push(`missing ${label}/${name}`)
    else if (!fs.readFileSync(file).equals(Buffer.from(content))) {
      stale.push(`differs ${label}/${name}`)
    }
  }
  return stale
}

function writeFiles(directory, outputs) {
  fs.rmSync(directory, { recursive: true, force: true })
  for (const [name, content] of outputs) {
    const file = path.join(directory, name)
    fs.mkdirSync(path.dirname(file), { recursive: true })
    fs.writeFileSync(file, content)
  }
}

const check = process.argv.includes("--check")
const pages = await build()
const assets = buildAssets()
if (check) {
  const stale = [
    ...staleFiles(PAGES, pages, "pages"),
    ...staleFiles(ASSETS, assets, "../assets"),
  ]
  if (stale.length) {
    console.error(`reference pages or assets are stale:\n  ${stale.join("\n  ")}`)
    console.error("run `node build.mjs` in components/reference")
    process.exit(1)
  }
  console.log(`reference pages and assets are current (${pages.size} pages, ${assets.size} assets)`)
} else {
  writeFiles(PAGES, pages)
  writeFiles(ASSETS, assets)
  console.log(`wrote ${pages.size} files to pages/ and ${assets.size} to ../assets/`)
}
