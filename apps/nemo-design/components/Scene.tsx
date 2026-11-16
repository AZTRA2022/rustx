// A real three.js viewport: a renderer, a cube mesh, a light, and an
// animation loop. Props come from the Rust page as plain values; this
// component reads them from a ref each frame so the render loop does
// not need to restart every time one changes.
import { useEffect, useRef } from "react"
import * as THREE from "three"

type SceneProps = {
  color?: string
  wireframe?: boolean
  spin?: number
}

export default function Scene({ color = "#8b5cf6", wireframe = false, spin = 0.6 }: SceneProps) {
  const mountRef = useRef<HTMLDivElement>(null)
  const propsRef = useRef({ color, wireframe, spin })
  propsRef.current = { color, wireframe, spin }

  useEffect(() => {
    const mount = mountRef.current
    if (!mount) return

    const width = mount.clientWidth || 480
    const height = mount.clientHeight || 360

    const scene = new THREE.Scene()
    scene.background = new THREE.Color(0x0a0a12)

    const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 100)
    camera.position.set(2.4, 2.0, 3.2)
    camera.lookAt(0, 0, 0)

    const renderer = new THREE.WebGLRenderer({ antialias: true })
    renderer.setSize(width, height)
    mount.appendChild(renderer.domElement)

    const geometry = new THREE.BoxGeometry(1.4, 1.4, 1.4)
    const material = new THREE.MeshStandardMaterial({
      color: propsRef.current.color,
      wireframe: propsRef.current.wireframe,
    })
    const mesh = new THREE.Mesh(geometry, material)
    scene.add(mesh)

    const light = new THREE.DirectionalLight(0xffffff, 1.2)
    light.position.set(3, 4, 5)
    scene.add(light)
    scene.add(new THREE.AmbientLight(0x404050))

    let running = true
    let frame = 0
    const animate = () => {
      if (!running) return
      mesh.rotation.y += 0.01 * propsRef.current.spin
      mesh.rotation.x += 0.004 * propsRef.current.spin
      material.color.set(propsRef.current.color)
      material.wireframe = propsRef.current.wireframe
      renderer.render(scene, camera)
      frame = requestAnimationFrame(animate)
    }
    animate()

    return () => {
      running = false
      cancelAnimationFrame(frame)
      mount.removeChild(renderer.domElement)
      geometry.dispose()
      material.dispose()
      renderer.dispose()
    }
    // Only the mount point matters for (re)creating the renderer; the
    // live values are read from propsRef every frame instead.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return <div ref={mountRef} style={{ width: "100%", height: "360px", borderRadius: "10px", overflow: "hidden" }} />
}
