import { useEffect, useRef } from "react";
import { useLocation } from "react-router";

export function ScrollToHash() {
  const location = useLocation();
  const initialLocation = useRef(location);
  const previousPath = useRef(location.pathname);

  useEffect(() => {
    const changedPage = previousPath.current !== location.pathname;
    previousPath.current = location.pathname;
    if (!location.hash) {
      if (changedPage) window.scrollTo({ top: 0, behavior: "instant" });
      return;
    }

    let frame = 0;
    const scroll = () => {
      frame = requestAnimationFrame(() => {
        const target = document.getElementById(location.hash.slice(1));
        if (!target) return;

        const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        // Start fresh hash loads at the top after native scroll restoration has finished.
        if (location === initialLocation.current && !reducedMotion) {
          window.scrollTo({ top: 0, behavior: "instant" });
        }
        // Give the new route a paint before starting the scroll animation.
        frame = requestAnimationFrame(() => {
          target.scrollIntoView({ behavior: reducedMotion ? "instant" : "smooth" });
        });
      });
    };

    if (document.readyState === "complete") scroll();
    else window.addEventListener("load", scroll, { once: true });

    return () => {
      window.removeEventListener("load", scroll);
      cancelAnimationFrame(frame);
    };
  }, [location]);

  return null;
}
