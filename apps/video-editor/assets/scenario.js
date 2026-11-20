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
    try {
      if (scenarios[name]) await scenarios[name](rest.join(":"));
      else report("error", "no scenario " + name);
    } catch (error) {
      report("error", String(error));
    }
    report("done", "1");
  })();
})();
