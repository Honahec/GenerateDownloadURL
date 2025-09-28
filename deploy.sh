#!/bin/bash

# 一键部署阿里云OSS签名下载链接项目
# 适用于 Ubuntu/Debian 系统

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否为 root 用户
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "此脚本需要 root 权限运行"
        log_info "请使用: sudo $0"
        exit 1
    fi
}

# 检查系统环境
check_system() {
    log_info "检查系统环境..."
    
    # 检查是否为 Debian/Ubuntu 系统
    if [ ! -f /etc/debian_version ]; then
        log_warn "此脚本专为 Debian/Ubuntu 系统设计，其他系统可能需要调整"
    fi
    
    # 检查必要命令是否存在
    local missing_commands=()
    
    if ! command -v apt &> /dev/null; then
        missing_commands+=("apt")
    fi
    
    if ! command -v useradd &> /dev/null && ! command -v adduser &> /dev/null; then
        missing_commands+=("useradd/adduser")
    fi
    
    if [ ${#missing_commands[@]} -gt 0 ]; then
        log_error "缺少必要命令: ${missing_commands[*]}"
        log_info "请先安装必要的包："
        log_info "  apt update && apt install -y passwd adduser"
        exit 1
    fi
    
    log_info "系统环境检查完成"
}

# 安装系统依赖
install_dependencies() {
    log_info "更新系统包..."
    apt update

    log_info "安装系统依赖..."
    apt install -y curl wget git build-essential pkg-config libssl-dev nginx sqlite3

    # 安装 Rust
    if ! command -v rustc &> /dev/null; then
        log_info "安装 Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        log_info "Rust 已安装"
    fi

    # 安装 Node.js 和 pnpm
    if ! command -v node &> /dev/null; then
        log_info "安装 Node.js..."
        curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
        apt install -y nodejs
    else
        log_info "Node.js 已安装"
    fi

    if ! command -v pnpm &> /dev/null; then
        log_info "安装 pnpm..."
        npm install -g pnpm
    else
        log_info "pnpm 已安装"
    fi
}

# 配置项目
configure_project() {
    local PROJECT_DIR="/opt/generate-download-url"
    local SERVICE_USER="gurl"

    log_info "创建项目目录..."
    mkdir -p $PROJECT_DIR
    
    # 创建服务用户
    if ! id "$SERVICE_USER" &>/dev/null; then
        log_info "创建服务用户 $SERVICE_USER..."
        if command -v useradd &> /dev/null; then
            useradd --system --home $PROJECT_DIR --shell /bin/false $SERVICE_USER
        elif command -v adduser &> /dev/null; then
            adduser --system --home $PROJECT_DIR --shell /bin/false --no-create-home $SERVICE_USER
        else
            log_error "无法找到 useradd 或 adduser 命令"
            log_info "请安装必要的包: apt install -y passwd adduser"
            exit 1
        fi
        log_info "服务用户 $SERVICE_USER 创建成功"
    else
        log_info "服务用户 $SERVICE_USER 已存在"
    fi

    # 复制项目文件
    log_info "复制项目文件..."
    cp -r . $PROJECT_DIR/
    chown -R $SERVICE_USER:$SERVICE_USER $PROJECT_DIR

    cd $PROJECT_DIR
}

# 构建后端
build_backend() {
    local PROJECT_DIR="/opt/generate-download-url"
    local SERVICE_USER="gurl"
    
    log_info "构建 Rust 后端..."
    cd $PROJECT_DIR/backend
    
    # 设置 Rust 环境
    export PATH="$HOME/.cargo/bin:$PATH"
    
    # 构建后端（在 root 用户下构建，因为 Rust 安装在 root 用户环境中）
    log_info "构建 Rust 后端..."
    cd $PROJECT_DIR/backend
    cargo build --release
    
    # 确保二进制文件可执行并设置正确的所有权
    chmod +x $PROJECT_DIR/backend/target/release/backend
    chown $SERVICE_USER:$SERVICE_USER $PROJECT_DIR/backend/target/release/backend
}

# 构建前端
build_frontend() {
    local PROJECT_DIR="/opt/generate-download-url"
    local SERVICE_USER="gurl"
    
    log_info "构建 React 前端..."
    cd $PROJECT_DIR/frontend
    
    # 构建前端（在 root 用户下构建，因为 Node.js/pnpm 安装在 root 用户环境中）
    pnpm install
    pnpm run build
    
    # 设置构建产物的正确所有权
    chown -R $SERVICE_USER:$SERVICE_USER $PROJECT_DIR/frontend/dist
    chown -R $SERVICE_USER:$SERVICE_USER $PROJECT_DIR/frontend/node_modules
}

# 创建 systemd 服务
create_systemd_service() {
    local PROJECT_DIR="/opt/generate-download-url"
    local SERVICE_USER="gurl"
    
    log_info "创建 systemd 服务文件..."
    
    cat > /etc/systemd/system/generate-download-url.service << EOF
[Unit]
Description=Generate Download URL Backend Service
After=network.target
Wants=network.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$PROJECT_DIR/backend
Environment=PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
EnvironmentFile=$PROJECT_DIR/backend/.env
ExecStart=$PROJECT_DIR/backend/target/release/backend
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=generate-download-url
KillMode=mixed
TimeoutStopSec=30

# 安全设置
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$PROJECT_DIR/backend/data
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
EOF

    # 重新加载 systemd
    systemctl daemon-reload
    
    # 启用并启动服务
    log_info "启动服务..."
    systemctl enable generate-download-url
    systemctl start generate-download-url
    
    # 检查服务状态
    sleep 3
    if systemctl is-active --quiet generate-download-url; then
        log_info "后端服务启动成功！"
    else
        log_error "后端服务启动失败！"
        systemctl status generate-download-url
        exit 1
    fi
}

# 配置 Nginx
configure_nginx() {
    local PROJECT_DIR="/opt/generate-download-url"
    
    log_info "配置 Nginx..."
    
    # 备份原始配置
    if [[ -f /etc/nginx/sites-available/default ]]; then
        cp /etc/nginx/sites-available/default /etc/nginx/sites-available/default.backup
    fi
    
    # 创建前端站点配置
    cat > /etc/nginx/sites-available/gurl.honahec.cc << 'EOF'
server {
    listen 80;
    server_name gurl.honahec.cc;
    
    # 重定向到 HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name gurl.honahec.cc;
    
    # SSL 证书配置 (请替换为实际证书路径)
    # ssl_certificate /etc/ssl/certs/gurl.honahec.cc.crt;
    # ssl_certificate_key /etc/ssl/private/gurl.honahec.cc.key;
    
    # SSL 配置
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    
    # 静态文件服务
    location / {
        root /opt/generate-download-url/frontend/dist;
        try_files $uri $uri/ /index.html;
        
        # 缓存静态资源
        location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
            expires 1y;
            add_header Cache-Control "public, immutable";
        }
    }
    
    # 管理功能 API 代理
    location ~ ^/(login|sign|links) {
        proxy_pass http://127.0.0.1:8003;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # CORS headers
        add_header Access-Control-Allow-Origin *;
        add_header Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS";
        add_header Access-Control-Allow-Headers "Content-Type, Authorization";
        
        # 处理预检请求
        if ($request_method = OPTIONS) {
            return 204;
        }
    }
    
    # 安全头
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Referrer-Policy strict-origin-when-cross-origin;
}
EOF

    # 创建后端 API 站点配置
    cat > /etc/nginx/sites-available/api.honahec.cc << 'EOF'
server {
    listen 80;
    server_name api.honahec.cc;
    
    # 重定向到 HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name api.honahec.cc;
    
    # SSL 证书配置 (请替换为实际证书路径)
    # ssl_certificate /etc/ssl/certs/api.honahec.cc.crt;
    # ssl_certificate_key /etc/ssl/private/api.honahec.cc.key;
    
    # SSL 配置
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    
    # 下载重定向服务
    location /download/ {
        proxy_pass http://127.0.0.1:8003/download/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # 禁用缓存下载链接
        add_header Cache-Control "no-cache, no-store, must-revalidate";
        add_header Pragma "no-cache";
        add_header Expires "0";
    }
    
    # 健康检查
    location /health {
        access_log off;
        return 200 "OK\n";
        add_header Content-Type text/plain;
    }
    
    # 安全头
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
}
EOF
    
    # 启用站点
    ln -sf /etc/nginx/sites-available/gurl.honahec.cc /etc/nginx/sites-enabled/
    ln -sf /etc/nginx/sites-available/api.honahec.cc /etc/nginx/sites-enabled/
    
    # 移除默认站点
    rm -f /etc/nginx/sites-enabled/default
    
    # 测试 Nginx 配置
    nginx -t
    
    # 重启 Nginx
    systemctl restart nginx
    systemctl enable nginx
    
    log_info "Nginx 配置完成！"
}

# SSL 证书提示
ssl_certificate_reminder() {
    log_warn "重要提醒: SSL 证书配置"
    echo "1. 请为域名申请 SSL 证书："
    echo "   - gurl.honahec.cc"
    echo "   - api.honahec.cc"
    echo ""
    echo "2. 推荐使用 Let's Encrypt 免费证书："
    echo "   apt install certbot python3-certbot-nginx"
    echo "   certbot --nginx -d gurl.honahec.cc"
    echo "   certbot --nginx -d api.honahec.cc"
    echo ""
    echo "3. 或者手动配置证书，编辑以下文件："
    echo "   - /etc/nginx/sites-available/gurl.honahec.cc"
    echo "   - /etc/nginx/sites-available/api.honahec.cc"
    echo ""
}

# 显示部署信息
show_deployment_info() {
    log_info "部署完成！"
    echo ""
    echo "=========================================="
    echo "部署信息:"
    echo "=========================================="
    echo "项目目录: /opt/generate-download-url"
    echo "服务名称: generate-download-url"
    echo "服务用户: gurl"
    echo "配置文件: /opt/generate-download-url/backend/.env"
    echo "数据库文件: /opt/generate-download-url/backend/data/downloads.db"
    echo ""
    echo "常用命令:"
    echo "  查看服务状态: systemctl status generate-download-url"
    echo "  查看服务日志: journalctl -u generate-download-url -f"
    echo "  重启服务:     systemctl restart generate-download-url"
    echo "  重启 Nginx:   systemctl restart nginx"
    echo ""
    echo "访问地址:"
    echo "  管理界面: https://gurl.honahec.cc"
    echo "  下载服务: https://api.honahec.cc/download/{token}"
    echo ""
    echo "下一步:"
    echo "1. 确保域名 DNS 解析到此服务器"
    echo "2. 配置 SSL 证书"
    echo "3. 检查防火墙设置"
    echo "4. 验证服务正常运行"
    echo "=========================================="
}

# 主函数
main() {
    log_info "开始部署阿里云 OSS 签名下载链接项目..."
    
    check_root
    check_system
    install_dependencies
    configure_project
    build_backend
    build_frontend
    create_systemd_service
    configure_nginx
    ssl_certificate_reminder
    show_deployment_info
    
    log_info "部署脚本执行完成！"
}

# 错误处理
trap 'log_error "脚本执行失败！请检查错误信息。"' ERR

# 执行主函数
main "$@"
