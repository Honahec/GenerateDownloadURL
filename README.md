# 阿里云 OSS 签名下载链接项目

该项目提供一套分离式部署的前后端应用，用于生成阿里云 OSS 私有资源的签名下载链接，支持下载次数限制、有效期控制和历史记录管理。

## 架构说明

- **前端应用**：部署在 `gurl.honahec.cc`，提供：
  - 管理界面和登录页面
  - API 接口：`/login`（登录验证）、`/sign`（生成链接）
- **后端服务**：部署在 `api.honahec.cc`，提供：
  - 下载服务：`/download/<token>`（重定向到 OSS）
  - 数据持久化：SQLite 数据库存储下载链接记录
- **请求流程**：
  1. 用户访问 `gurl.honahec.cc/login` 进行登录
  2. 前端调用 `gurl.honahec.cc/login` 获取 JWT
  3. 前端调用 `gurl.honahec.cc/sign` 生成下载链接
  4. 生成的链接格式：`api.honahec.cc/download/<token>`

## 功能特性

- **后端（Rust + Axum + SQLite）**：
  - JWT 身份认证与鉴权
  - 阿里云 OSS 签名 URL 生成
  - 下载次数限制和有效期控制
  - 重定向到真实 OSS 链接
  - SQLite 数据库持久化存储
  - 自动数据库迁移
- **前端（React + TypeScript + Chakra UI）**：
  - 管理员登录界面
  - 填写对象信息生成签名链接
  - 历史链接列表管理，显示下载状态
  - 实时显示使用次数 (已使用 m/n 次)
  - 过期链接和达到限制的链接自动标记为失效
  - 响应式设计，移动端友好
- **部署特性**：
  - 支持前后端分离部署
  - CORS 跨域配置
  - 生产环境优化

## 目录结构

```
GenerateDownloadURL/
├── backend/          # Rust 后端服务
├── frontend/         # React 前端应用
├── .env.example      # 后端环境变量模板
└── README.md
```

## 环境准备

- Rust 1.75+
- Node.js 18+ 与 pnpm 8+（建议执行 `corepack enable` 以启用 pnpm）
- 可访问 crates.io 与 npm registry 的网络，用于首次安装依赖并构建（当前环境未联网，`cargo` 与 `pnpm` 均无法下载依赖，请在联网环境执行）

## 后端服务

### 环境变量

复制 `.env.example` 为 `.env` 并补充真实配置：

| 变量                                | 说明                                                                                          |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| `ALIYUN_ACCESS_KEY_ID`              | 阿里云 AccessKey ID                                                                           |
| `ALIYUN_ACCESS_KEY_SECRET`          | 阿里云 AccessKey Secret（建议仅授予只读权限）                                                 |
| `ALIYUN_ENDPOINT`                   | OSS Endpoint（如 `oss-cn-hangzhou.aliyuncs.com`，也支持 `https://{bucket}.example.com` 模板） |
| `ALIYUN_DEFAULT_BUCKET`             | 默认 Bucket 名（可在前端覆盖）                                                                |
| `DEFAULT_EXPIRY_SECS`               | 默认链接有效期（秒，默认 3600）                                                               |
| `PUBLIC_BASE_URL`                   | 对外访问的域名根（如 `https://api.honahec.cc`，生成的下载地址基于此）                         |
| `DOWNLOAD_PATH_PREFIX`              | 下载路径前缀（默认 `download`，最终访问路径为 `/{prefix}/{id}`）                              |
| `ADMIN_USERNAME` / `ADMIN_PASSWORD` | 管理员登录凭证                                                                                |
| `JWT_SECRET`                        | JWT 签名密钥                                                                                  |
| `JWT_EXP_MINUTES`                   | 登录 Token 有效期（分钟，默认 60）                                                            |
| `CORS_ALLOWED_ORIGINS`              | 允许的前端来源（逗号分隔，如 `https://gurl.honahec.cc,http://localhost:5173`）                |
| `API_HOST` / `API_PORT`             | 监听地址与端口（默认 `0.0.0.0:8003`）                                                         |

### 运行

```bash
cd backend
# 首次在联网环境执行依赖下载
cargo check
# 开发运行
cargo run
```

服务启动后将监听 `API_HOST:API_PORT`，主要接口：

**管理功能接口（通过 gurl.honahec.cc 访问）：**

- `POST /login`：管理员登录获取 JWT
- `POST /sign`：生成签名下载链接（需 JWT 认证）
- `GET /links`：获取下载链接列表，包含状态和使用次数（需 JWT 认证）
- `GET /links/{id}`：获取单个链接详情（需 JWT 认证）
- `DELETE /links/{id}`：删除指定链接（需 JWT 认证）

**公共访问接口（通过 api.honahec.cc 访问）：**

- `GET /download/{id}`：外部下载入口，重定向至 OSS 并统计次数

## 前端应用

```bash
cd frontend
pnpm install    # 首次需要联网下载依赖
pnpm run dev    # 默认运行在 http://localhost:5173
```

启动后即可在浏览器中打开页面，先登录再填写 Object Key、有效期、下载次数等信息生成链接。

### 开发环境配置

- 开发时使用 `vite.config.ts` 中的代理，前端请求会自动转发到后端
- 修改 `frontend/src/config.ts` 可以调整 API 地址

### 生产环境构建

```bash
cd frontend
pnpm run build    # 构建生产版本到 dist/ 目录
```

## 快速部署

### 一键部署脚本

项目提供了自动化部署脚本，支持 Ubuntu/Debian 系统：

```bash
# 1. 克隆项目到服务器
git clone <your-repo-url> /tmp/generate-download-url
cd /tmp/generate-download-url

# 2. 执行一键部署脚本
sudo chmod +x deploy.sh
sudo ./deploy.sh

# 3. 配置 SSL 证书（可选）
sudo chmod +x ssl-setup.sh
sudo ./ssl-setup.sh
```

**部署脚本功能：**

- 自动安装系统依赖（Rust, Node.js, pnpm, Nginx, SQLite）
- 构建前后端应用
- 创建系统服务（systemctl）
- 配置 Nginx 反向代理
- 设置防火墙规则
- 创建专用服务用户

**部署后管理：**

```bash
# 使用管理脚本
sudo chmod +x manage.sh

# 查看服务状态
sudo ./manage.sh status

# 查看服务日志
sudo ./manage.sh logs

# 重启服务
sudo ./manage.sh restart

# 备份数据库
sudo ./manage.sh backup

# 更新服务
sudo ./manage.sh update

# 更多命令
sudo ./manage.sh help
```

## 部署架构

### 分离式部署（推荐）

**手动部署（高级用户）**

如果需要手动部署，参考以下步骤：

```bash
# 1. 部署后端到服务器
cd backend
cargo build --release
./target/release/backend

# 2. 自动部署的 Nginx 配置示例
# 前端域名配置 - 处理管理功能API
server {
    listen 443 ssl;
    server_name gurl.honahec.cc;

    # 静态文件服务
    location / {
        root /path/to/frontend/dist;
        try_files $uri $uri/ /index.html;
    }

    # 管理功能API路由代理到后端
    location /login {
        proxy_pass http://127.0.0.1:8003/login;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location /sign {
        proxy_pass http://127.0.0.1:8003/sign;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location /links {
        proxy_pass http://127.0.0.1:8003/links;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}# 后端域名配置 - 处理下载重定向
server {
    listen 443 ssl;
    server_name api.honahec.cc;

    location /download/ {
        proxy_pass http://127.0.0.1:8003/download/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 域名配置要点

- `gurl.honahec.cc`：前端静态文件 + 登录/签名 API 代理
- `api.honahec.cc`：下载重定向服务
- 同一个后端进程同时服务两个域名的不同功能
- 确保 CORS 配置正确，允许跨域访问
- SSL 证书配置（推荐使用 Let's Encrypt）

## 使用流程

1. **访问管理页面**：打开 `https://gurl.honahec.cc`
2. **管理员登录**：使用配置的管理员账户登录
3. **生成下载链接**：填写文件路径、有效期等信息，生成下载链接
4. **分享链接**：将生成的 `https://api.honahec.cc/download/xxx` 链接分享给用户
5. **用户下载**：用户访问链接后会自动重定向到阿里云 OSS 进行下载
