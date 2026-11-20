// The page's half of the host: runs the commands the Rust side wrote, and
// says what happened. Injected before the page's own scripts, once per
// document. It holds no application state: ids and nodes, nothing else.
(function () {
  if (window.top !== window || window.__rustx) return;

  const nodes = new Map(); // id -> DOM node
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
    return {
      kind,
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

  const isField = (node) => /^(INPUT|TEXTAREA|SELECT)$/.test(node.tagName || "");

  const commands = {
    // create an element / a text node / bind an element the page already has
    el: (id, tag) => bind(id, document.createElement(tag)),
    tx: (id, text) => bind(id, document.createTextNode(text)),
    by: (id, elementId) => bind(id, document.getElementById(elementId)),

    st: (id, text) => {
      const node = nodes.get(id);
      for (const child of Array.from(node.childNodes)) forget(child);
      node.textContent = text;
    },
    sa: (id, name, value) => nodes.get(id).setAttribute(name, value),
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
      node[name] = value;
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
    natives(names) {
      if (location.origin !== window.__rustx_origin) return;
      for (const name of names) {
        window[name] = (...args) => {
          const request = new XMLHttpRequest();
          request.open("POST", window.__rustx_origin + "/__rustx/native/" + encodeURIComponent(name), false);
          request.send(JSON.stringify(args));
          const answer = JSON.parse(request.responseText);
          if ("err" in answer) throw new Error(answer.err);
          return answer.ok;
        };
      }
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
