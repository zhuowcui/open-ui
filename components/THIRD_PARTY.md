# Third-party material in `components/`

| Material | Version | License | Where | How it is used |
| --- | --- | --- | --- | --- |
| shadcn/ui component and theme sources | `shadcn@4.23.0` (`96da9cc`) | MIT | `reference/shadcn/` with [LICENSE.md](reference/shadcn/LICENSE.md) | Vendored; rendered into the reference pages. The Rust components are reimplementations of the same designs. |
| Tailwind CSS (preflight and theme) | 4.3.5 | MIT | compiled into `reference/pages/reference.css` | Build tool; its generated CSS is committed in the reference pages. |
| tw-animate-css | 1.4.0 | MIT | compiled into `reference/pages/reference.css` | Imported by shadcn's theme. |
| React, React DOM | 19.3.1 | MIT | npm, `reference/package-lock.json` | Build tool; renders the fixtures to static markup. Not committed. |
| Radix UI (`radix-ui`) | 1.8.1 | MIT | npm | Build tool; the primitives shadcn's components render. Not committed. |
| class-variance-authority | 0.7.1 | Apache-2.0 | npm | Build tool. Not committed. |
| cn | 0.5.2 | MIT | npm | Build tool. Not committed. |
| esbuild | 0.28.2 | MIT | npm | Build tool. Not committed. |
| Geist and Geist Mono (`geist`) | 1.9.2 | SIL OFL 1.1 | `assets/fonts/geist/` with [OFL.txt](assets/fonts/geist/OFL.txt) | Bundled: `Geist-Variable.woff2` and `GeistMono-Variable.woff2`, unmodified. Loaded by the reference pages and compiled into `openui-components`. |
| Inter (`inter-ui`) | 4.1.1 | SIL OFL 1.1 | `assets/fonts/inter/` with [OFL.txt](assets/fonts/inter/OFL.txt) | Bundled: `InterVariable.woff2`, unmodified. As for Geist. |
| Lucide icons (`lucide-static`) | 1.70.0 | ISC | `assets/icons/lucide/` with [LICENSE](assets/icons/lucide/LICENSE) | Bundled: the 20 icons the new-york-v4 components import, unmodified. Not drawn by Open UI yet. |

The fonts are under the SIL Open Font License 1.1: they may be bundled and
redistributed with their license, but not sold by themselves, and a modified
font (subsetting counts) may not keep the Reserved Font Name. They are
therefore bundled unmodified, byte for byte as published; `build.mjs --check`
and `components/assets/manifest.json` hold their SHA-256 digests, and
`.gitattributes` keeps git from converting the CRLF line endings of the
license and SVG files.
