# Use the official Bun image as the base image.
# See all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1 AS base
WORKDIR /app

# ---- Dependencies ----
# Install dependencies including devDependencies.
# This layer is cached to speed up subsequent builds.
FROM base AS deps
WORKDIR /app
COPY package.json bun.lockb* ./
# Bun automatically caches dependencies, no need for --mount=type=cache
RUN bun install --frozen-lockfile

# ---- Builder ----
# Build the Next.js application.
FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .

# Set NEXT_TELEMETRY_DISABLED to 1 to disable telemetry during build.
ENV NEXT_TELEMETRY_DISABLED=1
ENV NODE_ENV=production

RUN bun run build

# ---- Production Dependencies ----
# Install only production dependencies for the final image.
FROM base AS prod-deps
WORKDIR /app
COPY package.json bun.lockb* ./
RUN bun install --frozen-lockfile --production

# ---- Runner ----
# Create the final image for running the application.
# Use the built-in non-root 'bun' user for security.
FROM base AS runner
WORKDIR /app

ENV NODE_ENV=production
# Disable telemetry during runtime.
ENV NEXT_TELEMETRY_DISABLED=1

# Copy necessary files from the builder and prod-deps stages.
# Ensure your next.config.js has `output: 'standalone'` enabled.
COPY --from=prod-deps --chown=bun:bun /app/node_modules ./node_modules
COPY --from=builder --chown=bun:bun /app/public ./public
COPY --from=builder --chown=bun:bun /app/.next/standalone ./
COPY --from=builder --chown=bun:bun /app/.next/static ./.next/static

# Use the non-root 'bun' user provided by the base image.
USER bun

EXPOSE 3000

# Set the default command to run the application.
# The Next.js standalone output creates a server.js file.
# The oven/bun image includes Node.js, so we can run it directly.
CMD ["node", "server.js"]
