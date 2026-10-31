// The JavaScript half of the React boundary, brought by the framework.
//
// An application does not write this. It says which libraries it uses:
//
//   import React from "react";
//   import * as reactDom from "react-dom";
//   import * as reactDomClient from "react-dom/client";
//   import * as recharts from "recharts";
//   window.rustxLibraries = { react: React, reactDom, reactDomClient, recharts };
//
// A plain assignment, and deliberately so: this file is put in the page
// by the engine, through the frame, which schedules it — so it is not
// there when the page's own `<script>` runs. Nothing a page's first line
// needs may live here. What the framework injects reads what the page
// left for it, never the other way round.
//
// and then writes its components in `.rsx`:
//
//   <js::recharts::LineChart data={points}>
//       <js::recharts::XAxis dataKey="at" />
//   </js::recharts::LineChart>
//
// rustx puts the container on the page, hands over the tree it should
// hold, and does not look inside. React builds it. Neither side touches
// what the other owns; that is the whole contract.
//
// This file is evaluated when the page's JavaScript context is created,
// before the page's own scripts, so `rustx.use` is there for a bundle's
// first line.

(() => {
  // The libraries the application declared, by the name it gave them.
  const libraries = {};

  // One React root per node rustx handed over, kept by the node itself:
  // a second hand-over of the same boundary is a props change, not a new
  // tree, and React is what decides what that costs.
  const roots = new WeakMap();

  /// What the application said it uses.
  const use = (modules) => {
    for (const [name, module] of Object.entries(modules ?? {})) {
      libraries[name] = module;
    }
  };

  /// And what it left on the window, read every time rather than once:
  /// a bundle may be loaded after this file, or add a library later.
  const declared = () => {
    if (typeof window !== "undefined") {
      use(window.rustxLibraries);
    }
    return libraries;
  };

  /// React itself, under whichever of the usual names it was given.
  const react = () => declared().react ?? declared().React;

  /// `createRoot`, which lives in `react-dom/client`.
  const createRoot = () => {
    for (const module of [declared().reactDomClient, declared().reactDom]) {
      if (module !== undefined && typeof module.createRoot === "function") {
        return module.createRoot;
      }
    }
    return undefined;
  };

  /// `flushSync`, which lives in `react-dom`.
  const flushSync = () => {
    for (const module of [declared().reactDom, declared().reactDomClient]) {
      if (module !== undefined && typeof module.flushSync === "function") {
        return module.flushSync;
      }
    }
    return undefined;
  };

  /// A rendered React element from a `{ component, props, children }`.
  ///
  /// Exported for its own test: resolution is where a typo lands, and a
  /// typo must name what was asked for and where it was looked up.
  const resolve = (name) => {
    const at = name.lastIndexOf("::");
    if (at === -1) {
      // No module: a component the page itself registered, which is
      // what an application that brought its own bundle does.
      const own = (declared().components ?? {})[name];
      if (own === undefined) {
        return {
          error:
            `rustx: the page asked for the component \`${name}\`, and no library ` +
            `exports it. Write \`<js::module::${name} />\` with the module it comes ` +
            `from, or register it with \`rustx.use({ components: { ${name} } })\`.`,
        };
      }
      return { component: own };
    }
    const module = name.slice(0, at);
    const export_ = name.slice(at + 2);
    const library = declared()[module];
    if (library === undefined) {
      return {
        error:
          `rustx: the page asked for \`${export_}\` from \`${module}\`, and this ` +
          `application never said it uses \`${module}\`. Add it to the bundle: ` +
          `\`import * as ${module} from "${module}"; rustx.use({ ${module} });\`.`,
      };
    }
    const component = library[export_] ?? (library.default ?? {})[export_];
    if (component === undefined) {
      return {
        error:
          `rustx: the page asked for \`${export_}\` in \`${module}\`, which does not ` +
          `export it. What it does export: ${Object.keys(library).slice(0, 12).join(", ")}.`,
      };
    }
    return { component };
  };

  /// A described child — a string, or another component — as a React
  /// element.
  const element = (described) => {
    if (typeof described === "string") {
      return described;
    }
    const found = resolve(described.component);
    if (found.error !== undefined) {
      console.error(found.error);
      return null;
    }
    const children = (described.children ?? []).map(element);
    return react().createElement(found.component, described.props, ...children);
  };

  const mount = (name, node, props, children) => {
    const found = resolve(name);
    if (found.error !== undefined) {
      console.error(found.error);
      return false;
    }
    if (react() === undefined || createRoot() === undefined) {
      console.error(
        "rustx: a React component was asked for and this application never said it " +
          'uses React. Add `rustx.use({ react: React, reactDom, reactDomClient })` to ' +
          "its bundle."
      );
      return false;
    }

    let root = roots.get(node);
    if (root === undefined) {
      root = createRoot()(node);
      roots.set(node, root);
    }
    const tree = react().createElement(
      found.component,
      props,
      ...(children ?? []).map(element)
    );
    // Synchronously: rustx handed the node over during a render, and the
    // page shows that render at the end of it. React's own batching
    // would put the first paint one frame later.
    now(() => root.render(tree));
    return true;
  };

  const unmount = (node) => {
    const root = roots.get(node);
    if (root === undefined) {
      return false;
    }
    roots.delete(node);
    // Told while the node is still in the document, which is what lets
    // React run the cleanups of components that still exist.
    now(() => root.unmount());
    return true;
  };

  const now = (work) => {
    const flush = flushSync();
    if (flush === undefined) {
      work();
      return;
    }
    flush(work);
  };

  const rustx = { use, resolve, element };

  if (typeof window !== "undefined") {
    window.rustx = Object.assign(window.rustx ?? {}, rustx);
    window.__rustx_react = { mount, unmount };
  }
  // Read by the framework's own test, which runs this file under Node.
  if (typeof module !== "undefined") {
    module.exports = { use, resolve, mount, unmount };
  }
})();
