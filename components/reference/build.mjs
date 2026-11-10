// Build the shadcn/ui reference pages.
//
//   node build.mjs           write pages/ from the pinned sources
//   node build.mjs --check   rebuild in memory and fail if pages/ differs
//
// Inputs are pinned: package-lock.json fixes React, Radix, Tailwind and
// the other packages, and shadcn/ holds the shadcn/ui sources vendored from
// one commit (see shadcn/SOURCE.md). Each fixture is rendered with React's
// static renderer and wrapped in a page that links pages/reference.css,
// which Tailwind compiles from shadcn/globals.css with source detection
// limited to the generated pages.

import { spawnSync } from "node:child_process"
import { createHash } from "node:crypto"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

import * as esbuild from "esbuild"

const ROOT = path.dirname(fileURLToPath(import.meta.url))
const PAGES = path.join(ROOT, "pages")
const THEMES = ["light", "dark"]

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
  const entry = [...ENTRY_IMPORTS, ...lines.slice(GLOBALS_IMPORTS.length)].join("\n")
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

const check = process.argv.includes("--check")
const pages = await build()
if (check) {
  const expected = new Set(pages.keys())
  const actual = fs.existsSync(PAGES) ? fs.readdirSync(PAGES) : []
  const stale = []
  for (const name of actual) if (!expected.has(name)) stale.push(`extra ${name}`)
  for (const [name, text] of pages) {
    const file = path.join(PAGES, name)
    if (!fs.existsSync(file)) stale.push(`missing ${name}`)
    else if (fs.readFileSync(file, "utf8") !== text) stale.push(`differs ${name}`)
  }
  if (stale.length) {
    console.error(`reference pages are stale:\n  ${stale.join("\n  ")}`)
    console.error("run `node build.mjs` in components/reference")
    process.exit(1)
  }
  console.log(`reference pages are current (${pages.size} files)`)
} else {
  fs.rmSync(PAGES, { recursive: true, force: true })
  fs.mkdirSync(PAGES)
  for (const [name, text] of pages) fs.writeFileSync(path.join(PAGES, name), text)
  console.log(`wrote ${pages.size} files to pages/`)
}
