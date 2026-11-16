// The frame around every screen: a bar with the app's name and a way
// back to the board, which stays while the screen underneath moves
// between the board and a project's own detail screen.

fn Layout(children: Element) -> Element {
    return <div style={{ minHeight: "100vh", backgroundColor: "#0b0c10", color: "#e7e8ee", fontFamily: "Segoe UI, system-ui, sans-serif" }}>
        <header style={{ display: "flex", alignItems: "center", gap: "12px", borderBottom: "1px solid #1d2230", padding: "12px 24px" }}>
            <button onClick={move |_| rustx::router::navigate("/".to_string())} style={{ background: "none", border: "none", color: "#e7e8ee", fontWeight: "600", cursor: "pointer", padding: "0" }}>
                MultiTask Hub
            </button>
        </header>
        {children}
    </div>;
}
