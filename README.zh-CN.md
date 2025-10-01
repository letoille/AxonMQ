# AxonMQ

本文档也提供以下语言版本：[English](README.md) | [繁體中文](README.zh-TW.md) | [日本語](README.ja.md)

---

## 简体中文

AxonMQ 是一个用 Rust 构建的轻量级、高性能 MQTT 代理，旨在实现可靠性和可扩展性。AxonMQ 适用于从物联网数据收集到实时消息传递的广泛应用。

### ✨ 功能特性

- **多协议支持**：通过 TCP、TLS、WebSocket (WS) 和 Secure WebSocket (WSS) 支持 MQTT v3.1.1 和 v5.0。
- **高性能**：基于 Tokio 构建，利用 Rust 的性能和安全特性，实现低延迟、高吞吐量的消息传递。
- **轻量级**：资源占用极低，仅需 5MB 内存即可启动。以环保为目标，旨在消耗更少的资源、更少的电量，并排放更少的二氧化碳。
- **可配置**：通过简单的 `config.toml` 文件轻松配置监听器、TLS 设置及其他参数。
- **跨平台**：可在包括 Linux、macOS 和 Windows 在内的主流平台上编译和运行。

### 💎 支持的 MQTT 特性

| 功能                     | 支持度 | 备注                                |
| ------------------------ | :----: | ----------------------------------- |
| MQTT 协议版本            | v3.1.1, v5.0 |                                     |
| QoS 0 (最多一次)         |   ✔️    |                                     |
| QoS 1 (至少一次)         |   ✔️    |                                     |
| QoS 2 (仅有一次)         |   ✔️    |                                     |
| 保留消息 (Retained)      |   ✔️    |                                     |
| 遗嘱消息 (Last Will)     |   ✔️    |                                     |
| 持久性会话 (Persistent)  |   ✔️    | 针对 `clean_start = false`          |
| 共享订阅 (Shared)        |   ✔️    | MQTT v5 特性 (`$share/...`)         |
| 消息过期 (Expiry)        |   ✔️    | MQTT v5 特性                        |

### 🚀 快速入门

#### 0. 从发布页面下载

针对不同平台预构建的软件包可在 [GitHub 发布页面](https://github.com/letoille/AxonMQ/releases) 获取。您可以直接下载 `.deb` (适用于 Debian/Ubuntu)、`.rpm` (适用于 Rocky Linux/Centos) 和 `.zip` (适用于 Windows) 软件包。

#### 1. 从源码构建

确保您已经安装了 Rust 工具链。

```bash
git clone https://github.com/letoille/AxonMQ.git
cd AxonMQ
cargo build --release
```

#### 2. 配置 AxonMQ

编辑 `config.toml` 文件以设置您需要的监听器。默认情况下，监听器绑定到 `127.0.0.1`。如果您需要从其他机器访问代理，请将 `127.0.0.1` 更改为 `0.0.0.0`（绑定到所有可用的网络接口）或特定的 IP 地址。对于本机测试，默认的 `127.0.0.1` 已足够。

#### 3. 运行代理

```bash
./target/release/axonmq
```

代理服务将会启动，并在控制台输出其状态。

### 🔒 安全提示：TLS 证书

**警告：** `certs` 目录中包含的证书仅用于演示和测试目的。它们是不安全的，**绝不能**在生产环境中使用。

在任何正式部署中，您都应该将 `certs/server.crt` 和 `certs/server.key` 替换为您自己的证书。

- **生产环境**：强烈建议使用由受信任的证书颁发机构 (CA) 签发的证书，例如 [Let's Encrypt](https://letsencrypt.org/)。
- **开发/测试环境**：如果您需要生成新的自签名证书，可以使用以下 `openssl` 命令。这比使用默认的、公开的证书更安全。

```bash
cd certs

# 生成新的私钥和自签名证书
openssl req -x509 -newkey rsa:2048 -nodes -keyout server.key -out server.crt -days 3650 -subj "/CN=localhost"
```
此命令会为 `localhost` 域名创建一个有效期为 10 年的证书。您的 MQTT 客户端仍然需要配置为信任此自签名证书。

### 📜 许可证

本项目采用 **Business Source License 1.1** 许可证。完整信息请参阅 `LICENSE` 文件。

### 💡 未来规划

我们正在持续改进 AxonMQ。以下是未来版本中计划推出的一些主要功能：

- **深度集成 Sparkplug B**：全面支持 Sparkplug B 规范，包括高级状态管理和设备生命周期。
- **支持集群部署**：通过集群部署能力实现高可用性和水平扩展性。
- **基于 Web 的管理控制台**：一个用户友好的 Web 界面，用于监控、管理和配置 AxonMQ 代理。
- **访问控制列表 (ACL) 支持**：实现强大的 ACL，以管理客户端发布和订阅主题的权限。
- **基于磁盘的持久化**：实现强大的消息和客户端会话持久化，以确保代理重启后数据完整性。
- **高级认证机制**：支持客户端证书认证、LDAP、OAuth/JWT 及其他外部认证机制。
- **代理桥接/联邦**：允许连接多个 AxonMQ 实例或桥接到其他 MQTT 代理，以实现分布式部署。
- **指标与监控集成**：提供全面的指标，以便与 Prometheus 和 Grafana 等监控工具集成。
- **可插拔架构**：开发一个插件系统，允许用户通过自定义模块扩展代理功能，例如数据处理、与各种数据库整合，或将消息转发到 Kafka 等平台。
- **MQTT-SN 支持**：增加对 MQTT-SN 协议的支援，适用于资源受限的物联网设备。

### 🤝 贡献

欢迎提交贡献、问题和功能请求！请访问 [issues 页面](https://github.com/letoille/AxonMQ/issues)。