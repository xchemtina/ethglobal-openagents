# ChimiaClaw API gateway — deploy on Fly/Railway/any Docker host
FROM node:22-bookworm-slim

WORKDIR /app

# System deps for optional native modules; CLI binary is optional mount
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY package.json package-lock.json* ./
RUN npm install --omit=dev

COPY tsconfig.json ./
COPY src ./src

ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=4021
EXPOSE 4021

# DFT cache + CLI expected via volume or build stage in production
#   -e DFT_CACHE_DIR=/data/dft
#   -e CHIMIACLAW_CLI=/data/bin/chimiaclaw-cli

CMD ["npx", "tsx", "src/index.ts"]
