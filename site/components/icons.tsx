import { FileIcon, FileJsonIcon, FileTextIcon, TerminalIcon } from "lucide-react"

type IconProps = React.HTMLAttributes<SVGElement>

/**
 * The site's mark: a chevron and a hook, that is a tag and
 * Rust — nothing borrowed from anyone.
 */
export const Icons = {
  logo: (props: IconProps) => (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <path d="M7 4H5a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h2" />
      <path d="m11 8 4 4-4 4" />
      <path d="M17 4h2a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-2" />
    </svg>
  ),
  repository: (props: IconProps) => (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
      <path d="m3.3 7 8.7 5 8.7-5" />
      <path d="M12 22V12" />
    </svg>
  ),
}

export function getIconForLanguageExtension(language: string) {
  switch (language) {
    case "rust":
    case "rs":
    case "rsx":
      return <FileIcon />
    case "json":
    case "toml":
      return <FileJsonIcon />
    case "sh":
    case "bash":
    case "shell":
      return <TerminalIcon />
    default:
      return <FileTextIcon />
  }
}
