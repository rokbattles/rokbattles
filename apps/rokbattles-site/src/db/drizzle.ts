import { drizzle } from "drizzle-orm/node-postgres";
import * as schema from "./schema";

if (!process.env.POSTGRES_URL) {
  throw new Error("Missing environment: POSTGRES_URL");
}

export const db = drizzle(process.env.POSTGRES_URL, { schema });
