# ZfFS 对象存储系统 —— 自研产品完整改造方案


## 一、项目概述

### 1.1 背景与目标

基于 RustFS 服务端与 `rc` 客户端，通过源码级定制与品牌化改造，打造自研对象存储产品 **ZfFS（Zf File System）** 及其配套命令行客户端 **`zfc`（Zf Client）**，全面替代原有的 MinIO + `mc` 方案。

### 1.2 产品定位

| 组件 | 原方案 | 新方案（自研） |
|:---|:---|:---|
| 服务端 | MinIO | **ZfFS** |
| 命令行客户端 | `mc` | **`zfc`** |
| 技术栈 | Go | Rust |
| 开源协议 | AGPLv3 | Apache 2.0 |

RustFS 基于 Rust 构建，性能卓越——官方测试显示其对 4KB 小对象的吞吐量是 MinIO 的 **2.3 倍**。Apache 2.0 协议对商业应用友好，无 AGPL 的合规风险。

### 1.3 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                      ZfFS 产品体系                          │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │   ZfFS 服务端 │    │  zfc 客户端  │    │  Web 控制台  │ │
│  │  (S3 API)    │    │  (CLI工具)   │    │  (管理界面)  │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │        │
│         └────────────────────┼────────────────────┘        │
│                              ▼                             │
│                    ┌──────────────────┐                    │
│                    │  ARM 服务器集群   │                    │
│                    │  (aarch64/armv7) │                    │
│                    └──────────────────┘                    │
└─────────────────────────────────────────────────────────────┘
```


## 二、源码获取与工程准备

### 2.1 获取上游源码

**ZfFS 服务端（基于 RustFS）：**
```bash
git clone https://github.com/rustfs/rustfs.git zffs
cd zffs
# 切换到稳定的生产版本标签
git checkout v1.3.0  # 或最新稳定版本
```

**zfc 客户端（基于 `rc`）：**
```bash
git clone https://github.com/rustfs/cli.git zfc
cd zfc
git checkout <latest-stable-tag>
```

### 2.2 目录结构规划

建议将两个仓库纳入统一的代码管理：

```
zffs-project/
├── zffs/                    # ZfFS 服务端（fork 自 rustfs/rustfs）
│   ├── rustfs/              # 主二进制 crate
│   ├── crates/              # 39 个库 crate
│   ├── docs/                # 文档
│   ├── Makefile             # 构建文件
│   ├── Justfile             # 替代任务运行器
│   └── build-rustfs.sh      # 构建脚本
├── zfc/                     # zfc 客户端（fork 自 rustfs/cli）
│   ├── src/
│   ├── Cargo.toml
│   └── ...
├── packaging/               # 打包配置
│   ├── deb/
│   ├── rpm/
│   └── docker/
└── docs/                    # 产品文档
    ├── user-guide/
    ├── deployment/
    └── api/
```


## 三、品牌化改造（核心定制）

### 3.1 ZfFS 服务端品牌修改

> **重要提醒**：RustFS 采用 Apache 2.0 许可证，允许自由使用、修改甚至闭源商业化，只需保留版权声明。但需注意**不得修改 RustFS 商标**——品牌化改造时将所有 "RustFS" 替换为 "ZfFS" 属于产品名称变更，不涉及商标修改。

**需要修改的文件与内容：**

| 修改类别 | 涉及文件/位置 | 修改内容 |
|:---|:---|:---|
| **项目元数据** | `Cargo.toml` | `name = "zffs"`，`description` 改为 ZfFS 描述 |
| **二进制名称** | `Cargo.toml` 中的 `[[bin]]` | `name = "zffs"` |
| **启动横幅** | `rustfs/src/main.rs` | 启动时打印 "ZfFS vX.X.X" |
| **Web 控制台** | `rustfs/src/admin/` | 页面标题、Logo、品牌色、页脚版权 |
| **API 响应头** | `rustfs/src/server/` | `Server` 响应头改为 `ZfFS` |
| **版本信息** | `rustfs/src/config/` | `--version` 输出改为 ZfFS |
| **文档与注释** | 全仓库 `docs/`、`README` | 所有 "RustFS" 替换为 "ZfFS" |
| **默认端口** | 配置文件/环境变量默认值 | 可保留 9000/9001 或自定义 |

**示例修改（Cargo.toml）：**
```toml
[package]
name = "zffs"
version = "1.0.0"
description = "ZfFS is a high-performance, distributed object storage system designed for modern cloud-native applications"
```

### 3.2 `zfc` 客户端品牌修改

| 修改类别 | 涉及文件 | 修改内容 |
|:---|:---|:---|
| **项目元数据** | `Cargo.toml` | `name = "zfc"` |
| **二进制名称** | `Cargo.toml` | `name = "zfc"` |
| **命令名称** | CLI 解析入口 | 所有 `rc` 命令前缀改为 `zfc` |
| **帮助信息** | `--help` 输出 | 所有 "rc" 替换为 "zfc" |
| **版本信息** | `--version` 输出 | 改为 "zfc vX.X.X" |
| **默认别名** | 别名配置 | 默认服务别名可设为 `local` 或 `default` |

**命令对照表：**

| 原 `rc` 命令 | 新 `zfc` 命令 | 功能 |
|:---|:---|:---|
| `rc alias set` | `zfc alias set` | 配置服务别名 |
| `rc ls` | `zfc ls` | 列出存储桶/对象 |
| `rc mb` | `zfc mb` | 创建存储桶 |
| `rc cp` | `zfc cp` | 复制/上传/下载 |
| `rc mirror` | `zfc mirror` | 镜像同步 |
| `rc find` | `zfc find` | 查找对象 |
| `rc share` | `zfc share` | 生成分享链接 |
| `rc tree` | `zfc tree` | 目录树视图 |

### 3.3 视觉识别系统（VIS）

- **Logo 设计**：重新设计 ZfFS 品牌 Logo
- **配色方案**：定义主色、辅助色
- **控制台界面**：替换 Web 控制台所有 RustFS 品牌元素
- **文档模板**：统一使用 ZfFS 品牌样式


## 四、ARM 平台编译构建

### 4.1 环境准备

**在 ARM 构建机上安装 Rust 工具链：**
```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 添加 ARM 目标支持（如需要）
rustup target add aarch64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-musl
rustup target add armv7-unknown-linux-gnueabihf
```

**系统依赖：**
- 至少 4GB 内存（建议 8GB 以上）
- 支持 ARM 或 x86_64 架构
- 安装 `build-essential`、`pkg-config`、`libssl-dev` 等

### 4.2 ZfFS 服务端编译

RustFS 提供了完善的构建系统，包括 Makefile 和 Justfile。

**方式一：使用 Justfile（推荐）**

```bash
cd zffs

# 安装 just
cargo install just

# ARM64（aarch64）静态编译（musl）
just build-musl-arm64

# ARM64 动态编译（glibc）
just build-gnu-arm64
```

**方式二：使用 Makefile**

```bash
cd zffs

# 设置目标平台
export TARGET=aarch64-unknown-linux-musl
make build
```

**方式三：直接使用 Cargo**

```bash
cd zffs/rustfs

# ARM64 静态编译
cargo build --release --target aarch64-unknown-linux-musl

# ARM64 动态编译
cargo build --release --target aarch64-unknown-linux-gnu

# ARMv7 编译（如有需要）
cargo build --release --target armv7-unknown-linux-gnueabihf
```

> **说明**：RustFS 官方 CI 支持多平台构建，包括 `linux-aarch64-musl` 和 `linux-aarch64-gnu` 等目标。

### 4.3 `zfc` 客户端编译

```bash
cd zfc

# ARM64 静态编译（推荐生产环境使用）
cargo build --release --target aarch64-unknown-linux-musl

# ARM64 动态编译
cargo build --release --target aarch64-unknown-linux-gnu

# 验证编译产物
./target/aarch64-unknown-linux-musl/release/zfc --version
```

### 4.4 交叉编译（x86_64 构建机 → ARM 目标）

如果需要在 x86_64 机器上交叉编译 ARM 二进制，可使用 `cross` 工具：

```bash
# 安装 cross
cargo install cross

# 交叉编译 ARM64
cross build --release --target aarch64-unknown-linux-gnu

# 交叉编译 ARMv7
cross build --release --target armv7-unknown-linux-gnueabihf
```

### 4.5 编译产物验证

编译后需验证二进制文件：
```bash
# 检查架构
file target/aarch64-unknown-linux-musl/release/zffs
# 输出: ELF 64-bit LSB executable, ARM aarch64, ...

# 检查版本
./target/aarch64-unknown-linux-musl/release/zffs --version
# 输出: zffs 1.0.0

# 检查动态链接依赖
ldd ./target/aarch64-unknown-linux-gnu/release/zffs
```


## 五、打包与分发

### 5.1 DEB 包（适用于 Debian/Ubuntu）

使用 `rfpm` 或 `cargo-deb` 工具打包。

**使用 `cargo-deb`：**
```bash
# 安装
cargo install cargo-deb

# 在项目根目录配置 Cargo.toml 中的 [package.metadata.deb]
# 然后打包
cd zffs/rustfs
cargo deb --target aarch64-unknown-linux-gnu
```

**DEB 包目录结构：**
```
zffs_1.0.0_arm64.deb
├── /usr/bin/zffs          # 服务端二进制
├── /usr/bin/zfc           # 客户端二进制
├── /etc/zffs/config.toml  # 默认配置文件
├── /etc/systemd/system/zffs.service  # systemd 服务
└── /usr/share/doc/zffs/   # 文档
```

### 5.2 RPM 包（适用于 RHEL/CentOS/Fedora）

RustFS 官方提供了 `rustfs.spec` 文件用于 RPM 打包。

```bash
# 使用 rpmbuild
rpmbuild -ba --target aarch64 rustfs.spec
```

### 5.3 Docker 镜像

RustFS 支持多架构 Docker 镜像（amd64/arm64）。

**Dockerfile 示例（多阶段构建）：**
```dockerfile
# 第一阶段：构建
FROM rust:1.70-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --target aarch64-unknown-linux-musl

# 第二阶段：运行
FROM alpine:latest
COPY --from=builder /build/target/aarch64-unknown-linux-musl/release/zffs /usr/bin/
COPY --from=builder /build/target/aarch64-unknown-linux-musl/release/zfc /usr/bin/
EXPOSE 9000 9001
ENTRYPOINT ["zffs"]
```

**构建与推送：**
```bash
# 构建 ARM64 镜像
docker build --platform linux/arm64 -t zffs:1.0.0 .

# 推送到私有仓库
docker tag zffs:1.0.0 your-registry.com/zffs:1.0.0
docker push your-registry.com/zffs:1.0.0
```

### 5.4 压缩归档包（通用）

```bash
# 创建发布目录
mkdir -p zffs-1.0.0-linux-arm64/bin
cp zffs/target/release/zffs zffs-1.0.0-linux-arm64/bin/
cp zfc/target/release/zfc zffs-1.0.0-linux-arm64/bin/
cp -r config/ zffs-1.0.0-linux-arm64/config/

# 打包
tar -czvf zffs-1.0.0-linux-arm64.tar.gz zffs-1.0.0-linux-arm64/
```

### 5.5 分发渠道

| 分发方式 | 适用场景 | 格式 |
|:---|:---|:---|
| 内部 APT 源 | Debian/Ubuntu 用户 | `.deb` |
| 内部 YUM 源 | RHEL/CentOS 用户 | `.rpm` |
| 私有 Docker Registry | 容器化部署 | Docker 镜像 |
| 内部软件仓库 | 通用场景 | `.tar.gz` 归档 |


## 六、systemd 服务配置

### 6.1 ZfFS 服务单元文件

创建 `/etc/systemd/system/zffs.service`：

```ini
[Unit]
Description=ZfFS Object Storage Service
After=network.target

[Service]
Type=simple
User=zffs
Group=zffs
WorkingDirectory=/var/lib/zffs
Environment="ZFFS_VOLUMES=/data/zffs0,/data/zffs1,/data/zffs2,/data/zffs3"
Environment="ZFFS_ACCESS_KEY=your_access_key"
Environment="ZFFS_SECRET_KEY=your_secret_key"
Environment="ZFFS_CONSOLE_ENABLE=true"
ExecStart=/usr/bin/zffs
Restart=always
RestartSec=10
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

### 6.2 启用服务

```bash
sudo systemctl daemon-reload
sudo systemctl enable zffs
sudo systemctl start zffs
sudo systemctl status zffs
```


## 七、产品文档体系

### 7.1 用户文档

| 文档名称 | 内容 |
|:---|:---|
| **《ZfFS 部署指南》** | ARM 系统安装、配置、集群部署 |
| **《zfc 命令参考手册》** | 所有命令的完整说明与示例 |
| **《从 MinIO 迁移指南》** | 数据迁移、API 兼容性说明 |
| **《API 兼容性说明》** | S3 API 支持矩阵 |
| **《性能调优指南》** | 参数优化、硬件选型建议 |

### 7.2 运维文档

| 文档名称 | 内容 |
|:---|:---|
| **《监控与告警配置》** | Prometheus 指标、日志采集 |
| **《故障排查手册》** | 常见问题与解决方案 |
| **《备份与恢复》** | 数据备份策略、灾难恢复 |
| **《容量规划》** | 存储规模评估、扩容方案 |

### 7.3 开发文档（内部）

| 文档名称 | 内容 |
|:---|:---|
| **《ZfFS 架构设计》** | 系统架构、模块说明 |
| **《二次开发指南》** | 如何定制和扩展功能 |
| **《CI/CD 流水线》** | 自动化构建与发布流程 |


## 八、部署与运维

### 8.1 快速部署（单机模式）

```bash
# 1. 安装 ZfFS
dpkg -i zffs_1.0.0_arm64.deb  # Debian/Ubuntu
# 或
rpm -ivh zffs-1.0.0-1.aarch64.rpm  # RHEL/CentOS
# 或
tar -xzvf zffs-1.0.0-linux-arm64.tar.gz -C /opt/

# 2. 配置存储卷
mkdir -p /data/zffs{0,1,2,3}

# 3. 启动服务
systemctl start zffs

# 4. 验证
zfc alias set local http://localhost:9000 <ACCESS_KEY> <SECRET_KEY>
zfc ls local/
```

### 8.2 环境变量配置

ZfFS 继承 RustFS 的配置方式，支持以下关键环境变量：

| 环境变量 | 说明 | 示例 |
|:---|:---|:---|
| `ZFFS_VOLUMES` | 存储卷路径（多个用逗号分隔） | `/data/zffs0,/data/zffs1` |
| `ZFFS_ACCESS_KEY` | 访问密钥 | `admin` |
| `ZFFS_SECRET_KEY` | 密钥 | `password123` |
| `ZFFS_CONSOLE_ENABLE` | 启用 Web 控制台 | `true` |
| `ZFFS_CONSOLE_PORT` | 控制台端口 | `9001` |
| `ZFFS_DOMAIN` | 域名 | `s3.example.com` |

### 8.3 监控与告警

- **Prometheus 指标**：ZfFS 暴露 `/metrics` 端点
- **日志采集**：支持 JSON 格式日志输出
- **健康检查**：提供 `/health` 和 `/ready` 端点


## 九、实施路线图

| 阶段 | 周期 | 关键任务 | 交付物 |
|:---|:---|:---|:---|
| **第一阶段：源码准备** | 第 1-2 周 | Fork 仓库、搭建开发环境、理解代码结构 | 内部代码仓库 |
| **第二阶段：品牌化改造** | 第 3-4 周 | 全面替换品牌标识、修改 UI、定制命令 | ZfFS + zfc 定制源码 |
| **第三阶段：编译验证** | 第 5-6 周 | ARM 平台编译、交叉编译、功能测试 | ARM 二进制产物 |
| **第四阶段：打包分发** | 第 7-8 周 | DEB/RPM/Docker 打包、内部源搭建 | 安装包与镜像 |
| **第五阶段：文档编写** | 第 7-9 周 | 用户手册、运维手册、迁移指南 | 完整文档体系 |
| **第六阶段：灰度验证** | 第 10-12 周 | 测试环境部署、业务验证、性能测试 | 测试报告 |
| **第七阶段：生产上线** | 第 13-14 周 | 生产环境部署、监控接入、切换上线 | 正式投产 |


## 十、总结

本方案通过以下路径实现 ZfFS 自研产品的完整交付：

1. **源头可控**：基于 Apache 2.0 协议的 RustFS 源码，法律合规、商业友好
2. **品牌独立**：全面替换品牌标识，形成自有的 ZfFS 产品体系
3. **ARM 原生**：Rust 对 ARM 架构支持完善，编译产物性能优异
4. **分发多样**：支持 DEB/RPM/Docker/归档包等多种分发方式
5. **文档完备**：建立从部署到运维的完整文档体系
6. **平滑迁移**：100% S3 API 兼容，业务系统无缝切换

最终产出物包括：
- ✅ ZfFS 服务端（ARM 二进制 + 安装包）
- ✅ `zfc` 命令行客户端（ARM 二进制 + 安装包）
- ✅ Web 管理控制台（品牌化定制）
- ✅ 完整产品文档（用户手册 + 运维手册）
- ✅ Docker 镜像（ARM64）
- ✅ systemd 服务配置