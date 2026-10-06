import * as Sentry from "@sentry/nextjs";

try {
  if (window.localStorage.getItem("__rokb_landing") === null) {
    window.localStorage.setItem("__rokb_landing", "true");

    if (window.location.pathname === "/") {
      window.location.replace("/home");
    }
  }
} catch {}

Sentry.init({
  dsn: process.env.NEXT_PUBLIC_SENTRY_DSN,

  // Adds request headers and IP for users
  sendDefaultPii: false,

  // Capture 100% in dev, 10% in production
  // Adjust based on your traffic volume
  tracesSampleRate: process.env.NODE_ENV === "development" ? 1.0 : 0.1,
});

// This export will instrument router navigations
export const onRouterTransitionStart = Sentry.captureRouterTransitionStart;
