# ZfFS 单机二进制部署手册

[English version](zffs-single-node.md)

本文用于在一台 Linux 主机上直接运行 ZfFS 二进制程序，不注册 systemd 或其他系统服务。ZfFS 的品牌层只改变产品名称、发布物和配置入口，底层 RustFS 存储格式、S3/MinIO 兼容接口和已有数据目录保持不变。

## 1. 部署前提

支持与制品匹配的 Linux 架构：

- ARM64：`aarch64`，例如 `aarch64-unknown-linux-gnu` 或对应 MUSL 制品。
- AMD64：`x86_64`，例如 `x86_64-unknown-linux-gnu` 或对应 MUSL 制品。

部署主机需要有可执行权限、数据盘空间和一个专用运行账户。本文不要求 root 运行程序；root 只用于创建目录和调整权限。生产环境请使用实际生成的随机访问密钥和秘密密钥，不要使用示例密码。

建议的目录结构如下：

```text
/opt/zffs/bin/zffs             # 二进制
/etc/zffs/zffs.toml             # TOML 配置
/etc/zffs/access-key            # 访问密钥文件
/etc/zffs/secret-key            # 秘密密钥文件
/srv/zffs/data/                 # 对象数据目录（可替换为多个卷）
/var/lib/zffs/kms/              # 本地 KMS 数据（启用时）
```

## 2. 校验二进制

将发布包解压到临时目录，先核对架构、摘要和版本，再复制到安装目录：

```bash
tar -xzf zffs-1.0.0-linux-arm64-with-console.tar.gz -C /tmp
cd /tmp/zffs-1.0.0-linux-arm64-with-console
file ./zffs
sha256sum ./zffs
./zffs --version
```

`file` 的架构必须与主机匹配；如果发布方提供了 `.sha256` 文件，应使用 `sha256sum -c` 校验。版本输出应显示 `zffs`、ZfFS 产品版本、对应的 RustFS 基线版本以及构建提交。确认无误后安装：

```bash
install -d -m 0755 /opt/zffs/bin
install -m 0755 ./zffs /opt/zffs/bin/zffs
```

## 3. 创建目录和凭证

以下命令创建单机部署所需目录。请将权限调整为实际运行账户，而不是让数据目录长期由 root 独占：

```bash
install -d -m 0700 /etc/zffs
install -d -m 0700 /srv/zffs/data
install -d -m 0700 /var/lib/zffs/kms
install -m 0600 /dev/null /etc/zffs/access-key
install -m 0600 /dev/null /etc/zffs/secret-key
chown -R zffs:zffs /etc/zffs /srv/zffs /var/lib/zffs
```

把部署专用的访问密钥和秘密密钥分别写入文件，每个文件只包含一行值：

```bash
printf '%s\n' '<实际访问密钥>' > /etc/zffs/access-key
printf '%s\n' '<实际秘密密钥>' > /etc/zffs/secret-key
chmod 0600 /etc/zffs/access-key /etc/zffs/secret-key
```

不要把密钥写入命令历史、日志、镜像层或公开的配置仓库。若必须在 TOML 中内嵌密钥，配置文件也必须使用 `0600` 或 `0400` 权限。

## 4. 编写 TOML 配置

配置文件没有隐式默认路径，启动时必须显式传入。创建 `/etc/zffs/zffs.toml`：

```toml
version = 1

[server]
volumes = ["/srv/zffs/data"]
address = "0.0.0.0:9000"
region = "us-east-1"

[credentials]
access_key_file = "/etc/zffs/access-key"
secret_key_file = "/etc/zffs/secret-key"

[console]
enabled = true
address = "0.0.0.0:9001"
```

单机可以配置一个或多个卷。多卷示例：

```toml
[server]
volumes = [
  "/opt/yyy/rustfs/data01/rustfs0",
  "/opt/yyy/rustfs/data01/rustfs1",
  "/opt/yyy/rustfs/data01/rustfs2",
  "/opt/yyy/rustfs/data01/rustfs3",
]
```

文件必须是 UTF-8，顶层 `version` 必须为 `1`，大小不得超过 1 MiB。未知字段、未知区块、必填字段的空字符串、空卷列表、空卷项和 NUL 字符会导致启动失败。域名、可观测性、TLS、KMS 和 buffer 等可选区块只在实际使用时添加，字段格式见下表和英文完整字段表。完成后设置权限：

```bash
chmod 0600 /etc/zffs/zffs.toml
```

常用字段和环境变量对应关系：

| TOML 字段 | `ZFFS_*` 环境变量 | 说明 |
| --- | --- | --- |
| `server.volumes` | `ZFFS_VOLUMES` | TOML 是字符串数组；环境变量使用空格分隔，路径含空格时优先使用 TOML。 |
| `server.address` | `ZFFS_ADDRESS` | S3/API 监听地址，如 `0.0.0.0:9000`。 |
| `server.domains` | `ZFFS_SERVER_DOMAINS` | 环境变量使用逗号分隔。 |
| `server.region` | `ZFFS_REGION` | S3 区域。 |
| `credentials.access_key_file` | `ZFFS_ACCESS_KEY_FILE` | 推荐使用文件，不在环境中暴露密钥。 |
| `credentials.secret_key_file` | `ZFFS_SECRET_KEY_FILE` | 推荐使用文件。 |
| `console.enabled` | `ZFFS_CONSOLE_ENABLE` | 是否启用内置 Console。 |
| `console.address` | `ZFFS_CONSOLE_ADDRESS` | Console 监听地址。 |
| `tls.path` | `ZFFS_TLS_PATH` | TLS 证书目录。 |
| `kms.enabled` | `ZFFS_KMS_ENABLE` | 是否启用 KMS。 |
| `kms.backend` | `ZFFS_KMS_BACKEND` | 例如 `local`、`vault`、`aws`。 |
| `kms.key_dir` | `ZFFS_KMS_KEY_DIR` | 本地 KMS 密钥目录。 |
| `buffer.profile` | `ZFFS_BUFFER_PROFILE` | 例如 `GeneralPurpose`、`SecureStorage`。 |

完整字段表见 [英文单机部署手册](zffs-single-node.md)；未纳入 ZfFS allowlist 的兼容设置继续使用对应的 `RUSTFS_*` 名称。

## 5. 配置优先级和兼容前缀

同一配置项的优先级为：

```text
命令行参数 > TOML > 环境变量 > 内置默认值
```

ZfFS 环境变量是启动早期的兼容层，会映射到现有 RustFS 解析器；现有 `RUSTFS_*` 以及受支持的 `MINIO_*` 变量仍然保留。规则如下：

1. 只设置 `ZFFS_*` 时，映射到对应的 RustFS 配置。
2. `ZFFS_*` 与 `RUSTFS_*`（或受支持的 `MINIO_*`）值相同，可以正常启动。
3. `ZFFS_*` 与 `RUSTFS_*`/`MINIO_*` 值冲突时，在启动早期失败，不按出现顺序猜测最终值。
4. 没有 `ZFFS_*` 时，旧的 RustFS/MinIO 兼容行为继续生效。
5. 未知的 `ZFFS_*` 变量会导致启动失败；错误信息只显示变量名，不显示密钥、令牌或实际值。

例如，以下配置会被拒绝：

```bash
export ZFFS_REGION=cn-north-1
export RUSTFS_REGION=us-east-1
```

请删除冲突变量或将它们设为相同值，即使 TOML 中已经配置了 `region` 也一样。不要同时使用 TOML、环境变量和命令行设置不同的凭证来源。

## 6. 启动方式

### 6.1 使用 TOML 前台运行

直接运行二进制，不注册服务：

```bash
cd /opt/zffs/bin
./zffs server --config /etc/zffs/zffs.toml
```

前台运行时可直接看到启动错误，停止时按 `Ctrl-C`。若使用专用账户：

```bash
sudo -u zffs /opt/zffs/bin/zffs server --config /etc/zffs/zffs.toml
```

### 6.2 现有命令行参数启动

也可以完全使用命令行参数。下面是四卷 ARM64/AMD64 单机部署的示例：

```bash
/opt/zffs/bin/zffs server \
  --address 0.0.0.0:9210 \
  --console-address 0.0.0.0:9211 \
  --access-key-file /opt/yyy/rustfs/config/access-key \
  --secret-key-file /opt/yyy/rustfs/config/secret-key \
  /opt/yyy/rustfs/data01/rustfs0 \
  /opt/yyy/rustfs/data01/rustfs1 \
  /opt/yyy/rustfs/data01/rustfs2 \
  /opt/yyy/rustfs/data01/rustfs3
```

命令行参数会覆盖 TOML 和环境变量，因此排查配置时应确认没有遗留的高优先级参数。

### 6.3 可选的 `nohup` 后台运行

这仍然是直接运行二进制，不会创建系统服务：

```bash
nohup /opt/zffs/bin/zffs server --config /etc/zffs/zffs.toml \
  > /var/lib/zffs/zffs.log 2>&1 &
echo $! > /var/lib/zffs/zffs.pid
```

查看日志和停止进程：

```bash
tail -f /var/lib/zffs/zffs.log
kill "$(cat /var/lib/zffs/zffs.pid)"
```

请根据主机权限和日志轮转策略选择日志目录；不要把凭证写入启动命令或日志。

## 7. API、Console 和健康检查

在示例配置中，S3/API 使用 `9000`，Console 使用 `9001`。如果部署使用 `9210` 和 `9211`，请相应替换端口。

Console 的正确地址是：

```text
http://<服务器IP>:<Console端口>/rustfs/console/
```

例如：

```text
http://20.10.100.37:9211/rustfs/console/
```

`/zffs` 不是 Console 路由。直接访问 `http://<服务器IP>:9211/zffs` 会被当作 S3/API 请求，返回 `AccessDenied` 是预期行为，不代表登录页面损坏。

从服务器本机检查服务：

```bash
curl --noproxy '*' -f http://127.0.0.1:9000/health/live
curl --noproxy '*' -f http://127.0.0.1:9000/health/ready
```

`live` 表示进程存活；`ready` 还会检查存储、IAM 和锁等依赖是否已就绪。远程访问时确认防火墙放行 API 和 Console 端口，并检查监听地址不是 `127.0.0.1`。

## 8. Console 资源和重新构建

Console 静态资源是构建输入的一部分，`cargo build` 本身不会自动下载或生成 Console 资源。构建前必须确认：

```bash
test -s rustfs/static/index.html
```

如果文件不存在，使用仓库提供的构建脚本获取 Console 资源并完成服务端构建：

```bash
./build-rustfs.sh --force-console-update
```

如果资源已经存在，也可以直接执行 `RUSTUP_TOOLCHAIN=1.97.1 cargo build --release --locked --bin zffs`。打包前再次检查 `rustfs/static/index.html` 已嵌入制品，并在目标主机按第 7 节访问 `/rustfs/console/`。仅有服务端二进制而没有静态资源时，S3 API 仍可工作，但 Console 不会出现登录页面。

## 9. 升级、回滚和数据保护

升级前停止旧进程并备份配置和凭证，并按现有存储备份策略对数据卷执行一致性快照或离线备份。不要删除或重新格式化已有卷，也不要改变卷顺序：

```bash
cp -a /etc/zffs /etc/zffs.backup-$(date +%Y%m%d%H%M%S)
```

先用新二进制在同一数据目录启动并执行健康检查；确认 API、Console 和对象读写正常后，再替换旧二进制。发生问题时停止新进程，恢复旧的二进制和配置，使用同一组数据目录启动。品牌化改造不改变 `xl.meta`、`.metadata.bin`、纠删码、bitrot、quorum/heal 或内部 RPC 数据，因此旧 RustFS 二进制应能读取同一数据目录，但仍应在预发布环境先验证回滚。

不要在新旧进程同时写入同一组卷，不要在未完成备份时执行迁移或删除操作。升级和回滚都应记录二进制版本、配置摘要（不含密钥）和健康检查结果。

## 10. 常见问题

| 现象 | 处理方法 |
| --- | --- |
| `--config requires a file path` | 在 `--config` 后显式提供 TOML 文件路径。 |
| `Unsupported schema version` | 设置顶层 `version = 1`。 |
| TOML 校验失败 | 删除未知字段，检查类型、空值、卷路径和 URL 格式。 |
| `Insecure configuration permissions` | 对包含内嵌密钥的 TOML 和凭证文件执行 `chmod 0600` 或 `0400`。 |
| ZfFS/RustFS/MinIO 配置冲突 | 删除冲突变量或将三者设为相同值；冲突检查先于 TOML 优先级。 |
| `Unsupported ZFFS_* variable` | 修正变量名；不在 allowlist 内的设置使用对应 `RUSTFS_*` 变量。 |
| 端口已被占用 | 使用 `ss -ltnp` 检查占用进程，改用空闲 API 或 Console 端口。 |
| 健康检查出现代理错误 | 对本机检查使用 `curl --noproxy '*'`。 |
| 访问 `/zffs` 返回 `AccessDenied` | 使用 `/rustfs/console/`，例如 `http://主机:9211/rustfs/console/`。 |
| Console 没有登录页面 | 检查 `rustfs/static/index.html` 是否存在并重新执行 `build-rustfs.sh --force-console-update` 后打包。 |
| 二进制无法执行 | 用 `file ./zffs` 检查 ARM64/AMD64 是否与主机匹配，并确认文件有 `0755` 权限。 |

## 11. 兼容边界

部署时不要重命名或删除以下兼容标识：`RUSTFS_*`、受支持的 `MINIO_*`、`rustfs_*` 指标、`/rustfs/*` 和 `/minio/*` 路由、RustFS/MinIO 内部元数据键、RPC/protobuf 身份以及存储文件。详细边界见[《ZfFS 品牌化与兼容性架构约束》](../architecture/zffs-branding-compatibility.md)。
