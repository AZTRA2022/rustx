import Link from "next/link"

import { siteConfig } from "@/lib/config"
import { highlightCode } from "@/lib/highlight-code"
import { CopyButton } from "@/components/copy-button"
import {
  PageActions,
  PageHeader,
  PageHeaderDescription,
  PageHeaderHeading,
} from "@/components/page-header"
import { Button } from "@/components/ui/button"

const EXEMPLE = `let mut order = vec!["a".to_string(), "b".to_string(), "c".to_string()];

fn Page() -> Element {
    return <div>
        <div id="first">{Counter()}</div>
        <div id="second">{Counter()}</div>

        <ul id="rows">
            {order.iter().map(|key| <li key={key.clone()}>{Counter()}</li>).collect::<Vec<_>>()}
        </ul>

        <button id="rotate" onClick={move |_| order.rotate_right(1)}>rotate</button>
    </div>;
}

/// One \`let mut n = 0;\`, as many counters as there are places it is
/// written — and as many as there are keys in a list.
fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={move |_| n += 1}>{n}</button>;
}`

const INSTALLER = `cargo install rustx-cli

rustx new my-app
cd my-app
rustx dev`

const ARGUMENTS = [
  {
    titre: "One process",
    texte:
      "A Chromium window and the Rust in the same process: an onClick is a function call, not a serialized message, and props cross the boundary as V8 values.",
  },
  {
    titre: "Zero lines of JavaScript",
    texte:
      "The framework compiles the .rsx files, writes the route table, builds the package and the style sheet. There is no build.rs, and nothing to configure.",
  },
  {
    titre: "State as let mut",
    texte:
      "At the top of a file, it is shared by the application; inside a component, it is one cell per place in the tree. No annotation, no macro, no store::.",
  },
  {
    titre: "npm libraries from tags",
    texte:
      "A package.json, an npm install, and a capitalized tag is resolved at runtime. React, recharts, shadcn, Tailwind — with no prefix to write.",
  },
]

export default async function HomePage() {
  const [example, install] = await Promise.all([
    highlightCode(EXEMPLE, "rust"),
    highlightCode(INSTALLER, "bash"),
  ])

  return (
    <div className="flex flex-1 flex-col">
      <PageHeader>
        <span className="rounded-full border border-border bg-surface px-3 py-1 font-mono text-xs text-muted-foreground">
          0.2.0 · Windows
        </span>
        <PageHeaderHeading>
          Native software, written in Rust, drawn by a browser engine.
        </PageHeaderHeading>
        <PageHeaderDescription>
          Pages in <code className="font-mono">.rsx</code> — the syntax of
          JSX, the language of Rust —, a state made of{" "}
          <code className="font-mono">let mut</code>, routes by folders, and
          npm libraries usable from tags. There is no JavaScript to write.
        </PageHeaderDescription>
        <PageActions>
          <Button asChild size="sm">
            <Link href="/docs/getting-started/installation">Get started</Link>
          </Button>
          <Button asChild size="sm" variant="ghost">
            <Link href="/showcase">See the showcase</Link>
          </Button>
        </PageActions>
      </PageHeader>

      <section className="border-t border-grid">
        <div className="container-wrapper">
          <div className="container grid gap-px bg-border/60 py-0 md:grid-cols-2 xl:grid-cols-4">
            {ARGUMENTS.map((argument) => (
              <div
                key={argument.titre}
                className="flex flex-col gap-2 bg-background p-6"
              >
                <h2 className="font-mono text-sm font-medium tracking-tight">
                  {argument.titre}
                </h2>
                <p className="text-sm text-muted-foreground">
                  {argument.texte}
                </p>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="border-t border-grid py-12 lg:py-16">
        <div className="container-wrapper">
          <div className="container grid gap-10 lg:grid-cols-[minmax(0,1fr)_20rem]">
            <div className="flex min-w-0 flex-col gap-4">
              <div className="flex flex-col gap-1">
                <h2 className="text-xl font-semibold tracking-tight">
                  A whole page
                </h2>
                <p className="text-sm text-muted-foreground">
                  <code className="font-mono">
                    apps/counter/app/counter/page.rsx
                  </code>{" "}
                  from the repository, as it is. Two counters side by side come
                  from a single line, and the rows of the list each keep their
                  own through a rotation.
                </p>
              </div>
              <figure className="relative min-w-0 overflow-hidden rounded-2xl bg-code text-sm text-code-foreground">
                <CopyButton value={EXEMPLE} />
                <div
                  className="min-w-0 [&_pre]:max-h-[32rem]"
                  dangerouslySetInnerHTML={{ __html: example }}
                />
              </figure>
            </div>

            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-1">
                <h2 className="text-xl font-semibold tracking-tight">
                  Install
                </h2>
                <p className="text-sm text-muted-foreground">
                  Rust 1.88 or later, and nothing else: CEF arrives at the first
                  build.
                </p>
              </div>
              <figure className="relative overflow-hidden rounded-2xl bg-code text-sm text-code-foreground">
                <CopyButton value={INSTALLER} />
                <div dangerouslySetInnerHTML={{ __html: install }} />
              </figure>
              <p className="text-sm text-muted-foreground">
                The package is called{" "}
                <Link
                  href={siteConfig.links.crates}
                  target="_blank"
                  rel="noreferrer"
                  className="font-medium underline underline-offset-4"
                >
                  rustx-app
                </Link>{" "}
                on crates.io, because <code className="font-mono">rustx</code>{" "}
                is taken there by another project; the library is called{" "}
                <code className="font-mono">rustx</code>, so all the code is
                written <code className="font-mono">rustx::</code>.
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="border-t border-grid py-12">
        <div className="container-wrapper">
          <div className="container flex flex-col items-start gap-4">
            <h2 className="text-xl font-semibold tracking-tight">
              What the web cannot do
            </h2>
            <p className="max-w-2xl text-sm text-muted-foreground">
              A page cannot read the clipboard when it wants to, nor open a
              dialog that returns a path, nor place an icon next to the clock.
              This is what{" "}
              <code className="font-mono">rustx-os</code> does, and it is
              written in a page like the rest: each module answers{" "}
              <code className="font-mono">supported()</code> before anything is
              asked of it, and each thing registered with the system is a token
              that removes it when released.
            </p>
            <Button asChild size="sm" variant="secondary">
              <Link href="/docs/the-system">
                Clipboard, dialogs, tray, shortcuts
              </Link>
            </Button>
          </div>
        </div>
      </section>
    </div>
  )
}
