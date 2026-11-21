// The 3D viewport: a glTF model, its animation driven by the timeline's playhead,
// orbit controls, picking, and the light, camera and quality the settings ask for.
import * as React from "react"
const { forwardRef, useEffect, useImperativeHandle, useRef } = React
import * as THREE from "three"
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js"
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js"
import { LIBRARY } from "./library"
import type { Transport } from "./transport"

export type ViewportProps = {
  transport: Transport
  model: string
  animation: string
  mode: string
  think: string
  light: string
  camera: string
  reference: string | null
  tool: string
  clip: [number, number]
  onAnimations: (names: string[], chosen: string) => void
  onPick: (name: string) => void
}

export type ViewportHandle = { canvas: () => HTMLCanvasElement | null }

const LIGHTS: Record<string, { key: number; fill: number; ambient: number; bg: number }> = {
  Studio: { key: 0xffffff, fill: 0x9db4ff, ambient: 0x505060, bg: 0x0b0b0d },
  Warm: { key: 0xffd2a1, fill: 0xff9a6b, ambient: 0x4a3a30, bg: 0x120d0a },
  Cold: { key: 0xbfd9ff, fill: 0x6a8cff, ambient: 0x2a3550, bg: 0x090b12 },
}

const SPEED: Record<string, number> = { Fun: 1.25, Serious: 1, Cinematic: 0.7 }

const Viewport = forwardRef<ViewportHandle, ViewportProps>(function Viewport(props, ref) {
  const mount = useRef<HTMLDivElement>(null)
  const live = useRef(props)
  live.current = props
  const api = useRef<{ load: (id: string) => void; apply: () => void; canvas: HTMLCanvasElement | null }>({
    load: () => {},
    apply: () => {},
    canvas: null,
  })

  useImperativeHandle(ref, () => ({ canvas: () => api.current.canvas }), [])

  useEffect(() => {
    const el = mount.current!
    const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true })
    renderer.shadowMap.enabled = true
    el.appendChild(renderer.domElement)
    renderer.domElement.className = "block size-full"
    api.current.canvas = renderer.domElement

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 100)
    camera.position.set(0, 1.4, 5.2)
    const controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = true
    controls.target.set(0, 1.0, 0)

    const key = new THREE.DirectionalLight(0xffffff, 2.4)
    key.position.set(3, 5, 4)
    key.castShadow = true
    const fill = new THREE.DirectionalLight(0x9db4ff, 1.0)
    fill.position.set(-4, 2, -3)
    const ambient = new THREE.AmbientLight(0x505060, 1.4)
    scene.add(key, fill, ambient)

    const floor = new THREE.Mesh(new THREE.CircleGeometry(2.2, 48), new THREE.ShadowMaterial({ opacity: 0.35 }))
    floor.rotation.x = -Math.PI / 2
    floor.receiveShadow = true
    scene.add(floor)

    let current: THREE.Object3D | null = null
    let mixer: THREE.AnimationMixer | null = null
    let action: THREE.AnimationAction | null = null
    let clips: THREE.AnimationClip[] = []
    let loadedId = ""
    const loader = new GLTFLoader()
    let loadToken = 0

    const play = (name: string) => {
      if (!mixer || !clips.length) return
      const clip = clips.find((c) => c.name === name) ?? clips[0]
      action?.stop()
      action = mixer.clipAction(clip)
      action.play()
      action.paused = true
    }

    const load = (id: string) => {
      const info = LIBRARY.find((m) => m.id === id) ?? LIBRARY[0]
      loadedId = info.id
      const token = ++loadToken
      loader.load(info.url, (gltf) => {
        if (token !== loadToken) return
        if (current) scene.remove(current)
        current = gltf.scene
        // Fit it: two units tall, standing on the floor, centred.
        const box = new THREE.Box3().setFromObject(current)
        const size = box.getSize(new THREE.Vector3())
        const scale = 2 / (size.y || 1)
        current.scale.setScalar(scale)
        const fitted = new THREE.Box3().setFromObject(current)
        const centre = fitted.getCenter(new THREE.Vector3())
        current.position.set(-centre.x, -fitted.min.y, -centre.z)
        current.traverse((o) => {
          if ((o as THREE.Mesh).isMesh) {
            o.castShadow = true
            o.receiveShadow = true
          }
        })
        scene.add(current)
        mixer = new THREE.AnimationMixer(current)
        clips = gltf.animations
        const names = clips.map((c) => c.name)
        const wanted = live.current.animation && names.includes(live.current.animation) ? live.current.animation : names.includes(info.prefer) ? info.prefer : names[0] ?? ""
        live.current.onAnimations(names, wanted)
        play(wanted)
      })
    }

    const size = () => {
      const w = el.clientWidth || 600
      const h = el.clientHeight || 400
      renderer.setSize(w, h, false)
      camera.aspect = w / h
      camera.updateProjectionMatrix()
    }
    const observer = new ResizeObserver(size)
    observer.observe(el)
    size()

    const applySettings = () => {
      const p = live.current
      const l = LIGHTS[p.light] ?? LIGHTS.Studio
      key.color.setHex(l.key)
      fill.color.setHex(l.fill)
      ambient.color.setHex(l.ambient)
      const pr = p.think === "Deep" ? 2 : p.think === "Balanced" ? 1.5 : 1
      renderer.setPixelRatio(Math.min(pr, window.devicePixelRatio * 1.0 || 1) || 1)
      renderer.shadowMap.enabled = p.think !== "Fast"
      key.castShadow = p.think !== "Fast"
      controls.enabled = p.tool !== "select"
      if (p.camera === "Front") camera.position.set(0, 1.2, 5.2)
      if (p.camera === "Side") camera.position.set(5.2, 1.2, 0)
      if (p.camera !== "Orbit") controls.target.set(0, 1.0, 0)
      if (p.reference) {
        new THREE.TextureLoader().load(p.reference, (t) => {
          t.colorSpace = THREE.SRGBColorSpace
          scene.background = t
        })
      } else {
        scene.background = new THREE.Color(l.bg)
      }
      if (loadedId !== p.model) load(p.model)
      else if (clips.length) {
        const names = clips.map((c) => c.name)
        if (p.animation && names.includes(p.animation) && action?.getClip().name !== p.animation) play(p.animation)
      }
    }
    api.current.load = load
    api.current.apply = applySettings
    load(live.current.model)
    applySettings()

    // Picking, in select mode: the part under the pointer is named and lit.
    const ray = new THREE.Raycaster()
    let lit: THREE.Mesh | null = null
    let litMaterial: THREE.Material | THREE.Material[] | null = null
    const onClick = (e: MouseEvent) => {
      if (live.current.tool !== "select" || !current) return
      const r = renderer.domElement.getBoundingClientRect()
      ray.setFromCamera(new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1), camera)
      const hit = ray.intersectObject(current, true)[0]
      if (lit && litMaterial) lit.material = litMaterial
      lit = null
      if (!hit) return live.current.onPick("")
      const mesh = hit.object as THREE.Mesh
      litMaterial = mesh.material
      const glow = (Array.isArray(mesh.material) ? mesh.material[0] : mesh.material).clone() as THREE.MeshStandardMaterial
      if ("emissive" in glow) glow.emissive = new THREE.Color(0x6d5dfc)
      mesh.material = glow
      lit = mesh
      live.current.onPick(mesh.name || mesh.parent?.name || "mesh")
    }
    renderer.domElement.addEventListener("click", onClick)

    let raf = 0
    let last = performance.now()
    const frame = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000)
      last = now
      const p = live.current
      const t = p.transport.time
      if (mixer && action) {
        // The animation follows the playhead, inside the clip the timeline shows.
        const len = action.getClip().duration
        const local = Math.max(0, t - p.clip[0]) * (SPEED[p.mode] ?? 1)
        mixer.setTime(len > 0 ? local % len : 0)
      } else if (current && p.transport.playing) {
        current.rotation.y += dt * 1.2
      }
      if (p.camera === "Orbit" && p.transport.playing) controls.autoRotate = true
      else controls.autoRotate = false
      controls.autoRotateSpeed = p.mode === "Cinematic" ? 1.2 : 3
      controls.update()
      renderer.render(scene, camera)
      raf = requestAnimationFrame(frame)
    }
    raf = requestAnimationFrame(frame)

    return () => {
      cancelAnimationFrame(raf)
      observer.disconnect()
      renderer.domElement.removeEventListener("click", onClick)
      controls.dispose()
      renderer.dispose()
      el.removeChild(renderer.domElement)
    }
  }, [])

  // Settings and the choice of model are applied when they change.
  const p = props
  useEffect(() => {
    api.current.apply()
  }, [p.model, p.animation, p.mode, p.think, p.light, p.camera, p.reference, p.tool])

  return <div ref={mount} className="absolute inset-0" />
})

export default Viewport
