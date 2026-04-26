# Go 服务端（独立于桌面端）

## 服务端的用处（像云存储）

- 把每台设备产生的本地“变更事件”（outbox）汇聚到云端
- 云端按 `eventId` 幂等去重，保证重复上传不会导致数据翻倍
- 其他设备用 `cursor` 增量拉取并应用，实现同账号多端一致
- 客户端应当自动同步：周期性 + 应用回到前台时触发，不需要用户手动点“同步”
- 客户端仍然是 Tauri + Rust commands（不会改成 Go）

## 本地运行

```bash
cd server-go
go run ./cmd/syncserver
```

同步服务默认监听 `127.0.0.1:8788`，并在数据目录下持久化：

- `users.json`（账号）
- `tokens.json`（登录态）
- `events/<userId>/events.json`（按账号分区的事件流）

可用环境变量：

- `PORT`：端口（默认 8788）
- `SYNC_REPO_DIR`：数据目录（默认 `./sync-repo`）

## 内容更新服务（Go 版）

```bash
cd server-go
go run ./cmd/contentserver
```

默认监听 `127.0.0.1:8787`，读取内容仓库：

- `content-repo/versions.json`
- `content-repo/content-pack.zip`

可用环境变量：

- `PORT`：端口（默认 8787）
- `CONTENT_REPO_DIR`：内容仓库目录（默认 `./content-repo`）

## 单进程（推荐）

```bash
cd server-go
go run ./cmd/server
```

默认监听 `127.0.0.1:8788`，同时提供：

- 同步：`/v1/auth/login`、`/v1/sync/push`、`/v1/sync/pull`
- 内容更新：`/versions.json`、`/content-pack.zip`
