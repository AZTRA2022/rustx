# rustx documentation: pages still to write

Method: the navigation of nextjs.org/docs (App Router) and ui.shadcn.com/docs was read from their live indexes (https://nextjs.org/docs/llms.txt, https://ui.shadcn.com/llms.txt) and from both home pages, on 2026-11-07. The rustx side was compared against the page inventory supplied by the requester, not against the files in `site/content/docs` (being renamed). Slugs are proposals in English; sections follow the inventory.

Facts about rustx features (keychain, deep links, config file name, error codes, native menus) were not verified in the code. Each such assumption is flagged "if it exists" and must be checked before writing. A page about a feature that does not exist belongs in `docs/manques-du-framework.md`, not in the docs. Note also that `docs/publier.md` exists in the repo; if it already holds packaging material, page 1 should be built from it.

## Summary table

| # | Page | Slug | Answers to | Class |
|---|---|---|---|---|
| 1 | Packaging and distributing your app | `tooling/packaging` | Next: Building, Deploying, Production checklist | Needed |
| 2 | Where your code runs (Rust process vs CEF page) | `getting-started/where-code-runs` | Next: Server and Client Components | Needed |
| 3 | Calling Rust from JavaScript, and back | `js-libraries/rust-bridge` | Next: Route Handlers, Server Actions | Needed |
| 4 | Persisting data and app directories | `system/app-data` | Next: Mutating Data (local analogue) | Needed |
| 5 | Security model and permissions | `system/security` | Next: CSP, Data Security | Needed |
| 6 | Application menu and context menus | `system/menus` | shadcn: Menubar, Context Menu | Needed if feature exists |
| 7 | Theming and dark mode | `js-libraries/theming-dark-mode` | shadcn: Theming, Dark Mode | Needed |
| 8 | Forms | `getting-started/forms` | Next: Forms; shadcn: Forms | Needed |
| 9 | Static assets, images and fonts | `getting-started/assets` | Next: Image, Font, public | Needed |
| 10 | Configuration reference | `tooling/configuration` | next.config.js; components.json | Needed |
| 11 | API reference | `reference/*` | Next: API Reference | Needed |
| 12 | JS library compatibility matrix | `js-libraries/compatibility` | shadcn: Components, React 19, Tailwind v4 | Needed |
| 13 | Testing your app | `tooling/testing` | Next: Testing | Needed |
| 14 | DevTools and debugging | `tooling/debugging` | Next: Debugging | Needed |
| 15 | Environment variables, build-time constants | `tooling/environment` | Next: Environment Variables | Needed |
| 16 | Window title and metadata per route | `router/window-metadata` | Next: Metadata, generateMetadata | Needed unless covered |
| 17 | Special files and route groups cheat sheet | `router/special-files` | Next: File-system conventions, Route Groups | Needed unless covered |
| 18 | Upgrading rustx | `tooling/upgrading` | Next: Upgrading; shadcn: Changelog | Needed |
| 19 | Error handling beyond error.rsx | `getting-started/error-handling` | Next: Error Handling | Optional (high) |
| 20 | Coming from Electron, Tauri or a web app | `getting-started/migrating` | Next: Migrating | Optional (high) |
| 21 | Tutorial: build a small app | `getting-started/tutorial` | nextjs.org/learn | Optional (high) |
| 22 | Secure storage and sign-in | `system/secure-storage` | Next: Authentication | Optional |
| 23 | Deep links and file associations | `system/deep-links` | (desktop) | Optional |
| 24 | Internationalization and RTL | `guides/i18n` | Next: i18n; shadcn: RTL | Optional |
| 25 | Logging, tracing, crash reports | `tooling/observability` | Next: Instrumentation, OpenTelemetry | Optional |
| 26 | Performance and memory | `tooling/performance` | Next: Memory Usage, Lazy Loading | Optional |
| 27 | Accessibility | `guides/accessibility` | Next: Accessibility | Optional |
| 28 | Hot reload behaviour | `tooling/hot-reload` | Next: Fast Refresh | Optional |
| 29 | Workspaces and multi-crate apps | `tooling/workspaces` | shadcn: Monorepo | Optional |
| 30 | Compiler error index | `reference/errors` | Next: docs/messages | Optional (high if codes exist) |
| 31 | Styling beyond Tailwind | `js-libraries/css` | Next: CSS, Sass, CSS-in-JS | Optional |
| 32 | AI coding agents guide | `tooling/ai-agents` | Next: AI Coding Agents; shadcn: Skills, MCP | Optional |
| 33 | Window state persistence | `window/remember-state` | (desktop) | Optional |
| 34 | Glossary | `glossary` | Next: Glossary | Optional |
| 35 | Templates and starters | `getting-started/templates` | create-next-app; shadcn: Create | Optional |

---

## 1. Missing and clearly needed (ranked by value)

### 1. Packaging and distributing your app
Slug `tooling/packaging`. Answers Next "Building", "Deploying", "Production checklist", "Self-hosting". A desktop framework has no host to deploy to; the equivalent is shipping an installer. Cover a release build, bundling CEF with the binary (size, locale and resource files), an installer per platform (MSI/NSIS, dmg or app bundle, deb/AppImage), code signing and notarization, the app icon, and auto-update (or its absence), ending with a release checklist. This is the largest hole: a user can write an app but cannot hand it to anyone.

### 2. Where your code runs (Rust process vs CEF page)
Slug `getting-started/where-code-runs`. Answers Next "Server and Client Components", "Server and Client Boundary", "Rendering Philosophy". The mental-model page: what runs in the Rust process (state, tasks, OS access), what runs in the CEF renderer (React and JS libraries), how `.rsx` is compiled, and what crosses the boundary and at what cost. State that this replaces Next's server/client split and that there is no SSR or hydration.

### 3. Calling Rust from JavaScript, and back
Slug `js-libraries/rust-bridge`. Answers Next "Route Handlers", "Server Actions", "Mutating Data". The desktop equivalent of an API route: a Rust function invoked from a React component or JS library callback. Document the declaration syntax, which argument and return types serialize, async and cancellation, errors, and events pushed from Rust to the page. The existing "props that cross" and "a rustx component in React" pages cover only the component direction; link rather than duplicate.

### 4. Persisting data and app directories
Slug `system/app-data`. Answers Next "Mutating Data" (local analogue). Where to put settings, caches and user data (per-platform app-data directories), reading and writing from tasks, a worked settings file (serde with JSON or TOML) and a SQLite example, and format migration between app versions. The state pages cover memory only; nothing in the inventory covers disk.

### 5. Security model and permissions
Slug `system/security`. Answers Next "Content Security Policy", "Data Security", "Production checklist". What the page loaded in CEF may reach: which Rust functions are exposed, what OS APIs (clipboard, files) allow, handling of remote content and navigation to external URLs, the CSP in effect, why not to load untrusted URLs in the main window. Say plainly what is not protected. A framework that exposes OS access needs this page.

### 6. Application menu and context menus (if the feature exists in `rustx-os`)
Slug `system/menus`. Answers shadcn "Menubar", "Context Menu", "Dropdown Menu" (the web versions; desktop also needs native ones). Native menu bar (macOS top bar, Windows/Linux window menu), accelerators, native context menus, and when to use the shadcn components instead. The system section has tray and notifications but no menus. If the feature does not exist, record it as a gap.

### 7. Theming and dark mode
Slug `js-libraries/theming-dark-mode`. Answers shadcn "Theming" and "Dark Mode". The whole path: CSS variables and theme tokens in `components.json`, switching light and dark, following the OS theme live, syncing the native frame and title bar colour with the page theme, and no flash at startup.

### 8. Forms
Slug `getting-started/forms`. Answers Next "Forms" and shadcn "Forms" (React Hook Form, TanStack Form, Field). Forms in `.rsx` (controlled inputs bound to state, validation, submit running a task) and forms using shadcn `Field` with React Hook Form or TanStack Form in a React island, with the submit path into Rust (page 3). The events page covers events but not form patterns.

### 9. Static assets, images and fonts
Slug `getting-started/assets`. Answers Next "Image Optimization", "Font Optimization", `public`. Where files live, how they are bundled or shipped, how `src` paths resolve under the app's internal origin, bundled fonts (no network assumption), icons and SVG. There is no optimization pipeline or CDN: tell the user what to do instead (pre-size images, WebP). The logo page covers app identity only.

### 10. Configuration reference
Slug `tooling/configuration`. Answers Next "next.config.js" options and shadcn "components.json". Every setting of the project manifest and any rustx config (window defaults, bundle identifier, features, JS toolchain, component paths), with defaults and an example, using the real file name (verify in code). Today settings are scattered across feature pages.

### 11. API reference
Slug `reference/` section. Answers Next "API Reference" (Components, File conventions, Functions), about half of Next's docs. Provide a file-conventions table, router functions and hooks (equivalents of `useRouter`, `usePathname`, `useParams`, `useSearchParams`, `redirect`, `notFound`), state and task functions (keep, effect, task), window and OS signatures, and links to the rustdoc of the eight crates. Generate it from the code where possible.

### 12. JS library compatibility matrix
Slug `js-libraries/compatibility`. Answers shadcn "Components" (60+ pages), "React 19", "Tailwind CSS v4", "Manual Installation". Do not write a page per component. One table: which shadcn components and common npm libraries are tested, which have caveats (reliance on `next/*` imports, portals and focus in frameless windows), and the supported React and Tailwind versions. Note that Sonner/Toast are in-page and differ from native notifications.

### 13. Testing your app
Slug `tooling/testing`. Answers Next "Testing". Unit tests of Rust logic and state, testing `.rsx` components without a window, driving a real window end to end (the repo's engine tests share a harness; document the supported approach), CI setup, headless CEF on Linux CI.

### 14. DevTools and debugging
Slug `tooling/debugging`. Answers Next "Debugging", "Development Environment". Opening CEF DevTools, attaching a Rust debugger, mapping compiler diagnostics back to `.rsx` lines. Keep the troubleshooting page for known problems and this one for technique.

### 15. Environment variables and build-time constants
Slug `tooling/environment`. Answers Next "Environment Variables". Which variables the CLI reads, build-time constants versus runtime variables, why secrets must not be embedded in a distributed binary, `.env` handling in dev.

### 16. Window title and metadata per route
Slug `router/window-metadata`. Answers Next "Metadata and OG images", `generateMetadata`. Window title (and optionally icon) per route and dynamic from params. Drop SEO, OG and sitemaps. Write only if the router pages do not already cover it.

### 17. Special files and route groups cheat sheet
Slug `router/special-files`. Answers Next "File-system conventions", "Route Groups", "template.js", "default.js", "src". One table of every reserved file name and folder syntax rustx recognizes (dynamic, catch-all, groups, private folders if supported), and which Next features are unsupported (parallel and intercepting routes). People search for this page; the current split is by topic.

### 18. Upgrading rustx
Slug `tooling/upgrading`. Answers Next "Upgrading", "Codemods"; shadcn "Changelog". Version policy, how to update CLI and crates, breaking-change notes per release, link to the site changelog. The project changes fast, so users need a migration trail.

---

## 2. Missing but optional

19. **Error handling beyond error.rsx** (`getting-started/error-handling`, Next "Error Handling"): expected errors as `Result`, panics in tasks, what the boundary does and does not catch. High if panic behaviour surprises users.
20. **Coming from Electron, Tauri or a web app** (`getting-started/migrating`, Next "Migrating"): concept-mapping tables and a "port a React app" walk-through. High adoption value, low risk.
21. **Tutorial: build a small app** (`getting-started/tutorial`, nextjs.org/learn): about 30 minutes, touching routes, state, a task, a file dialog and packaging. Better after pages 1 to 4.
22. **Secure storage and sign-in** (`system/secure-storage`, Next "Authentication"): OS keychain for tokens, OAuth through the system browser with a loopback or deep-link callback. Only if a keychain API exists; otherwise a recipe.
23. **Deep links and file associations** (`system/deep-links`): open the app from a URL scheme or double-clicked file and hand it to the running instance (ties into single-instance). Only if implemented.
24. **Internationalization and RTL** (`guides/i18n`, Next "Internationalization", shadcn "RTL"): OS locale detection, message catalogs, `dir="rtl"`. Skip URL-based locale routing.
25. **Logging, tracing, crash reports** (`tooling/observability`, Next "Instrumentation", "OpenTelemetry", "Analytics"): `tracing` setup, log file location, opt-in crash reporting and analytics, privacy.
26. **Performance and memory** (`tooling/performance`, Next "Memory Usage", "Lazy Loading", "Package Bundling"): CEF footprint, startup time, re-renders, lazy-loading heavy JS, binary size.
27. **Accessibility** (`guides/accessibility`, Next architecture "Accessibility"): keyboard navigation, screen readers in CEF per platform, focus in frameless windows.
28. **Hot reload behaviour** (`tooling/hot-reload`, Next "Fast Refresh"): what `rustx dev` reloads without restart, what triggers a Rust rebuild. May be a section of the commands page.
29. **Workspaces and multi-crate apps** (`tooling/workspaces`, shadcn "Monorepo").
30. **Compiler error index** (`reference/errors`, Next `docs/messages/*`): only if the compiler emits stable codes; then one entry per code with cause and fix.
31. **Styling beyond Tailwind** (`js-libraries/css`, Next "CSS", "Sass", "CSS-in-JS"): global CSS, CSS modules, per-component styles; fold into the Tailwind page if short.
32. **AI coding agents guide** (`tooling/ai-agents`, Next "AI Coding Agents", "MCP"; shadcn "Skills", "MCP Server"): rules file for agents plus a pointer to `llms.txt`. Cheap, and models do not know `.rsx`.
33. **Window state persistence** (`window/remember-state`): remember size, position, maximised state across launches. No upstream equivalent; fold into page 4 if short.
34. **Glossary** (`glossary`, Next "Glossary").
35. **Templates and starters** (`getting-started/templates`, create-next-app, shadcn "Create"): only once more than one `rustx new` template exists.

---

## 3. Deliberately not applicable (do not copy)

### Next.js
- Caching, Revalidating, `use cache` (all variants), `cacheLife`, `cacheTag`, `revalidatePath`, `revalidateTag`, `updateTag`, `unstable_cache`, Cache Components, CDN Caching, CI Build Caching, How Revalidation Works: server and CDN caching of rendered output; nothing renders on a server.
- ISR (both pages), Keeping pages static, Public pages, Static Exports, SPAs, `output`, `generateStaticParams`, `dynamicParams`, `generateSitemaps`: static generation for web hosting; the app is always a local binary.
- Proxy / `proxy.js` (middleware), `NextRequest`, `NextResponse`, `headers`, `cookies`, `userAgent`, `connection`, Draft Mode: HTTP request pipeline; there are no inbound HTTP requests.
- Route Handlers as an HTTP API (`route.js`), Backend for Frontend, Custom Server, `after`, `useServerInsertedHTML`, Server Actions as HTTP endpoints: the Rust bridge (page 3) replaces them.
- Server Components, `use client`, `use server`, Streaming, Partial Prefetching, PPR, Instant navigation, Prefetching, Optimizing the static shell, `prefetch`, `staleTimes`: SSR and network prefetch; routing is local.
- Image component and `images` config, `next/font`, `ImageResponse`, `opengraph-image`, `twitter-image`, `robots.txt`, `sitemap.xml`, `manifest.json`, JSON-LD, `htmlLimitedBots`, `generateViewport`: image CDN, SEO and social previews; a local app has no crawlers. Page 9 covers the local need.
- Deploying, Deploying to Platforms, Self-Hosting, Adapters (13 pages), Multi-zones, Multi-tenant, `basePath`, `assetPrefix`, `deploymentId`, `distDir`, `redirects`, `rewrites`, `headers` config, `poweredByHeader`, `compress`, `trailingSlash`: web hosting and URL management; packaging (page 1) replaces them.
- Edge Runtime, `runtime`, `maxDuration`, `preferredRegion`, Turbopack options, webpack, Rspack, Next.js Compiler, Supported Browsers, `reactCompiler`, `transpilePackages`, `serverExternalPackages`, `typedRoutes`, ESLint/TypeScript config: Next's own toolchain; rustx has one engine (CEF) and its own compiler.
- PWAs, Offline support, `useOffline`, `useReportWebVitals`, `webVitalsAttribution`: a desktop app is installed and local already; Web Vitals do not map to a CEF window.
- `forbidden.js`, `unauthorized.js`, `authInterrupts`, Authentication with Cache Components: HTTP 401/403 UI. Page 22 covers desktop sign-in if wanted.
- Pages Router, MDX, `mdx-components.js`, Sass options, Tailwind v3, Lightning CSS, `cssChunking`, `inlineCss`: web build options or content features.
- Parallel Routes, Intercepting Routes: web modal-over-URL patterns, unsupported; state this on page 17 rather than document them.
- Third Party Libraries, Script component, Analytics scripts: web embed tooling.
- Upgrade guides for Next 14, 15 and 16, and Next's community pages: history of another product; rustx already has a contributing page.

### shadcn
- Installation for Next.js, Vite, Remix, Astro, Laravel, Gatsby, React Router, TanStack Router and Start, plus the matching Dark Mode and RTL variants: rustx has one host, and the `shadcn` page covers it.
- The 60-odd individual component pages: shadcn documents them; link out. Page 12 is the rustx-specific layer.
- Registry (Getting Started, registry.json, registry-item.json, Namespaces, Authentication, MCP, Open in v0, FAQ, Examples, Add a Registry), Directory, Figma, v0, Typeset: publishing and discovering component registries and design tooling. Revisit only if rustx ships its own registry.
- JavaScript (non-TypeScript) setup, Tailwind v4 and React 19 notes as separate pages: one line each in page 12.
- Charts and Blocks galleries: shadcn showcases. At most a block or two on the rustx showcase.

---

## Structural site pages (beyond docs)

Found: Next has docs, blog, showcase, learn, per-page `.md`, `/docs/llms.txt`, `/docs/llms-full.txt`, `/docs/messages`, governance, support policy, telemetry, versioned docs (`/docs/15`, `/docs/canary`) and a docs search API. shadcn has docs, components, blocks (`/blocks`), charts, directory, typeset, create (`/create`), changelog and `/llms.txt`.

| Upstream | rustx recommendation |
|---|---|
| `llms.txt`, `llms-full.txt`, per-page `.md` | Yes, high value and nearly free with fumadocs. rustx syntax is new to models, so it matters more than for Next. |
| Docs search | Yes (fumadocs provides it). Skip the OpenAPI search endpoint. |
| Changelog (shadcn `/docs/changelog`, Next blog releases) | Yes: `/changelog`, linked from page 18. Needed for a fast-moving 0.x project. |
| Blog (Next) | Optional; a changelog suffices until there is news. |
| Showcase | Already exists. |
| Learn course (Next) | Page 21, docs-only. |
| Create / templates (shadcn `/create`) | Optional (page 35); no web configurator. |
| Blocks, Charts, Components gallery | No. Page 12 is a table. Revisit when rustx-specific blocks exist (title bars, sidebars). |
| Directory, Themes, Colors | No. |
| FAQ | Optional: a short "Why rustx rather than Tauri or Electron" page linked from the home page. |
| Error messages index | See page 30, only with stable codes. |
| Versioned docs | Not before 1.0. |
| Governance, support policy, telemetry | A `/telemetry` note only if the CLI collects anything; a support note inside page 18. |
| Sitemap, OG images, robots | Yes as site housekeeping, not as docs pages. |
