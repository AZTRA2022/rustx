// Nemo Design: the whole window. The graph of cards, the code written from it, the 3D
// viewport, the timeline, the tabs, undo and redo, saving and exporting.
//
// Every control is a shadcn component and every look a Tailwind class; the page has no `style`
// attribute and no `<style>` tag. What it does not do is call a generation model: the AI
// MODELS card is a choice that is kept and shown, and nothing is sent anywhere.
import * as React from "react"
import {
  ArrowLeft, ArrowRight, ChevronDown, Code2, Grid3x3, Layers, Lightbulb, Link2, Moon, Mouse, MousePointer2, Orbit, PanelLeft, Paperclip, Pause, Play, Plus, Share2,
  ShoppingBag, Star, Sun, Timer, Type, X, Minus, Square,
} from "lucide-react"
import NodeCanvas from "./nemo/NodeCanvas"
import Timeline from "./nemo/Timeline"
import CodePanel, { generate } from "./nemo/CodePanel"
import Viewport, { type ViewportHandle } from "./nemo/Viewport"
import { Transport } from "./nemo/transport"
import { renderMusic, peaks as peaksOf, SAMPLE_RATE } from "./nemo/audio"
import { LIBRARY, modelFor } from "./nemo/library"
import { DURATIONS, freshProject, starterProjects, clamp, type Project, type Workspace } from "./nemo/model"
import { copyText, openText, pickImage, saveBase64, toBase64, windowOrder } from "./nemo/natives"
import { Button } from "@/components/ui/button"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

const { Component, useCallback, useEffect, useMemo, useRef, useState } = React

const KEY = "nemo-design-workspace-v1"

// A driven run (CI, `--scenario=…`) starts from the starter projects and leaves no trace.
const driven = typeof (window as unknown as { rustx_scenario?: () => string }).rustx_scenario === "function" && !!(window as unknown as { rustx_scenario: () => string }).rustx_scenario()

function load(): Workspace {
  try {
    if (driven) throw new Error("driven")
    const raw = localStorage.getItem(KEY)
    if (raw) {
      const w = JSON.parse(raw) as Workspace
      if (w.projects?.length && w.projects.some((p) => p.id === w.active)) return w
    }
  } catch {}
  const projects = starterProjects()
  return { projects, active: projects[0].id }
}

// A window that shows what went wrong rather than nothing.
class Guard extends Component<{ children: React.ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null }
  static getDerivedStateFromError(error: Error) {
    return { error }
  }
  render() {
    const e = this.state.error
    if (!e) return this.props.children
    return (
      <pre className="fixed inset-0 m-0 overflow-auto whitespace-pre-wrap bg-stage p-6 text-[13px] text-rose-300">
        {["Nemo Design could not draw its window.", e.message, e.stack ?? ""].join(String.fromCharCode(10, 10))}
      </pre>
    )
  }
}

export default function NemoApp() {
  return (
    <Guard>
      <TooltipProvider delayDuration={300}>
        <Editor />
      </TooltipProvider>
    </Guard>
  )
}

function Tip({ label, children }: { label: string; children: React.ReactElement }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  )
}

const squareBase = "shrink-0 rounded-2xl border border-line bg-card text-muted hover:bg-accent hover:text-white"
const square = "size-11 md:size-[50px] " + squareBase

function Editor() {
  const [ws, setWs] = useState<Workspace>(load)
  const [tool, setTool] = useState("orbit")
  const [codeOpen, setCodeOpen] = useState(true)
  const [codeBig, setCodeBig] = useState(false)
  const [panelOpen, setPanelOpen] = useState(true)
  const [snap, setSnap] = useState(false)
  const [light, setLight] = useState(false)
  const [animations, setAnimations] = useState<string[]>([])
  const [picked, setPicked] = useState("")
  const [renaming, setRenaming] = useState("")
  const [playing, setPlaying] = useState(false)
  const [exporting, setExporting] = useState<null | number>(null)
  const [note, setNote] = useState("")
  const [genMs, setGenMs] = useState(0)
  const [arrange, setArrange] = useState(0)
  const transport = useMemo(() => new Transport(), [])
  const viewport = useRef<ViewportHandle>(null)

  const project = ws.projects.find((p) => p.id === ws.active)!

  // ---- history: the state at the last commit, and what came before and after it
  const history = useRef(new Map<string, { past: Project[]; future: Project[]; base: Project }>())
  const entry = (p: Project) => {
    let h = history.current.get(p.id)
    if (!h) history.current.set(p.id, (h = { past: [], future: [], base: p }))
    return h
  }
  const [, bump] = useState(0)

  const update = useCallback((patch: Partial<Project>, commit: boolean) => {
    setWs((w) => {
      const projects = w.projects.map((p) => {
        if (p.id !== w.active) return p
        let next = { ...p, ...patch }
        const h = entry(p)
        if (commit) {
          // An idea that names a model of the library changes the model with it.
          const chosen = modelFor(next.ideas, next.model)
          if (chosen !== next.model && next.ideas !== h.base.ideas) next = { ...next, model: chosen, animation: "" }
          if (JSON.stringify(next) !== JSON.stringify(h.base)) {
            h.past.push(h.base)
            if (h.past.length > 100) h.past.shift()
            h.future = []
            h.base = next
          }
        }
        return next
      })
      return { ...w, projects }
    })
    if (commit) bump((n) => n + 1)
  }, [])

  const undo = () => {
    const h = entry(project)
    const prev = h.past.pop()
    if (!prev) return
    h.future.push(h.base)
    h.base = prev
    setWs((w) => ({ ...w, projects: w.projects.map((p) => (p.id === w.active ? prev : p)) }))
    bump((n) => n + 1)
  }
  const redo = () => {
    const h = entry(project)
    const next = h.future.pop()
    if (!next) return
    h.past.push(h.base)
    h.base = next
    setWs((w) => ({ ...w, projects: w.projects.map((p) => (p.id === w.active ? next : p)) }))
    bump((n) => n + 1)
  }
  const h = entry(project)

  // ---- persistence, and the theme: one class on the root
  useEffect(() => {
    if (driven) return
    try {
      localStorage.setItem(KEY, JSON.stringify(ws))
    } catch {}
  }, [ws])
  useEffect(() => {
    document.documentElement.classList.toggle("light", light)
  }, [light])

  // ---- the transport follows the project's duration
  useEffect(() => {
    transport.pause()
    transport.setDuration(project.duration)
    transport.seek(0)
  }, [project.id, project.duration, transport])
  useEffect(() => transport.onChange(() => setPlaying(transport.playing)), [transport])

  // ---- music, and the waveform it is drawn from
  const settings = project.settings
  const music = useMemo(() => renderMusic(settings.music, project.duration), [settings.music, project.duration])
  const peaks = useMemo(() => (music ? peaksOf(music, 240) : []), [music])

  const audio = useRef<{ ctx: AudioContext | null; src: AudioBufferSourceNode | null; dest: MediaStreamAudioDestinationNode | null }>({ ctx: null, src: null, dest: null })
  const stopSound = () => {
    try {
      audio.current.src?.stop()
    } catch {}
    audio.current.src = null
    window.speechSynthesis?.cancel()
  }
  const startSound = useCallback(() => {
    stopSound()
    const t = transport.time
    const [a0, a1] = project.audio
    if (music && t < a1) {
      const ctx = (audio.current.ctx ??= new AudioContext())
      const dest = (audio.current.dest ??= ctx.createMediaStreamDestination())
      ctx.resume()
      const buffer = ctx.createBuffer(1, music.length, SAMPLE_RATE)
      buffer.copyToChannel(music, 0)
      const src = ctx.createBufferSource()
      src.buffer = buffer
      src.connect(ctx.destination)
      src.connect(dest)
      const from = Math.max(t, a0)
      src.start(ctx.currentTime + Math.max(0, a0 - t), from, Math.max(0.01, a1 - from))
      audio.current.src = src
    }
    const voice = settings.voice
    if (voice !== "Off" && project.ideas.trim() && "speechSynthesis" in window) {
      const u = new SpeechSynthesisUtterance(project.ideas)
      u.pitch = voice === "Happy" ? 1.4 : voice === "Deep" ? 0.5 : 1
      u.rate = voice === "Calm" ? 0.85 : voice === "Deep" ? 0.9 : 1.1
      window.speechSynthesis.speak(u)
    }
  }, [music, project.audio, project.ideas, settings.voice, transport])

  useEffect(() => {
    if (playing) startSound()
    else stopSound()
    return stopSound
  }, [playing]) // eslint-disable-line react-hooks/exhaustive-deps

  // The audio clip moved while playing: start it again from where the playhead is.
  useEffect(() => {
    if (transport.playing) startSound()
  }, [project.audio[0], project.audio[1], settings.music]) // eslint-disable-line react-hooks/exhaustive-deps

  // ---- the code, written from the project
  const code = useMemo(() => {
    const t0 = performance.now()
    const text = generate(project, animations)
    setTimeout(() => setGenMs(Math.max(1, Math.round(performance.now() - t0))), 0)
    return text
  }, [project, animations])

  // ---- trim: the clip is locked unless the setting is Manual
  const manual = settings.trim === "Manual"
  useEffect(() => {
    if (!manual && (project.clip[0] !== 0 || project.clip[1] !== project.duration)) update({ clip: [0, project.duration] }, false)
  }, [manual, project.duration]) // eslint-disable-line react-hooks/exhaustive-deps

  // ---- tabs
  const add = () => {
    const p = freshProject("Untitled")
    setWs((w) => ({ projects: [...w.projects, p], active: p.id }))
  }
  const close = (id: string) => {
    setWs((w) => {
      if (w.projects.length === 1) return w
      const projects = w.projects.filter((p) => p.id !== id)
      history.current.delete(id)
      return { projects, active: w.active === id ? projects[Math.max(0, w.projects.findIndex((p) => p.id === id) - 1)].id : w.active }
    })
  }
  const rename = (id: string, name: string) => setWs((w) => ({ ...w, projects: w.projects.map((p) => (p.id === id ? { ...p, name: name || p.name } : p)) }))

  // ---- files
  const attach = async () => {
    const image = await pickImage()
    if (image) update({ ref: image }, true)
  }
  const saveProject = async () => {
    const text = JSON.stringify({ nemo: 1, project }, null, 2)
    const path = await saveBase64(project.name.replace(/[^\w-]+/g, "-") || "project", btoa(unescape(encodeURIComponent(text))), "nemo")
    setNote(path ? `Saved to ${path}` : "")
  }
  const openProject = async () => {
    const text = await openText("nemo")
    if (!text) return
    try {
      const data = JSON.parse(text)
      if (!data.project) throw new Error("not a Nemo project")
      const p: Project = { ...freshProject(data.project.name ?? "Opened"), ...data.project, id: Math.random().toString(36).slice(2, 9) }
      setWs((w) => ({ projects: [...w.projects, p], active: p.id }))
      setNote(`Opened ${p.name}`)
    } catch (e) {
      setNote(`That file is not a Nemo project (${(e as Error).message})`)
    }
  }
  const share = async () => {
    await copyText(JSON.stringify({ nemo: 1, project }))
    setNote("The project is on the clipboard")
  }

  // ---- export: the viewport and the music, recorded for as long as the project lasts
  const exportVideo = async () => {
    const canvas = viewport.current?.canvas()
    if (!canvas || exporting !== null) return
    transport.pause()
    transport.seek(0)
    const stream = canvas.captureStream(30)
    startSound()
    transport.play()
    const tracks = audio.current.dest?.stream.getAudioTracks() ?? []
    tracks.forEach((t) => stream.addTrack(t))
    const type = ["video/webm;codecs=vp9,opus", "video/webm;codecs=vp8,opus", "video/webm"].find((t) => MediaRecorder.isTypeSupported(t))
    const recorder = new MediaRecorder(stream, { mimeType: type, videoBitsPerSecond: settings.think === "Deep" ? 8_000_000 : 4_000_000 })
    const chunks: Blob[] = []
    recorder.ondataavailable = (e) => e.data.size && chunks.push(e.data)
    setExporting(0)
    const off = transport.onFrame((t) => setExporting(Math.round((t / project.duration) * 100)))
    await new Promise<void>((done) => {
      recorder.onstop = () => done()
      recorder.start(250)
      const finish = transport.onChange(() => {
        if (!transport.playing) {
          finish()
          recorder.stop()
        }
      })
    })
    off()
    const blob = new Blob(chunks, { type: "video/webm" })
    const path = await saveBase64(project.name.replace(/[^\w-]+/g, "-") || "nemo", await toBase64(blob), "webm")
    setExporting(null)
    setNote(path ? `Exported ${(blob.size / 1024 / 1024).toFixed(1)} MB to ${path}` : "Export cancelled")
  }

  // ---- keys
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.matches?.("input,textarea,[role=combobox]")) return
      if (e.code === "Space") {
        e.preventDefault()
        transport.toggle()
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault()
        e.shiftKey ? redo() : undo()
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault()
        redo()
      }
    }
    window.addEventListener("keydown", on)
    return () => window.removeEventListener("keydown", on)
  })

  useEffect(() => {
    if (!note) return
    const t = setTimeout(() => setNote(""), 4500)
    return () => clearTimeout(t)
  }, [note])

  // ---- pieces
  const rail = (label: string, icon: React.ReactNode, onClick: () => void, active = false) => (
    <Tip key={label} label={label}>
      <Button aria-label={label} variant="ghost" size="icon" onClick={onClick} className={cn("size-10 rounded-xl text-muted hover:bg-accent hover:text-white", active && "bg-accent text-white")}>
        {icon}
      </Button>
    </Tip>
  )

  const tools: { id: string; label: string; icon: React.ReactNode }[] = [
    { id: "select", label: "Select a part", icon: <MousePointer2 /> },
    { id: "star", label: "Animations", icon: <Star /> },
    { id: "orbit", label: "Orbit the camera", icon: <Mouse /> },
    { id: "bag", label: "Save and open", icon: <ShoppingBag /> },
    { id: "bulb", label: "Light", icon: <Lightbulb /> },
  ]
  const toolButton = (t: { id: string; label: string; icon: React.ReactNode }) => (
    <Button
      aria-label={t.label}
      variant="ghost"
      size="icon"
      onClick={() => (t.id === "select" || t.id === "orbit") && setTool(t.id)}
      className={cn("size-12 rounded-2xl text-muted hover:bg-white/10 hover:text-white [&_svg]:size-[22px]", tool === t.id && "bg-white/10 text-white")}
    >
      {t.icon}
    </Button>
  )
  const menuItem = (text: string, on: boolean, click: () => void) => (
    <DropdownMenuItem key={text} checked={on} onSelect={click}>
      {text}
    </DropdownMenuItem>
  )

  return (
    <div className="fixed inset-0 flex flex-col bg-stage font-sans text-text">
      {/* ---------------------------------------------------------------- top bar */}
      <div className="flex flex-none items-center gap-2 py-2 pl-2.5 pr-32 max-lg:flex-wrap md:gap-2.5">
        <div className="flex min-w-0 flex-1 items-center gap-2 max-lg:basis-full md:gap-2.5">
        <Tip label="Show or hide the side panels">
          <Button aria-label="Show or hide the side panels" variant="ghost" size="icon" onClick={() => setPanelOpen(!panelOpen)} className={cn(square, panelOpen && "text-white")}>
            <PanelLeft className="size-5" />
          </Button>
        </Tip>
        <svg width="26" height="26" viewBox="0 0 22 22" aria-hidden="true" className="mx-1 flex-none">
          {[0, 1, 2, 3].map((r) => [0, 1, 2, 3].map((c) => <circle key={`${r}${c}`} cx={4 + c * 4.6} cy={4 + r * 4.6} r="1.6" className={["fill-red-400", "fill-amber-400", "fill-green-400", "fill-blue-400"][(r + c) % 4]} />))}
        </svg>
        <Tabs value={ws.active} onValueChange={(v) => setWs({ ...ws, active: v })} className="min-w-0 shrink overflow-hidden">
          <TabsList className="h-[50px] gap-2 overflow-x-auto bg-transparent p-0">
            {ws.projects.map((p) => (
              <TabsTrigger
                key={p.id}
                value={p.id}
                onDoubleClick={() => setRenaming(p.id)}
                className="h-[50px] gap-2.5 rounded-2xl bg-card px-4 text-[15px] font-normal text-muted data-[state=active]:bg-accent data-[state=active]:text-white data-[state=active]:shadow-none"
              >
                <Orbit className="size-[18px]" />
                {renaming === p.id ? (
                  <input
                    autoFocus
                    defaultValue={p.name}
                    onClick={(e) => e.stopPropagation()}
                    onKeyDown={(e) => {
                      e.stopPropagation()
                      if (e.key === "Enter") (e.target as HTMLInputElement).blur()
                    }}
                    onBlur={(e) => {
                      rename(p.id, e.target.value.trim())
                      setRenaming("")
                    }}
                    className="w-32 rounded bg-transparent text-[15px] text-white outline outline-1 outline-line"
                  />
                ) : (
                  <span>{p.name}</span>
                )}
                {ws.projects.length > 1 && (
                  <span
                    role="button"
                    aria-label={`Close ${p.name}`}
                    onClick={(e) => {
                      e.stopPropagation()
                      close(p.id)
                    }}
                    onPointerDown={(e) => e.stopPropagation()}
                    onMouseDown={(e) => e.stopPropagation()}
                    className="flex size-[18px] items-center justify-center rounded text-faint hover:text-white"
                  >
                    <X className="size-3.5" />
                  </span>
                )}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
        <Tip label="New project">
          <Button aria-label="New project" variant="ghost" size="icon" onClick={add} className="h-[50px] w-11 text-muted hover:bg-accent hover:text-white">
            <Plus className="size-5" />
          </Button>
        </Tip>
        <div className="h-11 min-w-2 flex-1 md:h-[50px]" onPointerDown={() => windowOrder("drag")} onDoubleClick={() => windowOrder("maximize")} />
        </div>
        <div className="flex flex-none items-center gap-2 max-lg:ml-auto md:gap-2.5">
        <Tip label="Undo (Ctrl+Z)">
          <Button aria-label="Undo" variant="ghost" size="icon" disabled={!h.past.length} onClick={undo} className={square}>
            <ArrowLeft />
          </Button>
        </Tip>
        <Tip label="Redo (Ctrl+Y)">
          <Button aria-label="Redo" variant="ghost" size="icon" disabled={!h.future.length} onClick={redo} className={square}>
            <ArrowRight />
          </Button>
        </Tip>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button aria-label="Duration" variant="ghost" className={cn(squareBase, "h-11 w-auto gap-2 px-3 text-[15px] text-text md:h-[50px] md:gap-2.5 md:px-4")}>
              <Timer className="size-[22px]" /> {project.duration}s <ChevronDown className="size-3.5 text-faint max-sm:hidden" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {DURATIONS.map((d) =>
              menuItem(`${d} seconds`, d === project.duration, () => update({ duration: d, clip: [0, d], audio: [Math.min(project.audio[0], d - 1), Math.min(project.audio[1], d)] }, true)),
            )}
          </DropdownMenuContent>
        </DropdownMenu>
        <Tip label="Show or hide the code">
          <Button aria-label="Show or hide the code" variant="ghost" size="icon" onClick={() => setCodeOpen(!codeOpen)} className={cn(square, codeOpen && "text-white")}>
            <Code2 className="size-5" />
          </Button>
        </Tip>
        <Tip label="Play or pause (Space)">
          <Button aria-label={playing ? "Pause" : "Play"} variant="ghost" size="icon" onClick={() => transport.toggle()} className={square}>
            {playing ? <Pause className="size-5" /> : <Play className="size-5" />}
          </Button>
        </Tip>
        <Tip label="Copy the project">
          <Button aria-label="Copy the project" variant="ghost" size="icon" onClick={share} className={square}>
            <Share2 className="size-5" />
          </Button>
        </Tip>
        <Button aria-label="Export" onClick={exportVideo} disabled={exporting !== null} className="h-[50px] min-w-[104px] rounded-2xl px-[22px] text-base font-semibold shadow-none">
          {exporting === null ? "Export" : `${exporting}%`}
        </Button>
        </div>
      </div>

      {/* ------------------------------------------------------------------ body */}
      <div className="relative flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-2 pb-2 lg:flex-row lg:overflow-hidden">
        {panelOpen && (
          <div className="flex flex-none items-center gap-1.5 pt-2 max-lg:overflow-x-auto max-lg:pb-1 lg:w-[58px] lg:flex-col">
            {rail("Write in the ideas", <Type className="size-5" />, () => (document.querySelector("[data-node=ideas] textarea") as HTMLTextAreaElement | null)?.focus())}
            {rail("Arrange the cards", <Layers className="size-5" />, () => { update({ nodes: freshProject("").nodes }, true); setArrange((n) => n + 1) })}
            {rail("Restore the wires", <Link2 className="size-5" />, () => update({ links: freshProject("").links }, true))}
            <DropdownMenu>
              <Tip label="3D models">
                <DropdownMenuTrigger asChild>
                  <Button aria-label="3D models" variant="ghost" size="icon" className="size-10 rounded-xl text-muted hover:bg-accent hover:text-white">
                    <Orbit className="size-5" />
                  </Button>
                </DropdownMenuTrigger>
              </Tip>
              <DropdownMenuContent side="right" align="start">
                {LIBRARY.map((m) => menuItem(m.name, m.id === project.model, () => update({ model: m.id, animation: "" }, true)))}
              </DropdownMenuContent>
            </DropdownMenu>
            {rail("Snap to the grid", <Grid3x3 className="size-5" />, () => setSnap(!snap), snap)}
            {rail("Attach an image", <Paperclip className="size-5" />, attach)}
            <div className="flex-1 max-lg:min-w-2" />
            {rail(light ? "Dark theme" : "Light theme", light ? <Moon className="size-5" /> : <Sun className="size-5" />, () => setLight(!light))}
            <Button aria-label="New project" size="icon" onClick={add} className="my-2 size-[52px] shrink-0 rounded-full shadow-[0_0_24px_rgba(58,75,255,0.45)]">
              <Plus className="size-6" />
            </Button>
          </div>
        )}

        <div className="flex min-h-[520px] min-w-0 flex-[1_1_52%] flex-col gap-2 lg:min-h-0">
          <div className="relative min-h-0 flex-1 overflow-hidden rounded-2xl border border-line">
            <NodeCanvas project={project} snap={snap} onChange={update} onPickImage={attach} arrangeKey={arrange} />
          </div>
          {codeOpen && (
            <div className={cn("flex-none", codeBig ? "h-[58%]" : "h-[230px]")}>
              <CodePanel code={code} ms={genMs} expanded={codeBig} onExpand={() => setCodeBig(!codeBig)} onReset={() => update({ nodes: freshProject("").nodes, animation: "" }, true)} />
            </div>
          )}
        </div>

        <div className="flex min-h-[460px] min-w-0 flex-[1_1_48%] flex-col gap-2 lg:min-h-0">
          <div className="relative min-h-0 flex-1 overflow-hidden rounded-2xl border border-line bg-black">
            <Viewport
              ref={viewport}
              transport={transport}
              model={project.model}
              animation={project.animation}
              mode={settings.mode}
              think={settings.think}
              light={settings.light}
              camera={settings.camera}
              reference={project.ref?.url ?? null}
              tool={tool}
              clip={project.clip}
              onAnimations={(names, chosen) => {
                setAnimations(names)
                if (chosen && chosen !== project.animation) update({ animation: chosen }, false)
              }}
              onPick={setPicked}
            />
            {settings.mode === "Cinematic" && (
              <>
                <div className="pointer-events-none absolute inset-x-0 top-0 h-[9%] bg-black" />
                <div className="pointer-events-none absolute inset-x-0 bottom-0 h-[9%] bg-black" />
              </>
            )}
            <div className="absolute right-3 top-3.5 z-[5] flex flex-col gap-1.5">
              {tools.map((t) => {
                if (t.id === "select" || t.id === "orbit") return <Tip key={t.id} label={t.label}>{toolButton(t)}</Tip>
                return (
                  <Popover key={t.id}>
                    <Tip label={t.label}>
                      <PopoverTrigger asChild>{toolButton(t)}</PopoverTrigger>
                    </Tip>
                    <PopoverContent side="left" align="start">
                      {t.id === "star" &&
                        (animations.length ? (
                          animations.map((a) => (
                            <Button key={a} variant="ghost" onClick={() => update({ animation: a }, true)} className={cn("h-9 w-full justify-start px-3 font-normal text-muted hover:text-white", a === project.animation && "bg-accent text-white")}>
                              {a}
                            </Button>
                          ))
                        ) : (
                          <p className="p-2.5 text-[13px] text-faint">This model has no animation.</p>
                        ))}
                      {t.id === "bag" && (
                        <>
                          <Button variant="ghost" onClick={saveProject} className="h-9 w-full justify-start px-3 font-normal text-muted hover:text-white">Save the project…</Button>
                          <Button variant="ghost" onClick={openProject} className="h-9 w-full justify-start px-3 font-normal text-muted hover:text-white">Open a project…</Button>
                          <Button variant="ghost" onClick={() => copyText(code)} className="h-9 w-full justify-start px-3 font-normal text-muted hover:text-white">Copy the code</Button>
                        </>
                      )}
                      {t.id === "bulb" &&
                        ["Studio", "Warm", "Cold"].map((l) => (
                          <Button key={l} variant="ghost" onClick={() => update({ settings: { ...settings, light: l } }, true)} className={cn("h-9 w-full justify-start px-3 font-normal text-muted hover:text-white", settings.light === l && "bg-accent text-white")}>
                            {l}
                          </Button>
                        ))}
                    </PopoverContent>
                  </Popover>
                )
              })}
            </div>
            {picked && <div className="absolute left-3.5 top-3.5 rounded-full border border-violet-500 bg-violet-500/20 px-3 py-1.5 text-[13px] text-violet-300">{picked}</div>}
            {note && <div className="absolute bottom-3.5 left-3.5 max-w-[80%] rounded-xl border border-line bg-popover px-3.5 py-2 text-[13px]">{note}</div>}
          </div>
          <div className="flex-none rounded-2xl border border-line bg-panel px-1.5 pt-[22px]">
            <Timeline
              transport={transport}
              duration={project.duration}
              clip={project.clip}
              audio={project.audio}
              peaks={peaks}
              locked={!manual}
              onClip={(clip, commit) => update({ clip, audio: [clamp(project.audio[0], clip[0], clip[1]), clamp(project.audio[1], clip[0], clip[1])] }, commit)}
              onAudio={(audioClip, commit) => update({ audio: audioClip }, commit)}
            />
          </div>
        </div>
      </div>
      <div className="fixed right-2 top-2 z-50 flex items-center rounded-full bg-stage/80 backdrop-blur">
          <Button aria-label="Minimize" variant="ghost" size="icon" onClick={() => windowOrder("minimize")} className="size-9 rounded-full text-faint hover:bg-accent hover:text-white">
            <Minus className="size-4" />
          </Button>
          <Button aria-label="Maximize" variant="ghost" size="icon" onClick={() => windowOrder("maximize")} className="size-9 rounded-full text-faint hover:bg-accent hover:text-white">
            <Square className="size-3.5" />
          </Button>
          <Button aria-label="Close" variant="ghost" size="icon" onClick={() => windowOrder("close")} className="size-9 rounded-full text-faint hover:bg-red-500 hover:text-white">
            <X className="size-4" />
          </Button>
        </div>
    </div>
  )
}
