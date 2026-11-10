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

## Checked for the assets unit (C4), not bundled yet

| Material | Version | License | Notes |
| --- | --- | --- | --- |
| Lucide icons (`lucide-static`) | 1.70.0 | ISC | Bundling is allowed with the license notice. |
| Geist and Geist Mono (`geist`) | 1.9.2 | SIL OFL 1.1 | Bundling is allowed with the license. Fonts may not be sold alone. A modified font (subsetting counts) may not keep the Reserved Font Name, so bundle the files unmodified. |
| Inter (`inter-ui`) | 4.1.1, to be pinned in C4 | SIL OFL 1.1 | As for Geist. |
