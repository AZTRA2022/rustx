// The page's half of the host: runs the commands the Rust side wrote, and
// says what happened. Injected before the page's own scripts, once per
// document. It holds no application state: ids and nodes, nothing else.
(function () {
  if (window.top !== window || window.__rustx) return;

  const nodes = new Map(); // id -> DOM node

  // The system's context menu is for text: cut, copy, paste. Anywhere else it
  // is a browser's menu (Reload, Inspect) in an application. The page may
  // still cancel it first, or ask for its own.
  document.addEventListener("contextmenu", (event) => {
    if (window.__rustx_devtools || event.defaultPrevented) return;
    const target = event.target;
    const editable = target instanceof Element && target.closest("input, textarea, [contenteditable]");
    if (!editable && String(window.getSelection()).length === 0) event.preventDefault();
  });
  const waiting = new Map(); // call id -> {resolve, reject}
  let calls = 0;
  const post = (message) => window.ipc.postMessage(JSON.stringify(message));

  // A node left the document: forget its id and the ids of everything
  // below it, so the table does not grow with every row ever drawn.
  function forget(root) {
    if (root.__rid !== undefined) nodes.delete(root.__rid);
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_ALL);
    while (walker.nextNode()) {
      const below = walker.currentNode;
      if (below.__rid !== undefined) nodes.delete(below.__rid);
    }
  }

  function bind(id, node) {
    node.__rid = id;
    nodes.set(id, node);
    return node;
  }

  // What the host may know about an event: a short list, not the event.
  function record(kind, event) {
    const target = event.target || {};
    // The element the handler is on: where the pointer is in it, and how big it is.
    const at = event.currentTarget;
    const box = at && at.getBoundingClientRect ? at.getBoundingClientRect() : null;
    const number = (value) => (Number.isFinite(value) ? value : 0);
    const x = number(event.clientX);
    const y = number(event.clientY);
    return {
      kind,
      x,
      y,
      local_x: box ? x - box.left : 0,
      local_y: box ? y - box.top : 0,
      width: box ? box.width : 0,
      height: box ? box.height : 0,
      button: number(event.button),
      delta_x: number(event.deltaX),
      delta_y: number(event.deltaY),
      current_id: (at && at.id) || "",
      value: target.value === undefined || target.value === null ? "" : String(target.value),
      id: target.id || "",
      key: event.key || "",
      checked: !!target.checked,
      shift: !!event.shiftKey,
      control: !!event.ctrlKey,
      alt: !!event.altKey,
      meta: !!event.metaKey,
    };
  }

  // What a library passes to a prop, as the host may know it: values stay,
  // an event becomes the short record of an event, and what cannot cross
  // (a node, a function, a cycle) becomes null rather than refusing the call.
  function wire(value, depth) {
    if (value === null || value === undefined) return null;
    switch (typeof value) {
      case "string":
      case "boolean":
        return value;
      case "number":
        return Number.isFinite(value) ? value : null;
      case "object":
        break;
      default:
        return null;
    }
    if (depth > 8 || value instanceof Node) return null;
    if (Array.isArray(value)) return value.map((each) => wire(each, depth + 1));
    if ("target" in value && ("nativeEvent" in value || "type" in value)) return record(String(value.type || ""), value);
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) return null;
    return Object.fromEntries(Object.entries(value).map(([name, each]) => [name, wire(each, depth + 1)]));
  }

  // The reverse of what the host wrote: `{__rustx_cb: n}` is a function
  // that says it was called.
  function revive(value) {
    if (Array.isArray(value)) return value.map(revive);
    if (value === null || typeof value !== "object") return value;
    if ("__rustx_cb" in value) {
      const id = value.__rustx_cb;
      return (...args) => post({ t: "callback", id, args: args.map((each) => wire(each, 0)) });
    }
    return Object.fromEntries(Object.entries(value).map(([name, each]) => [name, revive(each)]));
  }

  // A page may write `file://`, which the application's own origin may not
  // load. The address is sent to the host's file route instead, wherever it
  // appears: an attribute set from Rust, one a library set, inline CSS, and
  // what a script fetches.
  const local = location.origin === window.__rustx_origin;
  function localUrl(value) {
    if (!local || typeof value !== "string" || !/^file:\/\//i.test(value)) return value;
    const path = value.slice(7).replace(/^\/+/, "").split(/[?#]/)[0];
    let plain = path;
    try {
      plain = decodeURI(path);
    } catch (_) {}
    return window.__rustx_origin + "/__rustx/file/" + plain.split("/").map(encodeURIComponent).join("/");
  }
  const URL_ATTRIBUTES = new Set(["src", "href", "poster", "data"]);
  function rewritten(name, value) {
    if (typeof value !== "string") return value;
    if (URL_ATTRIBUTES.has(name)) return localUrl(value);
    if (name === "style" && /file:\/\//i.test(value))
      return value.replace(/url\(\s*(["']?)(file:\/\/[^"')]+)\1\s*\)/gi, (_, quote, address) => "url(" + quote + localUrl(address) + quote + ")");
    return value;
  }
  if (local) {
    const fix = (element) => {
      for (const name of [...URL_ATTRIBUTES, "style"]) {
        const value = element.getAttribute && element.getAttribute(name);
        if (value && rewritten(name, value) !== value) element.setAttribute(name, rewritten(name, value));
      }
    };
    // The document itself is observed, so this holds from the first byte:
    // a script that runs while the page is still loading is covered.
    new MutationObserver((records) => {
        for (const record of records) {
          if (record.type === "attributes") fix(record.target);
          for (const added of record.addedNodes) {
            if (added.nodeType !== 1) continue;
            fix(added);
            added.querySelectorAll("[src],[href],[poster],[data],[style]").forEach(fix);
          }
        }
    }).observe(document, {
      attributes: true,
      attributeFilter: [...URL_ATTRIBUTES, "style"],
      childList: true,
      subtree: true,
    });
    // A form that submits replaces the document, and with it every node the
    // host holds an id for. The handlers still run; the navigation does not.
    // Rust cannot cancel it (its handler runs after a round trip), so it is
    // decided here, once, for every form of the application's own pages.
    document.addEventListener("submit", (event) => event.preventDefault());
    const fetched = window.fetch;
    window.fetch = (input, init) =>
      fetched.call(window, typeof input === "string" ? localUrl(input) : input instanceof URL ? localUrl(input.href) : input, init);
    const opened = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function (method, url, ...rest) {
      return opened.call(this, method, localUrl(String(url)), ...rest);
    };
  }

  // Events that come in bursts, held until the next frame.
  const BURSTS = new Set(["mousemove", "pointermove", "wheel", "scroll", "drag", "dragover", "touchmove"]);
  const pending = new Map();
  let scheduled = false;
  function flush() {
    scheduled = false;
    for (const message of pending.values()) post(message);
    pending.clear();
  }

  // An element of an SVG drawing is made in the SVG namespace, or the browser
  // takes `<path>` for an unknown HTML element and draws nothing. Names both
  // languages have (`a`, `title`, `style`, `script`) stay HTML.
  const SVG = "http://www.w3.org/2000/svg";
  const SVG_TAGS = new Set(
    ("svg g defs symbol use path rect circle ellipse line polyline polygon text tspan textPath image " +
      "linearGradient radialGradient stop pattern clipPath mask marker filter foreignObject " +
      "feBlend feColorMatrix feComponentTransfer feComposite feConvolveMatrix feDiffuseLighting " +
      "feDisplacementMap feDropShadow feFlood feGaussianBlur feImage feMerge feMergeNode feMorphology " +
      "feOffset feSpecularLighting feTile feTurbulence animate animateMotion animateTransform set mpath").split(" ")
  );

  const isField = (node) => /^(INPUT|TEXTAREA|SELECT)$/.test(node.tagName || "");

  const commands = {
    // create an element / a text node / bind an element the page already has
    el: (id, tag) => bind(id, SVG_TAGS.has(tag) ? document.createElementNS(SVG, tag) : document.createElement(tag)),
    tx: (id, text) => bind(id, document.createTextNode(text)),
    by: (id, elementId) => bind(id, document.getElementById(elementId)),

    st: (id, text) => {
      const node = nodes.get(id);
      for (const child of Array.from(node.childNodes)) forget(child);
      node.textContent = text;
    },
    sa: (id, name, value) => {
      const node = nodes.get(id);
      // `xlink:href` lives in its own namespace in an SVG drawing.
      if (name.startsWith("xlink:")) node.setAttributeNS("http://www.w3.org/1999/xlink", name, value);
      else node.setAttribute(name, rewritten(name, value));
    },
    ra: (id, name) => nodes.get(id).removeAttribute(name),

    // A live property. `ack` is the last input the host has handled for
    // this node. A field somebody is typing in has newer input than that,
    // and what the host sends is older than what is on screen: keep it.
    sp: (id, name, value, ack) => {
      const node = nodes.get(id);
      if (name === "value" && typeof value === "string") {
        if (ack < (node.__seq || 0)) return;
        if (node.value === value) return;
      }
      node[name] = URL_ATTRIBUTES.has(name) ? localUrl(value) : value;
    },

    ap: (parent, child) => nodes.get(parent).appendChild(nodes.get(child)),
    ib: (parent, child, before) =>
      nodes.get(parent).insertBefore(nodes.get(child), before === null ? null : nodes.get(before)),
    rm: (id) => {
      const node = nodes.get(id);
      if (!node) return;
      if (node.parentNode) node.parentNode.removeChild(node);
      forget(node);
    },

    // React owns this node: `window.__rustx_react` is the framework's
    // runtime. `fu` comes before the node leaves, so cleanups run.
    fm: (id, name, props, children) => {
      const bridge = window.__rustx_react;
      if (!bridge) {
        post({ t: "error", message: "a node React owns was rendered, and the page has no window.__rustx_react" });
        return;
      }
      if (bridge.mount(name, nodes.get(id), revive(props), revive(children)) === false) {
        post({ t: "error", message: "the page has no React component named '" + name + "'" });
      }
    },
    fu: (id) => {
      const node = nodes.get(id);
      if (window.__rustx_react && node) window.__rustx_react.unmount(node);
    },

    // One listener per node and kind, for as long as the node lives.
    on: (id, kind) => {
      const node = nodes.get(id);
      node.addEventListener(kind, (event) => {
        const message = { t: "event", node: id, name: kind, seq: 0, event: record(kind, event) };
        // A pointer that moves or a wheel that turns fires faster than the
        // host can answer, and each event is a message and a render. One per
        // frame is sent, the last, with the turns of a wheel added up.
        if (BURSTS.has(kind)) {
          const key = id + ":" + kind;
          const before = pending.get(key);
          if (before && kind === "wheel") {
            message.event.delta_x += before.event.delta_x;
            message.event.delta_y += before.event.delta_y;
          }
          pending.set(key, message);
          if (!scheduled) {
            scheduled = true;
            requestAnimationFrame(flush);
          }
          return;
        }
        // Whatever was waiting happened first.
        flush();
        // A field's input is numbered, so the host can say which input a
        // value it sends back already includes.
        if (isField(node) && (kind === "input" || kind === "change")) {
          node.__seq = (node.__seq || 0) + 1;
          message.seq = node.__seq;
        }
        post(message);
      });
    },
  };

  window.__rustx = {
    // A native is a synchronous request to the host, so the function the
    // page gets answers with its value, or throws. Only the application's
    // own pages get them: arguments are never sent to another origin.
    natives(names, asynchronous) {
      if (location.origin !== window.__rustx_origin) return;
      // An asynchronous native answers with a promise, and the page keeps
      // running while the host works: the answer comes back to `settle`.
      for (const name of asynchronous || []) {
        window[name] = (...args) =>
          new Promise((resolve, reject) => {
            const id = ++calls;
            waiting.set(id, { resolve, reject });
            post({ t: "call", id, name, args: args.map((each) => wire(each, 0)) });
          });
      }
      for (const name of names) {
        window[name] = (...args) => {
          const request = new XMLHttpRequest();
          // The arguments go in the address when they are small: WebKitGTK
          // hands a custom protocol no request body. A large one goes in the
          // body, which the other systems deliver.
          const json = JSON.stringify(args);
          const inAddress = json.length < 200000;
          request.open(
            "POST",
            window.__rustx_origin + "/__rustx/native/" + encodeURIComponent(name) + (inAddress ? "?a=" + encodeURIComponent(json) : ""),
            false
          );
          request.send(inAddress ? "" : json);
          const answer = JSON.parse(request.responseText);
          if ("err" in answer) throw new Error(answer.err);
          return answer.ok;
        };
      }
    },
    settle(id, ok, value) {
      const call = waiting.get(id);
      if (!call) return;
      waiting.delete(id);
      if (ok) call.resolve(value);
      else call.reject(new Error(String(value)));
    },
    apply(ops) {
      for (const op of ops) {
        try {
          commands[op[0]](...op.slice(1));
        } catch (error) {
          post({ t: "error", message: String(error), op: String(op[0]) });
        }
      }
    },
  };

  // What the page's own scripts throw would otherwise stay in a console
  // nobody opened.
  window.addEventListener("error", (event) => post({ t: "error", message: String(event.message) }));
  window.addEventListener("unhandledrejection", (event) =>
    post({ t: "error", message: "unhandled rejection: " + String(event.reason) })
  );

  function ready() {
    const ids = Array.from(document.querySelectorAll("[id]"), (element) => element.id);
    post({ t: "ready", ids });
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", ready);
  } else {
    ready();
  }
})();
