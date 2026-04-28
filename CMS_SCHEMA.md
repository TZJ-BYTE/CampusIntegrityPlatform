# CMS 内容库字段 Schema（草案）

说明：这是 CMS 的“编辑数据模型”，最终发布时会被转换/写入 `content.db` 并随 `content-pack.zip` 分发。

## 0) 全局约定

- 主键 `id` 必须稳定：一旦发布后不要随意更改，否则客户端收藏/引用会断。
- 时间字段统一为 ISO 日期字符串或 unix ms（选一种并固定）。
- 所有实体都建议有：
  - `id`
  - `title/name`
  - `status`: `draft | published | archived`
  - `updatedAt`
  - `createdAt`

## 1) Regulation（法规）

### Regulation
- id: string
- title: string
- level: string（如 校内制度/学习指引）
- source?: string
- publishedAt?: string（YYYY-MM-DD）
- coverMediaId?: string（指向媒体库）
- keywords?: string[]
- notes?: string（CMS 内部备注）
- status: draft|published|archived
- sections: RegulationSection[]
- media: RegulationMedia（学习资源）

### RegulationSection
- id: string
- chapter?: string
- articleNo?: string
- title?: string
- body: string
- order: number

### RegulationMedia
- introVideoMediaId?: string
- images?: { mediaId: string; order: number; caption?: string }[]
- attachments?: { label: string; url: string }[]

## 2) Venue（廉洁文化场所）

### Venue
- id: string
- name: string
- type: string（文化展示/红色教育/实践体验…）
- location?: string
- openHours?: string
- contact?: string
- description?: string（富文本或纯文本）
- coverMediaId?: string
- media:
  - introVideoMediaId?: string
  - gallery?: { mediaId: string; order: number; caption?: string }[]
- status: draft|published|archived
- updatedAt/createdAt

## 3) Case（案例警示）

### Case
- id: string
- title: string
- scene: string
- summary: string
- body: string（情景描述）
- violation?: string（违纪风险）
- correctAction?: string（正确做法）
- coverMediaId?: string
- media?:
  - introVideoMediaId?: string
  - images?: { mediaId: string; order: number; caption?: string }[]
- status: draft|published|archived
- updatedAt/createdAt

## 4) Story（每日故事）

### Story
- id: string
- title: string
- source?: string
- body: string
- publishedAt?: string（可选，用于排序）
- coverMediaId?: string
- images?: { mediaId: string; order: number; caption?: string }[]
- status: draft|published|archived
- updatedAt/createdAt

## 5) Media（媒体库）

### MediaItem
- id: string
- type: image|video
- storage:
  - stage1: url（外链或本地静态目录 URL）
  - stage2: packPath（未来离线包 assets/ 相对路径）
- mime?: string
- size?: number
- width/height?: number（图片）
- duration?: number（视频）
- sha256?: string（可选）
- label?: string
- tags?: string[]
- createdAt

## 6) 发布（Version & Pack）

### Version（versions.json 的来源模型）
- version: string（必须每次发布变化）
- url: string（通常 `/content-pack.zip` 或对象存储直链）
- notes?: string
- releasedAt: string

### Manifest（打包进 zip）
- contentVersion: string
- minAppVersion?: string
- notes?: string
- sha256?: string（推荐后续加入，用于客户端校验）

## 7) 必要的校验规则（最小集）

- 所有实体 id 唯一且非空
- 法规 sections 至少 1 条（否则禁止发布）
- 富文本/正文不允许全空
- 删除已发布内容需要二次确认（且建议在发布中心统一处理）
- 发布前做“引用检查”（例如 mediaId 是否存在）

