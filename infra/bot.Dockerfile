FROM node:24-alpine AS base
WORKDIR /app

FROM base AS builder
RUN apk add --no-cache libc6-compat
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY apps/rokbattles-bot ./apps/rokbattles-bot
COPY datasets ./datasets
RUN corepack enable pnpm
RUN pnpm  install --frozen-lockfile
RUN pnpm --filter=@rokbattles/bot... run generate:datasets
RUN pnpm --filter=@rokbattles/bot... build

FROM base AS runner
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 bot
COPY --from=builder --chown=bot:nodejs /app /app
USER bot
CMD ["node", "apps/rokbattles-bot/dist/index.js"]
