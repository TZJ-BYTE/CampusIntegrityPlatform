# 开发进度与详细计划（可执行清单）

本文档用于“复盘当前进度 + 把后续开发拆到具体步骤/产物/验收点”，作为 DESIGN.md 的落地执行清单。

## 1. 当前进度（已完成）

### 1.1 工程与架构

- 桌面端：Tauri v2 + Rust commands
- 前端：Vue 3 + TypeScript + Vite（紫红主题、顶部导航）
- 数据：SQLite 双库
  - `content.db`（内容域：场所/案例/法规/故事/题库）
  - `user.db`（用户域：收藏/答题/积分/设置等）
- 离线优先：前端通过 `invoke` 调用本地命令，不依赖本地 HTTP 服务

### 1.2 已实现功能（可用闭环）

- 模块页面：文化场所 / 案例警示 / 法规学习 / 每日故事 / 知识竞答 / 设置 / 我的收藏
- 收藏：
  - 支持收藏与取消收藏
  - “我的收藏”可解析标题并跳转打开对应详情
- 竞答与积分：支持开始会话、提交答案、积分累计

### 1.3 数据安全与运维能力

- `user.db` 备份/恢复/删除备份（设置页可操作）
- 内容库更新：
  - 导入 `content.db`（.db/.sqlite）
  - 导入“内容更新包”（zip：manifest.json + content.db）
- 内容检索增强：cases/stories 接入 SQLite FTS5（keyword 搜索更快更准）
- 便携模式：若 `app.exe` 同目录存在 `data/`，则数据写入 `data/`（否则写入 AppData）
- 联机内容更新（内容库）：
  - 支持配置更新服务器 baseUrl（已内置默认本地地址；高级设置可改）
  - 检查更新、下载更新包、自动应用（应用前自动备份旧 content.db）
  - 提供最小更新服务器示例脚本（tools/content-update-server.mjs）
 - UI 入口重构：
  - 右上角头像为唯一入口（账号/设置/诊断/用户信息）
  - 设置为弹窗（不再作为路由页面）；登录为弹窗；账号与云同步为右侧抽屉

## 2. 里程碑复盘（对照 DESIGN.md）

> DESIGN.md 的里程碑（M0~M4）比较“方向性”；本节把它们标注为“已完成/进行中/未开始”，并给出下一步重点。

- M0：脚手架与离线数据闭环 ✅
- M1：五大模块可用 + 本地全文检索 ✅（cases/stories 已启用 FTS5）
- M2：内容更新包（离线导入）+ 备份/恢复 ✅
- M3：联机内容中心（版本检查与下载）✅（当前为最小可用实现：versions.json + content-pack.zip）
- M4：联机同步（outbox 幂等、冲突策略、可选账号）🟡 进行中（已完成最小可用闭环，下一步做工程化与验收）

## 3. 下一阶段详细计划（M4：联机同步）

目标：在不破坏离线体验的前提下，实现“可选登录 + outbox 幂等同步”，并保证本地数据不丢。

### 3.1 设计约束（必须满足）

- 离线时所有写操作只写本地，不能因网络失败而失败
- 同步必须幂等：服务端按 event_id 去重；客户端对下行事件去重
- 冲突策略固定、可解释、可复现（尤其是积分/答题记录）
- 不能把 `user.db` 直接整库覆盖（避免丢历史）

### 3.2 数据库（user.db）改造清单

- ✅ 新增 `sync_outbox`
  - 字段参考 DESIGN.md 11.3.2（event_id/device_id/event_type/entity_type/entity_id/payload_json/occurred_at/sent_at）
  - 索引：`idx_outbox_unsent(sent_at, occurred_at)`
- ✅ 新增 `device`（并记录 sync_cursor/last_sync_at）
  - 记录 device_id、created_at、last_seen_at、last_sync_at、sync_cursor
- ✅ 新增“已应用事件去重表”
  - 方案 A：`sync_applied(event_id PRIMARY KEY, applied_at)`
  - 方案 B：把去重集合与 cursor 绑定存储在 settings（可行但不如表结构清晰）

验收点：
- 能在断网状态下持续产生 outbox，重启后不丢
- outbox 的 event_id 全局唯一，重复 push 不会导致数据翻倍

### 3.3 后端（Tauri commands）清单

- ✅ `auth_get_state` / `auth_login` / `auth_logout` / `auth_set_server`
  - 先做最小模式：token 存在 settings，登录后才允许 sync_run
- ✅ `sync_get_state`
  - 返回：pendingCount、lastSyncAt、cursor
- ✅ `sync_run({ mode: push|pull|both })`
  - push：批量发送 outbox（未 sent）
  - pull：拉取服务端事件并应用到本地
- ✅ 将本地写操作挂钩到 outbox（关键）
  - favorites：收藏/取消收藏 → 写入 outbox
  - points_ledger：积分流水 → 写入 outbox
  - quiz_answers/quiz_sessions：作答 → 写入 outbox（可选先做“会话完成后汇总上报”）
  - settings：更新设置 → 写入 outbox

下一步（工程化补齐）：

- ⬜ push/pull 分页循环：一次 sync_run 内处理多批次，避免大量离线积压需要多次触发
- ⬜ 网络错误分级与退避：对临时错误静默重试，对 401 自动清理登录态并提示重新登录

验收点：
- 收藏/积分在两台设备分别操作后，联机同步可合并到一致结果
- 重复点击同步不会产生重复收藏/重复积分

### 3.4 前端（UI）清单

- ✅ 头像菜单入口（右上角）
  - 账号与云同步（抽屉）：登录/退出/切换账号、状态展示、（可选）高级服务器地址
  - 设置（弹窗）：本地数据/备份与恢复
  - 诊断（弹窗）：云同步与内容更新状态摘要
- ✅ 异常提示（不影响离线使用）
  - 同步失败静默沉淀到诊断信息；401 自动清理登录态并提示重新登录

验收点：
- 不登录时，所有离线功能照常工作
- 登录后，同步入口可用且状态清晰

### 3.5 服务端（最小实现）清单

为了让客户端可验收，服务端先做最小版本（可以本地跑）：

- ✅ `/v1/auth/login`（返回 access_token；本地最小模式：用户名不存在则创建）
- ✅ `/v1/sync/push`（按 event_id 幂等 ack）
- ✅ `/v1/sync/pull`（按 cursor 增量下发）
- ✅ 存储：文件存储（tokens/users/events），满足“重启不丢”

验收点：
- push 幂等：同 event_id 重复提交不会重复入库
- pull 稳定排序：occurred_at + event_id

## 4. 回归与验证（每一步都要跑）

### 4.1 Rust

- `cd src-tauri && cargo check`

### 4.2 前端

- `npm --prefix frontend run build`

### 4.3 桌面（便携构建）

- `npm run tauri:build -- --no-bundle`

### 4.4 手工验收用例（建议固定脚本）

- 用例 A：收藏/取消收藏 → 我的收藏列表正确、可跳转
- 用例 B：备份 user.db → 删除收藏 → 恢复备份 → 收藏恢复
- 用例 C：联机内容更新 → 检查更新 → 下载并应用 → contentVersion 更新
- 用例 D（M4）：两台设备离线操作后联机同步 → 数据一致（见 tools/m4-acceptance.md）

### 4.5 已知问题与暂不处理项（记录即可）

- 两设备模拟需要便携模式/两份可执行文件；若系统限制同名应用多开，需要改名后运行
- 用户信息（昵称/头像主色）已纳入同步域；头像图片数据暂不参与同步（仅本机保存）

## 5. 新阶段计划：内容库后台（CMS）

目标：解决“服务器端没有页面无法编辑内容库”的问题，提供一个带单密码锁的 CMS，用于编辑公共内容（法规/故事/场所/案例）并一键发布到客户端内容更新链路。

关键约束：
- 不做用户管理/角色；只需输入一次密码，之后用会话 cookie 保持登录态
- 发布产物保持兼容：`versions.json + content-pack.zip`

设计与计划文档：
- CMS 方案草案：[CMS_DESIGN.md](file:///d:/project/rustproject/CampusIntegrityPlatform/CMS_DESIGN.md)
- CMS 页面线框：[CMS_WIREFRAMES.md](file:///d:/project/rustproject/CampusIntegrityPlatform/CMS_WIREFRAMES.md)
- CMS 字段 schema：[CMS_SCHEMA.md](file:///d:/project/rustproject/CampusIntegrityPlatform/CMS_SCHEMA.md)
- CMS 开发计划（可执行清单）：[CMS_PLAN.md](file:///d:/project/rustproject/CampusIntegrityPlatform/CMS_PLAN.md)
