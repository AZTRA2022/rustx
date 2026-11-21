// What the Rust side of the application gives the page: file dialogs and files. Registered in
// src/main.rs as natives, they are plain functions on `window`. In a plain browser (no Rust)
// each falls back to what a browser can do, so the page can be developed alone.

type Native = (...args: unknown[]) => unknown
const native = (name: string): Native | undefined => (window as unknown as Record<string, Native>)[name]

export const hasHost = () => typeof native("nemo_pick_image") === "function"

export type Picked = { name: string; url: string }

export async function pickImage(): Promise<Picked | null> {
  const f = native("nemo_pick_image")
  if (f) return (await f()) as Picked | null
  return new Promise((resolve) => {
    const input = document.createElement("input")
    input.type = "file"
    input.accept = "image/*"
    input.onchange = () => {
      const file = input.files?.[0]
      if (!file) return resolve(null)
      const reader = new FileReader()
      reader.onload = () => resolve({ name: file.name, url: String(reader.result) })
      reader.readAsDataURL(file)
    }
    input.click()
  })
}

// Writes `base64` where the person chooses. Returns the path, or null when they cancelled.
export async function saveBase64(suggested: string, base64: string, extension: string): Promise<string | null> {
  const f = native("nemo_save")
  if (f) return (await f(suggested, base64, extension)) as string | null
  const a = document.createElement("a")
  a.href = `data:application/octet-stream;base64,${base64}`
  a.download = `${suggested}.${extension}`
  a.click()
  return a.download
}

export async function openText(extension: string): Promise<string | null> {
  const f = native("nemo_open_text")
  if (f) return (await f(extension)) as string | null
  return new Promise((resolve) => {
    const input = document.createElement("input")
    input.type = "file"
    input.accept = `.${extension}`
    input.onchange = async () => resolve((await input.files?.[0]?.text()) ?? null)
    input.click()
  })
}

export async function copyText(text: string) {
  const f = native("nemo_copy")
  if (f) return void f(text)
  await navigator.clipboard?.writeText(text)
}

export function toBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result).split(",")[1] ?? "")
    reader.onerror = () => reject(reader.error)
    reader.readAsDataURL(blob)
  })
}

// A press on the bar moves the window, and the three marks do what a frame's did. Without the
// Rust side (a plain browser) there is no window to order and nothing happens.
export function windowOrder(order: "drag" | "minimize" | "maximize" | "close") {
  const f = native("nemo_window")
  if (f) f(order)
}
