"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { useTheme } from "next-themes"

import { getPagesFromFolder, type PageTreePage } from "@/lib/page-tree"
import { type source } from "@/lib/source"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { ArrowRight, Box, Circle, CircleDashed, Hexagon, MessageCircleDashed, MoonStar, SquareDashed, SunDim, TriangleDashed } from "lucide-react"

/**
 * ⌘K.
 *
 * shadcn also puts its blocks, colors and themes there; here there is
 * only one set of documentation and three pages, so the menu searches them and
 * switches the theme.
 */
export function CommandMenu({
  tree,
  navItems,
  className,
}: {
  tree: typeof source.pageTree
  navItems: { href: string; label: string }[]
  className?: string
}) {
  const router = useRouter()
  const { setTheme } = useTheme()
  const [open, setOpen] = React.useState(false)

  React.useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.key === "k" && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        setOpen((open) => !open)
      }
    }

    document.addEventListener("keydown", down)
    return () => document.removeEventListener("keydown", down)
  }, [])

  const run = React.useCallback((command: () => void) => {
    setOpen(false)
    command()
  }, [])

  const groups = React.useMemo(() => {
    const groups: { name: string; pages: PageTreePage[] }[] = []

    for (const item of tree.children) {
      if (item.type === "folder") {
        groups.push({
          name: item.name?.toString() ?? "Documentation",
          pages: getPagesFromFolder(item),
        })
      } else if (item.type === "page") {
        groups.push({ name: "Documentation", pages: [item] })
      }
    }

    return groups
  }, [tree])

  return (
    <>
      <Button
        variant="secondary"
        className={cn(
          "h-8 w-full justify-start gap-2 bg-surface px-2.5 text-sm font-normal text-muted-foreground shadow-none md:w-56 lg:w-64",
          className
        )}
        onClick={() => setOpen(true)}
      >
        <span className="truncate">Search the documentation…</span>
        <KbdGroup className="ml-auto hidden md:flex">
          <Kbd>⌘</Kbd>
          <Kbd>K</Kbd>
        </KbdGroup>
      </Button>
      <CommandDialog
        open={open}
        onOpenChange={setOpen}
        title="Search"
        description="A documentation page, or a setting."
        showCloseButton={false}
      >
        <CommandInput placeholder="Type to search…"  />
        <CommandList  className="no-scrollbar">
          <CommandEmpty>Nothing by that name.</CommandEmpty>
          <CommandGroup  heading="The site">
            {navItems.map((item) => (
              <CommandItem
                key={item.href}
                value={`site ${item.label}`}
                onSelect={() => run(() => router.push(item.href))}
              >
                {/* Sorry guys, i know this code is bad, but i'm to lazy to make it better :) */}
                {item.href.includes("docs") && <CircleDashed size={14} /> }
                {item.href.includes("getting-started") && <SquareDashed  size={14} /> }
                {item.href.includes("/showcase") && <TriangleDashed size={14} /> }
                {item.href.includes("/faq") && <Hexagon size={14} /> }
                {item.href.includes("/changelog") && <Box size={14} /> }
                {item.href.includes("/blog") && <MessageCircleDashed size={14} /> }
                {item.label}
              </CommandItem>
            ))}
          </CommandGroup>
          {groups.map((group) => (
            <CommandGroup key={group.name} heading={group.name}>
              {group.pages.map((page) => (
                <CommandItem
                  key={page.url}
                  value={`${group.name} ${page.name}`}
                  onSelect={() => run(() => router.push(page.url))}
                >
                 <CircleDashed size={14} />  {page.name}
                </CommandItem>
              ))}
            </CommandGroup>
          ))}
          <CommandSeparator />
          <CommandGroup heading="Theme">
            <CommandItem onSelect={() => run(() => setTheme("light"))}>
              Light <SunDim size={14} />
            </CommandItem>
            <CommandItem onSelect={() => run(() => setTheme("dark"))}>
              Dark <MoonStar size={14} />
            </CommandItem>
            <CommandItem onSelect={() => run(() => setTheme("system"))}>
              Follow the system
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </CommandDialog>
    </>
  )
}
