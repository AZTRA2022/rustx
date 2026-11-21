// The code of the project, written as the project changes: the generated component for the
// character, its animation and its settings. Nothing here is asked of a model: it is a
// template filled from the project, and what is shown is what the project means.
import * as React from "react"
import { Copy, Maximize2, Minimize2, RotateCcw, WrapText } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { LIBRARY } from "./library"
import type { Project } from "./model"
import { copyText } from "./natives"

export function generate(p: Project, animations: string[]): string {
  const info = LIBRARY.find((m) => m.id === p.model) ?? LIBRARY[0]
  const speed = p.settings.mode === "Fun" ? 1.25 : p.settings.mode === "Cinematic" ? 0.7 : 1
  const lines = [
    `import { Character } from "./components/${info.id}-character";`,
    `import { Stage, Music } from "./components/stage";`,
    ``,
    `// ${p.ideas.trim().split("\n")[0] || "an idea goes here"}`,
    `export default function App() {`,
    `  return (`,
    `    <Stage light="${p.settings.light}" camera="${p.settings.camera}" quality="${p.settings.think}">`,
    `      <Character`,
    `        model="${info.url}"`,
    `        animation="${p.animation || animations[0] || "none"}"`,
    `        speed={${speed}}`,
    `        from={${p.clip[0].toFixed(1)}} to={${p.clip[1].toFixed(1)}}`,
    `      />`,
  ]
  if (p.settings.music !== "Off") lines.push(`      <Music kind="${p.settings.music}" from={${p.audio[0].toFixed(1)}} to={${p.audio[1].toFixed(1)}} />`)
  lines.push(`    </Stage>`, `  );`, `}`)
  return lines.join("\n")
}

const KEYWORDS = new Set(["import", "from", "export", "default", "function", "return", "const"])

// A highlighter small enough to read: strings, comments, keywords, JSX names, numbers.
function highlight(line: string) {
  const out: React.ReactNode[] = []
  const re = /(\/\/.*$)|("[^"]*")|(<\/?[A-Z][A-Za-z]*)|(\b[A-Za-z_]+\b)|(\d+(?:\.\d+)?)|([^\w"<]+|<|"|\w)/g
  let m: RegExpExecArray | null
  let i = 0
  while ((m = re.exec(line))) {
    const [t, comment, str, tag, word, num] = m
    const tone = comment ? "text-zinc-500" : str ? "text-fuchsia-300" : tag ? "text-sky-300" : word && KEYWORDS.has(word) ? "text-rose-300" : word && /^[A-Z]/.test(word) ? "text-blue-300" : num ? "text-amber-300" : "text-zinc-200"
    out.push(
      <span key={i++} className={tone}>
        {t}
      </span>,
    )
  }
  return out
}

function Tip({ label, children }: { label: string; children: React.ReactElement }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

export default function CodePanel({ code, ms, expanded, onExpand, onReset }: { code: string; ms: number; expanded: boolean; onExpand: () => void; onReset: () => void }) {
  const [wrap, setWrap] = React.useState(false)
  const [copied, setCopied] = React.useState(false)
  const lines = code.split("\n")
  const icon = "size-9 text-muted hover:bg-accent hover:text-white"
  return (
    <div className="flex h-full flex-col overflow-hidden rounded-2xl border border-line bg-panel">
      <div className="flex items-center justify-between border-b border-line px-[18px] py-3 text-[15px] text-zinc-300">
        <div className="flex items-center gap-3">
          <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden="true">
            {[0, 1, 2, 3].map((r) => [0, 1, 2, 3].map((c) => <circle key={`${r}${c}`} cx={4 + c * 4.6} cy={4 + r * 4.6} r="1.5" className={["fill-red-400", "fill-amber-400", "fill-green-400", "fill-blue-400"][(r + c) % 4]} />))}
          </svg>
          Generated in {ms} ms
        </div>
        <div className="flex gap-1">
          <Tip label="Copy the code">
            <Button aria-label="Copy the code" variant="ghost" size="icon" className={icon} onClick={() => { copyText(code); setCopied(true); setTimeout(() => setCopied(false), 1200) }}>
              <Copy className={cn(copied && "text-green-400")} />
            </Button>
          </Tip>
          <Tip label="Wrap lines">
            <Button aria-label="Wrap lines" variant="ghost" size="icon" className={cn(icon, wrap && "text-white")} onClick={() => setWrap(!wrap)}>
              <WrapText />
            </Button>
          </Tip>
          <Tip label="Regenerate">
            <Button aria-label="Regenerate" variant="ghost" size="icon" className={icon} onClick={onReset}>
              <RotateCcw />
            </Button>
          </Tip>
          <Tip label={expanded ? "Shrink" : "Expand"}>
            <Button aria-label={expanded ? "Shrink the code" : "Expand the code"} variant="ghost" size="icon" className={icon} onClick={onExpand}>
              {expanded ? <Minimize2 /> : <Maximize2 />}
            </Button>
          </Tip>
        </div>
      </div>
      <div className="flex-1 overflow-auto py-3.5 font-mono text-sm leading-[26px]">
        {lines.map((line, i) => (
          <div key={i} className={cn("flex", wrap ? "whitespace-pre-wrap" : "whitespace-pre")}>
            <span className="w-[52px] flex-none pr-[18px] text-right text-faint">{i + 1}</span>
            <span>{highlight(line)}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
