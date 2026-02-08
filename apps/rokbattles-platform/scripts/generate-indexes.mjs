import { MongoClient } from "mongodb";

async function main() {
  const uri = process.env.MONGODB_URI;
  if (!uri) {
    throw new Error("MONGODB_URI is required");
  }

  const client = new MongoClient(uri, { appName: "ROK Battles" });
  await client.connect();

  try {
    const db = client.db();

    await Promise.all([
      db.collection("oauthStates").createIndex({ state: 1 }, { unique: true }),
      db
        .collection("oauthStates")
        .createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 }),
      db
        .collection("userSessions")
        .createIndex({ sessionId: 1 }, { unique: true }),
      db
        .collection("userSessions")
        .createIndex({ expiresAt: 1 }, { expireAfterSeconds: 0 }),
      db.collection("users").createIndex({ discordId: 1 }, { unique: true }),
      db
        .collection("user_binds")
        .createIndex({ governorId: 1 }, { unique: true }),
      db.collection("user_binds").createIndex({ discordId: 1, updatedAt: -1 }),
      db.collection("user_binds").createIndex({ discordId: 1, isDefault: 1 }),
      db.collection("user_binds").createIndex({ discordId: 1, isVisible: 1 }),
      db.collection("user_binds").createIndex({ discordId: 1, appUid: 1 }),
      db
        .collection("user_binds")
        .createIndex({ pendingDeleteAt: 1 }, { expireAfterSeconds: 0 }),
    ]);

    console.log("Success");
  } finally {
    await client.close();
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
