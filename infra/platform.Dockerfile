FROM node:24-alpine AS base
WORKDIR /app

FROM base AS builder
RUN apk add --no-cache libc6-compat
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY apps/rokbattles-platform-legacy ./apps/rokbattles-platform-legacy
COPY legal ./apps/rokbattles-platform-legacy/legal
COPY datasets ./datasets
RUN corepack enable pnpm
RUN pnpm install --frozen-lockfile
RUN pnpm --filter=@rokbattles/platform-legacy... run generate:datasets
RUN pnpm --filter=@rokbattles/platform-legacy... build

FROM base AS runner
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs
COPY --from=builder /app/apps/rokbattles-platform-legacy/public ./apps/rokbattles-platform-legacy/public
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-platform-legacy/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-platform-legacy/.next/static ./apps/rokbattles-platform-legacy/.next/static
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-platform-legacy/legal ./apps/rokbattles-platform-legacy/legal
USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "apps/rokbattles-platform-legacy/server.js"]
