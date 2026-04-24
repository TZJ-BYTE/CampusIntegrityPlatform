# 完整设计方案（离线优先 + 可选联机）

本文档给出“校园廉洁教育平台”完整设计方案：默认离线单机可用；在具备网络时支持动态内容升级与（可选）多端同步，并保证本地学习数据不丢失。

## 1. 背景与目标

### 1.1 背景

校园廉洁教育需要稳定、可控、可离线运行的终端形态，便于在机房/办公室/展厅等场景快速部署与复制。与此同时，长期运营会产生内容更新、题库迭代、活动任务、以及跨设备同步的潜在需求。

### 1.2 总体目标

- 默认离线：无网络时功能完整可用（浏览/学习/答题/记录）
- 易交付：Windows 10/11 双击运行（压缩包目录复制即用）
- 可更新：内容（法规/案例/故事/题库/图片）可动态升级
- 可联机：支持帐号/同步/运营活动（可选开启）
- 数据安全：本地用户数据在升级、更新、同步中不丢失

### 1.3 非目标（阶段性不做）

- 不做浏览器网页形态交付（避免部署与环境差异）
- 不强制依赖外网（联机能力为可选增强）

## 2. 角色与使用场景

### 2.1 角色

- 学生：学习、答题、收藏、查看每日故事
- 老师/管理员（本地）：导入/更新内容包、查看本机统计（可选）
- 平台管理员（联机）：发布内容版本、题库更新、活动任务、审核与运营（可选）

### 2.2 关键场景

- S1：机房离线学习（无网络/网络不稳定）
- S2：办公室或展厅展示（快速复制到新电脑）
- S3：学校统一发布“内容升级包”（U 盘/局域网分发）
- S4：联机后自动拉取“增量内容更新”，同时同步个人学习记录

## 3. 总体架构

### 3.1 离线优先（Offline-first）原则

- 所有功能围绕本地数据完成：UI 不依赖云端才能工作
- 联机时做“同步/更新”，离线时保持“可用/可写”
- 用“内容库/用户库隔离”确保内容更新不会覆盖用户记录

### 3.2 逻辑分层

- 展示层：Vue 3（页面/组件/状态管理）
- 应用层：Tauri Commands（Rust，用例、校验、导入导出、同步编排）
- 数据层：SQLite（内容库 `content.db` + 用户库 `user.db`）
- 联机层（可选）：远程 API（内容发布/增量更新/用户同步）

### 3.3 数据域划分（关键）

- 内容域（Content）：场所/案例/法规/故事/题库/图片/索引
  - 特点：主要由管理员发布，客户端以读取为主
  - 存储：`content.db` + `resources/images`
- 用户域（User）：答题记录/收藏/积分/徽章/打卡/设置/本机身份
  - 特点：客户端持续写入，必须稳定、可迁移、可备份
  - 存储：`user.db`

## 4. 模块设计

### 4.1 廉洁文化场所

- 功能：列表、详情、图集、路线/地图（离线 SVG/图片）
- 关键能力：全文检索（名称/简介/标签），按类型筛选

### 4.2 身边违纪行为警示

- 功能：按场景分类、案例详情、关联法规条款
- 关键能力：标签体系（行为类型/场景/严重程度/关键词）

### 4.3 政策法规学习

- 功能：分层目录、条款浏览、全文检索、重点标注（用户域）
- 关键能力：法规文本结构化（章节/条款），支持定位与引用

### 4.4 每日廉洁故事

- 功能：每日推荐、历史回顾、收藏、分享（离线）
- 关键能力：每日计算规则可配置（支持节假日/主题日）

### 4.5 廉洁知识竞答

- 功能：每日挑战、闯关、专项练习
- 关键能力：判题、解析、统计、积分与徽章

### 4.6 管理（本地）

- 功能：导入内容包、查看当前内容版本、备份与恢复、导出统计（可选）
- 权限：本机管理员口令或 Windows 登录账户白名单（可选）

### 4.7 联机增强（可选）

- 内容中心：内容版本发布、增量补丁、灰度策略
- 同步中心：账户、设备绑定、学习记录同步、冲突处理
- 运营中心：活动任务、排行榜（匿名/实名可配置）

## 5. 数据库设计（建议）

### 5.1 content.db（只读为主）

表结构建议（示意）：

- `meta`：`content_version`, `released_at`, `min_app_version`
- 场所：`venues`, `venue_images`, `venue_tags`
- 案例：`cases`, `case_images`, `case_tags`, `case_regulation_refs`
- 法规：`regulations`, `regulation_sections`, `regulation_fts`（可选）
- 故事：`stories`, `story_tags`
- 题库：`questions`, `question_options`, `question_tags`, `question_explanations`
- 全文检索：SQLite FTS5（可选）对 `cases` / `regulation_sections` / `stories` 建索引

内容数据的主键推荐使用稳定的字符串 ID（例如 `venue_001`），便于跨版本对齐与增量更新。

### 5.2 user.db（读写）

- `device`：本机设备 ID、首次运行时间、最后同步时间
- `profile`：昵称/年级/班级（可选），不强制收集敏感信息
- `favorites`：`entity_type`, `entity_id`, `created_at`
- `notes`：对法规/案例的标注与笔记（可选）
- `quiz_sessions`：答题会话（每日挑战/闯关/专项）
- `quiz_answers`：题目作答明细（含正确性与用时）
- `points_ledger`：积分流水（可重放计算积分）
- `badges`：徽章与达成时间
- `sync_outbox`：待同步变更（联机时使用）

用户数据与内容数据之间只通过 `entity_id` 引用，不做外键强绑定（避免内容升级导致外键断裂）。

## 6. 动态升级（内容更新）设计

### 6.1 更新目标

- 更新内容不影响用户数据
- 支持离线更新（U 盘/局域网包）与联机自动更新
- 支持全量与增量两种策略

### 6.2 更新包形式（建议）

- `content-pack.zip`
  - `manifest.json`：版本号、校验、最小客户端版本、变更摘要
  - `content.db`（全量）或 `patch.sql` / `patch.bundle`（增量）
  - `images/`（新增/替换的图片资源）

客户端更新流程：

1) 校验包完整性（hash/签名可选）  
2) 备份当前 `content.db`（保留最近 N 份）  
3) 应用更新：全量替换或执行增量迁移  
4) 重建 FTS 索引（如启用）  
5) 写入 `meta`（记录版本、来源、时间）  

### 6.3 应用升级（exe 更新）与兼容性

- 应用版本与内容版本解耦：`content.db` 可要求 `min_app_version`
- 升级应用时必须迁移 `user.db`（schema migration），升级失败可回滚

## 7. 联机功能：同步与不丢数据保证

联机能力推荐采用“可选帐号 + 离线写入 + 可重试同步”的模式，确保任何时刻本地可用且不会因网络问题丢数据。

### 7.1 同步对象

- 同步“用户域数据”：收藏、答题记录、积分流水、徽章、设置
- 不同步或弱同步“内容域数据”：内容由内容中心统一发布（客户端按版本拉取）

### 7.2 同步策略（推荐：Outbox + 幂等）

- 本地写入时同时写 `sync_outbox`（记录变更事件）
- 联机时后台任务批量上传 outbox，服务端按 `event_id` 幂等处理
- 上传成功后标记 outbox 已同步

优点：

- 离线写入始终成功（只写本地）
- 同步可重试（网络抖动不影响数据正确性）
- 幂等避免重复上报导致的积分/记录翻倍

### 7.3 冲突处理

推荐规则：

- 收藏/徽章：集合并集（不会“互相覆盖”）
- 设置：最后写入优先（带时间戳）
- 积分：以 `points_ledger` 流水为准（可重放、可校验）
- 答题记录：按 `session_id` 合并，避免同一会话重复

### 7.4 账号与隐私

- 账号可选：不登录也能使用全部离线学习与答题
- 登录后解锁：跨设备同步、云端备份、活动任务（可选）
- 个人信息最小化：默认匿名标识，必要时才收集学号/班级等

### 7.5 远程服务（可选）模块

- Auth：登录/设备绑定/令牌
- Sync API：批量上报 outbox、拉取云端变更、同步状态
- Content API：内容版本列表、下载地址、增量包
- Admin：内容发布、审核、运营配置

## 8. 安全设计（本地与联机）

- 本地数据库加固：可选 SQLCipher（若确有敏感数据存储需求）
- 通信安全：HTTPS + 证书校验（联机模式）
- 最小权限：前端仅通过 Tauri Commands 访问文件与数据库
- 审计：内容发布与版本回滚留痕（服务端）

## 9. 运维与发布

### 9.1 发布形态

- 离线包：`exe + resources/`（可复制即用）
- 可选在线：同一客户端开启“联机增强”，不改变交付形态

### 9.2 备份与恢复（必须）

- 一键备份：导出 `user.db`（可附加导出配置与内容版本信息）
- 一键恢复：导入 `user.db` 并自动做 schema migration
- 自动备份：升级/更新内容前自动保留最近 N 份

## 10. 里程碑（建议）

- M0：脚手架与离线数据闭环（本地库、资源加载、基础页面）
- M1：五大模块可用 + 本地全文检索（FTS5 可选）
- M2：内容更新包（离线导入）+ 备份/恢复
- M3：联机内容中心（版本检查与下载）
- M4：联机同步（outbox 幂等、冲突策略、可选账号）

## 11. 实现级别接口清单

本节将方案落到“可实现级别”的接口与数据结构，覆盖：

- Tauri Commands（前端调用 Rust）
- SQLite 表字段（`content.db` / `user.db`）
- 联机 API（认证/内容/同步）请求与响应结构

### 11.1 本地路径与运行期约定

- 安装目录：只读为主（`.exe` 与 `resources/`），不承诺可写
- 用户目录：`%LOCALAPPDATA%/CampusIntegrityPlatform/`
  - `user.db`：用户域数据（必须可写）
  - `content.db`：可选复制（若启用“用户目录内容库”策略）
  - `backups/`：自动备份（升级/导入内容包前）
  - `logs/`：运行日志（可选）

内容库加载优先级建议：

1) 用户目录 `content.db`（若存在且版本合法）  
2) 安装目录 `resources/content.db`  

### 11.2 Tauri Commands（Rust 接口）

通用约定：

- 入参/出参均为 JSON（前端 TypeScript 可生成类型）
- 所有命令返回统一 envelope：`{ ok: true, data }` 或 `{ ok: false, error }`

统一错误结构：

```json
{
  "code": "INVALID_ARGUMENT|NOT_FOUND|IO_ERROR|DB_ERROR|UNAUTHORIZED|CONFLICT|INTERNAL",
  "message": "string",
  "details": {}
}
```

#### 11.2.1 系统与版本

- `app_get_status() -> { appVersion, contentVersion, userSchemaVersion, deviceId, isOnlineEnabled }`
- `app_open_data_dir() -> void`
- `app_backup_user_db() -> { backupPath }`
- `app_restore_user_db(backupPath) -> void`

#### 11.2.2 内容：场所/案例/法规/故事

- `content_list_venues({ keyword?, type?, limit, offset }) -> { items, total }`
- `content_get_venue({ id }) -> { venue, images, tags }`

- `content_list_cases({ keyword?, scene?, tagIds?, limit, offset }) -> { items, total }`
- `content_get_case({ id }) -> { case, images, tags, regulationRefs }`

- `content_list_regulations({ keyword?, level?, limit, offset }) -> { items, total }`
- `content_get_regulation({ id }) -> { regulation, sections }`
- `content_get_regulation_section({ id }) -> { section }`

- `content_get_today_story({ yyyyMMdd }) -> { story }`
- `content_list_stories({ keyword?, tagIds?, limit, offset }) -> { items, total }`
- `content_get_story({ id }) -> { story }`

#### 11.2.3 用户：收藏/笔记/设置

- `user_get_profile() -> { profile }`
- `user_update_profile({ profilePatch }) -> { profile }`

- `user_list_favorites({ entityType?, limit, offset }) -> { items, total }`
- `user_set_favorite({ entityType, entityId, isFavorite }) -> { isFavorite }`

- `user_get_settings() -> { settings }`
- `user_update_settings({ settingsPatch }) -> { settings }`

#### 11.2.4 竞答：会话与判题

- `quiz_start_session({ mode: "daily"|"stage"|"practice", seed?, rules? }) -> { sessionId, questions }`
- `quiz_submit_answer({ sessionId, questionId, answer }) -> { isCorrect, correctAnswer, explanation?, pointsDelta, totalPoints }`
- `quiz_finish_session({ sessionId }) -> { summary }`
- `quiz_get_progress() -> { streak, badges, totalPoints }`

计分推荐以“流水账”为准（见 `points_ledger`），避免联机重复上报导致积分翻倍。

#### 11.2.5 内容更新（离线包 / 联机下载）

- `content_get_version() -> { contentVersion, releasedAt }`
- `content_apply_pack({ packPath }) -> { newContentVersion }`
- `content_check_update({ channel? }) -> { hasUpdate, latestVersion, downloadUrl?, releaseNotes? }`
- `content_download_update({ version }) -> { packPath }`

#### 11.2.6 联机同步（可选启用）

未登录也可用离线功能；开启联机后提供：

- `auth_get_state() -> { isLoggedIn, userId?, deviceId }`
- `auth_login({ username, password }) -> { userId }`
- `auth_logout() -> void`

- `sync_get_state() -> { lastSyncAt?, cursor?, pendingCount }`
- `sync_run({ mode: "push"|"pull"|"both" }) -> { pushed, pulled, newCursor }`

### 11.3 SQLite 结构（字段级）

数据库建议开启：

- `PRAGMA foreign_keys = ON;`（仅对 user.db 关键表启用强约束）
- `WAL` 模式（提升并发读写体验）

#### 11.3.1 content.db

`meta`

- `key TEXT PRIMARY KEY`
- `value TEXT NOT NULL`

建议键：

- `content_version`
- `released_at`
- `min_app_version`

`venues`

- `id TEXT PRIMARY KEY`
- `name TEXT NOT NULL`
- `type TEXT NOT NULL`
- `location TEXT`
- `description TEXT`
- `contact TEXT`
- `open_hours TEXT`
- `updated_at INTEGER NOT NULL`

`venue_images`

- `id TEXT PRIMARY KEY`
- `venue_id TEXT NOT NULL`
- `path TEXT NOT NULL`
- `sort_order INTEGER NOT NULL DEFAULT 0`

索引：

- `INDEX idx_venue_images_venue_id (venue_id, sort_order)`

`cases`

- `id TEXT PRIMARY KEY`
- `title TEXT NOT NULL`
- `scene TEXT NOT NULL`
- `summary TEXT NOT NULL`
- `body TEXT NOT NULL`
- `violation TEXT`
- `correct_action TEXT`
- `updated_at INTEGER NOT NULL`

`case_tags`

- `case_id TEXT NOT NULL`
- `tag_id TEXT NOT NULL`
- `PRIMARY KEY (case_id, tag_id)`

`regulations`

- `id TEXT PRIMARY KEY`
- `title TEXT NOT NULL`
- `level TEXT NOT NULL`
- `source TEXT`
- `published_at TEXT`
- `updated_at INTEGER NOT NULL`

`regulation_sections`

- `id TEXT PRIMARY KEY`
- `regulation_id TEXT NOT NULL`
- `chapter TEXT`
- `article_no TEXT`
- `title TEXT`
- `body TEXT NOT NULL`
- `updated_at INTEGER NOT NULL`

`stories`

- `id TEXT PRIMARY KEY`
- `title TEXT NOT NULL`
- `body TEXT NOT NULL`
- `source TEXT`
- `day_of_year INTEGER`
- `updated_at INTEGER NOT NULL`

`questions`

- `id TEXT PRIMARY KEY`
- `module TEXT NOT NULL`
- `stem TEXT NOT NULL`
- `type TEXT NOT NULL`
- `difficulty INTEGER NOT NULL DEFAULT 1`
- `answer_key TEXT NOT NULL`
- `analysis TEXT`
- `updated_at INTEGER NOT NULL`

`question_options`

- `id TEXT PRIMARY KEY`
- `question_id TEXT NOT NULL`
- `opt_key TEXT NOT NULL`
- `opt_text TEXT NOT NULL`
- `sort_order INTEGER NOT NULL DEFAULT 0`

索引：

- `INDEX idx_question_options_qid (question_id, sort_order)`

可选全文检索（FTS5）：

- `regulation_fts(section_id, body, title)`
- `cases_fts(case_id, title, body)`
- `stories_fts(story_id, title, body)`

#### 11.3.2 user.db

`schema_meta`

- `key TEXT PRIMARY KEY`
- `value TEXT NOT NULL`

`device`

- `device_id TEXT PRIMARY KEY`
- `created_at INTEGER NOT NULL`
- `last_seen_at INTEGER NOT NULL`
- `last_sync_at INTEGER`
- `sync_cursor TEXT`

`profile`

- `key TEXT PRIMARY KEY`
- `value TEXT NOT NULL`

建议键：

- `nickname`
- `grade`
- `class_name`

`favorites`

- `entity_type TEXT NOT NULL`
- `entity_id TEXT NOT NULL`
- `created_at INTEGER NOT NULL`
- `PRIMARY KEY (entity_type, entity_id)`

索引：

- `INDEX idx_favorites_created_at (created_at)`

`quiz_sessions`

- `session_id TEXT PRIMARY KEY`
- `mode TEXT NOT NULL`
- `started_at INTEGER NOT NULL`
- `finished_at INTEGER`
- `seed TEXT`
- `summary_json TEXT`

`quiz_answers`

- `id TEXT PRIMARY KEY`
- `session_id TEXT NOT NULL`
- `question_id TEXT NOT NULL`
- `answer TEXT NOT NULL`
- `is_correct INTEGER NOT NULL`
- `answered_at INTEGER NOT NULL`
- `cost_ms INTEGER`

索引：

- `INDEX idx_quiz_answers_session (session_id, answered_at)`
- `INDEX idx_quiz_answers_question (question_id, answered_at)`

`points_ledger`

- `id TEXT PRIMARY KEY`
- `reason TEXT NOT NULL`
- `delta INTEGER NOT NULL`
- `occurred_at INTEGER NOT NULL`
- `ref_type TEXT`
- `ref_id TEXT`

`badges`

- `badge_id TEXT PRIMARY KEY`
- `earned_at INTEGER NOT NULL`

`settings`

- `key TEXT PRIMARY KEY`
- `value TEXT NOT NULL`
- `updated_at INTEGER NOT NULL`

`sync_outbox`（联机启用时写入）

- `event_id TEXT PRIMARY KEY`
- `user_id TEXT`
- `device_id TEXT NOT NULL`
- `event_type TEXT NOT NULL`
- `entity_type TEXT`
- `entity_id TEXT`
- `payload_json TEXT NOT NULL`
- `occurred_at INTEGER NOT NULL`
- `sent_at INTEGER`

索引：

- `INDEX idx_outbox_unsent (sent_at, occurred_at)`

### 11.4 联机 API（请求/响应结构）

约定：

- Base URL：`https://<host>/api`
- 认证：`Authorization: Bearer <access_token>`
- 统一响应 envelope：

```json
{ "ok": true, "data": {} }
```

失败：

```json
{
  "ok": false,
  "error": {
    "code": "UNAUTHORIZED|INVALID_ARGUMENT|CONFLICT|NOT_FOUND|RATE_LIMITED|INTERNAL",
    "message": "string",
    "details": {}
  }
}
```

#### 11.4.1 认证（可选）

`POST /v1/auth/login`

请求：

```json
{ "username": "string", "password": "string", "device_id": "string" }
```

响应：

```json
{
  "ok": true,
  "data": {
    "user_id": "string",
    "access_token": "string",
    "refresh_token": "string",
    "expires_in": 3600
  }
}
```

`POST /v1/auth/refresh`

请求：

```json
{ "refresh_token": "string", "device_id": "string" }
```

响应同 login。

#### 11.4.2 内容中心（仅内容更新也可独立使用）

`GET /v1/content/versions?channel=stable&since=<version?>`

响应：

```json
{
  "ok": true,
  "data": {
    "latest": { "version": "2026.04.23.1", "released_at": "2026-04-23T10:00:00Z" },
    "items": [
      {
        "version": "2026.04.23.1",
        "released_at": "2026-04-23T10:00:00Z",
        "min_app_version": "0.1.0",
        "notes": "string",
        "sha256": "hex"
      }
    ]
  }
}
```

`GET /v1/content/packs/{version}`

响应：

```json
{
  "ok": true,
  "data": {
    "version": "2026.04.23.1",
    "url": "https://.../content-pack.zip",
    "sha256": "hex",
    "expires_at": "2026-04-23T11:00:00Z"
  }
}
```

#### 11.4.3 同步（Outbox 幂等）

事件结构（客户端 outbox 与服务端一致）：

```json
{
  "event_id": "uuid",
  "user_id": "string|null",
  "device_id": "string",
  "event_type": "FAVORITE_SET|SETTINGS_PATCH|QUIZ_ANSWER|POINTS_ADD|BADGE_EARNED",
  "entity_type": "venue|case|reg_section|story|question|null",
  "entity_id": "string|null",
  "occurred_at": 1713868800000,
  "payload": {}
}
```

`POST /v1/sync/push`

请求：

```json
{ "cursor": "string|null", "events": [ { "event_id": "uuid", "device_id": "string", "event_type": "string", "occurred_at": 0, "payload": {} } ] }
```

响应：

```json
{
  "ok": true,
  "data": {
    "acked_event_ids": ["uuid"],
    "rejected": [{ "event_id": "uuid", "code": "INVALID_ARGUMENT", "message": "string" }],
    "server_time": "2026-04-23T10:00:00Z"
  }
}
```

`POST /v1/sync/pull`

请求：

```json
{ "cursor": "string|null", "limit": 500 }
```

响应：

```json
{
  "ok": true,
  "data": {
    "cursor": "string",
    "events": [ { "event_id": "uuid", "device_id": "string", "event_type": "string", "occurred_at": 0, "payload": {} } ],
    "server_time": "2026-04-23T10:00:00Z"
  }
}
```

客户端应用顺序：

- 拉取事件按 `occurred_at` + `event_id` 稳定排序应用
- 对已应用的 `event_id` 做去重（避免重复下发导致数据翻倍）

### 11.5 内容更新包 manifest.json（建议）

```json
{
  "schema_version": 1,
  "content_version": "2026.04.23.1",
  "min_app_version": "0.1.0",
  "released_at": "2026-04-23T10:00:00Z",
  "sha256": {
    "content.db": "hex",
    "images.zip": "hex"
  },
  "mode": "full|patch",
  "notes": "string"
}
```

### 11.6 数据不丢失的落地约束（必须满足）

- 内容更新不允许写入/覆盖 `user.db`
- 应用升级必须支持 `user.db` schema migration（可回滚）
- 同步必须幂等：服务端按 `event_id` 去重；客户端对下行事件去重
- 升级/导入内容包前自动备份（至少保留最近 N 份）
