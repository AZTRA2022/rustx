// PRICING — three plans, and a line about which one this window is.

let mut chosen: String = "CUSTODY".to_string();
let mut said: String = String::new();
let mut yearly: bool = true;

fn Page() -> Element {
    let all = plans();
    let picked = all.iter().find(|plan| plan.name == chosen).cloned();

    return <div key="pricing" id="screen-pricing" className="plain">
        <div className="row">
            <div>
                <div className="label">PRICING</div>
                <h1 className="h1" style={{ marginTop: "8px" }}>Three ways to hold a key</h1>
                <p className="lead" style={{ marginTop: "10px", maxWidth: "560px" }}>
                    The one running in this window is SINGLE NODE, and it is the whole product on
                    one machine. The other two are what an institution buys around it.
                </p>
            </div>
            <button className="ghost small" style={{ marginLeft: "auto" }} onClick={move |_| yearly = !yearly}>
                {if yearly { "BILLED YEARLY" } else { "BILLED MONTHLY" }}
            </button>
        </div>

        <div className="grid3" style={{ marginTop: "26px", alignItems: "stretch" }}>
            {all.iter().map(|plan| Card(plan.clone())).collect::<Vec<_>>()}
        </div>

        <div className="panel" style={{ marginTop: "22px" }}>
            <div className="label">WHAT YOU CHOSE</div>
            {match picked {
                Some(plan) => Chosen(plan),
                None => Nothing(),
            }}
        </div>
    </div>;
}

fn Card(plan: Plan) -> Element {
    let picked = chosen == plan.name;
    let border = if plan.headline || picked { "#0a0a0a" } else { "#e4e4e4" };
    let paint = format!(
        "border:1px solid {border};background:#ffffff;padding:18px;display:flex;flex-direction:column"
    );
    let name = plan.name.clone();

    return <div key={plan.name.clone()} style={paint}>
        <div className="row">
            <span style={{ fontSize: "12px", letterSpacing: "0.1em" }}>{plan.name.clone()}</span>
            {if plan.headline {
                <span className="chip on" style={{ marginLeft: "auto" }}>MOST CHOSEN</span>
            } else {
                <span></span>
            }}
        </div>
        <div style={{ fontSize: "24px", marginTop: "14px" }}>{plan.price.clone()}</div>
        <div style={{ fontSize: "10px", color: "#9a9a9a", marginTop: "3px" }}>{plan.per.clone()}</div>
        <p style={{ fontSize: "12px", lineHeight: "1.65", color: "#3a3a3a", margin: "12px 0 0" }}>{plan.about.clone()}</p>
        <ul className="list" style={{ marginTop: "12px", display: "flex", flexDirection: "column", gap: "6px" }}>
            {plan.lines.iter().map(|line| Line(plan.name.clone(), line.clone())).collect::<Vec<_>>()}
        </ul>
        <span style={{ marginTop: "auto", paddingTop: "16px" }}>
            {if picked {
                <button className="ghost" style={{ width: "100%" }} onClick={move |_| go(name.clone())}>CHOSEN</button>
            } else {
                <button className="cta" style={{ width: "100%" }} onClick={move |_| go(name.clone())}>CHOOSE</button>
            }}
        </span>
    </div>;
}

fn Line(plan: String, text: String) -> Element {
    return <li key={format!("{plan}-{text}")} style={{ display: "flex", gap: "7px", fontSize: "11px", lineHeight: "1.55", color: "#3a3a3a" }}>
        <span style={{ color: "#1f9cf0" }}>+</span>
        <span>{text}</span>
    </li>;
}

fn Chosen(plan: Plan) -> Element {
    return <div key={plan.name.clone()}>
        <p style={{ fontSize: "13px", lineHeight: "1.75", margin: "10px 0 0" }}>
            {if plan.name == "SINGLE NODE" {
                "SINGLE NODE is what this window is. Nothing to buy and nothing to activate — open the vault and start sealing.".to_string()
            } else {
                format!("{} is sold by conversation, not by form. The contact page collects what a custody officer would need to quote it, and puts it on your clipboard or in a file.", plan.name)
            }}
        </p>
        <div className="row" style={{ marginTop: "14px" }}>
            {if plan.name == "SINGLE NODE" {
                <button className="cta" onClick={move |_| rustx::router::navigate("/vault".to_string())}>Open the vault</button>
            } else {
                <button className="cta" onClick={move |_| rustx::router::navigate("/contact".to_string())}>Talk to custody</button>
            }}
            <button className="ghost" onClick={move |_| rustx::router::navigate("/security".to_string())}>What is actually protecting this</button>
        </div>
        <p className="readout" style={{ marginTop: "12px" }}>{said.clone()}</p>
    </div>;
}

fn Nothing() -> Element {
    return <p style={{ fontSize: "12px", color: "#9a9a9a", margin: "10px 0 0" }}>Choose a plan above.</p>;
}

fn go(name: String) {
    chosen = name.clone();
    said = format!("{name} is the plan this page is quoting.");
}
