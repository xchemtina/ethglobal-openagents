# Compile with locked build dependencies; production never downloads a runner.
FROM node:22-bookworm-slim AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY tsconfig.json ./
COPY src ./src
RUN npm run build && npm prune --omit=dev

FROM node:22-bookworm-slim
WORKDIR /app
COPY --from=build /app/package.json ./package.json
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/dist ./dist
RUN mkdir data && chown node:node data
USER node
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=4021
EXPOSE 4021
# Mount a DFT cache and executable CLI; configure their paths via environment.
CMD ["node", "dist/index.js"]
