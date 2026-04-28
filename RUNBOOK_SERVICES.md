# 各服务启动与操作步骤（本地开发 / 联机内容更新）

本文档说明：本仓库里的 Go 服务端（内容更新 + CMS + 同步）以及桌面客户端（Tauri）的启动方式，并给出“如何让客户端真正更新到 CMS 修改内容”的标准操作流程。

## 1. 环境准备

- Node.js（提供 npm）
- Rust toolchain（stable）
- Go（建议 1.22+）

## 2. 服务端（Go）

### 2.1 一体服务（推荐）

一个进程同时提供：
- CMS（/cms/ + /cms/api/*）
- 内容更新（/versions.json + /content-pack.zip + /media/*）
- 云同步（/v1/sync/*）

在仓库根目录运行：

```powershell
npm run server
```

默认监听：`http://127.0.0.1:8788`

常用检查：
- 健康检查：`GET http://127.0.0.1:8788/health`
- CMS：`http://127.0.0.1:8788/cms/`
- 内容更新：`GET http://127.0.0.1:8788/versions.json`
- 媒体：`GET http://127.0.0.1:8788/media/<file>`

### 2.2 仅同步服务（可选）

```powershell
npm run sync-server
```

### 2.3 仅内容更新服务（可选）

```powershell
npm run content-server
```

### 2.4 服务端环境变量（常用）

- `PORT`：监听端口（默认 8788）
- `CONTENT_REPO_DIR`：内容仓库目录（默认 `server-go/content-repo`）
  - `content-pack.zip`、`versions.json`、`media/` 会在这里生成/读取
- `SYNC_REPO_DIR`：同步仓库目录（默认 `server-go/sync-repo`）

CMS 鉴权（密码锁）：
- `CMS_PASSWORD`：CMS 登录密码（默认 `admin`）
- `CMS_PASSWORD_HASH`：可选，sha256 hex（优先级高于明文密码）
- `CMS_SESSION_SECRET`：会话签名密钥（部署时必须设置；本地可不设）
- `CMS_COOKIE_SECURE=1`：HTTPS 部署时启用 secure cookie

## 3. 客户端（Tauri 桌面端）

### 3.1 本地开发（热更新）

```powershell
npm run tauri:dev
```

### 3.2 构建可执行文件

```powershell
npm run tauri:build -- --no-bundle
```

产物一般在：
- `target/release/app.exe`

## 4. “真正更新”的标准流程（CMS → 客户端看到变化）

关键结论：
- 客户端的“内容数据”来自 `content-pack.zip`（里面是 `content.db + manifest.json`）
- 图片/视频等媒体文件不在 content-pack.zip 里，客户端通过 `cover_url`（例如 `/media/xxx.png`）去服务端取
- 所以要更新成功：既要发布新内容包，也要保证媒体 URL 可访问

### 4.1 服务端准备（必须）

1) 启动一体服务（推荐用 8788）：

```powershell
npm run server
```

2) 确认能访问：
- `http://127.0.0.1:8788/versions.json`
- `http://127.0.0.1:8788/cms/`

### 4.2 CMS 修改内容并发布

1) 打开 CMS：`http://127.0.0.1:8788/cms/`
2) 登录（默认密码 `admin`，可用环境变量改）
3) 如果要换封面：
   - 先到“媒体库”上传图片
   - 在内容条目里把 `cover_url` 填成 `/media/<文件名>`
4) 右上角点击“发布”
   - 版本号必须变化（推荐直接用时间戳）
   - 发布完成后会更新 `CONTENT_REPO_DIR` 下的 `versions.json` 和 `content-pack.zip`

### 4.3 客户端拉取更新

1) 打开客户端
2) 在“诊断”里点击“立即检查更新”（或等待自动检查）
3) 确认“内容版本”已变化（等于你发布时的版本号）

### 4.4 客户端显示封面（媒体加载）

如果 `cover_url` 形如 `/media/xxx.png`：
- 客户端会去请求 `http://<内容服务器>/media/xxx.png`
- 本地开发默认是 `http://127.0.0.1:8788`
- 端口/地址不对会导致仍然显示占位图

## 5. 常见问题排查

- 客户端没变化：
  - 检查 CMS 是否点击“发布”，版本号是否变化
  - 打开 `http://127.0.0.1:8788/versions.json` 看 latest.version 是否更新
- 内容版本更新了但封面还是占位：
  - 检查图片 URL 在浏览器能否打开：`http://127.0.0.1:8788/media/<file>`
  - 确认客户端使用的内容服务器地址与端口正确

