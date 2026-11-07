// Many libraries, one page.
//
// Each one is a different shape of the same problem:
//
//   lucide-react       components whose props are numbers and strings
//   react-markdown     a component whose content is its children
//   @radix-ui/react-tabs  a library that keeps state of its own, around
//                      components that rustx keeps state in
//
// What this page cannot do yet is the point of it: a library component
// that takes a callback has nothing to be given, because a Rust closure
// does not cross. That is written down, not hidden.

let mut clicks = 0;
let mut chosen = "one".to_string();

/// The page.
///
/// `onValueChange` is a prop the library calls: the closure stays in
/// Rust, and what crosses is a function that calls it — so the page
/// learns what the user chose inside a tree it does not own.
fn Page() -> Element {
    let mut x = 0;

    return <div id="zoo">
        <h1 id="title">Zoo</h1>
        <h1>the value of x:{x}</h1>
        <section id="icons">
            <Rocket size={28} color="#a78bfa" strokeWidth={2} />
            <Boxes size={28} color="#34d399" />
            <Gauge size={28} color="#f472b6" />
        </section>

        <section id="markdown">
            <Markdown>{"# Un titre\n\nEt du **gras**, écrit en Rust."}</Markdown>
        </section>

        <section id="tabs">
            <Tabs.Root defaultValue="one" onValueChange={move |value: rustx::Value| {
                if let rustx::Value::String(value) = value {
                    chosen = value;
                }
            }}>
                <Tabs.List>
                    <Tabs.Trigger id="tab-one" value="one">un</Tabs.Trigger>
                    <Tabs.Trigger id="tab-two" value="two">deux</Tabs.Trigger>
                </Tabs.List>
                <Tabs.Content id="panel-one" value="one">
                    le premier panneau
                </Tabs.Content>
                <Tabs.Content id="panel-two" value="two">
                    le second panneau
                </Tabs.Content>
            </Tabs.Root>
        </section>

        <section id="rust">
            <span id="chosen">{chosen}</span>
            <button id="click" onClick={move |_| clicks += 1}>cliquer</button>
            <span id="clicks">{clicks}</span>
        </section>
    </div>;
}
