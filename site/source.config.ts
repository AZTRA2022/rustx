import {
  defineCollections,
  defineConfig,
  defineDocs,
  frontmatterSchema,
} from "fumadocs-mdx/config"
import rehypePrettyCode from "rehype-pretty-code"
import { z } from "zod"

import { transformers } from "@/lib/highlight-code"

export default defineConfig({
  mdxOptions: {
    rehypePlugins: (plugins) => {
      plugins.shift()
      plugins.push([
        rehypePrettyCode,
        {
          theme: {
            dark: "vesper",
            light: "github-light-default",
          },
          transformers,
        },
      ])

      return plugins
    },
  },
})

export const docs = defineDocs({
  dir: "content/docs",
})

// One MDX file per post. The date is the frontmatter's `date`, as YYYY-MM-DD.
export const blog = defineCollections({
  type: "doc",
  dir: "content/blog",
  schema: frontmatterSchema.extend({
    date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/),
  }),
})
