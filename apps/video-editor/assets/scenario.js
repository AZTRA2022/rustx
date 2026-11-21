// Driven runs of the editor, for the tests in `tests/`. A scenario is named
// by `--scenario=<name>`; the page reports what it saw with `rustx_report`
// and ends with `done`. Nothing here runs in a normal start.
(() => {
  if (typeof rustx_scenario !== "function") return;
  const scenario = rustx_scenario();
  if (!scenario) return;

  const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
  const report = (key, value) => rustx_report(key, String(value));
  const wait = async (selector, ms = 20000, test = () => true) => {
    for (let waited = 0; waited < ms; waited += 50) {
      const found = document.querySelector(selector);
      if (found && test(found)) return found;
      await sleep(50);
    }
    return null;
  };
  // What a person does: put text in a field the way a keyboard does, press a button.
  const type = async (selector, text) => {
    const field = await wait(selector);
    field.focus();
    field.value = text;
    field.dispatchEvent(new Event("input", { bubbles: true }));
    await sleep(150);
  };
  const press = async (selector) => {
    const button = await wait(selector);
    button.click();
    await sleep(150);
  };

  const importClip = async (path) => {
    await type("#typed", path);
    await press("#add-typed");
    report("clip_name", (await wait("#clip-name"))?.textContent);
    return wait("#monitor", 30000, (each) => each.readyState >= 2);
  };
  // A pointer event at a fraction of an element's width, as a hand would make one.
  const pointer = (element, kind, fraction, extra = {}) => {
    const box = element.getBoundingClientRect();
    element.dispatchEvent(
      new MouseEvent(kind, {
        bubbles: true,
        clientX: box.left + box.width * fraction,
        clientY: box.top + box.height / 2,
        ...extra,
      })
    );
  };

  const scenarios = {
    // Scrub the timeline with the pointer, and zoom it with the wheel.
    async scrub(path) {
      const video = await importClip(path);
      const track = await wait("#track");
      const at = () => (document.querySelector("#monitor")?.currentTime ?? -1).toFixed(1);
      pointer(track, "mousedown", 0.5);
      await sleep(700);
      report("at_middle", at());
      pointer(track, "mousemove", 0.75);
      pointer(track, "mousemove", 0.8);
      await sleep(700);
      report("dragged", at());
      pointer(track, "mouseup", 0.8);
      pointer(track, "mousemove", 0.1);
      await sleep(500);
      report("after_release", at());
      for (let i = 0; i < 3; i++) track.dispatchEvent(new WheelEvent("wheel", { bubbles: true, ctrlKey: true, deltaY: -100 }));
      await sleep(600);
      const wide = document.querySelector("#track").getBoundingClientRect().width;
      report("zoom", (wide / document.querySelector("#track-scroll").clientWidth).toFixed(2));
    },

    // Move a Radix slider with the keyboard, which is how its own tests do it,
    // and read what Rust made of it.
    async grade(path) {
      await importClip(path);
      const thumb = await wait("#brightness [role=slider], [id=brightness] [role=slider], [role=slider]");
      const readout = () => document.querySelector("#brightness-value")?.textContent;
      report("brightness_start", readout());
      thumb.focus();
      for (let i = 0; i < 4; i++) {
        thumb.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
        await sleep(120);
      }
      await sleep(500);
      report("brightness_after", readout());
      report("thumb_after", thumb.getAttribute("aria-valuenow"));
      const monitor = document.querySelector("#monitor");
      report("monitor_style_attribute", monitor ? monitor.getAttribute("style") : "no monitor");
      report("monitor_css", monitor?.style.filter || "");
    },

    // Four key presses before Rust has answered any of them: a drag is faster
    // than the round trip, and a controlled component reads the value it was
    // last given.
    async grade_burst(path) {
      await importClip(path);
      const thumb = await wait("[role=slider]");
      thumb.focus();
      // One task each, a few milliseconds apart: faster than the host's answer,
      // slower than a single task, which no user can produce.
      for (let i = 0; i < 4; i++) {
        thumb.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
        await sleep(4);
      }
      await sleep(900);
      report("burst_brightness", document.querySelector("#brightness-value")?.textContent);
    },

    // Two clips, the second moved before the first, then the switch on the row
    // that is now second: it must switch off the clip that row names.
    async layers(paths) {
      const [first, second] = paths.split("|");
      await importClip(first);
      await type("#typed", second);
      await press("#add-typed");
      await sleep(600);
      const rows = () => [...document.querySelectorAll("#layers [data-state]")].map((each) => each.getAttribute("data-state"));
      const names = () => [...document.querySelectorAll("#layers [id^=layer-row-]")].map((each) => each.id.replace("layer-row-", ""));
      report("rows_before", names().join(","));
      await press("#move-left");
      await sleep(600);
      report("rows_after_move", names().join(","));
      // The row now second is the clip that was first.
      const switches = [...document.querySelectorAll("#layers button[role=switch]")];
      switches[1].click();
      await sleep(600);
      report("states_after_toggle", names().map((name, at) => name + "=" + document.querySelectorAll("#layers button[role=switch]")[at].getAttribute("data-state")).join(","));
    },

    // The editing of a cut: its pictures, the razor, undo and redo, a new project.
    async edit(path) {
      await importClip(path);
      const cuts = () => document.querySelectorAll("#track > div:first-child > div").length;
      const pictures = (selector) => [...document.querySelectorAll(selector)].filter((img) => img.complete && img.naturalWidth > 0).length;
      // ffmpeg draws the filmstrip and the waveform of the cut, one picture at a time.
      for (let i = 0; i < 200 && (pictures("#track img") < 2); i++) await sleep(100);
      report("filmstrip_and_wave", pictures("#track img"));
      report("cuts_before", cuts());
      // The razor cuts where the playhead is.
      const slider = await wait("#playhead");
      const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
      set.call(slider, String(slider.max / 2));
      slider.dispatchEvent(new Event("input", { bubbles: true }));
      await sleep(500);
      const menu = async (trigger, label) => {
        (await wait(trigger)).dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, pointerType: "mouse" }));
        await sleep(300);
        const item = [...document.querySelectorAll('[role="menuitem"]')].find((e) => e.textContent.includes(label));
        item.click();
        await sleep(500);
      };
      await menu("#tool", "Razor");
      report("cuts_after_razor", cuts());
      await press("#undo");
      report("cuts_after_undo", cuts());
      await press("#redo");
      report("cuts_after_redo", cuts());
      await press("#new-project");
      report("cuts_after_new_project", cuts());
      await press("#undo");
      report("cuts_after_undoing_it", cuts());
    },

    // Import a file by its path, and play it.
    async import(path) {
      const video = await importClip(path);
      report("decodes", !!video);
      report("duration", video ? video.duration.toFixed(1) : "none");
      await press("#play");
      await sleep(1500);
      const moved = document.querySelector("#monitor");
      report("played", moved && !moved.paused && moved.currentTime > 0.3);
      report("status", document.querySelector("#status")?.textContent || "");
    },
  };

  const [name, ...rest] = scenario.split(":");
  (async () => {
    report("started", name);
    try {
      if (scenarios[name]) await scenarios[name](rest.join(":"));
      else report("error", "no scenario " + name);
    } catch (error) {
      report("error", String(error));
    }
    report("done", "1");
  })();
})();
