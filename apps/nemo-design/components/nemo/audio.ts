// Music made here, from the setting: no file, no model. A few voices are added
// into one buffer, which is both what plays and what the waveform is drawn from.

export const SAMPLE_RATE = 22050

const NOTES = [261.63, 329.63, 392.0, 523.25, 392.0, 329.63, 293.66, 349.23, 440.0, 349.23, 293.66, 392.0]

function tone(out: Float32Array, start: number, length: number, freq: number, shape: (p: number) => number, gain: number, decay: number) {
  const from = Math.floor(start * SAMPLE_RATE)
  const n = Math.floor(length * SAMPLE_RATE)
  for (let i = 0; i < n && from + i < out.length; i++) {
    const t = i / SAMPLE_RATE
    const env = Math.exp(-t * decay) * Math.min(1, i / 200)
    out[from + i] += shape(2 * Math.PI * freq * t) * env * gain
  }
}

function noise(out: Float32Array, start: number, length: number, gain: number, decay: number, seed: number) {
  let s = seed
  const from = Math.floor(start * SAMPLE_RATE)
  const n = Math.floor(length * SAMPLE_RATE)
  for (let i = 0; i < n && from + i < out.length; i++) {
    s = (s * 1664525 + 1013904223) >>> 0
    out[from + i] += ((s / 4294967296) * 2 - 1) * Math.exp(-(i / SAMPLE_RATE) * decay) * gain
  }
}

export function renderMusic(kind: string, seconds: number): Float32Array | null {
  if (kind === "Off") return null
  const out = new Float32Array(Math.floor(seconds * SAMPLE_RATE))
  if (kind === "Piano") {
    // A soft arpeggio: a sine and its octave, struck and left to fade.
    const step = 0.42
    for (let k = 0; k * step < seconds; k++) {
      const f = NOTES[k % NOTES.length]
      tone(out, k * step, 1.6, f, Math.sin, 0.22, 2.6)
      tone(out, k * step, 1.2, f * 2, Math.sin, 0.07, 4)
      if (k % 4 === 0) tone(out, k * step, 2.0, f / 2, Math.sin, 0.16, 1.8)
    }
  } else if (kind === "Synth") {
    // Slow pads: two detuned saws.
    const saw = (p: number) => ((p / Math.PI) % 2) - 1
    for (let k = 0; k * 1.6 < seconds; k++) {
      const f = NOTES[(k * 3) % NOTES.length] / 2
      tone(out, k * 1.6, 2.4, f, saw, 0.09, 0.9)
      tone(out, k * 1.6, 2.4, f * 1.005, saw, 0.09, 0.9)
      tone(out, k * 1.6, 2.4, f * 2, Math.sin, 0.05, 1.2)
    }
  } else if (kind === "Drums") {
    const beat = 0.5
    for (let k = 0; k * beat < seconds; k++) {
      const at = k * beat
      if (k % 2 === 0) tone(out, at, 0.35, 70, Math.sin, 0.6, 12)
      if (k % 4 === 2) noise(out, at, 0.22, 0.4, 18, 7 + k)
      noise(out, at + beat / 2, 0.06, 0.15, 60, 99 + k)
    }
  }
  // Keep it from clipping.
  let peak = 0
  for (let i = 0; i < out.length; i++) peak = Math.max(peak, Math.abs(out[i]))
  if (peak > 0.95) for (let i = 0; i < out.length; i++) out[i] *= 0.95 / peak
  return out
}

// One bar of the waveform per `bars` slice: the peak of the slice, 0 to 1.
export function peaks(samples: Float32Array, bars: number): number[] {
  const per = Math.max(1, Math.floor(samples.length / bars))
  const result: number[] = []
  for (let b = 0; b < bars; b++) {
    let peak = 0
    for (let i = b * per; i < Math.min(samples.length, (b + 1) * per); i++) peak = Math.max(peak, Math.abs(samples[i]))
    result.push(peak)
  }
  return result
}
