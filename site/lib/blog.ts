import { blog } from "@/.source/server"

export type Post = {
  slug: string
  title: string
  description?: string
  date: string
  body: (typeof blog)[number]["body"]
}

/** Every post, newest first. A second post is another file in content/blog. */
export function getPosts(): Post[] {
  return blog
    .map((entry) => ({
      slug: entry.info.path.replace(/\.mdx?$/, ""),
      title: entry.title,
      description: entry.description,
      date: entry.date,
      body: entry.body,
    }))
    .sort((a, b) => b.date.localeCompare(a.date))
}

export function formatDate(date: string): string {
  return new Date(`${date}T00:00:00Z`).toLocaleDateString("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  })
}
