// The playhead. One object, read every frame by the viewport and the timeline,
// so that playing does not re-render the whole window sixty times a second.

export class Transport {
  time = 0
  playing = false
  duration = 10
  private last = 0
  private raf = 0
  private listeners = new Set<() => void>()
  private frames = new Set<(time: number, playing: boolean) => void>()

  // `change` is told when playing starts or stops or the time jumps; `frame` every frame while playing.
  onChange(fn: () => void) {
    this.listeners.add(fn)
    return () => void this.listeners.delete(fn)
  }
  onFrame(fn: (time: number, playing: boolean) => void) {
    this.frames.add(fn)
    return () => void this.frames.delete(fn)
  }
  private notify() {
    this.listeners.forEach((fn) => fn())
    this.frames.forEach((fn) => fn(this.time, this.playing))
  }

  seek(time: number) {
    this.time = Math.min(this.duration, Math.max(0, time))
    this.notify()
  }

  play() {
    if (this.playing) return
    if (this.time >= this.duration - 0.01) this.time = 0
    this.playing = true
    this.last = performance.now()
    const tick = (now: number) => {
      if (!this.playing) return
      this.time += (now - this.last) / 1000
      this.last = now
      if (this.time >= this.duration) {
        this.time = this.duration
        this.playing = false
        this.notify()
        return
      }
      this.frames.forEach((fn) => fn(this.time, true))
      this.raf = requestAnimationFrame(tick)
    }
    this.raf = requestAnimationFrame(tick)
    this.notify()
  }

  pause() {
    if (!this.playing) return
    this.playing = false
    cancelAnimationFrame(this.raf)
    this.notify()
  }

  toggle() {
    this.playing ? this.pause() : this.play()
  }

  setDuration(seconds: number) {
    this.duration = seconds
    if (this.time > seconds) this.time = seconds
    this.notify()
  }
}
