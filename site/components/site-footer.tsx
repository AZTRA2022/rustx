import Link from "next/link"

import { siteConfig } from "@/lib/config"

export function SiteFooter() {
  return (
    <footer className="border-t border-grid group-has-[[data-slot=docs]]/body:hidden">
      <div className=" px-4 xl:px-6">
        <div className="flex h-(--footer-height) flex-col items-center justify-center gap-1 text-center text-xs leading-loose text-muted-foreground sm:text-sm">
          <nav className="flex flex-wrap justify-center gap-x-4 gap-y-1">
            {[
              ...siteConfig.navItems.filter((item) => item.href !== "/"),
              { href: "/llms.txt", label: "llms.txt" },
            ].map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className="underline-offset-4 hover:text-foreground hover:underline"
              >
                {item.label}
              </Link>
            ))}
          </nav>
          <p>
            <span className="font-medium text-foreground">rustx</span> —
            native software, written in Rust, drawn by a browser engine.
          </p>
          <p>
            Licensed under MIT or Apache-2.0, at your option. The packages are{" "}
            <Link
              href={siteConfig.links.crates}
              target="_blank"
              rel="noreferrer"
              className="font-medium underline underline-offset-4"
            >
              on crates.io
            </Link>
            .
          </p>
        </div>
      </div>
    </footer>
  )
}
