"use client"

import * as React from "react"
import { GalleryHorizontalIcon } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"

/**
 * The page width button: “full width” or “fixed”.
 *
 * The `layout-fixed` class on `<html>` is what the
 * `fixed:` variant of `globals.css` looks at. The choice lives in
 * `localStorage`, because it belongs to the reader.
 */
export function SiteConfig({ className }: { className?: string }) {
  const [layout, setLayout] = React.useState<"fixed" | "full">("full")

  React.useEffect(() => {
    try {
      const stored = window.localStorage.getItem("rustx-site-layout")
      if (stored === "fixed" || stored === "full") {
        setLayout(stored)
      }
    } catch {}
  }, [])

  React.useEffect(() => {
    document.documentElement.classList.toggle("layout-fixed", layout === "fixed")
    try {
      window.localStorage.setItem("rustx-site-layout", layout)
    } catch {}
  }, [layout])

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={() => setLayout(layout === "fixed" ? "full" : "fixed")}
      className={cn("size-8", className)}
      title="Change page width"
    >
      <span className="sr-only">Change page width</span>
      <GalleryHorizontalIcon />
    </Button>
  )
}
