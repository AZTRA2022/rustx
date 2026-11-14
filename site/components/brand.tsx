import Link from "next/link"
import {
  IconBrandChrome,
  IconBrandGithub,
  IconBrandNodejs,
  IconBrandNpm,
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
 * are MIT. They are drawn in one colour from `currentColor`, so a
 * brand sits in a sentence at the weight of the sentence rather than
 * dropping a logo into the middle of a paragraph — these are
 * references to other people's software, not endorsements by it, and
 * they should read as references.
 *
 * Anything the set has no mark for gets the closest honest generic
 * rather than an invented logo: a shape nobody recognises is worse
 * than a box, and a *wrong* logo is worse than both.
 */
// Two icon sets with two different ref types, used here for exactly
// the three props they agree on.
type Mark = React.ComponentType<{
  className?: string
  "aria-hidden"?: boolean
}>

const BRANDS: Record<string, { label: string; href: string; icon: Mark }> = {
  rust: { label: "Rust", href: "https://www.rust-lang.org", icon: IconBrandRust },
  // Cargo, crates.io and rust-analyzer are Rust's own; they have no
  // separate mark and borrowing Rust's is what the Rust project does.
  cargo: { label: "Cargo", href: "https://doc.rust-lang.org/cargo/", icon: IconBrandRust },
  "crates.io": { label: "crates.io", href: "https://crates.io", icon: IconBrandRust },
  "rust-analyzer": {
    label: "rust-analyzer",
    href: "https://rust-analyzer.github.io",
    icon: IconBrandRust,
  },
  vscode: {
    label: "VS Code",
    href: "https://code.visualstudio.com",
    icon: IconBrandVscode,
  },
  github: { label: "GitHub", href: "https://github.com", icon: IconBrandGithub },
  chromium: {
    label: "Chromium",
    href: "https://www.chromium.org",
    icon: IconBrandChrome,
  },
  cef: {
    label: "CEF",
    href: "https://bitbucket.org/chromiumembedded/cef",
    icon: IconBrandChrome,
  },
  react: { label: "React", href: "https://react.dev", icon: IconBrandReact },
  tailwind: {
    label: "Tailwind CSS",
    href: "https://tailwindcss.com",
    icon: IconBrandTailwind,
  },
  npm: { label: "npm", href: "https://www.npmjs.com", icon: IconBrandNpm },
  node: { label: "Node.js", href: "https://nodejs.org", icon: IconBrandNodejs },
  typescript: {
    label: "TypeScript",
    href: "https://www.typescriptlang.org",
    icon: IconBrandTypescript,
  },
  vite: { label: "Vite", href: "https://vite.dev", icon: IconBrandVite },
  // No mark in the set, so the closest honest shape.
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
      <Mark aria-hidden className="size-[1.05em] shrink-0 translate-y-[0.1em]" />
      {!bare && <span>{children ?? brand.label}</span>}
    </Link>
  )
}
