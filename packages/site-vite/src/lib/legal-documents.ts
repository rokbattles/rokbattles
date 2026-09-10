import { Cookie, FileText, ShieldCheck } from "lucide-react";
import { lazy } from "react";

export const legalDocuments = [
  {
    id: "terms-of-service",
    title: "Terms of Service",
    description: "The rules and conditions for using ROK Battles and its services.",
    icon: FileText,
    Content: lazy(() => import("../content/legal/terms-of-service.md")),
  },
  {
    id: "privacy-policy",
    title: "Privacy Policy",
    description: "How ROK Battles collects, uses, and protects personal data.",
    icon: ShieldCheck,
    Content: lazy(() => import("../content/legal/privacy-policy.md")),
  },
  {
    id: "cookie-policy",
    title: "Cookie Policy",
    description: "How ROK Battles uses cookies and similar technologies.",
    icon: Cookie,
    Content: lazy(() => import("../content/legal/cookie-policy.md")),
  },
];
