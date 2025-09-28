# 部署脚本使用指南

本项目提供三个自动化脚本来简化部署和管理过程。

## 脚本列表

1. **deploy.sh** - 一键部署脚本
2. **manage.sh** - 服务管理脚本
3. **ssl-setup.sh** - SSL 证书配置脚本

## 使用步骤

### 1. 准备服务器环境

- **系统要求**: Ubuntu 18.04+ 或 Debian 10+
- **硬件要求**: 最小 1GB RAM, 1 CPU 核心, 10GB 磁盘空间
- **网络要求**: 服务器需要公网 IP，开放 22, 80, 443 端口

### 2. 域名配置

在部署前，请确保以下域名 DNS 记录指向您的服务器公网 IP：

- `gurl.honahec.cc` (A 记录)
- `api.honahec.cc` (A 记录)

### 3. 执行部署

```bash
# 克隆项目到临时目录
git clone <your-repo-url> /tmp/generate-download-url
cd /tmp/generate-download-url

# 赋予执行权限
chmod +x *.sh

# 执行一键部署
sudo ./deploy.sh
```

### 4. 配置环境变量

部署脚本会提示您编辑环境配置文件：

```bash
sudo nano /opt/generate-download-url/backend/.env
```

**必须配置的参数：**

```bash
ALIYUN_ACCESS_KEY_ID=your_access_key_id
ALIYUN_ACCESS_KEY_SECRET=your_access_key_secret
ALIYUN_DEFAULT_ENDPOINT=oss-cn-hangzhou.aliyuncs.com
ALIYUN_DEFAULT_BUCKET=your_bucket_name
JWT_SECRET=your-secure-random-jwt-secret
ADMIN_USERNAME=admin
ADMIN_PASSWORD=your-secure-password
```

### 5. 配置 SSL 证书（推荐）

```bash
sudo ./ssl-setup.sh
```

该脚本会：

- 安装 Certbot
- 申请 Let's Encrypt 免费证书
- 配置自动续期
- 更新 Nginx 配置

### 6. 验证部署

```bash
# 检查服务状态
sudo ./manage.sh status

# 查看服务日志
sudo ./manage.sh logs

# 测试访问
curl -k https://gurl.honahec.cc
curl -k https://api.honahec.cc/health
```

## 服务管理

### 常用命令

```bash
# 查看服务状态
sudo ./manage.sh status

# 启动/停止/重启服务
sudo ./manage.sh start
sudo ./manage.sh stop
sudo ./manage.sh restart

# 查看日志
sudo ./manage.sh logs        # 最近50行
sudo ./manage.sh logs 100    # 最近100行
sudo ./manage.sh follow      # 实时跟踪

# 数据库管理
sudo ./manage.sh backup      # 备份数据库
sudo ./manage.sh restore     # 恢复数据库
sudo ./manage.sh clean       # 清理过期数据

# 服务更新
sudo ./manage.sh update      # 从git拉取更新并重新构建

# Nginx 管理
sudo ./manage.sh nginx       # 交互式Nginx管理
```

### 文件位置

- **项目目录**: `/opt/generate-download-url`
- **服务配置**: `/etc/systemd/system/generate-download-url.service`
- **Nginx 配置**:
  - `/etc/nginx/sites-available/gurl.honahec.cc`
  - `/etc/nginx/sites-available/api.honahec.cc`
- **数据库文件**: `/opt/generate-download-url/backend/data/downloads.db`
- **日志文件**: `journalctl -u generate-download-url`

## 故障排除

### 服务无法启动

```bash
# 查看详细错误信息
sudo systemctl status generate-download-url
sudo journalctl -u generate-download-url -n 50

# 检查配置文件
sudo nano /opt/generate-download-url/backend/.env

# 检查端口占用
sudo netstat -tlnp | grep :8003
```

### Nginx 配置错误

```bash
# 测试 Nginx 配置
sudo nginx -t

# 查看 Nginx 日志
sudo tail -f /var/log/nginx/error.log

# 重新加载配置
sudo systemctl reload nginx
```

### SSL 证书问题

```bash
# 查看证书状态
sudo certbot certificates

# 手动续期测试
sudo certbot renew --dry-run

# 重新申请证书
sudo certbot delete
sudo ./ssl-setup.sh
```

### 域名访问问题

```bash
# 检查域名解析
nslookup gurl.honahec.cc
nslookup api.honahec.cc

# 检查防火墙
sudo ufw status
sudo iptables -L

# 检查端口监听
sudo netstat -tlnp | grep -E ':80|:443|:8003'
```

## 监控和维护

### 定期维护任务

1. **数据库备份** (建议每天)

   ```bash
   sudo ./manage.sh backup
   ```

2. **清理过期数据** (建议每周)

   ```bash
   sudo ./manage.sh clean
   ```

3. **检查服务状态** (建议每天)

   ```bash
   sudo ./manage.sh status
   ```

4. **查看系统资源使用**
   ```bash
   top
   df -h
   free -m
   ```

### 性能优化

1. **数据库优化**

   ```bash
   # 定期 VACUUM 数据库
   sqlite3 /opt/generate-download-url/backend/data/downloads.db "VACUUM;"
   ```

2. **日志轮转** (已自动配置)

   ```bash
   # 查看日志大小
   sudo journalctl --disk-usage

   # 手动清理旧日志
   sudo journalctl --vacuum-time=30d
   ```

3. **Nginx 缓存** (已配置静态资源缓存)

## 安全建议

1. **定期更新系统**

   ```bash
   sudo apt update && sudo apt upgrade -y
   ```

2. **配置防火墙**

   ```bash
   sudo ufw enable
   sudo ufw allow ssh
   sudo ufw allow 80/tcp
   sudo ufw allow 443/tcp
   ```

3. **使用强密码**

   - JWT_SECRET 使用安全的随机字符串
   - ADMIN_PASSWORD 使用强密码

4. **限制阿里云 AccessKey 权限**
   - 仅授予必要的 OSS 只读权限
   - 定期轮换 AccessKey

## 备份策略

### 完整备份

```bash
#!/bin/bash
# 创建完整备份脚本
BACKUP_DIR="/backup/generate-download-url"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# 备份数据库
sudo ./manage.sh backup

# 备份配置文件
sudo cp /opt/generate-download-url/backend/.env $BACKUP_DIR/env_$DATE

# 备份 Nginx 配置
sudo cp -r /etc/nginx/sites-available/gurl.honahec.cc $BACKUP_DIR/nginx_gurl_$DATE
sudo cp -r /etc/nginx/sites-available/api.honahec.cc $BACKUP_DIR/nginx_api_$DATE

# 打包备份
sudo tar -czf $BACKUP_DIR/full_backup_$DATE.tar.gz -C /opt generate-download-url
```

## 联系支持

如果在部署过程中遇到问题，请：

1. 查看本指南的故障排除部分
2. 检查项目 README.md 文档
3. 查看服务日志获取详细错误信息
4. 提交 Issue 到项目仓库
