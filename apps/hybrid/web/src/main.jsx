// The JavaScript half of the boundary: React, react-dom and a chart
// library nobody here wrote.
//
// Four calls make the whole contract, and both halves of it are here:
//
//   window.__rustx_react.mount(name, node, props)   rustx hands a node over
//   window.__rustx_react.unmount(node)              rustx asks for it back
//   rustx_component(name, id)                       JavaScript asks for a component
//   rustx_release(id)                               JavaScript hands it back
//
// Nothing below reaches into a node rustx owns, and rustx does not look
// inside a node React owns. That is the only rule; everything else is
// ordinary React.

import React from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { Line, LineChart, Tooltip, XAxis, YAxis } from "recharts";

// What a test can see from the outside, and the application cannot.
window.__rustx_hybrid = { mounts: 0, unmounts: 0, unmountedWhileAttached: false };

// The component rustx asks for by name, drawn with a library off npm.
function Chart({ points }) {
  const data = Array.from({ length: points + 1 }, (value, at) => ({
    at,
    value: at * at,
  }));

  return (
    <div>
      <span className="chart">{points} points</span>
      <LineChart width={320} height={120} data={data}>
        <XAxis dataKey="at" />
        <YAxis />
        <Tooltip />
        <Line type="monotone" dataKey="value" stroke="#7f5af0" dot={false} />
      </LineChart>
    </div>
  );
}

// Every component this bundle will answer to a name with.
const components = { Chart };

// One React root per node rustx hands over, kept by the node itself: a
// second `mount` of the same boundary is a props change, not a new tree,
// and React is what decides what that costs.
const roots = new WeakMap();

// React 18 batches, and a boundary drawn on this frame cannot wait for
// the next one: rustx handed the node over during a render, and the
// render is what the page shows at the end of it.
const now = (work) => flushSync(work);

window.__rustx_react = {
  mount(name, node, props) {
    const Component = components[name];
    if (Component === undefined) {
      return false;
    }
    let root = roots.get(node);
    if (root === undefined) {
      root = createRoot(node);
      roots.set(node, root);
      window.__rustx_hybrid.mounts += 1;
    }
    now(() => root.render(<Component {...props} />));
    return true;
  },

  unmount(node) {
    const root = roots.get(node);
    if (root === undefined) {
      return false;
    }
    window.__rustx_hybrid.unmounts += 1;
    // Told while the node is still in the document, which is what lets
    // React run its own cleanup on nodes that still exist.
    window.__rustx_hybrid.unmountedWhileAttached ||= node.isConnected;
    roots.delete(node);
    now(() => root.unmount());
    return true;
  },
};

// A panel React builds, with a component rustx owns inside it.
//
// The host is React's: it renders the frame and leaves one empty node
// for rustx to fill. `rustx_component` is the reverse direction of the
// same contract.
window.__rustx_panel = (host) => {
  const root = createRoot(host);
  now(() =>
    root.render(
      <section className="panel">
        points, from rustx: <div id="panel-slot" />
      </section>
    )
  );
  // The node React has just put in the document.
  rustx_component("Counter", "panel-slot");
};
