import type { Metadata } from "next"
import Link from "next/link"
import { notFound } from "next/navigation"

import { formatDate, getPosts } from "@/lib/blog"
import { absoluteUrl } from "@/lib/utils"
import { mdxComponents } from "@/mdx-components"

export const revalidate = false
export const dynamic = "force-static"
export const dynamicParams = false

export function generateStaticParams() {
  return getPosts().map((post) => ({ slug: post.slug }))
}

export async function generateMetadata(props: {
  params: Promise<{ slug: string }>
}): Promise<Metadata> {
  const { slug } = await props.params
  const post = getPosts().find((p) => p.slug === slug)

  if (!post) notFound()

  return {
    title: post.title,
    description: post.description,
    alternates: { canonical: `/blog/${post.slug}` },
    openGraph: {
      title: post.title,
      description: post.description,
      type: "article",
      publishedTime: post.date,
      url: absoluteUrl(`/blog/${post.slug}`),
      images: [
        {
          url: `/og?title=${encodeURIComponent(
            post.title
          )}&description=${encodeURIComponent(post.description ?? "")}`,
        },
      ],
    },
  }
}

export default async function PostPage(props: {
  params: Promise<{ slug: string }>
}) {
  const { slug } = await props.params
  const post = getPosts().find((p) => p.slug === slug)

  if (!post) notFound()

  const MDX = post.body

  return (
    <div className="flex flex-1 flex-col pb-8 text-[1.05rem] sm:text-[15px]">
      <div className="mx-auto flex w-full max-w-160 min-w-0 flex-1 flex-col gap-6 px-4 py-6 text-foreground md:px-0 lg:py-8">
        <div className="flex flex-col gap-2">
          <Link
            href="/blog"
            className="text-sm text-muted-foreground underline underline-offset-4 hover:text-foreground"
          >
            Blog
          </Link>
          <h1 className="scroll-m-24 text-3xl font-semibold tracking-tight">
            {post.title}
          </h1>
          <p className="font-mono text-xs text-muted-foreground">
            <time dateTime={post.date}>{formatDate(post.date)}</time>
          </p>
          {post.description ? (
            <p className="text-[1.05rem] text-muted-foreground sm:text-base sm:text-balance">
              {post.description}
            </p>
          ) : null}
        </div>
        <div className="typeset w-full flex-1 pb-16 sm:pb-0">
          <MDX components={mdxComponents} />
        </div>
      </div>
    </div>
  )
}
