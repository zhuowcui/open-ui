// Reference fixtures: each renders one shadcn/ui component state.
//
// The same fixture IDs are implemented with the public `openui` crate in
// `components/reference-render`. Each fixture is rendered in the light and
// the dark theme; the build appends `-light` or `-dark` to the ID.
//
// A fixture's markup sits inside the page's `<main data-fixture>` element,
// which has `p-4` (16px padding). Text uses the bundled fonts: Geist is the
// page's sans-serif face (`--font-sans`), as on the shadcn/ui site.
//
// Inline-level components (button, badge) sit in a `flex` row, as on the
// shadcn/ui site's examples, so each is a flex item rather than an inline
// box on a line of text.

import * as React from "react"

import { Badge } from "./shadcn/ui/badge"
import { Button } from "./shadcn/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./shadcn/ui/card"
import { Label } from "./shadcn/ui/label"
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

type ButtonVariant =
  | "default"
  | "destructive"
  | "outline"
  | "secondary"
  | "ghost"
  | "link"

// Every variant at the default size. The icon sizes need a Lucide icon,
// which Open UI cannot draw yet, so they are not fixtures.
const buttonVariants: [ButtonVariant, string][] = [
  ["default", "Button"],
  ["destructive", "Destructive"],
  ["outline", "Outline"],
  ["secondary", "Secondary"],
  ["ghost", "Ghost"],
  ["link", "Link"],
]

const buttonSizes: ["xs" | "sm" | "lg", string][] = [
  ["xs", "Extra small"],
  ["sm", "Small"],
  ["lg", "Large"],
]

const badgeVariants: [ButtonVariant, string][] = [
  ["default", "Badge"],
  ["secondary", "Secondary"],
  ["destructive", "Destructive"],
  ["outline", "Outline"],
  ["ghost", "Ghost"],
  ["link", "Link"],
]

function Row({ children }: { children: React.ReactNode }) {
  return <div className="flex">{children}</div>
}

const buttonFixtures: Fixture[] = [
  ...buttonVariants.map(([variant, text]) => ({
    id: `button-${variant}`,
    component: "button",
    description: `Button, ${variant} variant, default size`,
    render: () => (
      <Row>
        <Button variant={variant}>{text}</Button>
      </Row>
    ),
  })),
  ...buttonSizes.map(([size, text]) => ({
    id: `button-size-${size}`,
    component: "button",
    description: `Button, default variant, size ${size}`,
    render: () => (
      <Row>
        <Button size={size}>{text}</Button>
      </Row>
    ),
  })),
  {
    id: "button-disabled",
    component: "button",
    description: "Disabled button, default variant (opacity 50%)",
    render: () => (
      <Row>
        <Button disabled>Disabled</Button>
      </Row>
    ),
  },
]

const badgeFixtures: Fixture[] = badgeVariants.map(([variant, text]) => ({
  id: `badge-${variant}`,
  component: "badge",
  description: `Badge, ${variant} variant`,
  render: () => (
    <Row>
      <Badge variant={variant}>{text}</Badge>
    </Row>
  ),
}))

// The card from the shadcn/ui docs' usage example, without and with an
// action in the header, at `w-full max-w-sm` (384px, or the 343px column of
// the 375px profile).
function ExampleCard({ action }: { action: boolean }) {
  return (
    <Card className="w-full max-w-sm">
      <CardHeader>
        <CardTitle>Card Title</CardTitle>
        <CardDescription>Card Description</CardDescription>
        {action ? <CardAction>Card Action</CardAction> : null}
      </CardHeader>
      <CardContent>
        <p>Card Content</p>
      </CardContent>
      <CardFooter>
        <p>Card Footer</p>
      </CardFooter>
    </Card>
  )
}

const cardFixtures: Fixture[] = [
  {
    id: "card-basic",
    component: "card",
    description: "Card with header (title, description), content and footer",
    render: () => <ExampleCard action={false} />,
  },
  {
    id: "card-action",
    component: "card",
    description: "Card whose header also has an action (two-column grid)",
    render: () => <ExampleCard action={true} />,
  },
]

const labelFixtures: Fixture[] = [
  {
    id: "label-basic",
    component: "label",
    description: "Label on its own",
    render: () => <Label>Accept terms and conditions</Label>,
  },
]

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
  ...buttonFixtures,
  ...badgeFixtures,
  ...cardFixtures,
  ...labelFixtures,
]
