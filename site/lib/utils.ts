import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function absoluteUrl(path: string) {
  const base =
    process.env.NEXT_PUBLIC_APP_URL ?? "https://rustx.dev"

  return `${base}${path}`
}
