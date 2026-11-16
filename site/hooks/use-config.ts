"use client"

import * as React from "react"

type Config = {
  packageManager: "npm" | "yarn" | "pnpm" | "bun"
  installationType: "cli" | "manual"
}

const STORAGE_KEY = "rustx-site-config"

const DEFAULT_CONFIG: Config = {
  packageManager: "npm",
  installationType: "cli",
}

/**
 * Le choix de gestionnaire de paquets, et l'onglet d'installation.
 *
 * shadcn tient ceci dans un atome `jotai` persisté ; une seule valeur
 * ne vaut pas une dépendance de plus, donc c'est un état de React et
 * `localStorage`, avec un événement pour que deux blocs de code de la
 * même page bougent ensemble.
 */
export function useConfig(): [Config, (config: Config) => void] {
  const [config, setState] = React.useState<Config>(DEFAULT_CONFIG)

  React.useEffect(() => {
    const read = () => {
      try {
        const stored = window.localStorage.getItem(STORAGE_KEY)
        if (stored) {
          setState({ ...DEFAULT_CONFIG, ...JSON.parse(stored) })
        }
      } catch {}
    }

    read()
    window.addEventListener(STORAGE_KEY, read)
    return () => window.removeEventListener(STORAGE_KEY, read)
  }, [])

  const setConfig = React.useCallback((next: Config) => {
    setState(next)
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
    } catch {}
    window.dispatchEvent(new Event(STORAGE_KEY))
  }, [])

  return [config, setConfig]
}
