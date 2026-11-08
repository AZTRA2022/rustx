import { siteConfig } from "@/lib/config"
import { mdUrl, orderedPages } from "@/lib/llms"

export const revalidate = false
export const dynamic = "force-static"

export function GET() {
  const lines = [
    `# ${siteConfig.name}`,
    "",
    `> ${siteConfig.description}`,
    "",
    "rustx is a Rust desktop GUI framework. Components are written in `.rsx` files: Rust with JSX-style markup in the `return` statements. `.rsx` is not a format that exists outside this project, so read the documentation before writing it. Each page below has a plain Markdown version; /llms-full.txt holds all of them in one file.",
    "",
    "## Documentation",
    "",
    ...orderedPages().map(
      (page) =>
        `- [${page.data.title}](${mdUrl(page)})${
          page.data.description ? `: ${page.data.description}` : ""
        }`
    ),
    "",
    "## Site",
    "",
    `- [Changelog](${siteConfig.url}/changelog): what changed in each release`,
    `- [FAQ](${siteConfig.url}/faq): platforms, size, Node, and what is not ready`,
    `- [Showcase](${siteConfig.url}/showcase): the applications of the repository`,
    `- [Blog](${siteConfig.url}/blog)`,
    "",
  ]

  return new Response(lines.join("\n"), {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  })
}
