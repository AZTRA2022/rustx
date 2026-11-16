import Link from "next/link"

import { siteConfig } from "@/lib/config"
import { Icons } from "@/components/icons"
import { Button } from "@/components/ui/button"

/**
 * The link to the code.
 *
 * shadcn counted its GitHub stars here; ours is not
 * public yet, so the link goes to the packages, and
 * `NEXT_PUBLIC_REPO_URL` replaces it the day there is a repository to
 * show.
 */
export function GitHubLink() {
  return (
    <Button asChild size="sm" variant="ghost" className="h-8 shadow-none">
      <Link href={siteConfig.links.repository} target="_blank" rel="noreferrer">
        <Icons.repository className="size-4" />
        <span className="text-xs text-muted-foreground">the packages</span>
      </Link>
    </Button>
  )
}
