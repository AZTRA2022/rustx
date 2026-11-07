"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"

import { cn } from "@/lib/utils"
import { Icons } from "@/components/icons"
import { Button } from "@/components/ui/button"

export function MainNav({
  items,
  className,
  ...props
}: React.ComponentProps<"nav"> & {
  items: { href: string; label: string }[]
}) {
  const pathname = usePathname()

  return (
    <nav className={cn("items-center gap-0", className)} {...props}>
      <Link href="/" className="mr-4 flex items-center gap-2">
        <Icons.logo className="size-5" />
        <span className="font-mono text-base font-semibold tracking-tight">
          rustx
        </span>
      </Link>
      {items.map((item) => (
        <Button
          key={item.href}
          variant="ghost"
          asChild
          size="sm"
          className="px-2.5"
        >
          <Link
            href={item.href}
            data-active={pathname === item.href}
            className="relative items-center data-[active=true]:text-foreground"
          >
            {item.label}
          </Link>
        </Button>
      ))}
    </nav>
  )
}
