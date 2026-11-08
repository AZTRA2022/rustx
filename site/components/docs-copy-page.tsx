"use client"

import { IconCheck, IconCopy } from "@tabler/icons-react"

import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard"
import { Button } from "@/components/ui/button"

/**
 * “Copy page”.
 *
 * shadcn opens a menu here to v0, ChatGPT and Claude; we keep the
 * only thing that belongs to no one: the text of the page, as
 * it is written.
 */
export function DocsCopyPage({ page }: { page: string; url?: string }) {
  const { copyToClipboard, isCopied } = useCopyToClipboard()

  return (
    <Button
      variant="secondary"
      size="sm"
      className="h-8 shadow-none md:h-7"
      onClick={() => copyToClipboard(page)}
    >
      {isCopied ? (
        <IconCheck className="size-3.5" />
      ) : (
        <IconCopy className="size-3.5" />
      )}
      {isCopied ? "Copied" : "Copy page"}
    </Button>
  )
}
