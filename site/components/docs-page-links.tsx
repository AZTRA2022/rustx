"use client"

import Link from "next/link"
import { IconCheck, IconCopy } from "@tabler/icons-react"

import { siteConfig } from "@/lib/config"
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard"

/** What can be done with a page, in the right margin. */
export function DocsPageLinks({ page }: { page: string; url?: string }) {
  const { copyToClipboard, isCopied } = useCopyToClipboard()

  return (
    <div className="flex flex-col gap-3 px-6">
      <ul className="flex flex-col gap-2 text-[0.8rem] text-muted-foreground">
        <li>
          <button
            onClick={() => copyToClipboard(page)}
            className="inline-flex items-center gap-2 transition-colors hover:text-foreground"
          >
            {isCopied ? (
              <IconCheck className="size-4" />
            ) : (
              <IconCopy className="size-4" />
            )}
            Copy this page
          </button>
        </li>
        <li>
          <Link
            href={siteConfig.links.crates}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-2 transition-colors hover:text-foreground"
          >
            Les paquets sur crates.io
          </Link>
        </li>
      </ul>
    </div>
  )
}
