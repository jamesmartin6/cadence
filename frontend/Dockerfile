## Build stage
# No Rust/wasm-pack needed here -- frontend/src/wasm is already committed to the repo.
FROM node:22-slim AS build
WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .

# Baked in at build time (Vite env vars are compile-time): the relay server's
# *browser-reachable* URL. Defaults to the docker-compose published port on localhost.
ARG VITE_RELAY_HTTP_URL=http://localhost:3001
ENV VITE_RELAY_HTTP_URL=$VITE_RELAY_HTTP_URL
RUN npm run build

## Runtime stage
FROM nginx:alpine AS runtime
COPY --from=build /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
