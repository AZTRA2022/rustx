import * as React from "react"

// Where something is, when the number is not known until the page is running: a card dragged
// across the canvas, a playhead, a clip that is 37% of the timeline. A class cannot say it, and
// the page has no `style` attribute or `<style>` tag, so it is written to the element's own
// CSS object model (`element.style`) after each render. Everything that is not geometry is a
// Tailwind class.
export function useBox<T extends HTMLElement | SVGElement>(css: Record<string, string>) {
  const ref = React.useRef<T | null>(null)
  const key = JSON.stringify(css)
  React.useLayoutEffect(() => {
    const el = ref.current
    if (!el) return
    for (const [name, value] of Object.entries(css)) el.style.setProperty(name, value)
  }, [key]) // eslint-disable-line react-hooks/exhaustive-deps
  return ref
}

type BoxProps = { css: Record<string, string> } & React.HTMLAttributes<HTMLDivElement>

// A div whose geometry is `css`.
export const Box = React.forwardRef<HTMLDivElement, BoxProps>(function Box({ css, ...props }, forwarded) {
  const own = useBox<HTMLDivElement>(css)
  const set = (el: HTMLDivElement | null) => {
    own.current = el
    if (typeof forwarded === "function") forwarded(el)
    else if (forwarded) forwarded.current = el
  }
  return <div ref={set} {...props} />
})
