# RustX Extension for VS Code

A complete development experience for RustX directly inside Visual Studio Code.

The RustX extension brings native language support to `.rsx` files, combining Rust syntax highlighting, intelligent code completion, real-time diagnostics, and seamless integration with the Rust development toolchain.

## Features

- **RSX Language Support** — Native `.rsx` file recognition, syntax highlighting, comment support (`//`, `/* */`), automatic indentation, bracket matching, and tag pairing.

- **Syntax Grammar** — A dedicated grammar combining Rust and RSX syntax. HTML-like elements, attributes, event handlers, and embedded Rust expressions are recognized independently, providing accurate syntax highlighting throughout the document.

- **Real-time Diagnostics** — Powered by `rustx-lsp`, the extension compiles RSX files as you type and reports compiler errors directly at their original source locations. No Cargo invocation is required when opening a file.

- **Intelligent RSX Completion** — Context-aware suggestions for HTML and SVG elements, attributes, event handlers, style properties, locally declared components, component parameters, and installed UI libraries. Component suggestions are derived directly from the compiler, ensuring that available completions reflect what RustX actually supports.

- **Rust Language Intelligence** — Integration with `rust-analyzer` for Rust code completion, hover information, and go-to-definition. The language server automatically manages communication with `rust-analyzer` using the generated Rust source, giving you access to your application's modules, RustX APIs, and dependencies.

- **Embedded Rust Completion** — Full Rust-aware completion inside RSX expressions, including type information and contextual suggestions. The extension reconstructs the expression under the cursor within its original application context, allowing `rust-analyzer` to provide accurate suggestions even inside markup.

- **Automatic Toolchain Integration** — The extension manages its language server and `rust-analyzer` integration automatically. If `rust-analyzer` is not available in your system PATH, you can specify its location through `rustx.rustAnalyzer.path`. RSX syntax support and compiler diagnostics remain available even without it.

## Requirements

- Rust toolchain.
- RustX CLI and compiler.
- Rust analyser extension.
- `rust-analyzer` for advanced Rust language intelligence.
- At least one successful application build for full type-aware completion inside embedded Rust expressions.

## Current Limitations

**No automatic imports yet.**

The extension supports path-based completion, such as `std::fs::read`, but does not currently provide automatic import insertion or unqualified symbol suggestions.

**Initial workspace loading.**

The first Rust completion request may take a few seconds while `rust-analyzer` initializes and loads the application workspace. The language server automatically retries the request to accommodate this initialization period.

## Built for RustX

The goal is simple: make `.rsx` development feel as natural as writing ordinary Rust, without sacrificing the flexibility of component-based interfaces.
