# AxonMQ

本文件另有以下語言版本：[English](README.md) | [日本語](README.ja.md) | [简体中文](README.zh-CN.md)

---

一款基於 Rust 建構的輕量級、高效能 MQTT 代理，為可靠性與可擴展性而設計。AxonMQ 適用於從物聯網資料收集到即時訊息傳遞的廣泛應用。

### ✨ 功能特性

- **多協定支援**: 支援基於 TCP、TLS、WebSocket (WS) 和 Secure WebSocket (WSS) 的 MQTT v3.1.1 和 v5.0。
- **高效能**: 基於 Tokio 建構，充分利用 Rust 的高效能和安全特性，實現低延遲、高吞吐量的訊息傳遞。
- **輕量級**：資源佔用極低，僅需 5MB 記憶體即可啟動。以環保為目標，旨在消耗更少的資源、更少的電量，並排放更少的二氧化碳。
- **可設定**: 透過簡單的 `config.toml` 檔案輕鬆設定接聽器、TLS 設定及其他參數。
- **跨平台**: 可在包括 Linux、macOS 和 Windows 在內的主流平台上編譯和執行。

### 💎 支援的 MQTT 特性

| 功能                     | 支援度 | 備註                                |
| ------------------------ | :----: | ----------------------------------- |
| MQTT 協定版本            | v3.1.1, v5.0 |                                     |
| QoS 0 (最多一次)         |   ✔️    |                                     |
| QoS 1 (至少一次)         |   ✔️    |                                     |
| QoS 2 (僅有一次)         |   ✔️    |                                     |
| 保留訊息 (Retained)      |   ✔️    |                                     |
| 遺囑訊息 (Last Will)     |   ✔️    |                                     |
| 持久性會話 (Persistent)  |   ✔️    | 針對 `clean_start = false`          |
| 共享訂閱 (Shared)        |   ✔️    | MQTT v5 特性 (`$share/...`)         |
| 訊息過期 (Expiry)        |   ✔️    | MQTT v5 特性                        |

### 🚀 快速入門

#### 1. 從源碼建置

確保您已經安裝了 Rust 工具鏈。

```bash
git clone https://github.com/letoille/AxonMQ.git
cd AxonMQ
cargo build --release
```

#### 2. 設定 AxonMQ

編輯 `config.toml` 檔案以設定您需要的接聽器。預設情況下，接聽器綁定到 `127.0.0.1`。如果您需要從其他機器存取代理，請將 `127.0.0.1` 更改為 `0.0.0.0`（綁定到所有可用的網路介面）或特定的 IP 位址。對於本機測試，預設的 `127.0.0.1` 已足夠。


#### 3. 執行代理

```bash
./target/release/axonmq
```

代理服務將會啟動，並在主控台輸出其狀態。

### 🔒 安全提示：TLS 憑證

**警告：** `certs` 目錄中包含的憑證僅用於示範和測試目的。它們是不安全的，**絕對不能**在生產環境中使用。

在任何正式部署中，您都應該將 `certs/server.crt` 和 `certs/server.key` 替換為您自己的憑證。

- **生產環境**：強烈建議使用由受信任的憑證頒發機構 (CA) 簽發的憑證，例如 [Let's Encrypt](https://letsencrypt.org/)。
- **開發/測試環境**：如果您需要生成新的自簽名憑證，可以使用以下 `openssl` 命令。這比使用預設的、公開的憑證更安全。

```bash
# 生成新的私鑰和自簽名憑證
openssl req -x509 -newkey rsa:2048 -nodes -keyout server.key -out server.crt -days 3650 -subj "/CN=localhost"

cd ..
```
此命令會為 `localhost` 域名建立一個有效期為 10 年的憑證。您的 MQTT 客戶端仍然需要設定為信任此自簽名憑證。

### 📜 授權條款

本專案採用 **Business Source License 1.1** 授權。完整資訊請參閱 `LICENSE` 檔案。

### 💡 未來規劃

我們正在持續改進 AxonMQ。以下是未來版本中計劃推出的一些主要功能：

- **深度整合 Sparkplug B**：全面支援 Sparkplug B 規範，包括進階狀態管理和設備生命週期。
- **支援叢集部署**：透過叢集部署能力實現高可用性和水平擴展性。
- **基於 Web 的管理控制台**：一個使用者友好的 Web 介面，用於監控、管理和配置 AxonMQ 代理。
- **存取控制列表 (ACL) 支援**：實作強大的 ACL，以管理客戶端發布和訂閱主題的權限。
- **基於磁碟的持久化**：實作強大的訊息和客戶端會話持久化，以確保代理重啟後資料完整性。
- **進階認證機制**：支援客戶端憑證認證、LDAP、OAuth/JWT 及其他外部認證機制。
- **代理橋接/聯邦**：允許連接多個 AxonMQ 實例或橋接到其他 MQTT 代理，以實現分散式部署。
- **指標與監控整合**：提供全面的指標，以便與 Prometheus 和 Grafana 等監控工具整合。
- **可插拔架構**：開發一個插件系統，允許使用者透過自訂模組擴展代理功能，例如資料處理、與各種資料庫整合，或將訊息轉發到 Kafka 等平台。
- **MQTT-SN 支援**：增加對 MQTT-SN 協定的支援，適用於資源受限的物聯網設備。

### 🤝 貢獻

歡迎提交貢獻、問題和功能請求！請造訪 [issues 頁面](https://github.com/letoille/AxonMQ/issues)。
