// A driven run of the editor, for the tests in CI. A scenario is named by `--scenario=<name>`; the
// page reports what it saw with `rustx_report` and ends with `done`. Nothing here runs in a normal
// start.
(() => {
  if (typeof rustx_scenario !== "function") return;
  const scenario = rustx_scenario();
  if (scenario !== "nemo") return;

  const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
  const report = (key, value) => rustx_report(key, String(value));
  const wait = async (find, ms = 20000) => {
    for (let waited = 0; waited < ms; waited += 50) {
      const found = find();
      if (found) return found;
      await sleep(50);
    }
    return null;
  };
  const q = (selector) => () => document.querySelector(selector);
  const label = (text) => () => document.querySelector(`[aria-label="${text}"]`);
  const click = async (find) => {
    const el = await wait(find);
    if (!el) throw new Error("no element for " + find);
    el.click();
    await sleep(200);
  };
  // Radix opens a menu on the press, not on the click.
  const open = async (find) => {
    const el = await wait(find);
    el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, pointerType: "mouse" }));
    await sleep(250);
  };
  const choose = async (text) => {
    const item = await wait(() => [...document.querySelectorAll('[role="menuitem"]')].find((e) => e.textContent.trim() === text));
    item.click();
    await sleep(250);
  };
  const code = () => document.body.textContent;

  (async () => {
    try {
      await wait(label("Export"));
      await sleep(1500);
      report("tabs", document.querySelectorAll('[role="tab"]').length);
      report("cards", document.querySelectorAll("[data-node]").length);
      report("wires", [...document.querySelectorAll("svg path")].filter((p) => p.getAttribute("stroke") === "transparent").length);

      // A new project is a new tab.
      await click(label("New project"));
      report("tabs_after_new", document.querySelectorAll('[role="tab"]').length);

      // An idea that names a model of the library changes the model, and Undo and Redo walk back and forth.
      const ideas = await wait(q("[data-node=ideas] textarea"));
      ideas.focus();
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value").set.call(ideas, "A fox in a studio");
      ideas.dispatchEvent(new Event("input", { bubbles: true }));
      await sleep(150);
      ideas.blur();
      await sleep(400);
      report("fox_after_idea", code().includes("fox-character"));
      await click(label("Undo"));
      report("robot_after_undo", code().includes("robot-character") && !code().includes("fox-character"));
      await click(label("Redo"));
      report("fox_after_redo", code().includes("fox-character"));

      // A card moves with its header.
      const card = await wait(q("[data-node=refs]"));
      const before = card.getBoundingClientRect().left;
      const head = card.firstElementChild.getBoundingClientRect();
      const at = (x, y) => ({ bubbles: true, clientX: x, clientY: y, pointerId: 1, pointerType: "mouse", button: 0 });
      card.firstElementChild.dispatchEvent(new PointerEvent("pointerdown", at(head.left + 20, head.top + 10)));
      window.dispatchEvent(new PointerEvent("pointermove", at(head.left + 120, head.top + 10)));
      window.dispatchEvent(new PointerEvent("pointerup", at(head.left + 120, head.top + 10)));
      await sleep(300);
      report("card_moved", Math.round(document.querySelector("[data-node=refs]").getBoundingClientRect().left - before) > 20);

      // A wire is removed with a double click.
      const wire = [...document.querySelectorAll("svg path")].find((p) => p.getAttribute("stroke") === "transparent");
      wire.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      await sleep(300);
      report("wires_after_remove", [...document.querySelectorAll("svg path")].filter((p) => p.getAttribute("stroke") === "transparent").length);

      // A setting is added from the "+" of the card.
      await open(() => document.querySelector('[data-node=settings] [aria-label="Add a setting"]'));
      await choose("Light");
      report("light_row", document.querySelector("[data-node=settings]").textContent.includes("Light"));

      // Play moves the playhead; Pause stops it.
      await click(label("Play"));
      await wait(label("Pause"));
      await sleep(1500);
      report("playing", !!document.querySelector('[aria-label="Pause"]'));
      await click(label("Pause"));
      report("playhead", (document.body.textContent.match(/0:0[1-9]/g) || []).length > 0);

      // Export: five seconds, recorded for real, written through the native (stood in for here).
      if (typeof MediaRecorder === "undefined") {
        report("export", "no MediaRecorder");
      } else {
        await open(label("Duration"));
        await choose("5 seconds");
        window.nemo_save = (name, base64, extension) => {
          report("export_ext", extension);
          report("export_bytes", base64.length);
          return "scenario";
        };
        await click(label("Export"));
        const finished = await wait(() => document.body.textContent.includes("Exported") && true, 30000);
        report("exported", !!finished);
      }
      report("done", "yes");
    } catch (e) {
      report("error", e && e.message ? e.message : e);
      report("done", "no");
    }
  })();
})();
