import React, { Suspense, lazy } from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/space-grotesk/latin-400.css";
import "@fontsource/space-grotesk/latin-500.css";
import "@fontsource/space-grotesk/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import { App } from "./App";
import "./generated/anatomy.css";
import "./styles.css";

// One entry for the whole site: the path picks the page. /docs and /changelog
// load their own code on demand (the old .html addresses still work).
const Docs = lazy(() => import("./docs/Docs"));
const Changelog = lazy(() => import("./changelog/Changelog"));
const path = location.pathname.replace(/\/+$/, "").replace(/\.html$/, "");
const page = path === "/docs" ? "docs" : path === "/changelog" ? "changelog" : "home";
if (page === "docs") document.title = "Astrid docs";
if (page === "changelog") document.title = "Astrid changelog";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Suspense fallback={null}>{page === "docs" ? <Docs /> : page === "changelog" ? <Changelog /> : <App />}</Suspense>
  </React.StrictMode>,
);
