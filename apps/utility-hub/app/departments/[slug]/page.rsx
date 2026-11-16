// CREW — the roster of one department, which is the screen the mock
// is of.
//
// A row is a link: it opens that person's own page at `/crew/<id>`.
// The three buttons on the right do not navigate, they act — the
// address on the real clipboard, a real desktop notification, and a
// line filed in Recent Activity.

let mut departments: Vec<Department> = Vec::new();
let mut activity: Vec<Happened> = Vec::new();
let mut note: String = String::new();
let mut search: String = String::new();

fn Page(slug: String) -> Element {
    let department = departments.iter().find(|held| held.slug == slug).cloned();
    let needle = search.to_lowercase();
    let crew: Vec<Person> = department
        .map(|held| held.crew)
        .unwrap_or_default()
        .into_iter()
        .filter(|person| {
            needle.is_empty()
                || person.name.to_lowercase().contains(&needle)
                || person.email.to_lowercase().contains(&needle)
        })
        .collect();

    return <div key={format!("crew-{slug}")}>
        <table id="crew" className="w-full text-left">
            <thead>
                <tr className="text-[11px] text-muted">
                    <th className="border-b border-border pb-2 font-normal">Name</th>
                    <th className="border-b border-border pb-2 font-normal">Email</th>
                    <th className="border-b border-border pb-2 font-normal">Role</th>
                    <th className="border-b border-border pb-2 text-right font-normal">Actions</th>
                </tr>
            </thead>
            <tbody>
                {crew.iter().map(|person| Row(person.clone())).collect::<Vec<_>>()}
            </tbody>
        </table>
        {if crew.is_empty() {
            <p className="mt-4 text-[11px] text-faint">
                {if needle.is_empty() {
                    "This department has nobody in it yet.".to_string()
                } else {
                    format!("Nobody in this department matches “{}”.", search.clone())
                }}
            </p>
        } else {
            <span></span>
        }}
    </div>;
}

fn Row(person: Person) -> Element {
    let badge = format!(
        "border-radius:4px;padding:2px 9px;font-size:11px;background-color:{};color:{}",
        role_background(&person.role),
        role_ink(&person.role)
    );
    let going = format!("/crew/{}", person.id);
    let to_copy = person.email.clone();
    let to_ping = person.name.clone();
    let to_note = person.name.clone();

    return <tr key={person.id.clone()} className="text-[12px] hover:bg-[#11151b]">
        <td className="py-[7px]">
            <button className="flex items-center gap-2.5 text-left hover:text-white" onClick={move |_| rustx::router::navigate(going.clone())}>
                <span style={avatar_paint(&person.name, 22)}>{initials(&person.name)}</span>
                <span>{person.name.clone()}</span>
            </button>
        </td>
        <td className="py-[7px] text-foreground">{person.email.clone()}</td>
        <td className="py-[7px]"><span style={badge}>{person.role.clone()}</span></td>
        <td className="py-[7px]">
            <span className="flex items-center justify-end gap-3.5">
                <button className="text-faint hover:text-foreground" onClick={move |_| copy_email(to_copy.clone())}>
                    <Copy size={13} color="#5a6572" />
                </button>
                <button className="text-faint hover:text-foreground" onClick={move |_| ping(to_ping.clone())}>
                    <Mail size={13} color="#5a6572" />
                </button>
                <button className="text-faint hover:text-foreground" onClick={move |_| remark(to_note.clone())}>
                    <MessageSquare size={13} color="#5a6572" />
                </button>
            </span>
        </td>
    </tr>;
}

fn copy_email(email: String) {
    match rustx::clipboard::set_text(&email) {
        Ok(()) => note = format!("{email} is on the clipboard"),
        Err(error) => note = format!("the clipboard refused: {error}"),
    }
}

/// What a local demo can honestly send where the product sends mail: a
/// real notification from this machine, named as one.
fn ping(who: String) {
    match rustx::notify::show(rustx::notify::What::new(
        "Reelhouse",
        format!("Call sheet sent to {who}."),
    )) {
        Ok(()) => note = format!("a notification went out about {who}"),
        Err(error) => note = format!("the notification failed: {error}"),
    }
    remember("was sent the call sheet".to_string(), who);
}

fn remark(who: String) {
    remember("was flagged for a note".to_string(), who.clone());
    note = format!("a note was filed on {who}");
}

fn remember(what: String, who: String) {
    let mut all = activity.clone();
    all.insert(0, Happened { who, what, when_said: "just now".to_string(), week: false });
    all.truncate(14);
    activity = all;
}
