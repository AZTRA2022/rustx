// The JavaScript half of the boundary, by hand.
//
// This stands in for React and react-dom, which are an npm dependency
// and belong to the packaging plan. What the hybrid application proves
// is the *contract* the two rules of ownership make, and that contract
// is what React consumes:
//
//   window.__rustx_react.mount(name, node, props)   rustx hands a node over
//   window.__rustx_react.unmount(node)              rustx asks for it back
//   rustx_component(name, id)                       JavaScript asks for a component
//   rustx_release(id)                               JavaScript hands it back
//
// Replace this file with a bundle of `react-dom/client` and the four
// calls above are the same four calls.

window.__rustx_hybrid = { mounts: 0, unmounts: 0, unmountedWhileAttached: false };

window.__rustx_react = {
  mount(name, node, props) {
    if (name !== "Chart") {
      return false;
    }
    let own = node.querySelector(".chart");
    if (own === null) {
      own = document.createElement("span");
      own.className = "chart";
      node.appendChild(own);
      window.__rustx_hybrid.mounts += 1;
    }
    own.textContent = props.points + " points";
    return true;
  },
  unmount(node) {
    window.__rustx_hybrid.unmounts += 1;
    window.__rustx_hybrid.unmountedWhileAttached ||= node.isConnected;
    node.textContent = "";
    return true;
  },
};

// A panel JavaScript builds, which asks rustx for a component to put in
// it. The node is JavaScript's; what is inside it is rustx's.
window.__rustx_panel = (host) => {
  const panel = document.createElement("section");
  panel.className = "panel";
  const slot = document.createElement("div");
  slot.id = "panel-slot";
  panel.append("points, from rustx: ", slot);
  host.appendChild(panel);
  rustx_component("Counter", "panel-slot");
};
