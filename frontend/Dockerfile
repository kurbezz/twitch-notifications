# Build stage
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package.json package-lock.json* pnpm-lock.yaml* yarn.lock* ./

# Install dependencies
RUN if [ -f pnpm-lock.yaml ]; then \
        corepack enable && pnpm install --frozen-lockfile; \
    elif [ -f yarn.lock ]; then \
        yarn install --frozen-lockfile; \
    elif [ -f package-lock.json ]; then \
        npm ci; \
    else \
        npm install; \
    fi

# Copy source files
COPY . .

# Build argument for API URL
ARG VITE_API_URL=http://localhost:8080
ENV VITE_API_URL=${VITE_API_URL}

# Build the application
RUN if [ -f pnpm-lock.yaml ]; then \
        pnpm build; \
    elif [ -f yarn.lock ]; then \
        yarn build; \
    else \
        npm run build; \
    fi

# Production stage
FROM nginx:alpine

# Copy custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets from builder stage
COPY --from=builder /app/dist /usr/share/nginx/html

# Add healthcheck
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:80/health || exit 1

# Expose port
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]