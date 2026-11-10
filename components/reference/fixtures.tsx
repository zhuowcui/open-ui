// Reference fixtures: each renders one shadcn/ui component state.
//
// The same fixture IDs are implemented with the public `openui` crate in
// `components/reference-render`. Each fixture is rendered in the light and
// the dark theme; the build appends `-light` or `-dark` to the ID.
//
// A fixture's markup sits inside the page's `<main data-fixture>` element,
// which has `p-4` (16px padding). Keep fixtures free of text until the
// fonts are bundled (C4), so these measure box geometry and color only.

import * as React from "react"

import { Separator } from "./shadcn/ui/separator"

export type Fixture = {
  id: string
  component: string
  description: string
  render: () => React.ReactElement
}

export const fixtures: Fixture[] = [
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
