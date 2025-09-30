declare namespace NodeJS {
  interface ProcessEnv {
    NODE_ENV: "development" | "production" | "test";

    // Mongo
    MONGO_URI: string;

    // Discord
    DISCORD_CLIENT_ID: string;
    DISCORD_CLIENT_SECRET: string;
    DISCORD_REDIRECT_URI: string;
  }
}
