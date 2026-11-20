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
