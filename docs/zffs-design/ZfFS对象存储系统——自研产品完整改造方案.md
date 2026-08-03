# ZfFS 品牌化发行与兼容改造方案

> 状态：实施基线
>
> 更新日期：2026-08-03
>
> 适用范围：ZfFS 服务端、ZfFS Console、`zfc` 客户端与独立发布工程

## 一、目标与定位

ZfFS 是基于 RustFS 持续演进的品牌化发行版。项目目标是形成独立的产品标识、配置体验、管理控制台、命令行客户端和发布体系，同时保留 RustFS 的 S3、MinIO、磁盘格式和集群协议兼容能力。

产品组成：

| 组件 | 产品名称 | 职责 | 源码边界 |
|:---|:---|:---|:---|
| 对象存储服务端 | ZfFS | S3 数据面、管理 API、集群 RPC、后台服务 | 当前仓库 |
| 管理控制台 | ZfFS Console | 管理界面、身份登录、运维入口 | 独立 console 源码和制品 |
| 命令行客户端 | `zfc` | 桶、对象、镜像、策略和管理命令 | 独立 CLI 仓库 |
| 发布工程 | ZfFS Release | 输入锁定、构建、签名、物料、镜像、升级与回滚 | 独立 release 仓库 |

### 1.1 核心原则

1. **品牌层独立**：二进制、容器镜像、安装包、服务名、界面和用户文档统一使用 ZfFS。
2. **兼容内核稳定**：不因品牌化修改磁盘格式、纠删码、元数据键、S3 语义、RPC 签名和已有兼容路由。
3. **变更可回滚**：品牌化版本写入的数据必须仍能被同基线 RustFS 读取，除非后续有独立的格式迁移项目。
4. **上游可同步**：保留内部 crate、模块和协议名称，将品牌差异集中到少数文件和发布层。
5. **宣称与证据一致**：仅宣称已通过自动化或可重复人工验证的 S3、平台和性能范围。

### 1.2 非目标

首个 ZfFS 版本不包含以下变更：

- 不重命名全部 `rustfs-*` crate 或 Rust 模块。
- 不修改 `xl.meta`、`.metadata.bin`、bitrot、纠删码分片和 quorum 规则。
- 不删除 `/minio/*`、`/rustfs/*`、`x-minio-*` 或 `x-rustfs-*` 兼容面。
- 不因品牌化改变 S3 授权、IAM、STS、OIDC、SSE、KMS 或跨节点认证语义。
- 不宣称“100% S3 兼容”“完全兼容 AWS S3 Tables”或“无条件无缝迁移”。
- 不在品牌化项目中顺便重构存储热路径或修复无关问题。

## 二、实施基线与仓库边界

### 2.1 当前基线

| 项目 | 当前值 | 说明 |
|:---|:---|:---|
| 开发分支 | `zffs` | ZfFS 品牌化分支 |
| 本地上游基线 | `origin/main` at `035ce5d78` | 每次实施前重新 fetch 并记录实际 commit |
| Workspace 版本 | `1.0.0-beta.12` | 以根 `Cargo.toml` 为准 |
| Rust edition | 2024 | 以根 `Cargo.toml` 为准 |
| 最低 Rust 版本 | `1.97.1` | 构建镜像和 CI 必须满足 |
| Workspace 成员 | 47 | 不在文档中维护手写 crate 列表 |
| Linux ARM 发布目标 | AArch64 GNU/MUSL | 当前 CI 发布矩阵已覆盖 |
| ARMv7 | 实验性 | 脚本有目标字符串，但未进入当前正式 CI 发布矩阵 |

不得在实施和发布文档中使用未确认的“最新版本”或假定标签。每个 ZfFS 发布必须记录确切的 RustFS 版本和 commit。

### 2.2 多仓库模型

ZfFS 服务端、Console、`zfc` 和发布工程分别使用独立仓库。发布仓库不复制或长期保存三套产品源码，而是在隔离工作目录检出锁定的 commit，并负责统一构建、组装和验证。

```text
zffs-release/
|-- release-lock.toml       # 服务端、Console、zfc 和工具链的不可变输入
|-- packaging/              # RPM/DEB/systemd/OCI/Helm 配置
|-- pipelines/              # CI/CD 和签名策略
|-- scripts/                # 可重复构建与验证入口
`-- tests/                  # 安装、升级、回滚和制品 smoke
```

`release-lock.toml` 作为构建输入，至少包含：

```toml
schema_version = 1
product_version = "1.0.0"
server_repo = "<approved-zffs-server-repository>"
server_upstream_version = "1.0.0-beta.12"
server_upstream_commit = "<rustfs-commit>"
server_zffs_commit = "<zffs-server-commit>"
console_repo = "<approved-zffs-console-repository>"
console_commit = "<zffs-console-commit>"
console_asset_sha256 = "<sha256>"
cli_repo = "<approved-zfc-repository>"
cli_commit = "<zfc-commit>"
rust_toolchain = "1.97.1"
build_image_digest = "sha256:<digest>"
```

发布流水线只能消费该文件中的 commit、版本和 digest，不允许解析可变分支、`latest` 标签或未校验下载地址。构建完成后生成只读的 `release-manifest.json`，记录 release 仓库 commit、输入锁文件摘要、目标平台、每个制品的 digest、SBOM、签名和测试证据，并随所有发布渠道归档。输入锁文件负责回答“用什么构建”，输出清单负责回答“实际发布了什么”，两者不得混为一个可被流水线回写的文件。

Console 不是 `rustfs/src/admin/` 中的前端源码。当前服务端构建脚本会下载 console 静态制品，因此 ZfFS 必须将下载源、制品校验和版本锁定一并改为自有发布链路。

## 三、分层品牌化架构

```text
+------------------------------------------------------------------+
| ZfFS 品牌层                                                    |
| zffs binary | ZfFS Console | zfc | OCI/RPM/DEB/Helm | docs       |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| 品牌适配层                                                       |
| ZFFS_* aliases | product/version constants | release manifest    |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| RustFS 兼容内核                                                  |
| S3/IAM/KMS | /rustfs + /minio | x-rustfs + x-minio | xl.meta    |
+-------------------------------+----------------------------------+
                                |
+-------------------------------v----------------------------------+
| 存储与集群层                                                     |
| ECStore | erasure | quorum | heal | replication | internode RPC  |
+------------------------------------------------------------------+
```

### 3.1 改造边界

| 对象 | 首版策略 | 原因 |
|:---|:---|:---|
| 二进制名、CLI 帮助、版本输出 | 改为 ZfFS/`zffs` | 用户可见的产品标识 |
| 镜像、安装包、systemd、Helm | 改为 ZfFS | 独立发布所需 |
| Console 标题、Logo、主题、官网链接 | 改为 ZfFS | 产品体验 |
| 用户手册与部署示例 | 改为 ZfFS | 产品体验 |
| `rustfs` package/lib 和 `rustfs-*` crate | 保留 | 减少上游合并冲突和依赖重写 |
| 现有 `/rustfs/*`、`/minio/*` 路由 | 保留 | Console、mc、混合版本和管理 API 兼容 |
| `x-rustfs-*`、`x-minio-*` 头和元数据 | 保留 | 已有数据和 MinIO 互操作契约 |
| `rustfs_*` 指标名 | 首版保留 | 避免破坏已有告警和仪表盘 |
| tracing target、protobuf package、RPC method | 保留 | 内部诊断和混合集群兼容 |
| 源码版权头、LICENSE、上游署名 | 保留 | Apache 2.0 再发布要求 |

不允许使用“将全仓库的 RustFS 替换为 ZfFS”作为实施手段。测试固件、协议常量、元数据键、兼容路由、上游链接和版权声明中的 RustFS/MinIO 具有语义，不是品牌残留。

### 3.2 版权、许可证与商标

- 保留根 `LICENSE` 和源文件中与衍生作品有关的版权、专利、商标和署名声明。
- 按 Apache 2.0 要求为被修改文件保留变更可识别性，并随二进制、镜像和安装包提供许可证文本。
- ZfFS 对外材料应明确说明其基于 RustFS 的具体版本和 commit，不暗示得到 RustFS, Inc. 背书。
- ZfFS 名称、Logo、域名和对外宣称在正式发布前经法务和商标审核。
- 本节是工程交付要求，不替代法律意见。

## 四、服务端品牌层改造

### 4.1 产品标识集中化

增加一个轻量的产品标识边界，仅管理用户可见字段：

- 产品名：`ZfFS`
- 服务端命令：`zffs`
- 控制台名：`ZfFS Console`
- 默认镜像名：由发布注册表配置
- 产品文档和支持地址：由发布配置提供

用户可见的 CLI、启动信息、版本 API 和控制台可以复用该边界。指标名、tracing target、S3 头和持久化键不得使用该边界动态改名。

### 4.2 Cargo 和二进制策略

首版采用以下策略：

1. 保留 `[package] name = "rustfs"` 和 `[lib] name = "rustfs"`。
2. 将面向发布的 `[[bin]]` 目标改为 `zffs`，同步修改 `default-run`、构建脚本、CI 和打包路径。
3. 在 RPM/DEB 升级过渡期可提供 `/usr/bin/rustfs -> /usr/bin/zffs` 兼容链接，但容器入口仅使用 `zffs`。
4. 兼容链接要有任务编号、负责人、移除版本和验证条件；源码中的兼容入口同时登记 `RUSTFS_COMPAT_TODO(<task-id>)`，不维护第二套独立编译的服务端二进制。

不通过重命名所有 workspace dependency 获得表面上的品牌一致性。

### 4.3 `ZFFS_*` 配置适配

对外文档使用 `ZFFS_*`，内部仍以现有 `RUSTFS_*` 为解析目标。实现应扩展 `crates/utils/src/envs.rs` 的现有前缀兼容规划器，并在进程入口创建 Tokio runtime、初始化日志或读取任何配置之前完成一次解析；不得另写一套只服务 CLI 参数的映射器。

首批必须覆盖：

| ZfFS 变量 | 内部目标 | 值格式 |
|:---|:---|:---|
| `ZFFS_VOLUMES` | `RUSTFS_VOLUMES` | 多个卷使用空格分隔，支持现有 ellipsis 展开 |
| `ZFFS_ADDRESS` | `RUSTFS_ADDRESS` | `HOST:PORT` |
| `ZFFS_SERVER_DOMAINS` | `RUSTFS_SERVER_DOMAINS` | 逗号分隔的 S3 域名 |
| `ZFFS_ACCESS_KEY` | `RUSTFS_ACCESS_KEY` | 敏感值，不写入日志 |
| `ZFFS_ACCESS_KEY_FILE` | `RUSTFS_ACCESS_KEY_FILE` | 凭证文件路径 |
| `ZFFS_SECRET_KEY` | `RUSTFS_SECRET_KEY` | 敏感值，不写入日志 |
| `ZFFS_SECRET_KEY_FILE` | `RUSTFS_SECRET_KEY_FILE` | 凭证文件路径 |
| `ZFFS_ROOT_USER` | `RUSTFS_ROOT_USER` | 旧凭证别名；仅用于迁移兼容 |
| `ZFFS_ROOT_PASSWORD` | `RUSTFS_ROOT_PASSWORD` | 旧凭证别名；敏感值，仅用于迁移兼容 |
| `ZFFS_CONSOLE_ENABLE` | `RUSTFS_CONSOLE_ENABLE` | 布尔值 |
| `ZFFS_CONSOLE_ADDRESS` | `RUSTFS_CONSOLE_ADDRESS` | `HOST:PORT` |
| `ZFFS_TLS_PATH` | `RUSTFS_TLS_PATH` | TLS 材料目录 |
| `ZFFS_REGION` | `RUSTFS_REGION` | S3 region |
| `ZFFS_OBS_ENDPOINT` | `RUSTFS_OBS_ENDPOINT` | OTLP/HTTP base URL |

首批列表不是全部配置面。实施时从公开支持的 `RUSTFS_*` 配置清单生成逐项盘点，KMS、通知、审计、复制、协议边车等实例化配置采用“允许前缀 + 允许字段”规则；不得直接沿用一份不完整的 MinIO 后缀清单，也不得对任意环境变量做无限制前缀替换。

冲突规则：

| 输入状态 | 处理 |
|:---|:---|
| 只设置一个受支持前缀 | 解析为对应 `RUSTFS_*` 后使用现有配置读取路径 |
| 多个前缀的值相同 | 接受；只记录变量名和采用的规范键 |
| `ZFFS_*` 与 `RUSTFS_*` 值不同 | 启动失败；错误仅列变量名，不列值 |
| `ZFFS_*` 与 `MINIO_*` 值不同 | 启动失败；映射顺序不得决定结果 |
| 仅 `RUSTFS_*` 与 `MINIO_*` 值不同 | 保留现有行为：`RUSTFS_*` 优先并记录脱敏警告 |

规划器必须先读取原始环境并完成全部冲突检查，再一次性应用映射。发现任一 ZfFS 冲突时不得留下部分写入的 `RUSTFS_*` 环境，后续错误路径也不得继续启动服务。

配置解析错误、冲突日志和 `Debug` 输出不得包含 access key、secret key、KMS key、token、签名或原始凭证响应。

### 4.4 HTTP、S3 和集群路由

首版不重命名路由。

- 保留仓库当前实际注册的 `/rustfs/admin/*`、`/minio/admin/*`、`/rustfs/rpc/*` 和 tonic RPC 方法路径；不虚构当前不存在的 `/minio/rpc/*`。
- 保留 `/health`、`/health/live`、`/health/ready` 及 MinIO health 别名。
- 保留 S3 path-style 和 virtual-host-style 语义。
- 不在品牌化中改动 RPC HMAC 负载、方法路径、时间戳或密钥派生。
- 若后续增加 `/zffs/*` 别名，必须作为独立的兼容变更，验证鉴权、readiness bypass、CORS、限流和混合版本行为，不得只复制路由注册。

### 4.5 指标和日志

- 首版继续暴露 `rustfs_*` 指标，ZfFS 仪表盘直接消费它们。
- 不同时暴露一套等价 `zffs_*` 指标，避免双倍时序、额外注册和告警歧义。
- 用户可见启动文案可改为 ZfFS，结构化日志的稳定字段、event 和 tracing target 保持不变。
- 任何新日志均遵守敏感信息脱敏和热路径噪声限制。

## 五、Console 与 `zfc`

### 5.1 ZfFS Console

Console 品牌化在独立 console fork 中实施，服务端仅负责嵌入已锁定和已校验的静态制品。

必改项：

- Logo、产品名、favicon、页面标题、支持链接和版本页面。
- 发布资产名、下载地址、SHA-256 校验和制品版本锁定。
- Console 调用的管理 API 保持现有路由和权限模型。
- OIDC callback 使用配置和允许列表中的 origin，不从不可信 `Host` 或转发头构造凭证回调地址。
- CORS 默认关闭；不反射任意 Origin 同时允许凭证。
- 对象预览与 Console 凭证隔离 origin，不依赖文件扩展名判定安全性。

### 5.2 `zfc`

`zfc` 在独立 CLI fork 中实施，不在服务端仓库内伪造 CLI 构建产物。

改造范围：

- 命令名、帮助、版本、配置目录、默认别名和更新源。
- 保留 SigV4、S3 路径、AWS header 和 MinIO 客户端所需的协议语义。
- 配置迁移支持从原 CLI 读取或显式导入，不在首次启动时无提示覆盖原配置。
- 密钥只写入权限受限的配置或操作系统密钥存储，帮助、日志、shell 补全和崩溃输出不显示凭证。
- `zfc` 分别对 ZfFS、同基线 RustFS、MinIO 和已声明支持的 AWS S3 表面运行兼容测试。

## 六、构建、打包与发布

### 6.1 受支持的构建方式

构建环境使用根 `Cargo.toml` 声明的 Rust 版本，不在文档和 Dockerfile 中固定更低的工具链。

当前 ARM64 服务端基线命令：

```bash
make build-musl-arm64
make build-gnu-arm64
```

等价脚本入口：

```bash
./build-rustfs.sh --platform aarch64-unknown-linux-musl
./build-rustfs.sh --platform aarch64-unknown-linux-gnu
```

完成 `zffs` Cargo bin 改造后，上述脚本必须显式构建 `--bin zffs`。在那之前，不得通过仅修改输出文件名来假装产物已经完成品牌化。

ARMv7 必须在拥有独立 CI 构建、运行时启动和 S3 smoke 证据后才能进入支持矩阵。

### 6.2 OCI 镜像

- 复用当前 `Dockerfile`、`Dockerfile.glibc`、`Dockerfile.source` 和 `docker-buildx.sh` 的多架构逻辑。
- 构建阶段使用项目工具链版本，不使用 `rust:1.70-alpine` 之类无法满足 workspace 要求的基础镜像。
- 运行镜像继续使用非 root 用户，统一 UID/GID、卷权限、TLS 材料权限和只读根文件系统策略。
- Console 制品由 `release-lock.toml` 锁定，构建不从未版本化的 `latest` URL 静默获取生产资产。
- 镜像标签、OCI label、健康检查和 entrypoint 使用 ZfFS 发布信息。

### 6.3 发布物料

服务端和客户端分开打包，不要求安装 ZfFS 服务端时必须捆绑 `zfc`。

| 物料 | 建议名称 | 必需附件 |
|:---|:---|:---|
| 服务端归档 | `zffs-<version>-<target>.tar.gz` | SHA-256、签名、LICENSE、发布清单 |
| RPM | `zffs-server` | systemd unit、升级/卸载脚本、LICENSE |
| DEB | `zffs-server` | systemd unit、升级/卸载脚本、LICENSE |
| CLI 包 | `zfc` | shell completion、SHA-256、签名、LICENSE |
| OCI 镜像 | `<registry>/zffs/zffs:<version>` | OCI labels、SBOM、digest、签名 |
| Console 制品 | `zffs-console-<version>.zip` | SHA-256、源码 commit、与 server 的兼容声明 |

DEB 配置当前尚未存在，必须作为独立交付实现和验证；不将 `cargo deb` 当作已经可用的现有能力。

### 6.4 版本规则

ZfFS 使用独立的产品版本，同时暴露上游基线。版本不通过“查找最新 tag 并自动加一”生成，必须由发布输入决定并可重复构建。

建议输出：

```text
ZfFS 1.0.0
RustFS base: 1.0.0-beta.12
Build commit: <zffs-commit>
```

同一产品版本和目标平台只能对应一组不可变 digest。发布任务按产品版本互斥执行：先将所有制品、SBOM、签名和测试证据写入暂存区，校验完整后再发布 `release-manifest.json`，最后原子更新下载索引或 `latest` 渠道指针。重试只能复用相同 digest；发现同版本已有不同 digest 时必须失败，不得覆盖或拼接部分成功的发布。

## 七、部署基线

### 7.1 环境文件

在 `ZFFS_*` 映射实现后，单节点四卷示例为：

```bash
ZFFS_VOLUMES="/data/zffs0 /data/zffs1 /data/zffs2 /data/zffs3"
ZFFS_ADDRESS="0.0.0.0:9000"
ZFFS_CONSOLE_ENABLE="true"
ZFFS_CONSOLE_ADDRESS="0.0.0.0:9001"
ZFFS_ACCESS_KEY_FILE="/run/credentials/zffs-access-key"
ZFFS_SECRET_KEY_FILE="/run/credentials/zffs-secret-key"
```

不在文档、镜像、systemd unit 或 Helm values 中提供生产默认密钥。安装器应要求运维人员提供凭证或为每个安装生成独立随机凭证。

### 7.2 systemd

安装包提供 `zffs.service`，但凭证不直接内联在 unit 中。RPM/DEB 必须声明与旧 `rustfs` 服务端包的替代或冲突关系，禁止两个包同时管理同一数据目录和端口。升级脚本执行以下受控迁移：检测并停止 `rustfs.service`，保留原 unit 和环境文件，创建 `/etc/zffs/zffs.env` 或显式继续引用旧文件，沿用原服务用户访问已有数据目录，并只在 ZfFS readiness 通过后禁用旧服务。任一步失败都恢复旧 unit、配置和二进制，不删除用户数据。

```ini
[Unit]
Description=ZfFS Object Storage Service
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
User=<existing-storage-user>
Group=<existing-storage-group>
WorkingDirectory=/var/lib/zffs
EnvironmentFile=-/etc/zffs/zffs.env
ExecStart=/usr/bin/zffs server
Restart=on-failure
RestartSec=10
LimitNOFILE=65536
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

全新安装默认创建 `zffs` 用户并将上述占位符渲染为 `zffs`；从 RustFS 升级时默认沿用原用户和组。安装脚本只创建目录和服务用户，不递归修改已有数据目录的所有权；只有目标路径由当次安装新建且经运维人员确认时才设置其所有权。

### 7.3 健康与可观测性

- Liveness：`/health` 或 `/health/live`。
- Readiness：`/health/ready`。
- MinIO 兼容健康路径继续保留。
- S3 数据面在 `FullReady` 前返回 `503 Service Unavailable`，管理、RPC 和健康路径有各自的 readiness 规则。
- 仪表盘首版使用现有 `rustfs_*` 指标，对外面板标题显示 ZfFS。

## 八、升级、迁移与回滚

### 8.1 升级前置条件

- 记录当前 RustFS 二进制版本、commit、启动参数、环境变量和卷列表。
- 保留原二进制、安装包、镜像 digest 和 Console 制品。
- 备份 IAM、bucket metadata、KMS 配置和关键运维配置，不将对象数据备份等同于元数据备份。
- 确认 ZfFS 版本未包含未受控的存储格式或协议变更。
- 对使用 MinIO SSE-S3、SSE-KMS 或 SSE-C 写入的历史对象单独评估；当前 RustFS 不宣称可读取这些 MinIO 加密对象。

### 8.2 升级策略

1. 在隔离环境复制实际配置和数据样本。
2. 使用原 `RUSTFS_*` 配置启动 ZfFS，确认不需立即改名配置即可升级。
3. 切换到 `ZFFS_*`，验证与原配置的解析结果一致。
4. 验证 S3、IAM、KMS、Console、通知、审计、复制和节点 RPC。
5. 分布式升级前，分别验证“旧节点读取 ZfFS 新写入”和“ZfFS 节点读取旧节点新写入”，覆盖对象写入、版本、delete marker、Multipart、IAM/bucket metadata 更新和 RPC。只有测试通过的相邻版本对才允许滚动升级；否则执行全停机升级。

### 8.3 回滚标准

在 ZfFS 未修改磁盘或协议格式的前提下，回滚至记录的 RustFS 基线。回滚演练必须证明：

- ZfFS 写入的新对象可被原 RustFS 基线完整读取。
- 版本对象、delete marker、Multipart、标签、策略和 bucket metadata 未丢失。
- 原 `RUSTFS_*` 配置仍可直接使用。
- 恢复原包、unit、服务用户和环境文件后，原 RustFS 可以在不变更数据目录所有权的情况下启动。
- 监控、通知、审计和外部复制目标未因品牌迁移而丢失配置。

## 九、验证和发布门禁

### 9.1 品牌和配置测试

以下每一项都需要能在回退对应实现时失败的测试：

- `zffs --help`、`zffs --version` 和启动信息仅展示 ZfFS 产品标识。
- 仅 `ZFFS_*`、仅 `RUSTFS_*`、仅 `MINIO_*`、三方相同值、ZfFS/RustFS 冲突、ZfFS/MinIO 冲突和现有 RustFS/MinIO 冲突行为全部覆盖。
- 在冲突集合中混入可映射变量，验证失败时没有任何 `RUSTFS_*` 被部分写入；runtime 和日志类变量验证映射发生在首次读取之前。
- 凭证和 KMS 变量的冲突、格式错误和启动失败日志不包含原始值。
- `RUSTFS_*` 现有部署不做配置改动即可启动 ZfFS。
- 不存在因品牌替换而被改名的 `x-rustfs-*`、`x-minio-*`、RPC 方法或持久化字段。

### 9.2 兼容性测试

- 运行与变更相关的单元和集成测试。
- 运行 S3 兼容矩阵的 implemented 集合，不通过移除或放宽测试获得通过。
- 运行 MinIO 互操作和真实 `xl.meta` / bucket metadata fixture 测试。
- 使用 AWS SDK、`mc`、`zfc` 验证 path-style、virtual-host-style、签名请求和 Multipart。
- 验证旧 RustFS 写入 -> ZfFS 读取、ZfFS 写入 -> 旧 RustFS 读取。
- 混合集群测试必须按每个允许的相邻版本对双向执行，覆盖对象写入、版本、delete marker、Multipart、IAM/bucket metadata 更新、具体 RPC 方法签名、readiness、lock quorum 和节点关停。

### 9.3 安全测试

- 所有管理和诊断路由仍进行操作特定的管理鉴权，不将品牌路由错误加入公开白名单。
- Console 回归覆盖未授权管理访问、OIDC 回调 origin、CORS 凭证、对象预览隔离和敏感端点。
- RPC 回归覆盖错误 secret、过期时间戳、错误路径、跨方法重放和恶意负载。
- 容器以非 root 身份启动，数据、日志、TLS 和 console 资产权限正确。
- 安装和升级流程不会生成公开默认凭证或在日志中打印凭证。

### 9.4 构建与运行测试

- `cargo fmt --all --check`。
- 受影响 package 的定向测试和编译检查。
- 最终跨构建、CI、打包和发布变更运行 `make pre-pr`。
- Linux x86_64 GNU/MUSL 和 AArch64 GNU/MUSL 均产生可启动二进制。
- 交叉编译产物必须在对应架构上运行 `--version`、启动、readiness 和 S3 smoke，不只检查 `file` 输出。
- RPM、DEB、OCI 镜像和归档分别验证安装、升级、回滚、卸载和权限。
- RPM/DEB 在全新安装与 RustFS 原地升级两条路径验证包替代关系、服务用户、环境文件、失败回滚和重复执行；升级或卸载不得递归改权或删除已有数据。
- 发布测试覆盖同版本并发任务、上传中断、幂等重试和 digest 冲突，证明索引只指向完整且一致的一组制品。

### 9.5 文档与宣称测试

- 文档中的命令由 CI 脚本或可重复的发布演练执行。
- S3 功能宣称与 `docs/architecture/s3-compatibility-matrix.md` 一致。
- S3 Tables 宣称与 `docs/architecture/s3-tables-support-matrix.md` 一致。
- 性能数字必须包含工具、硬件、参数、对比版本和原始结果；没有可重复证据时不对外发布数字。
- 用户文档不将“基于 RustFS 的品牌化发行版”表述为“从零自研的存储引擎”。

## 十、分阶段实施

各阶段以可验证退出条件为准，不在工程方案中承诺未经资源评估的固定周数。

| 阶段 | 变更范围 | 退出条件 |
|:---|:---|:---|
| 0. 基线和方案 | 冻结上游 commit、边界、许可证、验收矩阵 | 方案通过架构、安全、发布评审 |
| 1. 服务端标识 | 产品常量、`zffs` bin、CLI、版本输出 | 内部 crate/协议无改名，品牌测试通过 |
| 2. 配置适配 | `ZFFS_*` 映射、冲突和脱敏 | ZfFS/RustFS/MinIO 配置组合测试通过 |
| 3. 发布工程 | 构建脚本、CI、镜像、RPM/DEB、Helm、systemd | 四个 Linux 目标和安装/升级/回滚通过 |
| 4. Console | 独立 fork、制品锁定、视觉和安全回归 | 版本、OIDC、CORS、预览和鉴权通过 |
| 5. `zfc` | 独立 fork、命令、配置迁移、跨实现兼容 | ZfFS/RustFS/MinIO 客户端矩阵通过 |
| 6. 发布候选 | 全量验收、安全、SBOM、签名、升级演练 | 所有必需证据绑定到同一 release manifest |
| 7. 灰度和正式发布 | 分批部署、指标对比、回滚演练 | 无未解决阻断问题，回滚窗口结束 |

每个阶段使用独立 PR，不在同一 PR 混合产品重命名、配置语义、Console、存储逻辑和发布流程。

## 十一、风险与控制

| 风险 | 具体失败 | 控制措施 |
|:---|:---|:---|
| 全局替换品牌字符串 | 破坏磁盘元数据、路由、RPC、指标和测试固件 | 使用明确边界清单，审查每个命中项的语义 |
| 配置前缀冲突 | 使用错误凭证、卷或 KMS 后端 | 涉及 ZfFS 的不同值失败关闭；保留现有 RustFS/MinIO 优先级；错误不显示值 |
| Console 路由变更 | 绕过鉴权、错误 OIDC 回调、CORS 凭证泄漏 | 保留路由，运行安全负向测试 |
| 上游同步困难 | 长期无法吸收安全和正确性修复 | 保留内部名称，品牌差异集中，定期演练 rebase/merge |
| 制品不可重现 | 同一版本下载到不同 Console 或二进制 | 固定 commit、digest、checksum 和 release manifest |
| 过度兼容宣称 | 业务依赖未实现的 S3 特性 | 发布文案与兼容矩阵同步 |
| 未经验证的 ARMv7 | 可编译但无法启动或依赖缺失 | 首版不列为支持，建立 CI 和运行证据后再加入 |

## 十二、完成标准

ZfFS 品牌化项目只有在以下条件全部满足后才算完成：

1. 服务端、Console、`zfc`、镜像、安装包和用户文档使用一致的 ZfFS 产品信息。
2. 产物可由锁定的源码、工具链和 Console 制品重复构建。
3. `RUSTFS_*` 和已支持的 `MINIO_*` 部署可以在不修改配置的情况下升级。
4. 旧数据、旧客户端、兼容路由、指标和节点 RPC 保持可用。
5. ZfFS 写入的数据通过同基线 RustFS 回滚读取测试。
6. 管理鉴权、IAM、STS、OIDC、RPC、CORS、Console 预览和凭证日志的安全回归通过。
7. Linux x86_64 GNU/MUSL 和 AArch64 GNU/MUSL 发布制品通过对应架构的启动和 S3 smoke。
8. RPM、DEB、OCI、归档、SBOM、checksum、签名和许可证材料与 release manifest 一致。
9. S3 和 S3 Tables 宣称与仓库支持矩阵一致，性能数据可重复。
10. 已完成至少一次升级、混合版本和回滚演练，并保留与发布 commit 对应的证据。
