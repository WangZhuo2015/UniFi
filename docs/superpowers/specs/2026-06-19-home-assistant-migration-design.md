# Home Assistant 迁移设计

日期：2026-06-19

## 目标

把 `192.168.2.62` 香橙派上的 Home Assistant Container 迁移到常开的 Apple M1 MacBook Air，解决 Docker Desktop on macOS 无法提供真正 host 网络而造成的 mDNS、HomeKit Bridge 和局域网发现问题。

迁移后使用 Home Assistant OS 虚拟机，不采用已经停止官方支持的 Python Home Assistant Core 安装方式。旧香橙派部署在验收完成前保留为可立即恢复的回滚点。

## 当前环境

- Mac：Apple M1，16 GB 内存，macOS 26.4.1，作为常开服务器使用。
- Mac 网络：千兆有线接口 `en5` 使用 `192.168.2.53`；Wi-Fi 接口 `en0` 同时在线。
- 旧主机：`orangepi@192.168.2.62`，aarch64。
- Home Assistant：Container `2026.4.2`，host 网络，配置目录 `/home/orangepi/homeassistant`，约 95 MB。
- Matter Server：独立容器，host 网络，数据目录 `/home/orangepi/matter-server/data`。
- 现有集成包括 HomeKit、Matter、Thread、Xiaomi Home、Neakasa 和 HACS。
- 没有需要直通虚拟机的 USB、Zigbee 或 Z-Wave 外设。

## 架构

在 Mac 上使用 VirtualBox 运行 ARM64 Home Assistant OS。虚拟机通过桥接模式连接 `en5`，直接成为 `192.168.2.0/24` 局域网成员，使 mDNS、SSDP、Matter 和 HomeKit 流量不经过 Docker Desktop 的 NAT 网络。

最终切换时，虚拟机接管旧地址 `192.168.2.62`。旧主机与新虚拟机绝不同时使用该地址，也不同时广播同一 Home Assistant/HomeKit 实例。

官方 macOS 安装指引：https://www.home-assistant.io/installation/macos/

## 资源与运行策略

- 1 个虚拟 CPU。
- 1 GB 内存。
- 16 GB 动态扩展磁盘。
- 虚拟机无界面运行。
- Mac 启动后自动启动虚拟机。
- 以 macOS 后台 QoS 启动虚拟机进程，让系统调度器优先使用能效核心。

Apple Silicon/macOS 没有受支持的虚拟机 CPU 硬亲和性接口，因此不承诺把进程固定在某一个能效核心。1 个 vCPU 限制虚拟机最多持续占用一个核心，后台 QoS 用于表达能效优先级。

1 GB 低于 Home Assistant 官方建议配置。若出现 OOM、异常重启、Matter 不稳定或持续内存压力，只增加内存到 1.5 GB；若仍不稳定则增加到 2 GB。除非验证证明 CPU 是瓶颈，否则不增加 vCPU。

## 数据迁移

1. 下载并校验 Home Assistant OS ARM64 镜像。
2. 创建虚拟机，但在恢复配置时保持虚拟网卡断开，防止重复广播。
3. 通过 Home Assistant Backup 集成生成可在 HAOS 引导页恢复的原生备份并下载到 Mac；同时分别为旧 Home Assistant `/config` 和 Matter Server `/data` 创建带时间戳的原始归档，作为独立回退副本。
4. 在创建最终备份前正常停止旧容器，确保 SQLite 数据库和 Matter 状态一致。
5. 在 HAOS 初次引导时恢复原生备份，把 Home Assistant 配置、身份、注册表、HomeKit 配对状态、自定义集成和历史数据库迁入新实例。
6. 安装并停止 HAOS Matter Server 应用，通过 HAOS 调试 SSH 把旧 Matter Server `/data` 恢复到 Supervisor 管理的 Matter Server 数据目录，再启动应用并验证既有 Matter Fabric；不在 Mac 主机额外运行第二个 Matter Server。
7. 配置虚拟机桥接 `en5`，让新实例接管 `192.168.2.62`。
8. 启动新实例并执行验收。

任何包含访问令牌、配对密钥或 `secrets.yaml` 的备份都只保存在本机受限目录中，不加入 Git，也不输出到会话日志。

## 切换顺序

切换窗口内先停止旧 Home Assistant 和 Matter Server 容器，确认 `192.168.2.62` 不再响应，再启用虚拟机桥接网络并启动新实例。这样避免 IP 冲突、重复 HomeKit Bridge、重复 Matter Fabric 会话和自动化的重复执行。

恢复完成后，旧香橙派容器保持停止状态，但镜像、配置和数据不删除。

## 验收

- `http://192.168.2.62:8123` 可访问。
- 原用户、实体、设备、历史数据和现有设置保留。
- Xiaomi Home、Neakasa、HACS、Matter、Thread 和 HomeKit 集成加载完成。
- Matter Server 已连接，现有 Matter 设备可以读取状态并执行控制。
- 从局域网验证 `_hap._tcp` 广播，HomeKit Bridge 能被家庭 App 发现并保持连接。
- 没有第二个同名 HomeKit Bridge 或 Home Assistant 实例在广播。
- Mac 重启后，虚拟机无需登录桌面即可自动启动。
- 连续观察至少 30 分钟，无 OOM、异常重启或明显内存压力。
- 检查 HAOS、Home Assistant Core、Matter Server 与虚拟化进程日志，没有阻断性错误。

## 回滚

如果 Web 访问、Matter、HomeKit、核心集成、自动启动或资源稳定性任一项未通过：

1. 关闭新 HAOS 虚拟机并确认它不再占用 `192.168.2.62`。
2. 在香橙派上重新启动旧 Matter Server 和 Home Assistant 容器。
3. 确认旧实例重新响应并恢复局域网广播。
4. 保留失败现场和日志用于修复，不在回滚过程中修改旧数据。

只有全部验收完成并经过稳定运行后，才另行决定是否清理香橙派部署；清理不属于本次迁移范围。
