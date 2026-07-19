# syntax=docker/dockerfile:1
FROM node:24-alpine@sha256:a0b9bf06e4e6193cf7a0f58816cc935ff8c2a908f81e6f1a95432d679c54fbfd AS base
WORKDIR /app

FROM base AS builder
ARG API_URL
ENV API_URL=${API_URL}
RUN apk add --no-cache git libc6-compat
COPY --link pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY --link packages/site/package.json ./packages/site/package.json
RUN corepack enable pnpm
RUN --mount=type=cache,id=rokbattles-pnpm-store,target=/pnpm/store \
    pnpm config set store-dir /pnpm/store && \
    pnpm install --frozen-lockfile
COPY --link packages/site ./packages/site
COPY --link datasets ./datasets
RUN pnpm --filter=@rokbattles/site... run sync:legal
RUN pnpm --filter=@rokbattles/site... run generate:datasets
RUN pnpm --filter=@rokbattles/site... build

FROM base AS runner
ENV NODE_ENV=production
RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 nextjs
COPY --from=builder --chown=nextjs:nodejs /app/packages/site/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/packages/site/.next/static ./packages/site/.next/static
COPY --from=builder --chown=nextjs:nodejs /app/packages/site/legal ./packages/site/legal
WORKDIR /app/packages/site
USER nextjs
EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"
CMD ["node", "server.js"]
