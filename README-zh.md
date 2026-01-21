███╗   ███╗ ██████╗██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗
████╗ ████║██╔════╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
██╔████╔██║██║     ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝
██║╚██╔╝██║██║     ██╔═══╝ ██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
██║ ╚═╝ ██║╚██████╗██║     ██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
╚═╝     ╚═╝ ╚═════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝

# McPacker

**一个命令行工具，用于将客户端模组包转换为可直接运行的 Minecraft 服务器**

[![Rust](https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-AGPL%20v3-blue?style=flat-square)](LICENSE)

转换客户端 Modrinth 和 CurseForge 模组包为完全配置的 Minecraft 服务器，只需一个命令。

[概述](#overview) • [特性](#features) • [安装](#installation) • [使用](#usage) • [示例](#examples)

---

## 概述

McPacker 是一个用 Rust 构建的快速、可靠的命令行工具，可以自动将客户端模组包转换为服务器安装。大多数模组包仅为客户端分发，需要手动设置才能作为服务器运行。McPacker 处理完整的转换管道：解析模组包文件、过滤仅客户端模组、使用哈希验证下载服务器模组、安装适当的服务器加载器，并使用您的自定义配置生成启动脚本。

无论您是为朋友设置服务器还是管理多个模组实例，McPacker 消除了手动工作，并确保可靠、可验证的安装。

## 特性

- **多种格式支持**：解析 Modrinth (`.mrpack`) 和 CurseForge (`.zip`) 模组包格式
- **智能模组过滤**：使用缓存的关键字数据库自动排除仅客户端模组
- **并行下载**：快速并发下载，具有可配置的并行性和进度跟踪
- **哈希验证**：内置 SHA-1 和 SHA-512 验证确保文件完整性
- **加载器安装**：自动检测和安装 Fabric、Forge、Quilt 和 NeoForge 加载器
- **代理支持**：完全支持 HTTP/HTTPS 代理进行下载和 API 请求
- **启动脚本生成**：使用自定义内存和 Java 设置创建平台特定的启动脚本
- **Jar 元数据提取**：从 `fabric.mod.json`、`mods.toml` 和旧版 `mcmod.info` 读取模组信息
- **增量退避**：当可用时，使用多个下载 URL 的智能重试逻辑
- **Content-Disposition 支持**：根据服务器标头自动重命名文件

## 安装

### 从源码安装

确保安装了 [Rust](https://www.rust-lang.org/tools/install) (1.75 或更高版本)，然后构建并安装：

```bash
git clone https://github.com/littlepenguin66/mcpacker.git
cd mcpacker
cargo install --path .
```

### 从 Crates.io 安装

直接从 Crates.io 安装：

```bash
cargo install mcpacker
```

### 预构建二进制文件

从 [releases 页面](https://github.com/littlepenguin66/mcpacker/releases) 下载适用于您的平台的最新版本。

## 使用

### 基本用法

转换模组包为服务器安装：

```bash
mcpacker your-modpack.mrpack
```

这将创建一个以您的模组包命名的文件夹，包含所有服务器文件、模组和启动脚本。

### 命令行选项

```
mcpacker [OPTIONS] <INPUT>

Arguments:
  <INPUT>  模组包文件路径 (.mrpack 或 .zip)

Options:
  -o, --output <PATH>           输出目录 [默认: 模组包名称]
  -m, --memory <SIZE>           服务器内存分配 [默认: 4G]
  --java-path <PATH>            Java 可执行文件路径 [默认: java]
  -p, --parallel <NUM>          并行下载 [默认: 10]
  -u, --update-list             更新仅客户端模组缓存并退出
  --keep-client                 保留仅客户端模组 (仅 Modrinth)
  --filter-client               过滤仅客户端模组 (CurseForge)
  --accept-eula                 自动接受 Minecraft EULA
  --skip-hash                   跳过模组哈希验证
  --skip-installer-verify       跳过加载器安装程序哈希验证
  --installer-hash <HASH>       加载器安装程序的预期哈希
  --proxy <URL>                 HTTP/HTTPS 代理 URL
  -h, --help                    打印帮助
  -v, --version                 打印版本
```

### 内存格式

使用标准格式指定内存分配：
- `4G` - 4 吉字节
- `4096M` - 4096 兆字节
- 不区分大小写：`4g` 或 `4G` 都可以

## 示例

### 基本服务器设置

从 Modrinth 模组包创建具有 6GB RAM 的服务器：

```bash
mcpacker my-modpack.mrpack --memory 6G
```

### 自定义输出目录

安装到特定位置：

```bash
mcpacker my-modpack.mrpack --output /path/to/server
```

### 使用代理

通过企业或隐私代理下载：

```bash
mcpacker my-modpack.mrpack --proxy http://proxy.example.com:8080
```

### 自动接受 EULA

跳过手动 EULA 接受步骤：

```bash
mcpacker my-modpack.mrpack --accept-eula
```

### CurseForge 与客户端过滤

处理 CurseForge 模组包并过滤仅客户端模组：

```bash
mcpacker my-curseforge-pack.zip --filter-client
```

### 更新仅客户端模组缓存

刷新缓存的仅客户端模组列表（适用于离线使用）：

```bash
mcpacker --update-list
```

### 高级设置

为生产服务器组合多个选项：

```bash
mcpacker my-modpack.mrpack \
  --output /srv/minecraft \
  --memory 8G \
  --java-path /usr/lib/jvm/java-17-openjdk/bin/java \
  --parallel 20 \
  --accept-eula \
  --proxy http://proxy.internal:3128
```

## 工作原理

McPacker 遵循简单的管道：

1. **解析**：读取模组包文件并提取元数据（Minecraft 版本、加载器类型、模组列表）
2. **过滤**：使用缓存的关键字匹配识别并可选排除仅客户端模组
3. **下载**：并行获取所有服务器模组，使用进度条和哈希验证
4. **安装**：下载并安装适当的服务器加载器（Fabric、Forge 等）
5. **生成**：创建平台特定的启动脚本和配置文件

该工具验证每个步骤，并在整个过程中提供清晰反馈。

## 仅客户端模组过滤

McPacker 维护一个缓存的关键字列表，用于避免下载仅客户端模组。该缓存存储在系统的标准缓存目录中：

- **Linux**：`~/.cache/mcpacker/`
- **macOS**：`~/Library/Caches/mcpacker/`
- **Windows**：`%LOCALAPPDATA%\mcpacker\cache\`

缓存会在首次运行时自动创建。使用 `--update-list` 手动刷新。

## 哈希验证

默认情况下，McPacker 使用模组包提供的 SHA-1 或 SHA-512 哈希验证下载的文件。这确保：

- 文件在下载过程中未被损坏
- 文件与模组包作者的意图完全匹配
- 防止中间人攻击

> [!WARNING]
> 使用 `--skip-hash` 禁用验证，仅应用于故障排除。同样，`--skip-installer-verify` 绕过加载器安装程序验证。

## 支持的加载器

McPacker 自动检测并安装以下 Minecraft 服务器加载器：

- **Fabric** - 轻量级模组工具链
- **Forge** - 传统模组平台
- **Quilt** - 现代 Fabric 分支
- **NeoForge** - 下一代 Forge

适当的加载器版本从模组包元数据中提取。

## 故障排除

### 下载失败

如果下载失败，McPacker 将自动：
1. 如果模组包提供了替代 URL，则尝试
2. 在重试前使用增量退避
3. 显示清晰的错误消息以进行手动干预

您可以使用 `--parallel` 增加并行性，或如果网络访问受限，使用 `--proxy`。

### 内存格式错误

确保内存值以数字开头，以 `M` 或 `G` 结尾：
- ✅ 有效：`4G`、`4096M`、`8g`
- ❌ 无效：`G4`、`4`、`4GB`

### 缺少 Java

如果加载器安装失败，请验证 Java 可用：

```bash
java -version  # 应显示 Java 17+ 用于现代模组包
```

使用 `--java-path` 指定自定义 Java 安装。

### 仅客户端模组

如果您的服务器中包含仅客户端模组：
- **Modrinth**：除非使用 `--keep-client`，否则自动过滤
- **CurseForge**：使用 `--filter-client` 启用过滤

## 性能提示

- **并行下载**：在高速连接上将 `--parallel` 增加到 20-30 以加快下载
- **代理缓存**：设置缓存代理以加快重复安装
- **本地缓存**：仅客户端模组列表在本地缓存；定期使用 `--update-list` 刷新

## 平台支持

McPacker 为您的平台生成适当的启动脚本：

- **Windows**：`start.bat`
- **Linux/macOS**：`start.sh`（具有可执行权限）

## 要求

- **Rust**：1.75 或更高版本（用于从源码构建）
- **Java**：运行 Minecraft 服务器所需（版本取决于 Minecraft 版本）
  - Minecraft 1.17+：推荐 Java 17 或更高版本
  - Minecraft 1.16.5 及更早：Java 8 或 11

## 项目结构

```
mcpacker/
├── src/
│   ├── main.rs           # CLI 接口和编排
│   ├── ops/              # 核心操作（下载、安装、生成）
│   ├── parsers/          # 模组包格式解析器
│   ├── models/           # 数据结构和类型
│   ├── ui/               # 终端 UI 助手
│   └── utils.rs          # 实用函数
├── Cargo.toml            # Rust 依赖
└── README.md             # 此文件
```

## 常见问题

**Q: 我可以用这个用于客户端安装吗？**  
A: McPacker 专门为服务器安装设计。使用原生启动器（Modrinth App、CurseForge）进行客户端设置。

**Q: 如果模组下载失败怎么办？**  
A: McPacker 将报告失败并继续其他模组。检查输出以获取特定错误消息。

**Q: 生成后可以自定义启动脚本吗？**  
A: 是的！生成的脚本是标准的 shell/batch 文件，您可以手动编辑。

**Q: 这适用于私有模组包吗？**  
A: McPacker 适用于任何本地模组包文件。首先下载包文件，然后用 McPacker 处理。

## 致谢

使用 Rust 构建，并由优秀 crate 提供支持，包括：
- [clap](https://github.com/clap-rs/clap) - 命令行解析
- [tokio](https://tokio.rs/) - 异步运行时
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP 客户端
- [indicatif](https://github.com/console-rs/indicatif) - 进度条
- [serde](https://serde.rs/) - 序列化框架

## 灵感来源

McPacker 从 Minecraft 模组生态系统中的两个优秀工具中汲取灵感：

- **PCL** - 由于其强大的模组包处理和用户友好的 Minecraft 实例管理方法
- **ServerPackCreator** - 由于其全面的服务器设置自动化和模组包转换能力

## 支持

如果您遇到问题或有疑问：

1. 检查[故障排除部分](#troubleshooting)
2. 搜索现有 [GitHub issues](https://github.com/littlepenguin66/mcpacker/issues)
3. 使用您的设置和错误的详细信息打开新 issue