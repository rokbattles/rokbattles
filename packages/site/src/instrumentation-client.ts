try {
  if (window.localStorage.getItem("__rokb_landing") === null) {
    window.localStorage.setItem("__rokb_landing", "true");

    if (window.location.pathname === "/") {
      window.location.replace("/home");
    }
  }
} catch {}
