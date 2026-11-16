import Link from "next/link"
import {
  IconBrandChrome,
  IconBrandGithub,
  IconBrandNodejs,
  IconBrandNpm,
  IconBrandPowershell,
  IconBrandReact,
  IconBrandRust,
  IconBrandTailwind,
  IconBrandTypescript,
  IconBrandVite,
  IconBrandVscode,
} from "@tabler/icons-react"
import { BoxIcon, PackageIcon, PanelsTopLeftIcon, TerminalIcon } from "lucide-react"

import { cn } from "@/lib/utils"

/**
 * One of the things rustx is built on, or next to, with its mark.
 *
 * The documentation names a lot of other people's software — Cargo on
 * nearly every page, React and Tailwind on most of them, VS Code and
 * `rust-analyzer` wherever the editor comes up. A name in running text
 * is easy to miss; a name with its mark beside it is found by someone
 * skimming for "does this work with the thing I already use", which is
 * the question most of these pages are being read to answer.
 *
 * The marks are Tabler's, which this site already depends on and which
 * are MIT. Each one is drawn in its own brand's colour and the name
 * beside it stays the colour of the sentence: the mark is what the eye
 * catches while skimming, and the text still has to read as text. A
 * fully coloured name in the middle of a paragraph reads as an advert,
 * which these are not — they are references to other people's
 * software, not endorsements by it.
 *
 * Two colours per brand, because a one-colour mark cannot survive both
 * themes: Rust's black and GitHub's near-black vanish on a dark
 * background, and esbuild's yellow vanishes on a light one. `tint` is
 * for light, `dark` for dark, and `dark` is only given where the light
 * value genuinely fails — most brands keep one colour in both.
 *
 * Anything the set has no mark for gets the closest honest generic
 * rather than an invented logo: a shape nobody recognises is worse
 * than a box, and a *wrong* logo is worse than both. A generic shape
 * is not given a brand colour either — colouring a box the colour of a
 * company implies a logo that is not there.
 */
// Two icon sets with two different ref types, used here for exactly
// the three props they agree on.
type Mark = React.ComponentType<{
  className?: string
  style?: React.CSSProperties
  "aria-hidden"?: boolean
}>

type Entry = {
  label: string
  href: string
  icon: Mark
  /** The brand's colour, for a light background. */
  tint?: string
  /** Its colour on a dark background, where `tint` would be lost. */
  dark?: string
}

const BRANDS: Record<string, Entry> = {
  // Rust's own logo is black, and black is unusable in a dark theme.
  // The orange is Rust's other long-standing colour -- the one the gear
  // is drawn in nearly everywhere -- and it holds up on both
  // backgrounds, which black and white each only manage on one.
  rust: {
    label: "Rust",
    href: "https://www.rust-lang.org",
    icon: IconBrandRust,
    tint: "#ce412b",
  },
  // Cargo, crates.io and rust-analyzer are Rust's own; they have no
  // separate mark and borrowing Rust's is what the Rust project does.
  cargo: {
    label: "Cargo",
    href: "https://doc.rust-lang.org/cargo/",
    icon: IconBrandRust,
    tint: "#ce412b",
  },
  "crates.io": {
    label: "crates.io",
    href: "https://crates.io",
    icon: IconBrandRust,
    tint: "#ce412b",
  },
  "rust-analyzer": {
    label: "rust-analyzer",
    href: "https://rust-analyzer.github.io",
    icon: IconBrandRust,
    tint: "#ce412b",
  },
  vscode: {
    label: "VS Code",
    href: "https://code.visualstudio.com",
    icon: IconBrandVscode,
    tint: "#0065a9",
    dark: "#2aa3e8",
  },
  github: {
    label: "GitHub",
    href: "https://github.com",
    icon: IconBrandGithub,
    tint: "#181717",
    dark: "#ffffff",
  },
  chromium: {
    label: "Chromium",
    href: "https://www.chromium.org",
    icon: IconBrandChrome,
    tint: "#1a73e8",
    dark: "#8ab4f8",
  },
  cef: {
    label: "CEF",
    href: "https://bitbucket.org/chromiumembedded/cef",
    icon: IconBrandChrome,
    tint: "#1a73e8",
    dark: "#8ab4f8",
  },
  // React's own site draws the mark in this blue on white and in its
  // familiar cyan on dark, which is exactly the pair needed here.
  react: {
    label: "React",
    href: "https://react.dev",
    icon: IconBrandReact,
    tint: "#087ea4",
    dark: "#58c4dc",
  },
  tailwind: {
    label: "Tailwind CSS",
    href: "https://tailwindcss.com",
    icon: IconBrandTailwind,
    tint: "#0891b2",
    dark: "#38bdf8",
  },
  npm: {
    label: "npm",
    href: "https://www.npmjs.com",
    icon: IconBrandNpm,
    tint: "#cb3837",
    dark: "#f1534f",
  },
  node: {
    label: "Node.js",
    href: "https://nodejs.org",
    icon: IconBrandNodejs,
    tint: "#5fa04e",
    dark: "#7fc46c",
  },
  typescript: {
    label: "TypeScript",
    href: "https://www.typescriptlang.org",
    icon: IconBrandTypescript,
    tint: "#3178c6",
    dark: "#6fa8e0",
  },
  vite: {
    label: "Vite",
    href: "https://vite.dev",
    icon: IconBrandVite,
    tint: "#646cff",
    dark: "#9499ff",
  },
  // No mark in the set, so the closest honest shape -- and no brand
  // colour on it either, because colouring a generic box in a company's
  // colour claims a logo that is not being shown.
  shadcn: { label: "shadcn/ui", href: "https://ui.shadcn.com", icon: PanelsTopLeftIcon },
  esbuild: { label: "esbuild", href: "https://esbuild.github.io", icon: PackageIcon },
  recharts: { label: "Recharts", href: "https://recharts.org", icon: BoxIcon },
  tauri: { label: "Tauri", href: "https://tauri.app", icon: BoxIcon },
  electron: { label: "Electron", href: "https://www.electronjs.org", icon: BoxIcon },
  zed: { label: "Zed", href: "https://zed.dev", icon: TerminalIcon },
  neovim: { label: "Neovim", href: "https://neovim.io", icon: TerminalIcon },
  jetbrains: {
    label: "JetBrains",
    href: "https://www.jetbrains.com",
    icon: PanelsTopLeftIcon,
  },
  // The three shells `rustx::terminal` speaks, one per platform. They
  // are the system's own programs and not dependencies of this project,
  // which is the whole reason the terminal page names them. Only
  // PowerShell has a mark of its own in the set, so only PowerShell
  // gets a colour.
  powershell: {
    label: "PowerShell",
    href: "https://learn.microsoft.com/powershell/",
    icon: IconBrandPowershell,
    tint: "#2b61b4",
    dark: "#5f9cea",
  },
  zsh: { label: "zsh", href: "https://www.zsh.org", icon: TerminalIcon },
  bash: {
    label: "bash",
    href: "https://www.gnu.org/software/bash/",
    icon: TerminalIcon,
  },
}

/** The names this component answers to, for the test beside it. */
export const BRAND_NAMES = Object.keys(BRANDS)

export function Brand({
  name,
  children,
  bare = false,
  className,
}: {
  /** A key of the table above: `vscode`, `rust`, `github`. */
  name: string
  /** What to call it here, when that is not its usual name. */
  children?: React.ReactNode
  /** The mark alone, for a list that already says the name. */
  bare?: boolean
  className?: string
}) {
  const brand = BRANDS[name.toLowerCase()]
  // An unknown name renders as the text that was written, rather than
  // as nothing: a typo in an `.mdx` should cost a missing icon, not a
  // missing sentence.
  if (!brand) {
    return <>{children ?? name}</>
  }

  const Mark = brand.icon
  // The colour travels as a custom property rather than as a `color`,
  // so that the dark value can be chosen by the theme instead of by
  // this component -- an inline style cannot ask which theme it is in.
  // A brand with no `tint` sets neither, and its mark inherits the
  // colour of the text, which is what a generic shape should do.
  const tinted = brand.tint !== undefined
  const paint = tinted
    ? ({
        "--brand-mark": brand.tint,
        "--brand-mark-dark": brand.dark ?? brand.tint,
      } as React.CSSProperties)
    : undefined

  return (
    <Link
      href={brand.href}
      target="_blank"
      rel="noreferrer"
      title={brand.label}
      data-not-typeset
      className={cn(
        "inline-flex items-center gap-1 align-baseline font-medium no-underline hover:underline",
        className
      )}
    >
      <Mark
        aria-hidden
        style={paint}
        className={cn(
          "size-[1.05em] shrink-0 translate-y-[0.1em]",
          tinted &&
            "text-[color:var(--brand-mark)] dark:text-[color:var(--brand-mark-dark)]"
        )}
      />
      {!bare && <span>{children ?? brand.label}</span>}
    </Link>
  )
}
