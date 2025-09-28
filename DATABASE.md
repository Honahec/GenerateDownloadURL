# 数据库迁移说明

本项目使用 SQLite 数据库来持久化存储下载链接的历史记录。

## 数据库初始化

应用启动时会自动创建数据库表结构，无需手动执行迁移脚本。

## 数据库文件位置

默认数据库文件为 `downloads.db`，位于后端项目根目录下。

可通过环境变量 `DATABASE_URL` 自定义数据库文件路径：

```bash
# 使用默认路径
DATABASE_URL=sqlite:downloads.db

# 使用自定义路径
DATABASE_URL=sqlite:/path/to/your/database.db

# 使用内存数据库（仅用于测试）
DATABASE_URL=sqlite::memory:
```

## 表结构

### download_links 表

| 字段名            | 类型    | 说明                      | 约束             |
| ----------------- | ------- | ------------------------- | ---------------- |
| id                | TEXT    | 链接唯一标识符（UUID）    | PRIMARY KEY      |
| object_key        | TEXT    | OSS 对象键                | NOT NULL         |
| bucket            | TEXT    | 存储桶名                  | 可选             |
| expires_at        | TEXT    | 过期时间（ISO 8601 格式） | NOT NULL         |
| max_downloads     | INTEGER | 最大下载次数              | 可选             |
| downloads_served  | INTEGER | 已下载次数                | NOT NULL, 默认 0 |
| created_at        | TEXT    | 创建时间（ISO 8601 格式） | NOT NULL         |
| download_filename | TEXT    | 下载文件名                | 可选             |

### 索引

- `idx_download_links_expires_at`：基于过期时间的索引，用于快速查询和清理过期链接
- `idx_download_links_created_at`：基于创建时间的索引，用于按时间排序查询

## 数据备份与恢复

### 备份数据库

```bash
# 简单复制文件
cp downloads.db downloads_backup_$(date +%Y%m%d_%H%M%S).db

# 或使用 sqlite3 导出
sqlite3 downloads.db ".backup downloads_backup.db"
```

### 恢复数据库

```bash
# 从备份文件恢复
cp downloads_backup.db downloads.db

# 或使用 sqlite3 恢复
sqlite3 downloads.db ".restore downloads_backup.db"
```

## 数据库维护

### 查看数据库信息

```bash
# 连接到数据库
sqlite3 downloads.db

# 查看表结构
.schema download_links

# 查看数据统计
SELECT
    COUNT(*) as total_links,
    COUNT(CASE WHEN expires_at < datetime('now') THEN 1 END) as expired_links,
    SUM(downloads_served) as total_downloads
FROM download_links;

# 退出
.quit
```

### 手动清理过期数据

```sql
-- 删除时间过期的链接
DELETE FROM download_links
WHERE expires_at < datetime('now');

-- 删除下载次数达到限制的链接
DELETE FROM download_links
WHERE max_downloads IS NOT NULL
  AND downloads_served >= max_downloads;
```

## 性能优化

对于大量数据的场景，建议：

1. 定期清理过期数据
2. 考虑数据分区或归档策略
3. 监控数据库文件大小
4. 必要时升级到 PostgreSQL 或 MySQL

## 故障排除

### 数据库文件权限问题

```bash
# 确保应用有读写权限
chmod 644 downloads.db
chown app_user:app_group downloads.db
```

### 数据库锁定问题

如果遇到"database is locked"错误：

1. 检查是否有其他进程在访问数据库
2. 重启应用
3. 检查磁盘空间是否充足

### 数据库损坏

```bash
# 检查数据库完整性
sqlite3 downloads.db "PRAGMA integrity_check;"

# 如果损坏，尝试恢复
sqlite3 downloads.db ".recover" | sqlite3 recovered.db
```
