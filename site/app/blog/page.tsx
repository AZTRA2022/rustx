import type { Metadata } from "next"
import Link from "next/link"

import { formatDate, getPosts } from "@/lib/blog"
import {
  PageHeader,
  PageHeaderDescription,
  PageHeaderHeading,
} from "@/components/page-header"

export const metadata: Metadata = {
  title: "Blog",
  description: "Posts about the rustx releases and the work behind them.",
}

export default function BlogPage() {
  const posts = getPosts()

  return (
    <div className="flex flex-1 flex-col">
      <PageHeader>
        <PageHeaderHeading>Blog</PageHeaderHeading>
        <PageHeaderDescription>
          What was done in each release, and why. The short list of changes is
          in the{" "}
          <Link href="/changelog" className="underline underline-offset-4">
            changelog
          </Link>
          .
        </PageHeaderDescription>
      </PageHeader>

      <section className="border-t border-grid pb-16">
        <div className="container-wrapper">
          <div className="container grid gap-px bg-border/60">
            {posts.map((post) => (
              <article
                key={post.slug}
                className="flex flex-col gap-2 bg-background p-6 lg:p-8"
              >
                <p className="font-mono text-xs text-muted-foreground">
                  <time dateTime={post.date}>{formatDate(post.date)}</time>
                </p>
                <h2 className="text-lg font-semibold tracking-tight">
                  <Link
                    href={`/blog/${post.slug}`}
                    className="underline-offset-4 hover:underline"
                  >
                    {post.title}
                  </Link>
                </h2>
                {post.description ? (
                  <p className="max-w-3xl text-sm leading-relaxed text-muted-foreground">
                    {post.description}
                  </p>
                ) : null}
              </article>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
