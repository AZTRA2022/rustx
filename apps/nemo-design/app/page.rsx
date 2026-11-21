// Nemo Design. The editor is `components/NemoApp.tsx`: the graph, the viewport and the
// timeline are React and three.js, because they are made of pointer drags and a render
// loop. Files, dialogs and the clipboard are Rust (src/main.rs), called from the page.

fn Page() -> Element {
    return <div id="nemo"><NemoApp /></div>;
}
