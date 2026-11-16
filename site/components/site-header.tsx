import { siteConfig } from "@/lib/config"
import { source } from "@/lib/source"
import { CommandMenu } from "@/components/command-menu"
import { GitHubLink } from "@/components/github-link"
import { MainNav } from "@/components/main-nav"
import { MobileNav } from "@/components/mobile-nav"
import { ModeSwitcher } from "@/components/mode-switcher"
import { Separator } from "@/components/ui/separator"

export function SiteHeader() {
  const pageTree = source.pageTree

  return (
    <header className="sticky top-0 z-50 w-full border-b border-grid bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/80">
      <div className=" px-6">
        <div className="flex h-(--header-height) items-center **:data-[slot=separator]:h-4!">
          <MobileNav
            tree={pageTree}
            items={siteConfig.navItems}
            className="flex lg:hidden"
          />
          <MainNav items={siteConfig.navItems} className="hidden lg:flex" />
          <div className="ml-auto flex items-center gap-2 md:flex-1 md:justify-end">
            <div className="hidden w-full flex-1 md:flex md:w-auto md:flex-none">
              <CommandMenu tree={pageTree} navItems={siteConfig.navItems} />
            </div>
            <Separator orientation="vertical" className="ml-2 hidden lg:block" />
            <GitHubLink />
            <Separator orientation="vertical" />
            <ModeSwitcher />
          </div>
        </div>
      </div>
    </header>
  )
}
