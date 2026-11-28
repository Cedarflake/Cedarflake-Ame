![Cedarflake Ame](docs/images/ame.png)

<p align="center"><a href="./README.md">English</a> | 简体中文 | <a href="./README.ja.md">日本語</a></p>

Ame 是一款面向日常使用的 Windows 本地图库，支持图片浏览、智能搜索、自动分类和重复图片整理。它直接管理电脑中已有的图片文件夹，提供时间轴浏览、收藏、相册和文件管理功能。软件免费、开源，采用 [Apache-2.0 许可证](LICENSE)。

![Ame 的图库、图片查看、搜索、相册、重复分组和文件整理界面](docs/images/ame-showcase.webp)

## 下载

Ame 目前处于 **Alpha 阶段**，主要功能已经实现，正在完善性能和稳定性，为 Beta 版本做准备。在[发布页面](https://github.com/Cedarflake/Cedarflake-Ame/releases)可以下载适用于 **Windows 11 x64** 的安装版或便携版。

- **MSI 安装程序**需要管理员权限。安装包自带图片分析模型和运行时，并会安装用于在后台更新图库的 Windows 服务。
- **便携版 ZIP** 无需安装，不安装 Windows 服务，图片分析模型在首次使用时下载。

[Windows 11 预览版](https://github.com/Cedarflake/Cedarflake-Ame/actions/workflows/quality_windows_arm64.yml)提供 x64 和 ARM64 两种 MSI 安装包，以及 ARM64 便携版 ZIP。从已安装的 x64 版 MSI 切换到 ARM64 版，需要先卸载 x64 版。

Ame 支持应用内更新，可选择自动检查或自动下载。发布包和预览包均未签名。各版本的改动和已知限制见[发行说明](docs/releases/README.md)。

## 主要功能

- **图库浏览**：照片墙、网格布局、时间轴和多文件夹管理。
- **自动同步**：监测文件变化，在后台更新图库。
- **图片搜索**：文件名、路径、自然语言搜索和以图找图，支持组合筛选。
- **智能相册**：使用本地 SigLIP 2 模型自动分类图片，支持 GPU 加速。
- **收藏与相册**：收藏图片、创建虚拟相册、自选相册图标和保存整理进度。
- **重复图片整理**：检测完全重复或视觉相似的图片，支持分组比较、相似度调整和清理。
- **文件管理**：复制、移动、重命名、删除、操作记录，以及部分中断操作的恢复。
- **图片查看**：缩放、旋转、双图对比和缩略图快速切换。

## 从源码运行

Ame 仍在开发中。在 Windows 11 x64 上构建需要 Flutter **3.50.0**、[rust-toolchain.toml](rust-toolchain.toml) 指定的 Rust 工具链，以及 Visual Studio 2022。安装 Visual Studio 时，请勾选「使用 C++ 的桌面开发」工作负载，并安装 Windows SDK 和 CMake 工具。在 Windows on Arm 设备上构建请参阅 [ARM64 构建指南](docs/acceptance/quality/commands.md#build-on-windows-arm64)。

```powershell
flutter pub get
flutter run -d windows
```

界面使用 Flutter 和 Material 3 构建。Rust 负责图库索引、文件跟踪、预览、图片分析和文件操作，图库数据存储在 SQLite 中。

更多信息见[路线图](docs/roadmap.md)、[开发指南](docs/development/repository-layout.md)、[构建与测试命令](docs/acceptance/quality-gates.md)和[架构说明](docs/architecture/README.md)。

## 许可证

版权所有 © 2026 Cedarflake Ame 贡献者。

除另有说明外，Cedarflake Ame 使用 [Apache 许可证 2.0](LICENSE)（`Apache-2.0`）。第三方代码、依赖项、模型权重和资源各自适用其原有的许可证和声明。

使用条款见[最终用户许可协议](EULA.md)（英文）。
信息处理方式见[隐私声明](PRIVACY.md)（英文）。
