import type { Metadata } from "next"

import {
  PageHeader,
  PageHeaderDescription,
  PageHeaderHeading,
} from "@/components/page-header"

export const metadata: Metadata = {
  title: "Showcase",
  description:
    "The applications of the repository: each exists to prove one thing, and says which.",
}

type Vitrine = {
  nom: string
  chemin: string
  quoi: string
  preuve: string
  mots: string[]
}

const APPLICATIONS: Vitrine[] = [
  {
    nom: "hybrid",
    chemin: "apps/hybrid",
    quoi: "A page where Rust and JavaScript share a single DOM: a recharts chart whose points are a `let mut` of the file, inside a shadcn `Card` installed by `npx shadcn add`.",
    preuve:
      "Its `main` is `async` and waits before there is a window, with no `#[tokio::main]` beside it; and an input field placed inside a React subtree gives the same event as a tag that the engine mounts itself.",
    mots: ["recharts", "shadcn", "async fn main", "loading.rsx"],
  },
  {
    nom: "catalogue",
    chemin: "apps/catalogue",
    quoi: "A list, a button and an area to drop files on — the interface is `app/page.rsx` and nowhere else.",
    preuve:
      "What a web page receives when a file is dropped on it is an object without a path; what `rustx::dropped` gives is the real path, the one you can reopen tomorrow.",
    mots: ["dropped::on_files", "effect", "a single page"],
  },
  {
    nom: "counter",
    chemin: "apps/counter",
    quoi: "The counter from the design document, written in `.rsx`: a home page, and the counter itself at `/counter`.",
    preuve:
      "A single `let mut n = 0;` gives two counters side by side, and the rows of a keyed list each keep their own count through a reordering — a cell's identity is its place in the tree.",
    mots: ["let mut", "component cells", "keyed lists"],
  },
  {
    nom: "monitor",
    chemin: "apps/monitor",
    quoi: "A monitor fed by a background thread: `let mut ticks = 0;` at the top of the page is a value that two components read.",
    preuve:
      "The thread that pushes it is a `task` that the panel keeps, so leaving the page stops it without a cleanup written anywhere; and a button opens a second window on the `/pause` route.",
    mots: ["task", "alive.post", "window::open"],
  },
  {
    nom: "zoo",
    chemin: "apps/zoo",
    quoi: "Many libraries, one page: the lucide-react icons, a markdown render whose content is its children, and Radix tabs that keep a state of their own around a state that rustx keeps.",
    preuve:
      "This application exists to find what the boundary cannot yet do — an `onValueChange` whose closure stays in Rust while what crosses is a function that calls it. What does not work is written, not hidden.",
    mots: ["lucide-react", "react-markdown", "radix-ui", "callbacks"],
  },
  {
    nom: "capture",
    chemin: "apps/capture",
    quoi: "Two screens, a layout that outlives both of them, and a flow that closes itself: the list of devices at `/`, the preview of one of them at `/device/[id]`.",
    preuve:
      "Nothing here mounts or unmounts a screen: opening a device is `navigate`, going back is `back`. The preview's stream is a kept value, so leaving the screen closes it, and the thread of the bar belongs to the layout, so it keeps going while the screens change under it.",
    mots: ["layout.rsx", "[id]", "keep", "navigate / back"],
  },
  {
    nom: "labo-rustx",
    chemin: "a separate repository",
    quoi: "A GitHub repository browser, written from the outside: nested routes, parameters, a command palette, search with *debounce* on a real HTTPS API, concurrent tasks, recharts graphs, a TipTap editor, native clipboard and dialogs.",
    preuve:
      "Nothing in it was written to please the framework: every line is what a developer writes first. This is where the `docs/manques-du-framework.md` log comes from. Its window is frameless, and the top bar is its own.",
    mots: ["frameless()", "reqwest", "TipTap", "not_found"],
  },
]

export default function ShowcasePage() {
  return (
    <div className="flex flex-1 flex-col">
      <PageHeader>
        <PageHeaderHeading>The showcase</PageHeaderHeading>
        <PageHeaderDescription>
          The applications of the repository serve as proof for the plans. Each
          exists for a specific reason, and says which. No screenshots here:
          what matters is what each one demonstrates.
        </PageHeaderDescription>
      </PageHeader>

      <section className="border-t border-grid pb-16">
        <div className="">
          <div className=" grid gap-px bg-border/60 lg:grid-cols-2">
            {APPLICATIONS.map((application) => (
              <article
                key={application.nom}
                className="flex flex-col gap-4 bg-background p-6 lg:p-8"
              >
                <header className="flex flex-col gap-1">
                  <h2 className="font-mono text-lg font-semibold tracking-tight">
                    {application.nom}
                  </h2>
                  <p className="font-mono text-xs text-muted-foreground">
                    {application.chemin}
                  </p>
                </header>
                <p className="text-sm leading-relaxed">{application.quoi}</p>
                <p className="border-l-2 border-border pl-4 text-sm leading-relaxed text-muted-foreground">
                  {application.preuve}
                </p>
                <ul className="mt-auto flex flex-wrap gap-1.5 pt-2">
                  {application.mots.map((mot) => (
                    <li
                      key={mot}
                      className="rounded-full border border-border bg-surface px-2.5 py-0.5 font-mono text-[0.7rem] text-muted-foreground"
                    >
                      {mot}
                    </li>
                  ))}
                </ul>
              </article>
            ))}
          </div>
        </div>
      </section>
    </div>
  )
}
