import { notFound } from "next/navigation"

import { pageMarkdown } from "@/lib/llms"
import { source } from "@/lib/source"

export const revalidate = false
export const dynamic = "force-static"
export const dynamicParams = false

// Reached through the rewrites in next.config.mjs:
// /docs/<slug>.md and /docs.md.
export async function GET(
  _request: Request,
  { params }: { params: Promise<{ slug?: string[] }> }
) {
  const { slug } = await params
  const page = source.getPage(slug)

  if (!page) notFound()

  return new Response(await pageMarkdown(page), {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  })
}

export function generateStaticParams() {
  return source.generateParams()
}
