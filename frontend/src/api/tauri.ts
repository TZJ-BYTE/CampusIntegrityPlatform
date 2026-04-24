import { invoke } from '@tauri-apps/api/core'

export type ApiError = {
  code: string
  message: string
  details: unknown
}

export type ApiResponse<T> =
  | { ok: true; data: T }
  | { ok: false; error: ApiError }

export type AppStatus = {
  appVersion: string
  contentVersion: string
  userSchemaVersion: number
  deviceId: string
  isOnlineEnabled: boolean
}

export async function appGetStatus(): Promise<ApiResponse<AppStatus>> {
  return await invoke('app_get_status')
}

export type DataDirInfo = { path: string; mode: string }

export async function appGetDataDir(): Promise<ApiResponse<DataDirInfo>> {
  return await invoke('app_get_data_dir')
}

export type BackupItem = {
  path: string
  fileName: string
  size: number
  modifiedAt: number
}

export type BackupsList = { items: BackupItem[] }

export async function appListBackups(): Promise<ApiResponse<BackupsList>> {
  return await invoke('app_list_backups')
}

export type BackupResult = { backupPath: string }

export async function appBackupUserDb(): Promise<ApiResponse<BackupResult>> {
  return await invoke('app_backup_user_db')
}

export async function appRestoreUserDb(args: { backupPath: string }): Promise<ApiResponse<boolean>> {
  return await invoke('app_restore_user_db', { args })
}

export async function appDeleteBackup(args: { backupPath: string }): Promise<ApiResponse<boolean>> {
  return await invoke('app_delete_backup', { args })
}

export type ContentVersion = { contentVersion: string }

export async function contentGetVersion(): Promise<ApiResponse<ContentVersion>> {
  return await invoke('content_get_version')
}

export type ContentImportDbResult = { backupPath: string; contentVersion: string }

export async function contentImportDb(args: { sourcePath: string }): Promise<ApiResponse<ContentImportDbResult>> {
  return await invoke('content_import_db', { args })
}

export type ContentApplyPackResult = { newContentVersion: string; backupPath: string }

export async function contentApplyPack(args: { packPath: string }): Promise<ApiResponse<ContentApplyPackResult>> {
  return await invoke('content_apply_pack', { args })
}

export type ContentCheckUpdateResult = { hasUpdate: boolean; latestVersion: string; downloadUrl: string; notes?: string | null }

export async function contentCheckUpdate(args: { baseUrl: string }): Promise<ApiResponse<ContentCheckUpdateResult>> {
  return await invoke('content_check_update', { args })
}

export type ContentDownloadUpdateResult = { packPath: string }

export async function contentDownloadUpdate(args: { url: string }): Promise<ApiResponse<ContentDownloadUpdateResult>> {
  return await invoke('content_download_update', { args })
}

export type VenueListItem = {
  id: string
  name: string
  type: string
}

export async function contentListVenues(args: {
  keyword?: string
  type?: string
  limit: number
  offset: number
}): Promise<ApiResponse<ListResponse<VenueListItem>>> {
  return await invoke('content_list_venues', { args })
}

export type VenueDetail = {
  id: string
  name: string
  type: string
  location?: string | null
  description?: string | null
  contact?: string | null
  openHours?: string | null
}

export async function contentGetVenue(args: { id: string }): Promise<ApiResponse<VenueDetail>> {
  return await invoke('content_get_venue', { args })
}

export type ListResponse<T> = { items: T[]; total: number }

export type CaseListItem = {
  id: string
  title: string
  scene: string
  summary: string
}

export type CaseDetail = {
  id: string
  title: string
  scene: string
  summary: string
  body: string
  violation?: string | null
  correctAction?: string | null
}

export async function contentListCases(args: {
  keyword?: string
  scene?: string
  limit: number
  offset: number
}): Promise<ApiResponse<ListResponse<CaseListItem>>> {
  return await invoke('content_list_cases', { args })
}

export async function contentGetCase(args: { id: string }): Promise<ApiResponse<CaseDetail>> {
  return await invoke('content_get_case', { args })
}

export type RegulationListItem = {
  id: string
  title: string
  level: string
  publishedAt?: string | null
}

export type RegulationSection = {
  id: string
  chapter?: string | null
  articleNo?: string | null
  title?: string | null
  body: string
}

export type RegulationDetail = {
  id: string
  title: string
  level: string
  source?: string | null
  publishedAt?: string | null
  sections: RegulationSection[]
}

export async function contentListRegulations(args: {
  keyword?: string
  level?: string
  limit: number
  offset: number
}): Promise<ApiResponse<ListResponse<RegulationListItem>>> {
  return await invoke('content_list_regulations', { args })
}

export async function contentGetRegulation(args: { id: string }): Promise<ApiResponse<RegulationDetail>> {
  return await invoke('content_get_regulation', { args })
}

export type StoryListItem = {
  id: string
  title: string
  source?: string | null
  dayOfYear?: number | null
}

export type StoryDetail = {
  id: string
  title: string
  body: string
  source?: string | null
  dayOfYear?: number | null
}

export async function contentListStories(args: {
  keyword?: string
  limit: number
  offset: number
}): Promise<ApiResponse<ListResponse<StoryListItem>>> {
  return await invoke('content_list_stories', { args })
}

export async function contentGetStory(args: { id: string }): Promise<ApiResponse<StoryDetail>> {
  return await invoke('content_get_story', { args })
}

export async function contentGetTodayStory(args: { yyyyMMdd: string }): Promise<ApiResponse<StoryDetail>> {
  return await invoke('content_get_today_story', { args })
}

export type ResolvedEntity = {
  entityType: string
  entityId: string
  exists: boolean
  title?: string | null
  subtitle?: string | null
}

export type ResolveEntitiesResult = { items: ResolvedEntity[] }

export async function contentResolveEntities(args: {
  items: { entityType: string; entityId: string }[]
}): Promise<ApiResponse<ResolveEntitiesResult>> {
  return await invoke('content_resolve_entities', { args })
}

export type QuizQuestionOption = { key: string; text: string }

export type QuizQuestion = {
  id: string
  stem: string
  type: string
  options: QuizQuestionOption[]
}

export type QuizStartSessionResult = { sessionId: string; questions: QuizQuestion[] }

export async function quizStartSession(args: { mode: 'daily' | 'stage' | 'practice' }): Promise<ApiResponse<QuizStartSessionResult>> {
  return await invoke('quiz_start_session', { args })
}

export type QuizSubmitAnswerResult = {
  isCorrect: boolean
  correctAnswer: string
  explanation?: string | null
  pointsDelta: number
  totalPoints: number
}

export async function quizSubmitAnswer(args: {
  sessionId: string
  questionId: string
  answer: string
}): Promise<ApiResponse<QuizSubmitAnswerResult>> {
  return await invoke('quiz_submit_answer', { args })
}

export type QuizProgress = { totalPoints: number }

export async function quizGetProgress(): Promise<ApiResponse<QuizProgress>> {
  return await invoke('quiz_get_progress')
}

export type FavoriteItem = {
  entityType: string
  entityId: string
  createdAt: number
}

export async function userListFavorites(args: {
  entityType?: string
  limit: number
  offset: number
}): Promise<ApiResponse<ListResponse<FavoriteItem>>> {
  return await invoke('user_list_favorites', { args })
}

export async function userSetFavorite(args: {
  entityType: string
  entityId: string
  isFavorite: boolean
}): Promise<ApiResponse<{ isFavorite: boolean }>> {
  return await invoke('user_set_favorite', { args })
}

export type UserSettings = { items: Record<string, string> }

export async function userGetSettings(): Promise<ApiResponse<UserSettings>> {
  return await invoke('user_get_settings')
}

export async function userUpdateSettings(args: { patch: Record<string, string> }): Promise<ApiResponse<UserSettings>> {
  return await invoke('user_update_settings', { args })
}
