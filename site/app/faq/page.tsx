import type { Metadata } from "next"
import Link from "next/link"

import {
  PageHeader,
  PageHeaderDescription,
  PageHeaderHeading,
} from "@/components/page-header"

export const metadata: Metadata = {
  title: "FAQ",
  description:
    "Is rustx ready, which platforms work, does it need Node, how big is an application, and where the state lives. Answered from the repository, limits included.",
}

type Answer = { question: string; paragraphs: string[]; links?: [string, string][] }

const ANSWERS: Answer[] = [
  {
    question: "Is it ready for production?",
    paragraphs: [
      "Yep. But the version is 0.2.0 and the API may still change. Windows is the only platform written and measured. There is no installer, no code signing and no updater, and the framework neither produces nor checks any of them.",
      "What has been tested is a set of real applications, written from the outside against the framework's sources. The repository keeps two logs of what broke and what is still open. Two entries are open today, and both are stated by the compiler instead of hidden.",
      "The packaging page also says that nothing there has been tested on a second machine. Copy your release folder to a machine where the project does not exist before you ship it.",
    ],
    links: [
      ["Contribute", "/docs/tooling/contributing"],
      ["What's left", "/docs/whats-left"],
      ["Packaging", "/docs/tooling/packaging"],
      ["Changelog", "/changelog"],
    ],
  },
  {
    question: "Which platforms work?",
    paragraphs: [
      "Windows. It is written and measured, on real windows and real devices.",
      "macOS and Linux compile, and the tests compile all three targets on any machine. Each call that is not written refuses by naming the platform and the file to write, instead of doing nothing. The window controls, such as frameless and full screen, are the part most tied to Windows.",
    ],
    links: [["Platforms", "/docs/the-system/platforms"]],
  },
  {
    question: "Why not Tauri or Electron?",
    paragraphs: [
      "There is no benchmark here and no claim that rustx is better. The difference is where your code sits. In rustx, your Rust runs in the same process as the page's JavaScript. An onClick is a function call, not a serialized message, and props cross to a React component as V8 values.",
      "If you want an installer, signing and an updater today, or a platform other than Windows, those tools have them and rustx does not. The page is drawn by a browser engine, CEF, which is Chromium. Your application logic is Rust, with no Node process beside it.",
    ],
    links: [
      ["Where code runs", "/docs/getting-started/where-code-runs"],
      ["The Rust bridge", "/docs/javascript-libraries/rust-bridge"],
    ],
  },
  {
    question: "Does it need Node?",
    paragraphs: [
      "Not for an application that uses no npm library. You need Rust 1.88 or newer. CEF arrives with the cef crate on the first build.",
      "If you use React libraries, Recharts, Tailwind or shadcn, you need Node and npm, as any project that uses them does.",
    ],
    links: [["Installation", "/docs/getting-started/installation"]],
  },
  {
    question: "How big is a built application?",
    paragraphs: [
      "Large, because the browser engine ships beside the executable. In one debug build on the development machine, libcef.dll was 281 MB and the locales folder 51 MB. The repository does not give a measured size for a release build, so this page does not either.",
      "Release output is a folder of files and an .exe with a name, a version and an icon. The executable starts itself again as the engine's helper processes, so it must stay next to those files.",
    ],
    links: [["Packaging", "/docs/tooling/packaging"]],
  },
  {
    question: "How does it compare to writing React in a browser?",
    paragraphs: [
      "There is no server, no use client, no server rendering and no hydration. Everything you write runs on the machine of the person using the window.",
      "You write components in .rsx files: Rust, with markup in the return statements, in the syntax of JSX. React is there when you ask for it, to run npm libraries from tags in the same file. The rest of the page is Rust.",
      "The router reads an app/ folder, and has been measured against Next's. It has the query string, the catch-all segment, route groups, layouts, and loading and error screens per segment.",
    ],
    links: [
      ["The .rsx file", "/docs/getting-started/the-rsx-file"],
      ["The router", "/docs/the-router"],
    ],
  },
  {
    question: "Where does the state live?",
    paragraphs: [
      "In a let mut, and where you write it sets its scope. At the top of a file it is shared by the application, and it has a name, so it survives a rustx dev rebuild. Inside a component it is private, one cell per place in the tree, freed when the component leaves the screen.",
      "The Rust code and V8 live in the renderer process. A second window has its own renderer process, so it has its own state.",
    ],
    links: [
      ["State", "/docs/getting-started/state"],
      ["Where code runs", "/docs/getting-started/where-code-runs"],
    ],
  },
  {
    question: "Does it work with an editor?",
    paragraphs: [
      "Yes. rustx-lsp answers what an editor asks about a .rsx file, and a VS Code extension closes the tag when you type the closing angle bracket. The editor offers the props of npm tags and the components of your project.",
    ],
    links: [["The editor", "/docs/tooling/the-editor"]],
  },
]

export default function FaqPage() {
  return (
    <div className="flex flex-1 flex-col">
      <PageHeader>
        <PageHeaderHeading>Questions</PageHeaderHeading>
        <PageHeaderDescription>
          What a visitor asks first. The answers come from the repository, and
          the limits are part of the answers.
        </PageHeaderDescription>
      </PageHeader>

      <section className="border-t border-grid pb-16">
        <div className="">
          <div className=" grid gap-px bg-border/60">
            {ANSWERS.map((answer) => (
              <article
                key={answer.question}
                className="flex flex-col gap-3 bg-background p-6 lg:p-8"
              >
                <h2 className="text-lg font-semibold tracking-tight">
                  {answer.question}
                </h2>
                <div className="flex max-w-3xl flex-col gap-3 text-sm leading-relaxed">
                  {answer.paragraphs.map((paragraph) => (
                    <p key={paragraph}>{paragraph}</p>
                  ))}
                </div>
                {answer.links ? (
                  <ul className="flex flex-wrap gap-x-4 gap-y-1 text-sm">
                    {answer.links.map(([label, href]) => (
                      <li key={href}>
                        <Link
                          href={href}
                          className="text-muted-foreground underline underline-offset-4 hover:text-foreground"
                        >
                          {label}
                        </Link>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </article>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
