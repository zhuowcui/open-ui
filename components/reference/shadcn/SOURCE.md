# Vendored shadcn/ui sources

These files are copied without change from
[shadcn-ui/ui](https://github.com/shadcn-ui/ui) at tag `shadcn@4.23.0`,
commit `96da9cc7f1aa10580a3741664e2a77c721206c00` (27 October 2026). They
are under the MIT license in [LICENSE.md](LICENSE.md).

| File | Source path |
| --- | --- |
| `ui/*.tsx` | `apps/v4/registry/new-york-v4/ui/*.tsx` |
| `lib/utils.ts` | `apps/v4/registry/new-york-v4/lib/utils.ts` |
| `tailwind.css` | `packages/shadcn/src/tailwind.css` (the `shadcn/tailwind.css` the theme imports) |
| `globals.css` | The `src/styles/globals.css` block of `apps/v4/content/docs/installation/manual.mdx` |
| `LICENSE.md` | `LICENSE.md` |

`new-york-v4` is the classic shadcn/ui style built on Radix, which the
shadcn documentation renders. The theme is the manual installation's
`globals.css`: the neutral base color with the default `0.625rem` radius.
Since late 2025 `shadcn init` defaults to the `base-nova` preset on Base UI;
that style is not the reference here.

The registry imports `cn` from the `cn` npm package (shadcn's replacement for
`clsx` plus `tailwind-merge`), which `package.json` pins.

To update, copy the same paths from a newer tag, record the tag and commit
here, rebuild the pages with `node build.mjs` and re-measure.
