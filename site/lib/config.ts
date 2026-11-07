export const siteConfig = {
  name: "rustx",
  url: process.env.NEXT_PUBLIC_APP_URL ?? "https://rustx.dev",
  description:
    "Native software, written in Rust, drawn by a browser engine. .rsx pages, a state in let mut, and npm libraries from tags.",
  links: {
    // The repository is not public yet: the packages, however, are.
    // `NEXT_PUBLIC_REPO_URL` replaces this link when the repository exists.
    repository: process.env.NEXT_PUBLIC_REPO_URL ?? "https://crates.io/crates/rustx-app",
    crates: "https://crates.io/crates/rustx-app",
  },
  navItems: [
    { href: "/", label: "Home" },
    { href: "/docs", label: "Documentation" },
    { href: "/showcase", label: "Showcase" },
    { href: "/blog", label: "Blog" },
    { href: "/changelog", label: "Changelog" },
    { href: "/faq", label: "FAQ" },
  ],
}

export const META_THEME_COLORS = {
  light: "#ffffff",
  dark: "#0a0a0a",
}
