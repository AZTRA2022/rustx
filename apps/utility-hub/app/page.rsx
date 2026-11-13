// Utility Hub.
//
// Three plain lists — notifications, invoices, automations — and one
// overview that rolls them up, laid out with shadcn/ui's `Tabs`,
// `Card`, `Table` and `Badge` on Tailwind v4 rather than hand-rolled
// look-alikes, and a real `recharts::BarChart` for the paid-vs-
// outstanding figure. Nothing is network-backed: pressing "notify"
// raises a real desktop notification, which stands in honestly for
// the email/WhatsApp send the showcase copy promises.

#[derive(rustx::ToValue, Clone)]
struct Bar {
    label: String,
    amount: f64,
}

#[derive(Clone)]
struct Notice {
    text: String,
    read: bool,
}

#[derive(Clone)]
struct Invoice {
    client: String,
    amount: f64,
    status: String,
}

#[derive(Clone)]
struct Automation {
    name: String,
    enabled: bool,
}

let mut notices: Vec<Notice> = vec![
    Notice { text: "deploy to production finished".to_string(), read: false },
    Notice { text: "new signup: Nova Textiles".to_string(), read: false },
    Notice { text: "weekly backup completed".to_string(), read: true },
];
let mut invoices: Vec<Invoice> = vec![
    Invoice { client: "Nova Textiles".to_string(), amount: 1200.0, status: "draft".to_string() },
    Invoice { client: "Arclight Studio".to_string(), amount: 480.0, status: "sent".to_string() },
    Invoice { client: "Birchwood".to_string(), amount: 2150.0, status: "paid".to_string() },
];
let mut automations: Vec<Automation> = vec![
    Automation { name: "auto-remind overdue invoices".to_string(), enabled: true },
    Automation { name: "post deploys to #tech".to_string(), enabled: true },
    Automation { name: "weekly finance digest".to_string(), enabled: false },
];

fn Page() -> Element {
    return <div className="mx-auto max-w-5xl px-6 py-8">
        <h1 className="text-2xl font-semibold tracking-tight">Utility Hub</h1>
        <p className="mt-1 text-sm text-muted-foreground">tech and finance, one dashboard.</p>

        <Tabs defaultValue="overview" className="mt-6">
            <TabsList>
                <TabsTrigger value="overview">overview</TabsTrigger>
                <TabsTrigger value="notifications">notifications</TabsTrigger>
                <TabsTrigger value="invoices">invoices</TabsTrigger>
                <TabsTrigger value="automations">automations</TabsTrigger>
            </TabsList>
            <TabsContent value="overview" className="mt-4">{Overview()}</TabsContent>
            <TabsContent value="notifications" className="mt-4">{Notifications()}</TabsContent>
            <TabsContent value="invoices" className="mt-4">{Invoices()}</TabsContent>
            <TabsContent value="automations" className="mt-4">{Automations()}</TabsContent>
        </Tabs>
    </div>;
}

fn Overview() -> Element {
    let unread = notices.iter().filter(|notice| !notice.read).count();
    let active_automations = automations.iter().filter(|automation| automation.enabled).count();
    let paid: f64 = invoices.iter().filter(|invoice| invoice.status == "paid").map(|invoice| invoice.amount).sum();
    let outstanding: f64 = invoices.iter().filter(|invoice| invoice.status != "paid").map(|invoice| invoice.amount).sum();
    let data = vec![Bar { label: "paid".to_string(), amount: paid }, Bar { label: "outstanding".to_string(), amount: outstanding }];

    return <div className="flex flex-col gap-5">
        <div className="grid grid-cols-3 gap-3">
            {Tile("unread".to_string(), unread.to_string())}
            {Tile("automations on".to_string(), format!("{}/{}", active_automations, automations.len()))}
            {Tile("outstanding".to_string(), format!("${:.0}", outstanding))}
        </div>

        <Card className="border-border bg-card">
            <CardHeader>
                <CardTitle className="text-xs uppercase tracking-wider text-muted-foreground">paid vs outstanding</CardTitle>
            </CardHeader>
            <CardContent>
                <recharts::BarChart width={600} height={180} data={data}>
                    <recharts::XAxis dataKey="label" stroke="#52525b" />
                    <recharts::YAxis stroke="#52525b" />
                    <recharts::Bar dataKey="amount" fill="#34d399" />
                </recharts::BarChart>
            </CardContent>
        </Card>
    </div>;
}

fn Tile(label: String, value: String) -> Element {
    return <Card className="border-border bg-card">
        <CardHeader>
            <CardTitle className="text-[11px] uppercase tracking-wider text-muted-foreground">{label}</CardTitle>
        </CardHeader>
        <CardContent className="text-2xl font-semibold">{value}</CardContent>
    </Card>;
}

fn Notifications() -> Element {
    return <ul id="notifications" className="flex flex-col gap-2">
        {notices.iter().enumerate().map(|(index, notice)| NoticeRow(index, notice.clone())).collect::<Vec<_>>()}
    </ul>;
}

fn NoticeRow(index: usize, notice: Notice) -> Element {
    return <li key={index.to_string()} className="flex items-center justify-between rounded-lg border border-border bg-card px-3 py-2.5">
        <span className={if notice.read { "text-muted-foreground" } else { "" }}>{notice.text.clone()}</span>
        {if !notice.read {
            <Button variant="outline" className="h-7 px-2 text-xs" onClick={move |_| {
                if let Some(notice) = notices.get_mut(index) { notice.read = true; }
            }}>mark read</Button>
        } else {
            <Badge variant="secondary">read</Badge>
        }}
    </li>;
}

fn Invoices() -> Element {
    return <Table>
        <TableHeader>
            <TableRow>
                <TableHead>client</TableHead>
                <TableHead className="text-right">amount</TableHead>
                <TableHead>status</TableHead>
                <TableHead></TableHead>
            </TableRow>
        </TableHeader>
        <TableBody>
            {invoices.iter().enumerate().map(|(index, invoice)| InvoiceRow(index, invoice.clone())).collect::<Vec<_>>()}
        </TableBody>
    </Table>;
}

fn InvoiceRow(index: usize, invoice: Invoice) -> Element {
    let next_status = match invoice.status.as_str() {
        "draft" => "sent",
        "sent" => "paid",
        _ => "draft",
    };
    return <TableRow key={index.to_string()}>
        <TableCell>{invoice.client.clone()}</TableCell>
        <TableCell className="text-right">{format!("${:.2}", invoice.amount)}</TableCell>
        <TableCell><Badge variant={if invoice.status == "paid" { "default" } else { "secondary" }}>{invoice.status.clone()}</Badge></TableCell>
        <TableCell className="flex justify-end gap-2">
            <Button variant="outline" className="h-7 px-2 text-xs" onClick={move |_| {
                if let Some(invoice) = invoices.get_mut(index) { invoice.status = next_status.to_string(); }
            }}>mark {next_status}</Button>
            <Button variant="ghost" className="h-7 px-2 text-xs" onClick={move |_| {
                let _ = rustx::notify::show(rustx::notify::What::new(
                    "Utility Hub",
                    &format!("Invoice for {} would be emailed/WhatsApp'd here.", invoice.client),
                ));
            }}>notify client</Button>
        </TableCell>
    </TableRow>;
}

fn Automations() -> Element {
    return <ul id="automations" className="flex flex-col gap-2">
        {automations.iter().enumerate().map(|(index, automation)| AutomationRow(index, automation.clone())).collect::<Vec<_>>()}
    </ul>;
}

fn AutomationRow(index: usize, automation: Automation) -> Element {
    return <li key={index.to_string()} className="flex items-center justify-between rounded-lg border border-border bg-card px-3 py-2.5">
        <span>{automation.name.clone()}</span>
        <Button
            className={if automation.enabled { "h-7 rounded-full bg-emerald-800 px-3 text-xs text-emerald-200 hover:bg-emerald-800" } else { "h-7 rounded-full bg-secondary px-3 text-xs text-muted-foreground" }}
            onClick={move |_| {
                if let Some(automation) = automations.get_mut(index) { automation.enabled = !automation.enabled; }
            }}>
            {if automation.enabled { "on" } else { "off" }}
        </Button>
    </li>;
}
