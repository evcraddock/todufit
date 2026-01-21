# Stage 1: Build frontend and server
FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .

# Sync server URL - baked into frontend at build time
# Browser connects directly to sync server (no proxy)
ARG VITE_SYNC_URL=wss://sync.todu.sh
ENV VITE_SYNC_URL=${VITE_SYNC_URL}

RUN npm run build
RUN npm run build:server

# Stage 2: Production image
FROM node:22-alpine
WORKDIR /app

# Sync server URL for server-side connections (passed from build args)
ARG VITE_SYNC_URL
ENV SYNC_URL=${VITE_SYNC_URL}

# Install build dependencies for native modules (better-sqlite3)
RUN apk add --no-cache python3 make g++

COPY --from=builder /app/dist ./dist
COPY --from=builder /app/dist-server ./dist-server
COPY --from=builder /app/server/db/migrations ./server/db/migrations
COPY --from=builder /app/package*.json ./
RUN npm ci --omit=dev

# Clean up build dependencies to reduce image size
RUN apk del python3 make g++

EXPOSE 3000
CMD ["node", "dist-server/server.js"]
