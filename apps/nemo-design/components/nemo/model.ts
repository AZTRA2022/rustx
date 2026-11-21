// The project: what a tab holds, and the reducer with undo and redo behind it.

export type SettingDef = {
  id: string
  label: string
  options: string[]
  color: string
}

// Every setting a project may have. The first five are there from the start;
// the "+" of the SETTINGS card adds the others.
export const SETTING_DEFS: SettingDef[] = [
  { id: "mode", label: "Mode", options: ["Fun", "Serious", "Cinematic"], color: "#f87171" },
  { id: "trim", label: "Trim", options: ["Auto", "Manual", "Off"], color: "#fbbf24" },
  { id: "think", label: "Think", options: ["Fast", "Balanced", "Deep"], color: "#4ade80" },
  { id: "voice", label: "Voice", options: ["Happy", "Calm", "Deep", "Off"], color: "#c4b5fd" },
  { id: "music", label: "Music", options: ["Piano", "Synth", "Drums", "Off"], color: "#7dd3fc" },
  { id: "light", label: "Light", options: ["Studio", "Warm", "Cold"], color: "#fde68a" },
  { id: "camera", label: "Camera", options: ["Orbit", "Front", "Side"], color: "#f9a8d4" },
]

export const DEFAULT_SETTINGS: Record<string, string> = {
  mode: "Fun",
  trim: "Auto",
  think: "Fast",
  voice: "Happy",
  music: "Piano",
  light: "Studio",
  camera: "Orbit",
}

export type NodeKind = "ideas" | "refs" | "settings" | "models"
export type GraphNode = { id: NodeKind; x: number; y: number }
export type Link = { from: NodeKind; to: NodeKind }

export type Project = {
  id: string
  name: string
  ideas: string
  ref: { name: string; url: string } | null
  nodes: GraphNode[]
  links: Link[]
  settingIds: string[] // the rows shown on the SETTINGS card, in order
  settings: Record<string, string>
  models: string[] // the AI MODELS chips
  model: string // the 3D model of the library
  animation: string // "" = the model's first
  duration: number // seconds
  clip: [number, number] // the visual clip on the timeline
  audio: [number, number] // the audio clip on the timeline
}

export const DURATIONS = [5, 10, 15, 30]

export function freshProject(name: string, patch: Partial<Project> = {}): Project {
  return {
    id: Math.random().toString(36).slice(2, 9),
    name,
    ideas: "",
    ref: null,
    nodes: [
      { id: "ideas", x: 40, y: 40 },
      { id: "refs", x: 40, y: 330 },
      { id: "settings", x: 520, y: 40 },
      { id: "models", x: 520, y: 420 },
    ],
    links: [
      { from: "ideas", to: "settings" },
      { from: "refs", to: "settings" },
      { from: "settings", to: "models" },
    ],
    settingIds: ["mode", "trim", "think", "voice", "music"],
    settings: { ...DEFAULT_SETTINGS },
    models: ["Seedance2"],
    model: "robot",
    animation: "",
    duration: 10,
    clip: [0, 10],
    audio: [2.2, 7.4],
    ...patch,
  }
}

export function starterProjects(): Project[] {
  return [
    freshProject("New Character", {
      ideas: "3D stylized cartoon robot, slightly short proportions, full body, big round eyes",
    }),
    freshProject("Framer Templates", {
      ideas: "A fox that runs through a studio, cinematic light",
      model: "fox",
      settings: { ...DEFAULT_SETTINGS, mode: "Cinematic", music: "Synth", light: "Warm" },
      duration: 15,
      clip: [0, 15],
      audio: [3, 12],
    }),
    freshProject("Untitled"),
  ]
}

// What the project is made of, as one object that can be saved and shared.
export type Workspace = { projects: Project[]; active: string }

// A change to the active project goes through here, so that undo sees it.
export type History = { past: Project[]; future: Project[] }

export function clamp(v: number, lo: number, hi: number) {
  return Math.min(hi, Math.max(lo, v))
}

export function fmt(seconds: number) {
  const s = Math.max(0, Math.floor(seconds))
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`
}
