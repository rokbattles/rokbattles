FROM node:22-alpine AS base
WORKDIR /app

FROM base AS builder
RUN apk add --no-cache libc6-compat
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY apps/rokbattles-marketing ./apps/rokbattles-marketing
COPY legal ./apps/rokbattles-marketing/legal
RUN corepack enable pnpm
RUN pnpm install --frozen-lockfile
RUN pnpm --filter=@rokbattles/marketing... build

FROM base AS runner
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-marketing/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-marketing/.next/static ./apps/rokbattles-marketing/.next/static
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-marketing/legal ./apps/rokbattles-marketing/legal
USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "apps/rokbattles-marketing/server.js"]
