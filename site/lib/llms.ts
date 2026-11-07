import type { Node, Root } from "fumadocs-core/page-tree"

import { siteConfig } from "@/lib/config"
import { source } from "@/lib/source"

type DocPage = NonNullable<ReturnType<typeof source.getPage>>

/** Every docs page, in the order of the sidebar. */
export function orderedPages(): DocPage[] {
  const seen = new Set<string>()
  const out: DocPage[] = []

  const visit = (node: Node | Root) => {
    if ("type" in node && node.type === "page") {
      const slugs = node.url.replace(/^\/docs\/?/, "").split("/").filter(Boolean)
      const page = source.getPage(slugs)
      if (page && !seen.has(page.url)) {
        seen.add(page.url)
        out.push(page)
      }
      return
    }
    if ("type" in node && node.type === "folder" && node.index) visit(node.index)
    if ("children" in node) node.children.forEach(visit)
  }

  visit(source.pageTree)

  // Anything the tree does not list still belongs in the file.
  for (const page of source.getPages()) {
    if (!seen.has(page.url)) out.push(page)
  }
  return out
}

/** The MDX source with its frontmatter and JSX wrappers turned into plain Markdown. */
function plain(raw: string): string {
  return raw
    .replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/, "")
    .replace(/^<Callout(?:[^>]*?title="([^"]*)")?[^>]*>\s*$/gm, (_, title) =>
      title ? `**${title}.**\n` : ""
    )
    .replace(/^<\/Callout>\s*$/gm, "")
    .replace(/\n{3,}/g, "\n\n")
    .trim()
}

/** The Markdown address of a docs page: /docs/<slug>.md, and /docs.md for the root. */
export function mdPath(url: string): string {
  return `${url}.md`
}

export function mdUrl(page: DocPage): string {
  return `${siteConfig.url}${mdPath(page.url)}`
}

export async function pageMarkdown(page: DocPage): Promise<string> {
  const raw = await page.data.getText("raw")
  const body = plain(raw)
    // A link to a docs page points at its Markdown twin.
    .replace(
      /\]\((\/docs(?:\/[^)\s#]*)?)(#[^)\s]*)?\)/g,
      (_, path: string, hash = "") =>
        `](${siteConfig.url}${mdPath(path)}${hash})`
    )
    // Any other site link becomes absolute.
    .replace(
      /\]\((\/(?!docs)[^)\s]*)\)/g,
      (_, path: string) => `](${siteConfig.url}${path})`
    )
  const description = page.data.description
    ? `> ${page.data.description}\n\n`
    : ""

  return `# ${page.data.title}\n\n${description}${body}\n`
}
