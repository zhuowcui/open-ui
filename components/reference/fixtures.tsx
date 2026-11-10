// Reference fixtures: each renders one shadcn/ui component state.
//
// The same fixture IDs are implemented with the public `openui` crate in
// `components/reference-render`. Each fixture is rendered in the light and
// the dark theme; the build appends `-light` or `-dark` to the ID.
//
// A fixture's markup sits inside the page's `<main data-fixture>` element,
// which has `p-4` (16px padding). Text uses the bundled fonts: Geist is the
// page's sans-serif face (`--font-sans`), as on the shadcn/ui site.

import * as React from "react"

import { Separator } from "./shadcn/ui/separator"

export type Fixture = {
  id: string
  component: string
  description: string
  render: () => React.ReactElement
}

// Three `text-sm` lines at the weights shadcn's components use (400, 500,
// 600), each short enough not to wrap at the 375px profile in any face.
function FontSpecimen({ className }: { className: string }) {
  return (
    <div className={`flex flex-col gap-1 text-sm ${className}`}>
      <p>Sphinx of black quartz, judge my vow.</p>
      <p className="font-medium">Pack my box with five dozen jugs.</p>
      <p className="font-semibold">{"0123456789 +-=*/ &@#%?!"}</p>
    </div>
  )
}

export const fixtures: Fixture[] = [
  {
    id: "font-geist",
    component: "font",
    description: "Geist (--font-sans) at text-sm, weights 400, 500 and 600",
    render: () => <FontSpecimen className="font-sans" />,
  },
  {
    id: "font-geist-mono",
    component: "font",
    description: "Geist Mono (--font-mono) at text-sm, weights 400, 500 and 600",
    render: () => <FontSpecimen className="font-mono" />,
  },
  {
    id: "font-inter",
    component: "font",
    description: "Inter (--font-inter) at text-sm, weights 400, 500 and 600",
    render: () => <FontSpecimen className="font-inter" />,
  },
  {
    id: "separator-horizontal",
    component: "separator",
    description: "Horizontal separator in a 256px wide column",
    render: () => (
      <div className="w-64">
        <Separator />
      </div>
    ),
  },
  {
    id: "separator-vertical",
    component: "separator",
    description: "Vertical separator in a 32px tall flex row",
    render: () => (
      <div className="flex h-8">
        <Separator orientation="vertical" />
      </div>
    ),
  },
]
