// Runs before site scripts, in the remote origin's storage context.
if (window.location.origin === "__ROKBATTLES_APP_ORIGIN__" && window.top === window) {
  try {
    window.localStorage.setItem("__rokb_landing", "true");
    window.localStorage.setItem("theme", "dark");
  } catch {
    // The site also handles unavailable storage without a landing redirect.
  }

  const forceDark = () => {
    const root = document.documentElement;
    root.classList.remove("light");
    root.classList.add("dark");

    if (root.style.colorScheme !== "dark") root.style.colorScheme = "dark";
  };

  const installAppearance = () => {
    forceDark();

    // The remote site's theme provider can change the root during hydration or
    // navigation. Desktop stays dark even when the site still offers light mode.
    new MutationObserver(() => {
      if (
        !document.documentElement.classList.contains("dark") ||
        document.documentElement.classList.contains("light") ||
        document.documentElement.style.colorScheme !== "dark"
      ) {
        forceDark();
      }
    }).observe(document.documentElement, { attributes: true, attributeFilter: ["class", "style"] });
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", installAppearance, { once: true });
  } else {
    installAppearance();
  }
}
