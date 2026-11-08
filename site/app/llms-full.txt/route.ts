import { siteConfig } from "@/lib/config"
import { mdUrl, orderedPages, pageMarkdown } from "@/lib/llms"

export const revalidate = false
export const dynamic = "force-static"

export async function GET() {
  const parts = [
    `# ${siteConfig.name}\n\n> ${siteConfig.description}\n\nThe whole documentation, in sidebar order. Each section begins with its source URL.\n`,
  ]

  for (const page of orderedPages()) {
    parts.push(`---\n\nSource: ${mdUrl(page)}\n\n${await pageMarkdown(page)}`)
  }

  return new Response(parts.join("\n"), {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  })
}
