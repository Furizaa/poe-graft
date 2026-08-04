import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

const mount = (Root: React.ComponentType) =>
  root.render(
    <React.StrictMode>
      <Root />
    </React.StrictMode>,
  );

/**
 * PROTOTYPE BRANCH ONLY — `prototype/craft-ui`, for
 * [#9](https://github.com/Furizaa/poe-graft/issues/9).
 *
 * In a development build the app is wrapped in the prototype shell, which hosts four candidate craft
 * windows plus today's one as a control.
 *
 * The import is **dynamic and inside the branch** rather than a static import at the top of the file,
 * which matters more than it looks. `import.meta.env.DEV` is statically `false` under `vite build`, so
 * written this way the whole branch is dead code and Rollup emits no chunk for it at all — verified:
 * the production bundle contains none of the variants, the mock feed, the sound bench, the prototype
 * stylesheet, or the 128 kB copy of `data/ghastly-eye-jewel.json` that `src/prototype/data.ts` reads.
 * A static import leaves the stylesheet and that JSON string in the bundle, because a CSS import is a
 * side effect and cannot be tree-shaken.
 *
 * None of that is what keeps the prototype off the gaming PC. That is the branch: `main` is what the
 * app auto-updates from, and every push touching `src/**` publishes a release the owner is offered.
 * This must not be merged. Delete this block and `src/prototype/` when the winning variant is folded
 * into `src/Craft.tsx`.
 */
if (import.meta.env.DEV) {
  void import("./prototype/Shell").then((module) => mount(module.default));
} else {
  mount(App);
}
