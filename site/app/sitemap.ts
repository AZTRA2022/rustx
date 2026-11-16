import type { MetadataRoute } from "next"

import { siteConfig } from "@/lib/config"
import { getPosts } from "@/lib/blog"
import { source } from "@/lib/source"

const staticRoutes = ["/", "/showcase", "/changelog", "/faq", "/blog"]

export default function sitemap(): MetadataRoute.Sitemap {
  const urls = [
    ...staticRoutes,
    ...getPosts().map((post) => `/blog/${post.slug}`),
    ...source.getPages().map((page) => page.url),
  ]

  return [...new Set(urls)].map((path) => ({
    url: new URL(path, siteConfig.url).toString(),
  }))
}
