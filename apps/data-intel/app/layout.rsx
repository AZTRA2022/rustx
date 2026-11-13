// The frame around every screen: a title bar that stays while the
// page underneath changes between the table browser and a table's
// own detail screen.

fn Layout(children: Element) -> Element {
    return <div className="min-h-screen bg-background text-foreground">
        <header className="flex items-center gap-3 border-b border-border px-6 py-3">
            <button className="font-semibold tracking-tight" onClick={move |_| rustx::router::navigate("/".to_string())}>
                R Intelligent Data
            </button>
            <Badge>in-memory</Badge>
        </header>
        {children}
    </div>;
}
