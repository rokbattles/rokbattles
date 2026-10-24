// Runs before site scripts, in the remote origin's storage context.
if (window.location.origin === "__ROKBATTLES_APP_ORIGIN__" && window.top === window) {
  try {
    window.localStorage.setItem("__rokb_landing", "true");
  } catch {
    // The site also handles unavailable storage without a landing redirect.
  }
}
