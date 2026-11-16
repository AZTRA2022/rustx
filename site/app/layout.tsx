import type { Metadata } from "next"

import { META_THEME_COLORS, siteConfig } from "@/lib/config"
import { DOCS_SIDEBAR_SCROLL_RESTORE_SCRIPT } from "@/lib/docs-sidebar-scroll"
import { fontVariables } from "@/lib/fonts"
import { cn } from "@/lib/utils"
import { ActiveThemeProvider } from "@/components/active-theme"
import { SiteFooter } from "@/components/site-footer"
import { SiteHeader } from "@/components/site-header"
import { ThemeProvider } from "@/components/theme-provider"
import { TooltipProvider } from "@/components/ui/tooltip"

import "@/app/globals.css"
import "@/app/typeset.css"

export const metadata: Metadata = {
  title: {
    default: `${siteConfig.name} — native software, written in Rust`,
    template: `%s - ${siteConfig.name}`,
  },
  metadataBase: new URL(siteConfig.url),
  description: siteConfig.description,
  keywords: ["rustx", "Rust", "bureau", "desktop", "CEF", "rsx", "npm"],
  openGraph: {
    type: "website",
    locale: "en_US",
    url: siteConfig.url,
    title: siteConfig.name,
    description: siteConfig.description,
    siteName: siteConfig.name,
    images: [
      {
        url: `/og?title=${encodeURIComponent(
          siteConfig.name
        )}&description=${encodeURIComponent(siteConfig.description)}`,
        width: 1200,
        height: 628,
        alt: siteConfig.name,
      },
    ],
  },
}

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={cn(
        fontVariables,
        "[--header-height:calc(var(--spacing)*14)] lg:[--header-height:calc(var(--spacing)*16)]"
      )}
    >
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html: DOCS_SIDEBAR_SCROLL_RESTORE_SCRIPT,
          }}
        />
        <script
          dangerouslySetInnerHTML={{
            __html: `
              try {
                if (localStorage.theme === 'dark' || ((!('theme' in localStorage) || localStorage.theme === 'system') && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
                  document.querySelector('meta[name="theme-color"]').setAttribute('content', '${META_THEME_COLORS.dark}')
                }
              } catch (_) {}
            `,
          }}
        />
        <meta name="theme-color" content={META_THEME_COLORS.light} />
      </head>
      <body className="group/body overscroll-none antialiased [--footer-height:calc(var(--spacing)*28)]">
        <ThemeProvider>
          <ActiveThemeProvider>
            <TooltipProvider delayDuration={0}>
              <div
                data-slot="layout"
                className="group/layout relative z-10 flex min-h-svh flex-col bg-background"
              >
                <SiteHeader />
                <main className="flex min-h-0 flex-1 flex-col">{children}</main>
                <SiteFooter />
              </div>
            </TooltipProvider>
          </ActiveThemeProvider>
        </ThemeProvider>
      </body>
    </html>
  )
}
