# 前端构建 Dockerfile - 仅 amd64
FROM node:20-alpine AS builder

# 设置时区和语言
ENV TZ=Asia/Shanghai \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN apk add --no-cache tzdata && \
    cp /usr/share/zoneinfo/$TZ /etc/localtime && \
    echo $TZ > /etc/timezone

# 安装 pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app

# 复制依赖文件
COPY package.json pnpm-lock.yaml ./

# 安装依赖
RUN pnpm install --frozen-lockfile

# 复制源码并构建
COPY . .
RUN pnpm build

# 输出阶段 - 仅保留构建产物
FROM scratch AS export
COPY --from=builder /app/dist /dist
