import { integer, pgTable, text, timestamp, varchar } from "drizzle-orm/pg-core";

export const usersTable = pgTable("users", {
  id: integer().primaryKey().generatedAlwaysAsIdentity(),
  discordId: varchar({ length: 255 }).unique().notNull(),
  email: varchar({ length: 255 }).unique().notNull(),
  username: varchar({ length: 255 }).notNull(),
  globalName: varchar({ length: 255 }),
  avatar: text(),
  createdAt: timestamp({ withTimezone: true }).defaultNow().notNull(),
  updatedAt: timestamp({ withTimezone: true }).defaultNow().notNull(),
});

export const userSessionsTable = pgTable("user_sessions", {
  id: integer().primaryKey().generatedAlwaysAsIdentity(),
  userId: integer()
    .notNull()
    .references(() => usersTable.id, { onDelete: "cascade" }),
  sessionId: varchar({ length: 255 }).unique().notNull(),
  createdAt: timestamp({ withTimezone: true }).defaultNow().notNull(),
  expiresAt: timestamp({ withTimezone: true }).notNull(),
});
