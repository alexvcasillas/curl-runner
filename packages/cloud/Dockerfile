# syntax=docker/dockerfile:1
# Build context is the repo root:
#   docker build -f packages/cloud/Dockerfile -t curl-runner-cloud .
#
# Docker reads packages/cloud/Dockerfile.dockerignore instead of the root
# .dockerignore, so the context is only the workspace files and packages/cloud.
# The build runs on the target platform: Next's standalone output carries
# sharp's native binary for the platform it was built on.
ARG NODE_VERSION=22

FROM node:${NODE_VERSION}-alpine AS base
# Keep in step with `packageManager` in the root package.json.
ARG PNPM_VERSION=12.16.1
ENV NEXT_TELEMETRY_DISABLED=1
RUN npm install -g "pnpm@${PNPM_VERSION}" && pnpm --version
WORKDIR /repo

FROM base AS build
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY packages/cloud/package.json packages/cloud/
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile --filter @curl-runner/cloud...
COPY packages/cloud packages/cloud
RUN pnpm --filter @curl-runner/cloud build

FROM node:${NODE_VERSION}-alpine AS runtime
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0
WORKDIR /app
RUN addgroup -S -g 1001 cloud && adduser -S -u 1001 -G cloud cloud
# The standalone server keeps the monorepo layout: packages/cloud/server.js.
COPY --from=build --chown=cloud:cloud /repo/packages/cloud/.next/standalone ./
COPY --from=build --chown=cloud:cloud /repo/packages/cloud/.next/static ./packages/cloud/.next/static
COPY --from=build --chown=cloud:cloud /repo/packages/cloud/public ./packages/cloud/public
USER cloud
EXPOSE 3000
HEALTHCHECK --interval=10s --timeout=5s --retries=5 --start-period=10s \
    CMD wget -qO- http://127.0.0.1:3000/healthz || exit 1
CMD ["node", "packages/cloud/server.js"]
