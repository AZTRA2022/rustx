// The 3D models of the library. They are glTF files in assets/models, downloaded
// from the three.js and Khronos sample sets (see assets/models/LICENSES.txt).

export type ModelInfo = {
  id: string
  name: string
  url: string
  // Words in the idea that choose this model.
  words: string[]
  // The animation a new project starts on, when the file has it.
  prefer: string
}

export const LIBRARY: ModelInfo[] = [
  { id: "robot", name: "Robot Expressive", url: "/models/robot.glb", words: ["robot", "character", "boy", "girl", "person", "human", "cartoon"], prefer: "Idle" },
  { id: "fox", name: "Fox", url: "/models/fox.glb", words: ["fox", "animal", "dog", "wolf", "cat", "fur"], prefer: "Survey" },
  { id: "duck", name: "Rubber Duck", url: "/models/duck.glb", words: ["duck", "bird", "toy", "rubber", "yellow"], prefer: "" },
]

// The library model an idea asks for, or the current one when it names none.
export function modelFor(ideas: string, current: string): string {
  const text = ideas.toLowerCase()
  let best = ""
  let hits = 0
  for (const model of LIBRARY) {
    const n = model.words.filter((w) => new RegExp(`\\b${w}`).test(text)).length
    if (n > hits) {
      hits = n
      best = model.id
    }
  }
  return best || current
}
