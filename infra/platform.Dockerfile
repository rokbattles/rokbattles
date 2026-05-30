FROM node:24-alpine AS base
WORKDIR /app

FROM base AS builder
ARG API_URL
ENV API_URL=${API_URL}
RUN apk add --no-cache git libc6-compat
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY packages/site ./packages/site
COPY datasets ./datasets
RUN corepack enable pnpm
RUN pnpm install --frozen-lockfile
RUN pnpm --filter=@rokbattles/site... run sync:legal
RUN pnpm --filter=@rokbattles/site... run generate:datasets
RUN pnpm --filter=@rokbattles/site... build

FROM base AS runner
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs
COPY --from=builder --chown=nextjs:nodejs /app/packages/site/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/packages/site/.next/static ./packages/site/.next/static
COPY --from=builder --chown=nextjs:nodejs /app/packages/site/legal ./packages/site/legal
WORKDIR /app/packages/site
USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "server.js"]
