FROM node:22-alpine AS base
WORKDIR /app

FROM base AS builder
RUN apk add --no-cache libc6-compat
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY apps ./apps
RUN corepack enable pnpm
RUN pnpm install --frozen-lockfile
RUN pnpm --filter=@rokbattles/site... build

FROM base AS runner
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-site/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/apps/rokbattles-site/.next/static ./apps/rokbattles-site/.next/static
USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "apps/rokbattles-site/server.js"]