// The frame around every screen of this app: a bar that counts how
// many times a screen has been opened since the window launched, a
// value that lives here — in the layout — rather than in any one
// page, so it survives moving from the overview to an agent's detail
// screen and back.

let mut visits: u32 = 0;

fn Layout(children: Element) -> Element {
    effect((), || {
        visits += 1;
    });

    return <div className="min-h-screen bg-background text-foreground">
        <header className="flex items-center justify-between border-b border-border px-6 py-3">
            <button className="font-semibold tracking-tight" onClick={move |_| rustx::router::navigate("/".to_string())}>
                AI Agent Manager
            </button>
            <span className="text-xs text-muted-foreground">{visits} screen(s) opened this session</span>
        </header>
        <main>{children}</main>
    </div>;
}
