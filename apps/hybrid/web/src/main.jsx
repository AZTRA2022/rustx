// What this application's JavaScript is: the list of libraries it uses.
//
// The bridge to React is the framework's — `rustx.use` is already there
// when this file runs, because rustx put it in the page before the
// page's own scripts. What is below is only the reverse direction of the
// boundary: a panel React builds, with a component rustx owns inside it.

import React from "react";
import * as reactDom from "react-dom";
import * as reactDomClient from "react-dom/client";
import * as recharts from "recharts";

// A plain assignment: the framework's runtime is put in the page by the
// engine, which schedules it, so it is not there when this file runs. It
// reads this when it needs it.
window.rustxLibraries = { react: React, reactDom, reactDomClient, recharts };

// A panel React builds. The host is React's: it renders the frame and
// leaves one empty node for rustx to fill.
window.__rustx_panel = (host) => {
  const root = reactDomClient.createRoot(host);
  reactDom.flushSync(() =>
    root.render(
      <section className="panel">
        points, from rustx: <div id="panel-slot" />
      </section>
    )
  );
  // The node React has just put in the document.
  rustx_component("Counter", "panel-slot");
};
