import { routing } from "@/i18n/routing";
import messages from "@/i18n/messages/en.json";

declare global {
  namespace NodeJS {
    interface ProcessEnv {
      ROKB_API_URL: string;
      ROKB_DATASETS_PATH: string;
    }
  }
}

declare module "next-intl" {
  interface AppConfig {
    Locale: (typeof routing.locales)[number];
    Messages: typeof messages;
  }
}
