// The node graph: IDEAS and REFERENCES feed SETTINGS, SETTINGS feeds AI MODELS.
// Nodes move by their header, wires are made by dragging from an output port to an
// input, a wire is removed with a double click, and the canvas pans and zooms.
// Every look is a Tailwind class and every control a shadcn component; only the numbers
// that exist at run time (a position) go through `Box`.
import * as React from "react"
import { ChevronDown, CircleDashed, GripVertical, ImagePlus, Plus, Sparkles, X } from "lucide-react"
import { Box } from "@/lib/geometry"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Textarea } from "@/components/ui/textarea"
import { SETTING_DEFS, clamp, type GraphNode, type Link, type NodeKind, type Project } from "./model"

const W = 256
const ACCEPTS: Record<NodeKind, NodeKind[]> = { ideas: [], refs: [], settings: ["ideas", "refs"], models: ["settings"] }
const WIRE: Record<NodeKind, [string, string]> = {
  ideas: ["#fbbf24", "#c4b5fd"],
  refs: ["#38bdf8", "#f0abfc"],
  settings: ["#a78bfa", "#60a5fa"],
  models: ["#ffffff", "#ffffff"],
}
const PORT: Record<NodeKind, string> = { ideas: "bg-amber-400", refs: "bg-sky-400", settings: "bg-violet-400", models: "bg-white" }
const MODELS = ["Seedance2", "Veo 3", "Kling 2", "Sora"]

type Props = {
  project: Project
  snap: boolean
  onChange: (patch: Partial<Project>, commit: boolean) => void
  onPickImage: () => void
  // Changes when the cards are arranged again, so the view fits them again.
  arrangeKey: number
}

export default function NodeCanvas({ project, snap, onChange, onPickImage, arrangeKey }: Props) {
  const [view, setView] = React.useState({ x: 0, y: 0, k: 1 })
  const [live, setLive] = React.useState<Record<string, { x: number; y: number }>>({})
  const [wire, setWire] = React.useState<{ from: NodeKind; x: number; y: number } | null>(null)
  const box = React.useRef<HTMLDivElement>(null)
  const els = React.useRef<Record<string, HTMLDivElement | null>>({})
  const [sizes, setSizes] = React.useState<Record<string, number>>({})

  // The height of each card, to put its ports in the middle.
  React.useLayoutEffect(() => {
    const next: Record<string, number> = {}
    for (const n of project.nodes) next[n.id] = els.current[n.id]?.offsetHeight ?? 0
    if (JSON.stringify(next) !== JSON.stringify(sizes)) setSizes(next)
  })

  // Until the person pans or zooms, the view fits all the cards in the window.
  const touched = React.useRef(false)
  const [frame, setFrame] = React.useState({ w: 0, h: 0 })
  React.useEffect(() => {
    touched.current = false
  }, [arrangeKey, project.id])
  React.useEffect(() => {
    const el = box.current
    if (!el) return
    const watch = new ResizeObserver(() => setFrame({ w: el.clientWidth, h: el.clientHeight }))
    watch.observe(el)
    return () => watch.disconnect()
  }, [])
  React.useEffect(() => {
    if (touched.current || !frame.w || !sizes.settings) return
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
    for (const n of project.nodes) {
      x0 = Math.min(x0, n.x)
      y0 = Math.min(y0, n.y)
      x1 = Math.max(x1, n.x + W)
      y1 = Math.max(y1, n.y + (sizes[n.id] || 200))
    }
    const k = clamp(Math.min((frame.w - 40) / (x1 - x0), (frame.h - 40) / (y1 - y0)), 0.4, 1)
    setView({ k, x: (frame.w - (x1 - x0) * k) / 2 - x0 * k, y: (frame.h - (y1 - y0) * k) / 2 - y0 * k })
  }, [frame, sizes, project.nodes, arrangeKey])

  // The AI MODELS card never sits on the SETTINGS card, however many rows that one has grown.
  const pos = (n: GraphNode) => {
    const p = live[n.id] ?? { x: n.x, y: n.y }
    if (n.id !== "models") return p
    const s = project.nodes.find((x) => x.id === "settings")
    const sp = s ? (live.settings ?? { x: s.x, y: s.y }) : null
    return sp && sizes.settings ? { x: p.x, y: Math.max(p.y, sp.y + sizes.settings + 56) } : p
  }
  const node = (id: NodeKind) => project.nodes.find((n) => n.id === id)!
  const outPort = (id: NodeKind) => {
    const p = pos(node(id))
    const h = sizes[id] || 120
    return { x: p.x + W, y: p.y + (id === "ideas" ? h * 0.42 : h * 0.5) }
  }
  const inPort = (id: NodeKind, from: NodeKind) => {
    const p = pos(node(id))
    const h = sizes[id] || 120
    const into = project.links.filter((l) => l.to === id && l.from !== "settings")
    const order = into.findIndex((l) => l.from === from)
    return { x: p.x, y: p.y + h / 2 + (order - (into.length - 1) / 2) * 50 }
  }
  const toWorld = (cx: number, cy: number) => {
    const r = box.current!.getBoundingClientRect()
    return { x: (cx - r.left - view.x) / view.k, y: (cy - r.top - view.y) / view.k }
  }

  // Dragging: a card by its header, the canvas by its background, a wire by its port.
  const drag = React.useRef<null | { kind: "node" | "pan" | "wire"; id?: NodeKind; ox: number; oy: number; sx: number; sy: number }>(null)

  React.useEffect(() => {
    const move = (e: PointerEvent) => {
      const d = drag.current
      if (!d) return
      if (d.kind === "pan") setView((v) => ({ ...v, x: d.ox + e.clientX - d.sx, y: d.oy + e.clientY - d.sy }))
      else if (d.kind === "node" && d.id) {
        let x = d.ox + (e.clientX - d.sx) / view.k
        let y = d.oy + (e.clientY - d.sy) / view.k
        if (snap) {
          x = Math.round(x / 24) * 24
          y = Math.round(y / 24) * 24
        }
        setLive((l) => ({ ...l, [d.id!]: { x, y } }))
      } else if (d.kind === "wire") {
        const w = toWorld(e.clientX, e.clientY)
        setWire({ from: d.id!, x: w.x, y: w.y })
      }
    }
    const up = (e: PointerEvent) => {
      const d = drag.current
      drag.current = null
      if (!d) return
      if (d.kind === "node" && d.id && live[d.id]) {
        const l = live[d.id]
        onChange({ nodes: project.nodes.map((n) => (n.id === d.id ? { ...n, x: l.x, y: l.y } : n)) }, true)
        setLive((s) => {
          const c = { ...s }
          delete c[d.id!]
          return c
        })
      }
      if (d.kind === "wire" && d.id) {
        const w = toWorld(e.clientX, e.clientY)
        const target = project.nodes.find((n) => {
          const p = pos(n)
          const h = sizes[n.id] || 120
          return w.x >= p.x - 12 && w.x <= p.x + W + 12 && w.y >= p.y && w.y <= p.y + h && ACCEPTS[n.id].includes(d.id!)
        })
        if (target && !project.links.some((l) => l.from === d.id && l.to === target.id)) {
          onChange({ links: [...project.links, { from: d.id, to: target.id } as Link] }, true)
        }
        setWire(null)
      }
    }
    window.addEventListener("pointermove", move)
    window.addEventListener("pointerup", up)
    return () => {
      window.removeEventListener("pointermove", move)
      window.removeEventListener("pointerup", up)
    }
  })

  const onWheel = (e: React.WheelEvent) => {
    touched.current = true
    const k = clamp(view.k * (e.deltaY < 0 ? 1.08 : 0.93), 0.4, 1.8)
    const r = box.current!.getBoundingClientRect()
    const mx = e.clientX - r.left
    const my = e.clientY - r.top
    setView({ k, x: mx - ((mx - view.x) / view.k) * k, y: my - ((my - view.y) / view.k) * k })
  }

  const startNode = (e: React.PointerEvent, id: NodeKind) => {
    e.stopPropagation()
    touched.current = true
    const p = pos(node(id))
    drag.current = { kind: "node", id, ox: p.x, oy: p.y, sx: e.clientX, sy: e.clientY }
  }

  const curve = (a: { x: number; y: number }, b: { x: number; y: number }) => {
    const dx = Math.max(60, Math.abs(b.x - a.x) * 0.5)
    return `M ${a.x} ${a.y} C ${a.x + dx} ${a.y}, ${b.x - dx} ${b.y}, ${b.x} ${b.y}`
  }

  const stop = (e: React.PointerEvent) => e.stopPropagation()

  // ---- the cards
  const card = (id: NodeKind, title: string, right: React.ReactNode, body: React.ReactNode, glow?: boolean) => {
    const p = pos(node(id))
    return (
      <Box
        key={id}
        css={{ left: `${p.x}px`, top: `${p.y}px` }}
        ref={(el) => void (els.current[id] = el)}
        data-node={id}
        className={cn("absolute w-64 select-none rounded-2xl border bg-card shadow-2xl", glow ? "border-teal-300/50 shadow-teal-300/10" : "border-line")}
      >
        <div onPointerDown={(e) => startNode(e, id)} className="flex cursor-grab items-center justify-between border-b border-line px-[18px] py-3.5 font-mono text-[11px] tracking-[0.14em] text-text">
          <span>{title}</span>
          <span onPointerDown={stop} className="flex items-center gap-1.5 text-muted">
            {right}
          </span>
        </div>
        {body}
        <div className="flex justify-center gap-[3px] pb-2 pt-1">
          {[0, 1, 2].map((i) => (
            <span key={i} className="size-[3px] rounded-full bg-faint" />
          ))}
        </div>
      </Box>
    )
  }

  const remaining = SETTING_DEFS.filter((d) => !project.settingIds.includes(d.id))
  const addSetting = (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button aria-label="Add a setting" variant="ghost" size="icon" disabled={!remaining.length} className="size-6 text-text hover:bg-transparent">
          <Plus />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {remaining.map((d) => (
          <DropdownMenuItem key={d.id} onSelect={() => onChange({ settingIds: [...project.settingIds, d.id] }, true)}>
            {d.label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )

  const ideas = card(
    "ideas",
    "IDEAS",
    <CircleDashed size={15} />,
    <Textarea
      value={project.ideas}
      placeholder="Describe what to make…"
      onChange={(e) => onChange({ ideas: e.target.value }, false)}
      onBlur={() => onChange({}, true)}
      onPointerDown={stop}
      className="block h-48 px-[18px] py-4 font-mono text-[13px] leading-[26px] text-text"
    />,
  )

  const refs = card(
    "refs",
    "REFERENCES",
    <span className={cn("size-[9px] rounded-full", project.ref ? "bg-green-500 shadow-[0_0_8px_#22c55e]" : "bg-faint")} />,
    <div className="p-2.5">
      {project.ref ? (
        <div className="relative h-56 overflow-hidden rounded-2xl bg-well">
          <img src={project.ref.url} alt={project.ref.name} draggable={false} className="size-full object-cover" />
          <Button aria-label="Remove the reference" variant="secondary" size="icon" onPointerDown={stop} onClick={() => onChange({ ref: null }, true)} className="absolute right-2.5 top-2.5 size-9 rounded-full bg-black/70 text-white hover:bg-black">
            <X />
          </Button>
        </div>
      ) : (
        <Button variant="outline" onPointerDown={stop} onClick={onPickImage} className="h-36 w-full flex-col gap-2 rounded-2xl border-dashed font-mono text-xs text-muted">
          <ImagePlus className="size-5" /> Add an image
        </Button>
      )}
    </div>,
    !!project.ref,
  )

  const rows = project.settingIds.map((sid, index) => {
    const def = SETTING_DEFS.find((d) => d.id === sid)!
    return (
      <div key={sid} className="mx-2.5 mb-1.5 flex items-center gap-2.5 rounded-2xl border border-line bg-well py-1.5 pl-3 pr-2.5">
        <Box css={{ background: def.color }} className="size-[22px] flex-none rounded-md opacity-90" />
        <span className="w-[52px] text-[13px] text-muted">{def.label}</span>
        <Select value={project.settings[sid]} onValueChange={(v) => onChange({ settings: { ...project.settings, [sid]: v } }, true)}>
          <SelectTrigger onPointerDown={stop} className="h-9 flex-1 rounded-xl border-line bg-accent/60 text-[13px] text-text [&>svg]:size-3.5 [&>svg]:text-muted">
            <SelectValue />
            <ChevronDown className="size-3.5" />
          </SelectTrigger>
          <SelectContent>
            {def.options.map((o) => (
              <SelectItem key={o} value={o}>
                {o}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <span
          title="Drag to reorder, click to remove"
          onPointerDown={(e) => {
            e.stopPropagation()
            const startY = e.clientY
            const up = (ev: PointerEvent) => {
              window.removeEventListener("pointerup", up)
              const delta = Math.round((ev.clientY - startY) / 48)
              if (delta === 0) {
                if (project.settingIds.length > 1) onChange({ settingIds: project.settingIds.filter((s) => s !== sid) }, true)
                return
              }
              const ids = [...project.settingIds]
              ids.splice(clamp(index + delta, 0, ids.length - 1), 0, ids.splice(index, 1)[0])
              onChange({ settingIds: ids }, true)
            }
            window.addEventListener("pointerup", up)
          }}
          className="flex cursor-grab text-faint"
        >
          <GripVertical className="size-4" />
        </span>
      </div>
    )
  })

  const settings = card("settings", "SETTINGS", addSetting, <div className="pb-1 pt-3">{rows}</div>)

  const models = card(
    "models",
    "AI MODELS",
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button aria-label="Add a model" variant="ghost" size="icon" className="size-6 text-text hover:bg-transparent">
          <Plus />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {MODELS.filter((m) => !project.models.includes(m)).map((m) => (
          <DropdownMenuItem key={m} onSelect={() => onChange({ models: [...project.models, m] }, true)}>
            {m}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>,
    <div className="flex flex-wrap gap-2 p-3.5">
      {project.models.map((m, i) => (
        <span key={m} className="flex items-center gap-2 rounded-2xl border border-line bg-well px-3 py-2.5">
          <Sparkles className="size-[18px] text-blue-400" />
          <span className="rounded-full border border-text px-3 py-1.5 text-[13px] font-semibold text-white">{m}</span>
          {project.models.length > 1 && (
            <Button aria-label={`Remove ${m}`} variant="ghost" size="icon" onClick={() => onChange({ models: project.models.filter((_, j) => j !== i) }, true)} className="size-5 text-faint hover:bg-transparent">
              <X className="size-3" />
            </Button>
          )}
        </span>
      ))}
    </div>,
  )

  const portDot = (id: NodeKind) => {
    const a = outPort(id)
    return (
      <Box
        key={`out-${id}`}
        title="Drag to a card to connect"
        css={{ left: `${a.x - 7}px`, top: `${a.y - 7}px` }}
        onPointerDown={(e) => {
          e.stopPropagation()
          drag.current = { kind: "wire", id, ox: 0, oy: 0, sx: e.clientX, sy: e.clientY }
          setWire({ from: id, x: a.x, y: a.y })
        }}
        className={cn("absolute z-[5] box-content size-3.5 cursor-crosshair rounded-full border-[3px] border-stage", PORT[id])}
      />
    )
  }

  // The curves are the wires into SETTINGS; SETTINGS to AI MODELS is the straight line below it.
  const wires = project.links.filter((l) => l.from !== "settings")
  const hasModels = project.links.some((l) => l.from === "settings" && l.to === "models")

  const dots = 24 * view.k

  return (
    <Box
      ref={box}
      onWheel={onWheel}
      onPointerDown={(e) => {
        touched.current = true
        drag.current = { kind: "pan", ox: view.x, oy: view.y, sx: e.clientX, sy: e.clientY }
      }}
      css={{ "background-size": `${dots}px ${dots}px`, "background-position": `${view.x}px ${view.y}px` }}
      className="absolute inset-0 overflow-hidden bg-stage bg-[radial-gradient(var(--line)_1px,transparent_1px)]"
    >
      <Box css={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.k})` }} className="absolute left-0 top-0 origin-top-left">
        <svg width="2000" height="1400" className="pointer-events-none absolute left-0 top-0 overflow-visible">
          <defs>
            {wires.map((l, i) => {
              const a = outPort(l.from)
              const b = inPort(l.to, l.from)
              const [c1, c2] = WIRE[l.from]
              return (
                <linearGradient key={i} id={`wire-${i}`} gradientUnits="userSpaceOnUse" x1={a.x} y1={a.y} x2={b.x} y2={b.y}>
                  <stop offset="0" stopColor={c1} />
                  <stop offset="1" stopColor={c2} />
                </linearGradient>
              )
            })}
          </defs>
          {wires.map((l, i) => {
            const a = outPort(l.from)
            const b = inPort(l.to, l.from)
            return (
              <g key={`${l.from}-${l.to}`}>
                <path d={curve(a, b)} fill="none" stroke={`url(#wire-${i})`} strokeWidth="2.5" strokeLinecap="round" />
                <path
                  d={curve(a, b)}
                  fill="none"
                  stroke="transparent"
                  strokeWidth="16"
                  className="pointer-events-auto cursor-pointer"
                  onDoubleClick={() => onChange({ links: project.links.filter((x) => !(x.from === l.from && x.to === l.to)) }, true)}
                >
                  <title>Double click to remove this wire</title>
                </path>
                <circle cx={b.x} cy={b.y} r="5" fill={WIRE[l.from][1]} className="stroke-stage" strokeWidth="3" />
              </g>
            )
          })}
          {wire && <path d={curve(outPort(wire.from), { x: wire.x, y: wire.y })} fill="none" stroke="#a78bfa" strokeWidth="2" strokeDasharray="6 5" />}
        </svg>
        {ideas}
        {refs}
        {settings}
        {models}
        {portDot("ideas")}
        {portDot("refs")}
        {portDot("settings")}
        {hasModels &&
          (() => {
            const sp = pos(node("settings"))
            const mp = pos(node("models"))
            const sh = sizes.settings || 300
            const x = sp.x + W / 2
            const midY = (sp.y + sh + mp.y) / 2
            return (
              <>
                <svg width="2000" height="1400" className="pointer-events-none absolute left-0 top-0 overflow-visible">
                  <line x1={x} y1={sp.y + sh} x2={mp.x + W / 2} y2={mp.y} className="stroke-faint" strokeWidth="2" />
                </svg>
                <Box css={{ left: `${x - 14}px`, top: `${midY - 14}px` }} onPointerDown={stop} className="absolute z-[6]">
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button aria-label="Add a setting" variant="outline" size="icon" disabled={!remaining.length} className="size-7 rounded-full bg-well text-text">
                        <Plus className="size-3.5" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      {remaining.map((d) => (
                        <DropdownMenuItem key={d.id} onSelect={() => onChange({ settingIds: [...project.settingIds, d.id] }, true)}>
                          {d.label}
                        </DropdownMenuItem>
                      ))}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </Box>
              </>
            )
          })()}
      </Box>
    </Box>
  )
}
