import type { Metadata } from "next"
import Link from "next/link"

import {
  PageHeader,
  PageHeaderDescription,
  PageHeaderHeading,
} from "@/components/page-header"

export const metadata: Metadata = {
  title: "Changelog",
  description:
    "What changed in each rustx release, newest first. Taken from the repository's logs and history.",
}

type Item = { text: string; href?: string }
type Group = { title: string; items: Item[] }
type Release = {
  version: string
  date: string
  summary: string
  groups: Group[]
}

const RELEASES: Release[] = [
  {
    version: "0.2.0",
    date: "2026-11-06 to 2026-11-07",
    summary:
      "The current workspace version. Two nights of writing real applications from the outside, against the framework's own sources, and fixing what broke. The bug hunt closed 41 entries. The log of gaps closed 32 of its 34, and two remain open.",
    groups: [
      {
        title: "Async",
        items: [
          {
            text: "`async fn main` works, and the window opens without waiting for it.",
            href: "/docs/async/async-fn-main",
          },
          {
            text: "A component can wait for a value with `.await`. The page draws without it first, and the frame after the value arrives shows it.",
            href: "/docs/async/awaiting-a-value",
          },
          {
            text: "A future runs beside the window and stops when its component leaves the tree. Work after an `await` goes with the future and no longer blocks drawing.",
            href: "/docs/async/cancellation",
          },
          {
            text: "A value of the page can be written from a future, and read from one. Both land on a frame.",
            href: "/docs/async/reading-and-writing-state",
          },
          {
            text: "`task(...)` around an `async` block no longer says nothing, and the path `rustx::tokio::`, which nobody writes, is no longer the way in.",
            href: "/docs/async",
          },
        ],
      },
      {
        title: "The router",
        items: [
          {
            text: "The router was measured against Next's and what it lacked was written. First, the query string.",
            href: "/docs/the-router/the-query-string",
          },
          {
            text: "The catch-all segment `[...path]` and the route groups `(section)`.",
            href: "/docs/the-router/routes-from-folders",
          },
          {
            text: "`forward` and `replace`, next to `navigate` and `back`, and a history stack the page can read.",
            href: "/docs/the-router/navigating",
          },
          {
            text: "A layout receives the parameters of the route.",
            href: "/docs/the-router/layouts",
          },
          {
            text: "`loading.rsx` and `error.rsx`, per segment.",
            href: "/docs/the-router/loading-and-errors",
          },
        ],
      },
      {
        title: "The React boundary",
        items: [
          {
            text: "An `Element` can be the child of a React component.",
            href: "/docs/javascript-libraries/props-that-cross",
          },
          {
            text: "`Event` carries the typed value, the key, the target and the modifiers, including on a tag written inside a React subtree. `Value` has accessors. A callback receives all of its arguments, not only the first.",
            href: "/docs/javascript-libraries/rust-bridge",
          },
          {
            text: "A project that follows the getting-started pages of Tailwind, shadcn and TipTap builds. Before, `@tailwindcss/cli` went into the browser bundle, a package with no root export brought the whole build down, and components outside `components/ui` were left out of the bundle.",
            href: "/docs/javascript-libraries",
          },
        ],
      },
      {
        title: "The window",
        items: [
          {
            text: "`toggle_maximize()` and `toggle_fullscreen()`, because a title bar has one button that toggles, not two. The place from before full screen is kept and restored.",
            href: "/docs/the-window/commanding-the-window",
          },
          {
            text: "A frameless window no longer shows seven extra pixels at the top. Windows painted them; `WM_NCCALCSIZE` now takes them back and the resize edges stay.",
            href: "/docs/the-window/frameless",
          },
          {
            text: "An `<input>` given only a `value` attribute no longer keeps its text when the page empties it.",
          },
        ],
      },
      {
        title: "The compiler",
        items: [
          {
            text: "Comments in markup are comments. Before, they were drawn on screen. `//` at the start of a line, `/* ... */` and `{/* ... */}` are read and dropped, between children and between attributes. `see https://example.com` stays a sentence.",
            href: "/docs/getting-started/the-rsx-file",
          },
          {
            text: "A non-English comment no longer makes the compiler panic.",
          },
          {
            text: "About a dozen rewrites that emitted wrong Rust were fixed, each with the test that found it: a write inside a `match` arm swallowing the arms after it, a lost type annotation on a page value, a method that mutates and returns a value, a `format!(\"{x}\")` capture, a plain function next to a page that could not reach its values, a name bound by `let` rewritten as a read.",
          },
          {
            text: "A `let mut` that is written and never read no longer produces invalid Rust. A `let mut` inside an `async` block is a local, not a cell.",
            href: "/docs/getting-started/state",
          },
          {
            text: "A refusal is a sentence. Three of them used to arrive with a wall of spaces, and a refused page used to add an `error[E0752]` on `src/main.rs`.",
          },
          {
            text: "The rewrite pass runs when it is asked for, not ahead of time.",
          },
        ],
      },
      {
        title: "The commands and the editor",
        items: [
          {
            text: "A new project compiles. It did not.",
            href: "/docs/getting-started/installation",
          },
          {
            text: "`rustx dev` no longer refuses its own rebuild on every save, and no longer feeds itself: one save used to start rebuilds without end.",
            href: "/docs/tooling/the-commands",
          },
          {
            text: "The editor offers the props of npm tags, with the right icon, and the project's own components outside `components/ui`. Typing `>` closes the tag. A formatter was added, and lines the rewrite rebuilt now get an answer.",
            href: "/docs/tooling/the-editor",
          },
        ],
      },
      {
        title: "Tests",
        items: [
          {
            text: "The compiler's tests hold every page of the repository to a set of laws, and the README's examples are compiled to check that what they produce is Rust that holds.",
          },
          {
            text: "A project written from outside ends with a test that opens a real window and reads what it draws: a shadcn `Card` mounted by React, Tailwind applied, a live TipTap, an `onClick` in Rust called from inside React.",
          },
        ],
      },
      {
        title: "Still open",
        items: [
          {
            text: "An `async fn` written beside a page cannot reach a value of the page. The compiler refuses it and says where to declare the value.",
            href: "/docs/whats-left",
          },
          {
            text: "A component `.await` that borrows a local does not compile, and the message comes from `rustc`, against a generated file.",
            href: "/docs/whats-left",
          },
        ],
      },
    ],
  },
  {
    version: "0.1.1",
    date: "2026-11-01",
    summary: "The first published release.",
    groups: [
      {
        title: "What shipped",
        items: [
          {
            text: "Eight packages on crates.io. An application names one of them, `rustx-app`, and writes `rustx::`.",
            href: "/docs/tooling/the-packages",
          },
          {
            text: "The minimum Rust version is 1.88, measured and not announced.",
            href: "/docs/getting-started/installation",
          },
          {
            text: "The README is the complete manual: install, project, the `.rsx` file, state, routes, JavaScript libraries, the window, threads, commands, the editor.",
          },
          {
            text: "Windows written and measured. macOS and Linux compile and refuse by naming the file that is missing.",
            href: "/docs/the-system/platforms",
          },
        ],
      },
    ],
  },
]

/** `code` spans in the data, rendered as code. */
function Text({ children }: { children: string }) {
  return (
    <>
      {children.split(/(`[^`]+`)/g).map((part, index) =>
        part.startsWith("`") ? (
          <code
            key={index}
            className="rounded border border-border bg-surface px-1 py-0.5 font-mono text-[0.8em]"
          >
            {part.slice(1, -1)}
          </code>
        ) : (
          part
        )
      )}
    </>
  )
}

export default function ChangelogPage() {
  return (
    <div className="flex flex-1 flex-col">
      <PageHeader>
        <PageHeaderHeading>Changelog</PageHeaderHeading>
        <PageHeaderDescription>
          What changed in each release, newest first. It is written from the
          repository&apos;s two logs, the bug hunt and the list of gaps, and
          from its history. What is still open is on the{" "}
          <Link href="/docs/whats-left" className="underline underline-offset-4">
            what&apos;s left
          </Link>{" "}
          page.
        </PageHeaderDescription>
      </PageHeader>

      <section className="border-t border-grid pb-16">
        <div className="">
          <div className=" grid gap-px bg-border/60">
            {RELEASES.map((release) => (
              <article
                key={release.version}
                id={release.version}
                className="flex flex-col gap-6 bg-background p-6 lg:p-8"
              >
                <header className="flex flex-col gap-2">
                  <h2 className="font-mono text-2xl font-semibold tracking-tight">
                    {release.version}
                  </h2>
                  <p className="font-mono text-xs text-muted-foreground">
                    {release.date}
                  </p>
                  <p className="max-w-3xl text-sm leading-relaxed">
                    {release.summary}
                  </p>
                </header>
                {release.groups.map((group) => (
                  <div key={group.title} className="flex flex-col gap-2">
                    <h3 className="text-sm font-semibold">{group.title}</h3>
                    <ul className="flex max-w-3xl list-disc flex-col gap-2 pl-5 text-sm leading-relaxed">
                      {group.items.map((item) => (
                        <li key={item.text}>
                          <Text>{item.text}</Text>
                          {item.href ? (
                            <>
                              {" "}
                              <Link
                                href={item.href}
                                className="whitespace-nowrap text-muted-foreground underline underline-offset-4 hover:text-foreground"
                              >
                                Read more
                              </Link>
                            </>
                          ) : null}
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </article>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
