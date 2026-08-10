import { useCallback, useEffect, useState } from "react";
import { DocList } from "./components/DocList";
import { DocumentPage } from "./components/DocumentPage";

type Route = { page: "list" } | { page: "doc"; docId: string };

function routeFromLocation(): Route {
  const match = /^\/doc\/([^/]+)\/?$/.exec(window.location.pathname);
  return match ? { page: "doc", docId: match[1] } : { page: "list" };
}

/** Minimal hand-rolled router -- the app only ever has two screens, so a routing
 * library would be more ceremony than the problem calls for. */
export function App() {
  const [route, setRoute] = useState<Route>(routeFromLocation);

  useEffect(() => {
    const onPopState = () => setRoute(routeFromLocation());
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);

  const openDoc = useCallback((docId: string) => {
    window.history.pushState(null, "", `/doc/${docId}`);
    setRoute({ page: "doc", docId });
  }, []);

  const goToList = useCallback(() => {
    window.history.pushState(null, "", "/");
    setRoute({ page: "list" });
  }, []);

  if (route.page === "doc") {
    return <DocumentPage key={route.docId} docId={route.docId} onBack={goToList} />;
  }
  return <DocList onOpenDoc={openDoc} />;
}
