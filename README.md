# 校园廉洁教育平台（桌面离线版）

面向校园廉洁教育场景的离线桌面应用：在 Windows 电脑上双击运行，前后端一体、数据本地化、可复制到新电脑直接使用。

完整设计方案见 [DESIGN.md](file:///d:/project/rustproject/CampusIntegrityPlatform/DESIGN.md)。

## 目标（需求确认）

- 交付形态：单一交付形式（桌面客户端）
- 使用方式：Windows 10/11 双击 `.exe` 运行
- 部署要求：不需要用户安装开发环境、不需要命令行操作
- 网络依赖：默认离线可用（所有内容与资源本地携带）
- 数据存储：SQLite（文件型数据库）

## 当前仓库状态

当前仓库已完成桌面端与前端脚手架搭建，并实现了离线优先的最小可用闭环：

- 桌面端：Tauri v2 + Rust Commands
- 前端：Vue 3 + TypeScript + Vite（紫红主题）
- 本地数据：SQLite 双库 `content.db`（内容）+ `user.db`（用户数据）
- 功能雏形：场所/案例/法规/故事/竞答页面可用，收藏与积分写入 `user.db`

完整设计方案见 [DESIGN.md](file:///d:/project/rustproject/CampusIntegrityPlatform/DESIGN.md)（含“可实现级别接口清单”）。

## 快速开始（开发与打包）

### 环境要求（Windows 10/11）

- Node.js（提供 `npm`）
- Rust toolchain（stable，含 `cargo`）
- Visual Studio Build Tools（C++ 构建工具，Tauri 在 Windows 下需要）
- WebView2：通常系统自带；若目标机器极少数缺失，需要安装运行时或采用 Fixed Runtime（体积更大）

### 安装依赖

在仓库根目录执行：

```powershell
npm install
npm --prefix frontend install
```

### 本地开发（桌面端）

```powershell
npm run tauri:dev
```

### 构建（桌面包）

```powershell
npm run tauri:build
```

若环境无法下载 WiX（Windows 打包 msi 依赖），可先只编译可执行文件：

```powershell
npm run tauri:build -- --no-bundle
```

产物通常在（以实际构建输出为准）：

- 可执行文件：`target/release/app.exe`
- 安装包/打包产物：`target/release/bundle/`

### 便携模式（可选）

默认情况下，应用数据（`content.db`/`user.db`/`backups/`）存放在系统 AppData。

若希望做到“拷贝即用、数据也跟随文件夹走”，可使用便携模式：

- 将 `app.exe` 放到可写目录
- 在 `app.exe` 同目录创建 `data` 文件夹
- 启动应用后，数据库与备份会写入 `data/`（含 `data/backups/`）

### 仅构建前端

```powershell
npm run build
```

### Rust 检查（可选）

```powershell
cd src-tauri
cargo check
```

## 方案结论（可行性与更优解）

### 可行性结论

“Vue 3 + Tauri + Rust + SQLite，打包为可复制即用的 Windows 桌面应用”是可行的；能满足“前后端一体、双击运行、离线可用”的核心目标。

### 更优解（对原方案的关键优化）

- 去掉“启动本地 HTTP 服务”作为主路径：优先使用 Tauri `invoke` 命令调用 Rust 后端，减少端口管理、防火墙弹窗、以及本机服务暴露面。需要时再补充内部 HTTP（仅本机回环、随机端口、不开外网监听）。
- 将“内容数据”和“用户数据”分库/分域：避免用“替换一个 db 文件”的方式更新内容时覆盖用户学习记录、收藏与答题进度。
- 明确 WebView2 策略：若必须做到“完全离线、无下载”，则需要随包携带 Fixed Runtime（体积显著增大）；若允许极少数机器首次联网安装，则使用系统 WebView2（更轻量）。

## 技术选型（推荐）

| 层级    | 选型                   | 说明                   |
| ----- | -------------------- | -------------------- |
| UI 前端 | Vue 3 + TypeScript   | 组件化与交互效率高            |
| 桌面容器  | Tauri（Rust）          | 打包为原生 `.exe`，资源可随包携带 |
| 本地后端  | Rust（Tauri Commands） | 负责数据读写、文件访问、搜索与导入导出  |
| 数据库   | SQLite               | 内嵌、可携带、易备份           |

## 交付形态与目录约定

推荐同时满足“可复制即用”和“可更新内容”的折中交付：

```
CampusIntegrityPlatform/
├── CampusIntegrityPlatform.exe
├── resources/
│   ├── content.db              # 预置内容（场所/案例/法规/故事/题库）
│   └── images/
│       ├── venues/
│       ├── cases/
│       └── stories/
└── data/                        # 可选：预置用户空库或示例数据
```

运行期用户数据建议落在用户目录（避免写入安装目录导致权限问题）：

- `%LOCALAPPDATA%/CampusIntegrityPlatform/`
  - `user.db`（答题记录、收藏、积分、设置等）
  - `content.db`（可选：首次运行复制一份到用户目录，便于热更新）

## 总体架构（逻辑分层）

- 展示层（Vue）：页面与组件、离线静态资源展示
- 应用层（Tauri Commands / Rust）：用例编排、权限校验、导入导出、搜索
- 数据层（SQLite）：内容库（只读为主）+ 用户库（读写）
- 资源层（images / assets）：图片与离线地图（SVG/图片）

## 功能模块（V1 范围）

### 1) 廉洁文化场所

- 场所列表：名称、类型、缩略图
- 详情页：图文介绍、照片轮播、位置说明、联系人/开放时间
- 导览：离线 SVG 校园地图标注或文字路线指引

### 2) 身边违纪行为警示（中学生视角）

- 分类浏览：学生组织/班级管理/日常行为/网络行为/升学环节
- 案例卡片：情景描述 + 违纪定性 + 正确做法
- 可扩展：按关键词/标签检索，关联法规条款

### 3) 政策法规学习

- 分层内容：国家/教育系统/校内制度
- 学习工具：全文检索、重点标注、一图读懂（图文卡片）

### 4) 每日廉洁故事

- 每日推送：按年内序号映射 1 篇
- 历史回顾：按日期浏览
- 收藏：与用户数据绑定，升级内容不丢失

### 5) 廉洁知识竞答

- 每日挑战：5 题/天，连续打卡加成
- 闯关模式：关卡递增、徽章解锁
- 专项练习：按模块分类练习
- 题型：单选/多选/判断/情景分析

## 数据设计（建议草案）

为保证“可更新内容且不覆盖用户记录”，建议：

- `content.db`（只读为主）
  - `venues` / `venue_images`
  - `cases` / `case_images` / `case_tags`
  - `regulations` / `regulation_sections`
  - `stories`
  - `questions` / `question_options` / `question_tags`
  - 可选：SQLite FTS5 索引表（用于全文检索）
- `user.db`（读写）
  - `favorites`（收藏：场所/案例/故事/法规条款）
  - `quiz_history`（答题记录）
  - `daily_streak` / `badges` / `points`
  - `settings`（本机设置）

内容更新推荐方式：

- 方案 A（简单可靠）：替换 `resources/content.db` 与图片资源；应用启动时检测版本并提示“更新内容包”，用户数据不变。
- 方案 B（更强）：内容包采用版本号与增量迁移脚本（适合频繁更新与体量变大场景）。

## 风险点与应对（可行性审查）

- WebView2 依赖
  - 风险：少量机器可能未安装，且离线场景无法下载
  - 应对：明确选择“系统运行时”或“随包 Fixed Runtime（更大体积）”
- “可复制即用”的可执行文件分发
  - 风险：不同打包方式可能产出安装包而非便携包
  - 应对：约定交付为压缩包目录（`.exe + resources`），并在发布流程中固定产物结构
- 内容更新覆盖用户数据
  - 风险：直接替换单库会覆盖收藏/答题
  - 应对：内容库与用户库隔离，或使用迁移策略
- 全文检索与性能
  - 风险：法规/案例文本较长时检索慢
  - 应对：使用 SQLite FTS5、预计算索引、分页加载
- 合规与版权
  - 风险：法规文本、图片、故事来源需授权或标注来源
  - 应对：为内容库增加 `source` 字段与版权声明，建立内容审核流程

## 里程碑（建议）

- M0：Tauri + Vue 脚手架、目录与打包产物定型
- M1：场所/案例/法规浏览与搜索（含内容库初始化）
- M2：每日故事、收藏与用户数据持久化
- M3：题库与竞答（积分/徽章/打卡）
- M4：内容更新机制（内容包/版本检测/导入导出）
