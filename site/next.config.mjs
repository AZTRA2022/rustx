import { createMDX } from "fumadocs-mdx/next"

/** @type {import('next').NextConfig} */
const nextConfig = {
  devIndicators: false,
  redirects() {
    return [{ source: "/documentation", destination: "/docs", permanent: true }]
  },
  // /docs/<slug>.md is the Markdown of a docs page, served by app/llms.mdx.
  rewrites() {
    return {
      beforeFiles: [
        { source: "/docs.md", destination: "/llms.mdx" },
        { source: "/docs/:path*.md", destination: "/llms.mdx/:path*" },
      ],
    }
  },
}

const withMDX = createMDX({})

export default withMDX(nextConfig)
