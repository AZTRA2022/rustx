// KIT — the same roster again, read as what each person has signed
// out, with a real total at the foot.
//
// The kit list is the production's own. What is not copy is the
// checkbox: a returned item leaves that person's row and the total at
// the foot drops, for as long as the window is open.

let mut departments: Vec<Department> = Vec::new();
let mut note: String = String::new();
let mut returned: Vec<String> = Vec::new();

fn Page(slug: String) -> Element {
    let crew = departments
        .iter()
        .find(|held| held.slug == slug)
        .map(|held| held.crew.clone())
        .unwrap_or_default();

    let back = returned.clone();
    let out: usize = crew
        .iter()
        .enumerate()
        .map(|(index, person)| kit_for(index, &person.role).iter().filter(|item| !back.contains(&tag(&person.id, item))).count())
        .sum();

    return <div key={format!("kit-{slug}")}>
        <div className="flex items-center rounded-md border border-border bg-[#0f131a] px-3 py-2.5">
            <span className="text-[11px] text-muted">
                {format!("{out} item(s) still signed out across {} people.", crew.len())}
            </span>
            <button
                className="ml-auto shrink-0 rounded-md border border-border bg-[#11151b] px-2.5 py-[5px] text-[10px] text-foreground hover:border-faint"
                onClick={move |_| returned = Vec::new()}
            >
                Sign everything back out
            </button>
        </div>

        <div className="mt-3 flex flex-col gap-1.5">
            {crew.iter().enumerate().map(|(index, person)| Row(index, person.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn Row(index: usize, person: Person) -> Element {
    let items = kit_for(index, &person.role);
    let back = returned.clone();
    let still: Vec<String> = items.iter().filter(|item| !back.contains(&tag(&person.id, item))).cloned().collect();
    let going = format!("/crew/{}", person.id);

    return <div key={person.id.clone()} className="rounded-md border border-border bg-[#0f131a] px-3 py-2">
        <div className="flex items-center gap-2.5">
            <button className="flex items-center gap-2.5" onClick={move |_| rustx::router::navigate(going.clone())}>
                <span style={avatar_paint(&person.name, 20)}>{initials(&person.name)}</span>
                <span className="text-[12px]">{person.name.clone()}</span>
            </button>
            <span className="ml-auto text-[11px] text-muted">{format!("{} of {} out", still.len(), items.len())}</span>
        </div>
        <div className="mt-2 flex flex-wrap gap-1.5">
            {items.iter().map(|item| Item(person.id.clone(), item.clone(), person.name.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn Item(id: String, item: String, who: String) -> Element {
    let back = returned.contains(&tag(&id, &item));
    let classes = format!(
        "rounded-[4px] border px-2 py-[3px] text-[10px] {}",
        if back { "border-border text-faint line-through" } else { "border-[#243040] text-foreground" }
    );
    let tagged = tag(&id, &item);
    let named = item.clone();
    return <button key={tag(&id, &item)} className={classes} onClick={move |_| hand_back(tagged.clone(), named.clone(), who.clone())}>
        {item}
    </button>;
}

fn hand_back(tagged: String, item: String, who: String) {
    let mut all = returned.clone();
    if let Some(at) = all.iter().position(|held| held == &tagged) {
        all.remove(at);
        note = format!("{who} signs the {item} back out");
    } else {
        all.push(tagged);
        note = format!("{who} returned the {item}");
    }
    returned = all;
}

fn tag(id: &str, item: &str) -> String {
    format!("{id}/{item}")
}

/// What a role carries. Short lists, so a roster of twelve does not
/// become a wall.
fn kit_for(index: usize, role: &str) -> Vec<String> {
    let common = ["radio", "torch", "hi-vis"];
    let lead = ["body", "prime set", "monitor", "light meter"];
    let operator = ["body", "zoom", "matte box"];
    let trainee = ["slate", "tape"];
    let own: &[&str] = match role {
        "Lead" => &lead,
        "Trainee" => &trainee,
        _ => &operator,
    };
    own.iter()
        .chain(common.iter().take(1 + index % 3))
        .map(|item| item.to_string())
        .collect()
}
