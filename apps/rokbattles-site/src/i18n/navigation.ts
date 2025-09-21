import { createNavigation } from "next-intl/navigation";
import { routing } from "@/i18n/routing";

export const { Link, getPathname, redirect, permanentRedirect, usePathname, useRouter } =
  createNavigation(routing);
