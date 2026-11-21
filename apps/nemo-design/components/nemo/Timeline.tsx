// The timeline: a ruler with the playhead, the clip of the picture, and the audio clip
// with the waveform of the music that is really playing. Everything drags.
import * as React from "react"
import { Box } from "@/lib/geometry"
import { clamp, fmt } from "./model"
import type { Transport } from "./transport"

type Props = {
  transport: Transport
  duration: number
  clip: [number, number]
  audio: [number, number]
  peaks: number[]
  locked: boolean
  onClip: (clip: [number, number], commit: boolean) => void
  onAudio: (audio: [number, number], commit: boolean) => void
}

const PAD = 12 // px at each side of the track

export default function Timeline({ transport, duration, clip, audio, peaks, locked, onClip, onAudio }: Props) {
  const track = React.useRef<HTMLDivElement>(null)
  const head = React.useRef<HTMLDivElement>(null)
  const label = React.useRef<HTMLSpanElement>(null)

  const secondsAt = (clientX: number) => {
    const r = track.current!.getBoundingClientRect()
    return clamp(((clientX - r.left - PAD) / (r.width - PAD * 2)) * duration, 0, duration)
  }
  const pct = (s: number) => `${(s / duration) * 100}%`
  const along = (s: number) => `calc(${PAD}px + (100% - ${PAD * 2}px) * ${s / duration})`

  // The playhead follows the transport without a React render.
  React.useEffect(() => {
    const place = (t: number) => {
      if (head.current) head.current.style.left = along(t)
      if (label.current) label.current.textContent = fmt(t)
    }
    place(transport.time)
    const a = transport.onFrame((t) => place(t))
    const b = transport.onChange(() => place(transport.time))
    return () => {
      a()
      b()
    }
  }, [transport, duration]) // eslint-disable-line react-hooks/exhaustive-deps

  const drag = (e: React.PointerEvent, move: (s: number, dx: number) => void, done: () => void) => {
    e.stopPropagation()
    e.preventDefault()
    const start = secondsAt(e.clientX)
    const onMove = (ev: PointerEvent) => move(secondsAt(ev.clientX), secondsAt(ev.clientX) - start)
    const onUp = () => {
      window.removeEventListener("pointermove", onMove)
      window.removeEventListener("pointerup", onUp)
      done()
    }
    window.addEventListener("pointermove", onMove)
    window.addEventListener("pointerup", onUp)
  }

  const scrub = (e: React.PointerEvent) => {
    transport.pause()
    transport.seek(secondsAt(e.clientX))
    drag(e, (s) => transport.seek(s), () => {})
  }

  const ticks = Array.from({ length: Math.floor(duration) }, (_, i) => i + 1).filter((s) => s < duration)
  const bars = peaks.length

  const handle = (side: "l" | "r", on: (e: React.PointerEvent) => void, tone: string) => (
    <div onPointerDown={on} className={`absolute inset-y-0 flex w-3 cursor-ew-resize items-center justify-center ${side === "l" ? "left-0" : "right-0"}`}>
      <div className={`h-6 w-1 rounded-sm ${tone}`} />
    </div>
  )

  return (
    <div ref={track} className="relative select-none pb-3.5">
      <div onPointerDown={scrub} className="relative h-[54px] cursor-pointer">
        {ticks.map((s) => (
          <Box key={s} css={{ left: along(s) }} className="absolute top-[22px] -translate-x-1/2 text-center text-[13px] text-muted">
            {fmt(s)}
            <div className="mx-auto mt-2 size-[3px] rounded-full bg-faint" />
          </Box>
        ))}
      </div>

      <div className="relative mx-3 h-[78px] overflow-hidden rounded-2xl bg-card">
        <Box
          css={{ left: pct(clip[0]), width: pct(clip[1] - clip[0]) }}
          onPointerDown={(e) => {
            if (locked) return
            const base = clip
            const len = base[1] - base[0]
            drag(e, (_s, d) => onClip([clamp(base[0] + d, 0, duration - len), clamp(base[1] + d, len, duration)], false), () => onClip(clip, true))
          }}
          className={`absolute inset-y-0 rounded-2xl bg-gradient-to-b from-clip to-purple-400 ${locked ? "" : "cursor-grab"}`}
        >
          {!locked && handle("l", (e) => drag(e, (s) => onClip([clamp(s, 0, clip[1] - 0.5), clip[1]], false), () => onClip(clip, true)), "bg-white/55")}
          {!locked && handle("r", (e) => drag(e, (s) => onClip([clip[0], clamp(s, clip[0] + 0.5, duration)], false), () => onClip(clip, true)), "bg-white/55")}
          {ticks
            .filter((s) => s > clip[0] && s < clip[1])
            .map((s) => (
              <Box key={s} css={{ left: `${((s - clip[0]) / (clip[1] - clip[0])) * 100}%` }} className="absolute inset-y-0 w-px bg-white/20" />
            ))}
        </Box>
      </div>

      <div className="relative mx-3 mt-2.5 h-[78px] overflow-hidden rounded-2xl border border-line bg-card">
        {bars > 0 && (
          <Box
            css={{ left: pct(audio[0]), width: pct(audio[1] - audio[0]) }}
            onPointerDown={(e) => {
              const base = audio
              const len = base[1] - base[0]
              drag(e, (_s, d) => onAudio([clamp(base[0] + d, 0, duration - len), clamp(base[1] + d, len, duration)], false), () => onAudio(audio, true))
            }}
            className="absolute inset-y-0 cursor-grab rounded-xl border border-line bg-well"
          >
            {handle("l", (e) => drag(e, (s) => onAudio([clamp(s, 0, audio[1] - 0.5), audio[1]], false), () => onAudio(audio, true)), "bg-zinc-300")}
            {handle("r", (e) => drag(e, (s) => onAudio([audio[0], clamp(s, audio[0] + 0.5, duration)], false), () => onAudio(audio, true)), "bg-zinc-300")}
            <div className="absolute inset-x-4 inset-y-2 flex items-center gap-0.5">
              {Array.from({ length: 80 }, (_, i) => {
                // The part of the music this clip plays, drawn as it is.
                const from = (audio[0] / duration) * bars
                const to = (audio[1] / duration) * bars
                const v = peaks[Math.min(bars - 1, Math.floor(from + ((to - from) * i) / 80))] ?? 0
                return <Box key={i} css={{ height: `${Math.max(6, v * 100)}%` }} className="flex-1 rounded-sm bg-zinc-300/85" />
              })}
            </div>
          </Box>
        )}
      </div>

      <div ref={head} className="pointer-events-none absolute inset-y-0 bottom-3.5 w-0">
        <div className="absolute -left-[7px] top-0 size-3.5 bg-fuchsia-300 [clip-path:polygon(0_0,100%_0,50%_100%)]" />
        <div className="absolute inset-y-3.5 left-0 w-px bg-fuchsia-300/55" />
        <span ref={label} className="absolute -top-4 left-3 text-[11px] text-fuchsia-300" />
      </div>
    </div>
  )
}
