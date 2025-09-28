#!/bin/bash

# SSL 证书配置脚本
# 使用 Let's Encrypt 免费证书

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查域名解析
check_domain_resolution() {
    local domain=$1
    log_info "检查域名 $domain 的解析..."
    
    if ! nslookup $domain > /dev/null 2>&1; then
        log_error "域名 $domain 解析失败！"
        log_warn "请确保域名 DNS 记录指向此服务器的公网 IP"
        return 1
    fi
    
    log_info "域名 $domain 解析正常"
    return 0
}

# 安装 Certbot
install_certbot() {
    log_info "安装 Certbot..."
    
    # 更新包列表
    apt update
    
    # 安装 Certbot 和 Nginx 插件
    apt install -y certbot python3-certbot-nginx
    
    log_info "Certbot 安装完成"
}

# 申请证书
obtain_certificate() {
    local domains="gurl.honahec.cc api.honahec.cc"
    local email=""
    
    # 获取邮箱地址
    while [[ -z "$email" ]]; do
        read -p "请输入用于接收证书通知的邮箱地址: " email
        if [[ ! "$email" =~ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]]; then
            log_error "邮箱格式无效，请重新输入"
            email=""
        fi
    done
    
    # 检查域名解析
    for domain in $domains; do
        if ! check_domain_resolution $domain; then
            log_error "域名解析检查失败，无法继续"
            exit 1
        fi
    done
    
    # 确保 Nginx 正在运行
    systemctl start nginx
    
    log_info "申请 SSL 证书..."
    log_warn "注意: 申请过程中请确保 80 和 443 端口可以从外网访问"
    
    # 申请证书
    certbot --nginx \
        --email $email \
        --agree-tos \
        --no-eff-email \
        --domains gurl.honahec.cc,api.honahec.cc \
        --redirect
    
    if [[ $? -eq 0 ]]; then
        log_info "SSL 证书申请成功！"
    else
        log_error "SSL 证书申请失败！"
        exit 1
    fi
}

# 测试证书
test_certificate() {
    log_info "测试 SSL 证书..."
    
    certbot certificates
    
    # 测试自动续期
    certbot renew --dry-run
    
    if [[ $? -eq 0 ]]; then
        log_info "证书测试通过，自动续期配置正常"
    else
        log_warn "证书测试有问题，请检查配置"
    fi
}

# 设置自动续期
setup_auto_renewal() {
    log_info "配置证书自动续期..."
    
    # 创建续期脚本
    cat > /etc/cron.d/certbot-renewal << 'EOF'
# 每天凌晨2点检查证书续期
0 2 * * * root /usr/bin/certbot renew --quiet --post-hook "systemctl reload nginx"
EOF
    
    log_info "自动续期已配置 (每天凌晨2点检查)"
}

# 显示证书信息
show_certificate_info() {
    log_info "证书配置完成！"
    echo ""
    echo "=========================================="
    echo "SSL 证书信息:"
    echo "=========================================="
    certbot certificates
    echo ""
    echo "证书文件位置:"
    echo "  证书文件: /etc/letsencrypt/live/gurl.honahec.cc/fullchain.pem"
    echo "  私钥文件: /etc/letsencrypt/live/gurl.honahec.cc/privkey.pem"
    echo ""
    echo "自动续期:"
    echo "  配置文件: /etc/cron.d/certbot-renewal"
    echo "  检查时间: 每天凌晨2点"
    echo ""
    echo "手动操作命令:"
    echo "  查看证书: certbot certificates"
    echo "  续期证书: certbot renew"
    echo "  测试续期: certbot renew --dry-run"
    echo ""
    echo "现在可以通过 HTTPS 访问:"
    echo "  https://gurl.honahec.cc"
    echo "  https://api.honahec.cc"
    echo "=========================================="
}

# 主函数
main() {
    log_info "开始配置 SSL 证书..."
    
    # 检查是否为 root
    if [[ $EUID -ne 0 ]]; then
        log_error "此脚本需要 root 权限运行"
        log_info "请使用: sudo $0"
        exit 1
    fi
    
    # 检查 Nginx 是否已安装
    if ! command -v nginx &> /dev/null; then
        log_error "Nginx 未安装，请先运行部署脚本"
        exit 1
    fi
    
    # 安装 Certbot
    install_certbot
    
    # 申请证书
    obtain_certificate
    
    # 测试证书
    test_certificate
    
    # 设置自动续期
    setup_auto_renewal
    
    # 显示证书信息
    show_certificate_info
    
    log_info "SSL 证书配置完成！"
}

# 错误处理
trap 'log_error "SSL 证书配置失败！"' ERR

# 执行主函数
main "$@"
