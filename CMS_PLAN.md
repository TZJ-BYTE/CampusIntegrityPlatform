# CMS 开发计划（完整可执行版）

范围：内容库后台（CMS），仅覆盖公共内容（法规/故事/场所/案例/题库），目标是“网页编辑 + 一键发布内容包 + 客户端自动更新”。

约束：
- 不做用户管理/角色；仅单密码锁；只需输入一次密码，使用会话 cookie 保持登录态。
- 部署到服务器后可公开访问到登录页，未鉴权禁止进入编辑与发布。
- 与现有内容更新链路兼容：`GET /versions.json` + `GET /content-pack.zip`。

## 0. 验收口径（最重要）

### 0.1 最小可用（MVP）验收
- 在 CMS 里新增/编辑/删除一条法规（含条款）
- 点击“发布新版本”后，服务端的 `versions.json/latest.version` 发生变化
- 客户端自动检测到新版本并更新，前端能看到新增/删除/修改后的真实内容

### 0.2 安全验收
- 未登录访问 CMS 任意编辑/发布接口返回 401/403
- 密码输入一次后，在 cookie 有效期内不重复输入
- 支持“退出登录”清除 cookie

## 1. 里程碑拆分（推荐顺序）

### M1：服务端骨架 + 密码锁（只输入一次）
产物：
- CMS 登录页（密码输入）
- 会话 cookie（12 小时默认，可配置；可选“记住我”7~30 天）
- 统一鉴权中间件（保护 `/cms/*` 与 `/api/cms/*`）

验收：
- 登录后可进入 CMS 主界面
- 刷新页面不需要重复输入密码

### M2：法规 CRUD + 预览（先把最复杂的打通）
产物：
- 法规列表：搜索/筛选（层级/状态）
- 法规编辑：基本信息 + 条款编辑器（增删改排序）
- 法规预览：模拟客户端学习页（右侧目录 + 主内容切换）

验收：
- 能从零创建一条法规并保存为草稿
- 能发布并被客户端更新到

### M3：场所/案例/故事 CRUD + 批量导入
产物：
- 场所编辑（含媒体占位字段）
- 案例编辑（结构化字段）
- 故事编辑（支持批量导入 CSV/Excel → 预览校验 → 入库）

验收：
- 三类内容都能发布并在客户端显示

### M4：发布中心（版本、说明、回滚、校验）
产物：
- 发布中心：生成包、发布新版本（写 versions.json + content-pack.zip）
- 校验：ID 唯一、必填字段、引用媒体存在
- 回滚：保留最近 N 个发布包，可一键回滚

验收：
- 可查看历史版本并回滚，客户端更新到回滚版本

### M5：媒体库（图片/视频的阶段 1）
产物：
- 媒体库：上传/删除、列表、复制链接/ID
- 内容编辑可选择封面/配图/视频（当前可先用占位 + URL）

验收：
- CMS 可选封面图，客户端能展示（至少封面/占位）

### M6：部署与可靠性（上线前必须做）
产物：
- 配置化：内容仓库目录、密码 hash、会话密钥、端口、反向代理建议
- 日志与错误提示：发布失败原因可见
- 基本限流/防暴力：登录失败次数限制

验收：
- 服务器重启不丢数据；发布包不损坏；错误不会导致服务崩溃

## 2. 技术方案（实现层面的“怎么做”）

### 2.1 运行形态
推荐把 CMS 集成进现有 `server-go/cmd/server`（同端口）：
- `/versions.json` 与 `/content-pack.zip` 继续静态返回（客户端依赖）
- 新增：
  - `/cms`：CMS 前端静态页面（Vite build 产物）
  - `/api/cms/*`：CMS API（CRUD、发布、媒体）

### 2.2 CMS 数据存储
建议新增一个独立数据库（例如 `cms.db`），用于草稿与发布记录：
- drafts 表：草稿内容（按实体类型分表或 JSON 存储）
- published 表：已发布快照（用于回滚/对比）
- versions 表：发布版本、notes、时间、产物路径
- media 表：媒体元信息（URL/本地路径/hash）

content.db 的生成方式（发布时）：
- 发布时从 cms.db 生成新的 content.db（可直接写 SQLite）
- 写入 `meta.content_version = <version>`
- 打包为 `content-pack.zip`

### 2.3 发布产物落地位置
仍沿用 `CONTENT_REPO_DIR`：
- `${CONTENT_REPO_DIR}/versions.json`
- `${CONTENT_REPO_DIR}/content-pack.zip`
- `${CONTENT_REPO_DIR}/history/<version>/content-pack.zip`（用于回滚）

## 3. API 草案（为实现做准备）

仅示例核心接口（都需要鉴权）：

- `POST /api/cms/login`（校验密码，发 cookie）
- `POST /api/cms/logout`（清 cookie）
- `GET /api/cms/me`（返回是否已登录）

- `GET /api/cms/regulations?keyword=&level=&status=&offset=&limit=`
- `POST /api/cms/regulations`（新建草稿）
- `GET /api/cms/regulations/:id`
- `PATCH /api/cms/regulations/:id`
- `DELETE /api/cms/regulations/:id`

- `POST /api/cms/publish`（生成包 + 写 versions.json + 记录版本）
- `GET /api/cms/versions`
- `POST /api/cms/rollback`（回滚到某版本）

媒体（阶段 1）：
- `POST /api/cms/media/upload`
- `GET /api/cms/media`
- `DELETE /api/cms/media/:id`

## 4. 风险清单（提前规避）

- ID 不稳定导致客户端收藏断链：必须制定 ID 规则（slug/uuid），并在发布校验中禁止改已发布实体的 id。
- 发布写文件并发：发布中心要做互斥锁，避免同时发布导致 versions.json/content-pack.zip 不一致。
- 大 zip 读写：服务端返回 zip 建议用流式输出（不要整文件读入内存）。

