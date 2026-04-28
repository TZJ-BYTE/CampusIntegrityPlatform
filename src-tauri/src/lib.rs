use std::sync::OnceLock;
use std::time::Duration;
use tauri::Manager;

pub mod db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app: &mut tauri::App| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let default_data_dir = app.path().app_data_dir()?;
      let mut data_dir = default_data_dir;
      let mut storage_mode = "system".to_string();

      if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
          let portable_dir = exe_dir.join("data");
          if portable_dir.is_dir() && std::fs::create_dir_all(&portable_dir).is_ok() {
            data_dir = portable_dir;
            storage_mode = "portable".to_string();
          }
        }
      }

      std::fs::create_dir_all(&data_dir)?;

      let backups_dir = data_dir.join("backups");
      std::fs::create_dir_all(&backups_dir)?;

      let logs_dir = data_dir.join("logs");
      std::fs::create_dir_all(&logs_dir)?;

      let device_id_path = data_dir.join("device_id.txt");
      let device_id = match std::fs::read_to_string(&device_id_path) {
        Ok(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
          let id = uuid::Uuid::new_v4().to_string();
          std::fs::write(&device_id_path, &id)?;
          id
        }
      };

      let user_db_path = data_dir.join("user.db");
      let user_db = db::open_user_db(&user_db_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

      let content_db_path = data_dir.join("content.db");
      let content_db = db::open_content_db(&content_db_path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

      app.manage(AppState {
        data_dir,
        storage_mode,
        device_id,
        online_enabled: true,
        user_db: std::sync::Arc::new(std::sync::Mutex::new(user_db)),
        content_db: std::sync::Arc::new(std::sync::Mutex::new(content_db)),
      });

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      app_get_status,
      app_get_data_dir,
      app_backup_user_db,
      app_list_backups,
      app_restore_user_db,
      app_delete_backup,
      content_get_version,
      content_import_db,
      content_apply_pack,
      content_check_update,
      content_download_update,
      content_list_venues,
      content_get_venue,
      content_list_cases,
      content_get_case,
      content_list_regulations,
      content_get_regulation,
      content_list_stories,
      content_get_story,
      content_get_today_story,
      content_resolve_entities,
      quiz_start_session,
      quiz_submit_answer,
      quiz_get_progress,
      user_list_favorites,
      user_is_favorite,
      user_set_favorite,
      user_get_profile,
      user_update_profile,
      user_get_settings,
      user_update_settings,
      auth_get_state,
      auth_set_server,
      auth_login,
      auth_logout,
      sync_get_state,
      sync_run,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[derive(Clone)]
struct AppState {
  data_dir: std::path::PathBuf,
  storage_mode: String,
  device_id: String,
  online_enabled: bool,
  user_db: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
  content_db: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

fn http_agent() -> &'static ureq::Agent {
  static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
  AGENT.get_or_init(|| {
    let config = ureq::Agent::config_builder()
      .timeout_global(Some(Duration::from_secs(12)))
      .timeout_connect(Some(Duration::from_secs(3)))
      .timeout_send_request(Some(Duration::from_secs(3)))
      .timeout_recv_response(Some(Duration::from_secs(6)))
      .timeout_recv_body(Some(Duration::from_secs(12)))
      .build();
    ureq::Agent::new_with_config(config)
  })
}

#[derive(serde::Serialize)]
struct ApiError {
  code: String,
  message: String,
  #[serde(skip_serializing_if = "serde_json::Value::is_null")]
  details: serde_json::Value,
}

#[derive(serde::Serialize)]
struct ApiResponse<T>
where
  T: serde::Serialize,
{
  ok: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  data: Option<T>,
  #[serde(skip_serializing_if = "Option::is_none")]
  error: Option<ApiError>,
}

fn ok<T>(data: T) -> ApiResponse<T>
where
  T: serde::Serialize,
{
  ApiResponse {
    ok: true,
    data: Some(data),
    error: None,
  }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
  app_version: String,
  content_version: String,
  user_schema_version: i64,
  device_id: String,
  is_online_enabled: bool,
}

#[tauri::command]
fn app_get_status(state: tauri::State<'_, AppState>) -> ApiResponse<AppStatus> {
  let content_version = match lock_content_db::<AppStatus>(&state)
    .ok()
    .and_then(|conn| {
      conn
        .query_row(
          "SELECT value FROM meta WHERE key = 'content_version'",
          [],
          |row| row.get::<_, String>(0),
        )
        .ok()
    }) {
    Some(v) => v,
    None => "0".to_string(),
  };

  ok(AppStatus {
    app_version: env!("CARGO_PKG_VERSION").to_string(),
    content_version,
    user_schema_version: db::USER_SCHEMA_VERSION,
    device_id: state.device_id.clone(),
    is_online_enabled: state.online_enabled,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DataDirInfo {
  path: String,
  mode: String,
}

#[tauri::command]
fn app_get_data_dir(state: tauri::State<'_, AppState>) -> ApiResponse<DataDirInfo> {
  ok(DataDirInfo {
    path: state.data_dir.to_string_lossy().to_string(),
    mode: state.storage_mode.clone(),
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupItem {
  path: String,
  file_name: String,
  size: i64,
  modified_at: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupsList {
  items: Vec<BackupItem>,
}

#[tauri::command]
fn app_list_backups(state: tauri::State<'_, AppState>) -> ApiResponse<BackupsList> {
  let dir = state.data_dir.join("backups");
  let mut items: Vec<BackupItem> = Vec::new();

  let rd = match std::fs::read_dir(&dir) {
    Ok(v) => v,
    Err(e) => return io_error(e),
  };

  for entry in rd.flatten() {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("db") {
      continue;
    }
    let meta = match entry.metadata() {
      Ok(m) => m,
      Err(_) => continue,
    };
    let modified_at = meta
      .modified()
      .ok()
      .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
      .map(|d| d.as_millis() as i64)
      .unwrap_or(0);
    let size = meta.len() as i64;
    let file_name = path
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or_default()
      .to_string();

    items.push(BackupItem {
      path: path.to_string_lossy().to_string(),
      file_name,
      size,
      modified_at,
    });
  }

  items.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
  ok(BackupsList { items })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupResult {
  backup_path: String,
}

#[tauri::command]
fn app_backup_user_db(state: tauri::State<'_, AppState>) -> ApiResponse<BackupResult> {
  let now = unix_ms();
  let backups_dir = state.data_dir.join("backups");
  if let Err(e) = std::fs::create_dir_all(&backups_dir) {
    return io_error(e);
  }

  let backup_path = backups_dir.join(format!("user-{}.db", now));
  let backup_path_str = backup_path.to_string_lossy().to_string();
  let escaped = backup_path_str.replace('\'', "''");
  let sql = format!("VACUUM INTO '{}';", escaped);

  let conn = match lock_user_db::<BackupResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  if let Err(e) = conn.execute_batch(&sql) {
    return db_error(e);
  }

  ok(BackupResult {
    backup_path: backup_path_str,
  })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreArgs {
  backup_path: String,
}

#[tauri::command]
fn app_restore_user_db(
  state: tauri::State<'_, AppState>,
  args: RestoreArgs,
) -> ApiResponse<bool> {
  let backups_dir = match std::fs::canonicalize(state.data_dir.join("backups")) {
    Ok(p) => p,
    Err(e) => return io_error(e),
  };
  let backup_path = match std::fs::canonicalize(&args.backup_path) {
    Ok(p) => p,
    Err(e) => return io_error(e),
  };
  if !backup_path.starts_with(&backups_dir) {
    return invalid_argument("backupPath 不在 backups 目录内");
  }
  if !backup_path.is_file() {
    return invalid_argument("backupPath 不是有效文件");
  }

  let user_db_path = state.data_dir.join("user.db");
  let now = unix_ms();

  let mut conn = match lock_user_db::<bool>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let before_path = backups_dir.join(format!("user-before-restore-{}.db", now));
  let before_path_str = before_path.to_string_lossy().to_string();
  let escaped = before_path_str.replace('\'', "''");
  let sql = format!("VACUUM INTO '{}';", escaped);
  if let Err(e) = conn.execute_batch(&sql) {
    return db_error(e);
  }

  let tmp = match rusqlite::Connection::open_in_memory() {
    Ok(c) => c,
    Err(e) => return db_error(e),
  };
  let old = std::mem::replace(&mut *conn, tmp);
  drop(old);

  if let Err(e) = std::fs::copy(&backup_path, &user_db_path) {
    return io_error(e);
  }

  let reopened = match db::open_user_db(&user_db_path) {
    Ok(c) => c,
    Err(e) => return db_error(e),
  };
  *conn = reopened;

  ok(true)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteBackupArgs {
  backup_path: String,
}

#[tauri::command]
fn app_delete_backup(
  state: tauri::State<'_, AppState>,
  args: DeleteBackupArgs,
) -> ApiResponse<bool> {
  let backups_dir = match std::fs::canonicalize(state.data_dir.join("backups")) {
    Ok(p) => p,
    Err(e) => return io_error(e),
  };
  let backup_path = match std::fs::canonicalize(&args.backup_path) {
    Ok(p) => p,
    Err(e) => return io_error(e),
  };
  if !backup_path.starts_with(&backups_dir) {
    return invalid_argument("backupPath 不在 backups 目录内");
  }
  if let Err(e) = std::fs::remove_file(&backup_path) {
    return io_error(e);
  }
  ok(true)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse<T>
where
  T: serde::Serialize,
{
  items: Vec<T>,
  total: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VenueListItem {
  id: String,
  name: String,
  #[serde(rename = "type")]
  type_: String,
  cover_url: Option<String>,
}

#[tauri::command]
fn content_get_version(state: tauri::State<'_, AppState>) -> ApiResponse<ContentVersion> {
  let conn = match lock_content_db::<ContentVersion>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let version: String = match conn.query_row(
    "SELECT value FROM meta WHERE key = 'content_version'",
    [],
    |row| row.get(0),
  ) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  ok(ContentVersion {
    content_version: version,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentVersion {
  content_version: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentImportDbArgs {
  source_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentImportDbResult {
  backup_path: String,
  content_version: String,
}

fn content_import_db_inner(
  state: &tauri::State<'_, AppState>,
  source_path: &std::path::Path,
) -> ApiResponse<ContentImportDbResult> {
  use rusqlite::OptionalExtension;

  let source_probe = match rusqlite::Connection::open(&source_path) {
    Ok(c) => c,
    Err(e) => return db_error(e),
  };
  let imported_version: Option<String> = match source_probe
    .query_row(
      "SELECT value FROM meta WHERE key = 'content_version'",
      [],
      |row| row.get(0),
    )
    .optional()
  {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };
  if imported_version.is_none() {
    return invalid_argument("sourcePath 不是有效的内容库（缺少 meta.content_version）");
  }

  let now = unix_ms();
  let backups_dir = state.data_dir.join("backups");
  if let Err(e) = std::fs::create_dir_all(&backups_dir) {
    return io_error(e);
  }
  let backup_path = backups_dir.join(format!("content-before-import-{}.db", now));
  let backup_path_str = backup_path.to_string_lossy().to_string();

  let content_db_path = state.data_dir.join("content.db");

  let mut conn = match lock_content_db::<ContentImportDbResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let escaped = backup_path_str.replace('\'', "''");
  let sql = format!("VACUUM INTO '{}';", escaped);
  if let Err(e) = conn.execute_batch(&sql) {
    return db_error(e);
  }

  let tmp = match rusqlite::Connection::open_in_memory() {
    Ok(c) => c,
    Err(e) => return db_error(e),
  };
  let old = std::mem::replace(&mut *conn, tmp);
  drop(old);

  if let Err(e) = std::fs::copy(&source_path, &content_db_path) {
    return io_error(e);
  }

  let reopened = match db::open_content_db(&content_db_path) {
    Ok(c) => c,
    Err(e) => return db_error(e),
  };

  let content_version: String = match reopened.query_row(
    "SELECT value FROM meta WHERE key = 'content_version'",
    [],
    |row| row.get(0),
  ) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  *conn = reopened;
  ok(ContentImportDbResult {
    backup_path: backup_path_str,
    content_version,
  })
}

#[tauri::command]
fn content_import_db(
  state: tauri::State<'_, AppState>,
  args: ContentImportDbArgs,
) -> ApiResponse<ContentImportDbResult> {
  let source_path = match std::fs::canonicalize(&args.source_path) {
    Ok(p) => p,
    Err(e) => return io_error(e),
  };
  if !source_path.is_file() {
    return invalid_argument("sourcePath 不是有效文件");
  }
  let ext = source_path.extension().and_then(|s| s.to_str()).unwrap_or("");
  if ext != "db" && ext != "sqlite" {
    return invalid_argument("sourcePath 仅支持 .db / .sqlite 文件");
  }

  content_import_db_inner(&state, &source_path)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentApplyPackArgs {
  pack_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentApplyPackResult {
  new_content_version: String,
  backup_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentPackManifest {
  content_version: String,
  min_app_version: Option<String>,
}

#[tauri::command]
fn content_apply_pack(
  state: tauri::State<'_, AppState>,
  args: ContentApplyPackArgs,
) -> ApiResponse<ContentApplyPackResult> {
  use std::io::Read;

  let pack_path = match std::fs::canonicalize(&args.pack_path) {
    Ok(p) => p,
    Err(e) => return io_error(e),
  };
  if !pack_path.is_file() {
    return invalid_argument("packPath 不是有效文件");
  }
  let ext = pack_path.extension().and_then(|s| s.to_str()).unwrap_or("");
  if ext != "zip" {
    return invalid_argument("packPath 仅支持 .zip");
  }

  let tmp_dir = state
    .data_dir
    .join("tmp")
    .join(format!("pack-{}", unix_ms()));
  if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
    return io_error(e);
  }

  let file = match std::fs::File::open(&pack_path) {
    Ok(f) => f,
    Err(e) => return io_error(e),
  };
  let mut zip = match zip::ZipArchive::new(file) {
    Ok(z) => z,
    Err(e) => return invalid_argument(&format!("无法读取 zip：{}", e)),
  };

  let mut manifest_text = String::new();
  {
    let mut mf = match zip.by_name("manifest.json") {
      Ok(f) => f,
      Err(_) => return invalid_argument("更新包缺少 manifest.json"),
    };
    if let Err(e) = mf.read_to_string(&mut manifest_text) {
      return invalid_argument(&format!("读取 manifest.json 失败：{}", e));
    }
  }

  let manifest: ContentPackManifest = match serde_json::from_str(&manifest_text) {
    Ok(v) => v,
    Err(e) => return invalid_argument(&format!("manifest.json 解析失败：{}", e)),
  };
  if manifest.content_version.trim().is_empty() {
    return invalid_argument("manifest.content_version 不能为空");
  }
  let _ = manifest.min_app_version;

  let mut content_entry: Option<String> = None;
  for i in 0..zip.len() {
    let name = match zip.by_index(i).ok().map(|f| f.name().to_string()) {
      Some(n) => n,
      None => continue,
    };
    if name == "content.db" || name.ends_with("/content.db") {
      content_entry = Some(name);
      break;
    }
  }
  let content_entry = match content_entry {
    Some(v) => v,
    None => return invalid_argument("更新包缺少 content.db"),
  };

  let extracted_db = tmp_dir.join("content.db");
  {
    let mut f = match zip.by_name(&content_entry) {
      Ok(v) => v,
      Err(_) => return invalid_argument("读取 content.db 失败"),
    };
    let mut out = match std::fs::File::create(&extracted_db) {
      Ok(v) => v,
      Err(e) => return io_error(e),
    };
    if let Err(e) = std::io::copy(&mut f, &mut out) {
      return io_error(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
    }
  }

  let imported = content_import_db_inner(&state, &extracted_db);
  let _ = std::fs::remove_dir_all(&tmp_dir);
  match imported {
    ApiResponse { ok: true, data: Some(d), .. } => ok(ContentApplyPackResult {
      new_content_version: d.content_version,
      backup_path: d.backup_path,
    }),
    ApiResponse { ok: false, error: Some(e), .. } => ApiResponse {
      ok: false,
      data: None,
      error: Some(e),
    },
    _ => ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "INTERNAL".to_string(),
        message: "content_apply_pack failed".to_string(),
        details: serde_json::Value::Null,
      }),
    },
  }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentCheckUpdateResult {
  has_update: bool,
  latest_version: String,
  download_url: String,
  notes: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentCheckUpdateArgs {
  base_url: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionsJson {
  latest: VersionsLatest,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionsLatest {
  version: String,
  url: String,
  notes: Option<String>,
}

#[tauri::command]
fn content_check_update(
  state: tauri::State<'_, AppState>,
  args: ContentCheckUpdateArgs,
) -> ApiResponse<ContentCheckUpdateResult> {
  let base = args.base_url.trim().trim_end_matches('/').to_string();
  if base.is_empty() {
    return invalid_argument("baseUrl 不能为空");
  }

  let conn = match lock_content_db::<ContentCheckUpdateResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let current: String = match conn.query_row(
    "SELECT value FROM meta WHERE key = 'content_version'",
    [],
    |row| row.get(0),
  ) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };
  drop(conn);

  let url = format!("{}/versions.json", base);
  let text = match http_agent().get(&url).call() {
    Ok(r) => match r.into_body().read_to_string() {
      Ok(s) => s,
      Err(e) => return invalid_argument(&format!("读取 versions.json 失败：{}", e)),
    },
    Err(e) => return invalid_argument(&format!("请求 versions.json 失败：{}", e)),
  };

  let payload: VersionsJson = match serde_json::from_str(&text) {
    Ok(v) => v,
    Err(e) => return invalid_argument(&format!("versions.json 解析失败：{}", e)),
  };
  let latest_version = payload.latest.version;
  let raw_url = payload.latest.url;
  let download_url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
    raw_url
  } else if raw_url.starts_with('/') {
    format!("{}{}", base, raw_url)
  } else {
    format!("{}/{}", base, raw_url)
  };
  let notes = payload.latest.notes;
  let has_update = latest_version != current;

  ok(ContentCheckUpdateResult {
    has_update,
    latest_version,
    download_url,
    notes,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentDownloadUpdateResult {
  pack_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentDownloadUpdateArgs {
  url: String,
}

#[tauri::command]
fn content_download_update(
  state: tauri::State<'_, AppState>,
  args: ContentDownloadUpdateArgs,
) -> ApiResponse<ContentDownloadUpdateResult> {
  let url = args.url.trim().to_string();
  if url.is_empty() {
    return invalid_argument("url 不能为空");
  }

  let tmp_dir = state.data_dir.join("tmp");
  if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
    return io_error(e);
  }
  let pack_path = tmp_dir.join(format!("content-pack-{}.zip", unix_ms()));
  let pack_path_str = pack_path.to_string_lossy().to_string();

  let resp = match http_agent().get(&url).call() {
    Ok(r) => r,
    Err(e) => return invalid_argument(&format!("下载失败：{}", e)),
  };

  let mut out = match std::fs::File::create(&pack_path) {
    Ok(f) => f,
    Err(e) => return io_error(e),
  };
  let mut body = resp.into_body();
  let mut reader = body.as_reader();
  if let Err(e) = std::io::copy(&mut reader, &mut out) {
    return io_error(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
  }

  ok(ContentDownloadUpdateResult {
    pack_path: pack_path_str,
  })
}

#[tauri::command]
fn content_list_venues(
  state: tauri::State<'_, AppState>,
  args: ContentListVenuesArgs,
) -> ApiResponse<ListResponse<VenueListItem>> {
  let ContentListVenuesArgs {
    keyword,
    type_,
    limit,
    offset,
  } = args;

  let limit = limit.clamp(1, 100);
  let offset = offset.max(0);

  let conn = match lock_content_db::<ListResponse<VenueListItem>>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let mut items: Vec<VenueListItem> = Vec::new();

  let like = keyword.as_ref().map(|k| format!("%{}%", k.trim()));

  let (count_sql, list_sql, params_count, params_list): (&str, &str, Vec<rusqlite::types::Value>, Vec<rusqlite::types::Value>) =
    match (like.as_ref(), type_.as_ref()) {
      (Some(k), Some(t)) => (
        "SELECT COUNT(1) FROM venues WHERE name LIKE ?1 AND type = ?2",
        "SELECT id, name, type, cover_url FROM venues WHERE name LIKE ?1 AND type = ?2 ORDER BY updated_at DESC LIMIT ?3 OFFSET ?4",
        vec![k.clone().into(), t.clone().into()],
        vec![k.clone().into(), t.clone().into(), limit.into(), offset.into()],
      ),
      (Some(k), None) => (
        "SELECT COUNT(1) FROM venues WHERE name LIKE ?1",
        "SELECT id, name, type, cover_url FROM venues WHERE name LIKE ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
        vec![k.clone().into()],
        vec![k.clone().into(), limit.into(), offset.into()],
      ),
      (None, Some(t)) => (
        "SELECT COUNT(1) FROM venues WHERE type = ?1",
        "SELECT id, name, type, cover_url FROM venues WHERE type = ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
        vec![t.clone().into()],
        vec![t.clone().into(), limit.into(), offset.into()],
      ),
      (None, None) => (
        "SELECT COUNT(1) FROM venues",
        "SELECT id, name, type, cover_url FROM venues ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2",
        vec![],
        vec![limit.into(), offset.into()],
      ),
    };

  let total: i64 = match conn.query_row(count_sql, rusqlite::params_from_iter(params_count), |row| row.get(0)) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  let result = (|| -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(list_sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params_list))?;
    while let Some(row) = rows.next()? {
      items.push(VenueListItem {
        id: row.get(0)?,
        name: row.get(1)?,
        type_: row.get(2)?,
        cover_url: row.get(3)?,
      });
    }
    Ok(())
  })();

  if let Err(e) = result {
    return db_error(e);
  }

  ok(ListResponse { items, total })
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentListVenuesArgs {
  keyword: Option<String>,
  #[serde(rename = "type")]
  type_: Option<String>,
  limit: i64,
  offset: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VenueDetail {
  id: String,
  name: String,
  #[serde(rename = "type")]
  type_: String,
  cover_url: Option<String>,
  location: Option<String>,
  description: Option<String>,
  contact: Option<String>,
  open_hours: Option<String>,
}

#[tauri::command]
fn content_get_venue(
  state: tauri::State<'_, AppState>,
  args: ContentGetByIdArgs,
) -> ApiResponse<VenueDetail> {
  let conn = match lock_content_db::<VenueDetail>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let res: rusqlite::Result<VenueDetail> = conn.query_row(
    "SELECT id, name, type, cover_url, location, description, contact, open_hours FROM venues WHERE id = ?1",
    rusqlite::params![args.id],
    |row| {
      Ok(VenueDetail {
        id: row.get(0)?,
        name: row.get(1)?,
        type_: row.get(2)?,
        cover_url: row.get(3)?,
        location: row.get(4)?,
        description: row.get(5)?,
        contact: row.get(6)?,
        open_hours: row.get(7)?,
      })
    },
  );

  match res {
    Ok(v) => ok(v),
    Err(rusqlite::Error::QueryReturnedNoRows) => ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "NOT_FOUND".to_string(),
        message: "venue not found".to_string(),
        details: serde_json::Value::Null,
      }),
    },
    Err(e) => db_error(e),
  }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseListItem {
  id: String,
  title: String,
  scene: String,
  summary: String,
  cover_url: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentListCasesArgs {
  keyword: Option<String>,
  scene: Option<String>,
  limit: i64,
  offset: i64,
}

#[tauri::command]
fn content_list_cases(
  state: tauri::State<'_, AppState>,
  args: ContentListCasesArgs,
) -> ApiResponse<ListResponse<CaseListItem>> {
  let ContentListCasesArgs {
    keyword,
    scene,
    limit,
    offset,
  } = args;

  let limit = limit.clamp(1, 100);
  let offset = offset.max(0);
  let keyword_fts = keyword
    .as_ref()
    .map(|k| format!("\"{}\"", k.trim().replace('"', "\"\"")));

  let conn = match lock_content_db::<ListResponse<CaseListItem>>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let (count_sql, list_sql, params_count, params_list): (
    &str,
    &str,
    Vec<rusqlite::types::Value>,
    Vec<rusqlite::types::Value>,
  ) = match (keyword_fts.as_ref(), scene.as_ref()) {
    (Some(k), Some(s)) => (
      "SELECT COUNT(1) FROM cases c JOIN cases_fts f ON f.rowid = c.rowid WHERE f MATCH ?1 AND c.scene = ?2",
      "SELECT c.id, c.title, c.scene, c.summary, c.cover_url FROM cases c JOIN cases_fts f ON f.rowid = c.rowid WHERE f MATCH ?1 AND c.scene = ?2 ORDER BY c.updated_at DESC LIMIT ?3 OFFSET ?4",
      vec![k.clone().into(), s.clone().into()],
      vec![k.clone().into(), s.clone().into(), limit.into(), offset.into()],
    ),
    (Some(k), None) => (
      "SELECT COUNT(1) FROM cases c JOIN cases_fts f ON f.rowid = c.rowid WHERE f MATCH ?1",
      "SELECT c.id, c.title, c.scene, c.summary, c.cover_url FROM cases c JOIN cases_fts f ON f.rowid = c.rowid WHERE f MATCH ?1 ORDER BY c.updated_at DESC LIMIT ?2 OFFSET ?3",
      vec![k.clone().into()],
      vec![k.clone().into(), limit.into(), offset.into()],
    ),
    (None, Some(s)) => (
      "SELECT COUNT(1) FROM cases WHERE scene = ?1",
      "SELECT id, title, scene, summary, cover_url FROM cases WHERE scene = ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
      vec![s.clone().into()],
      vec![s.clone().into(), limit.into(), offset.into()],
    ),
    (None, None) => (
      "SELECT COUNT(1) FROM cases",
      "SELECT id, title, scene, summary, cover_url FROM cases ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2",
      vec![],
      vec![limit.into(), offset.into()],
    ),
  };

  let total: i64 = match conn.query_row(count_sql, rusqlite::params_from_iter(params_count), |row| row.get(0)) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  let mut items: Vec<CaseListItem> = Vec::new();
  let result = (|| -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(list_sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params_list))?;
    while let Some(row) = rows.next()? {
      items.push(CaseListItem {
        id: row.get(0)?,
        title: row.get(1)?,
        scene: row.get(2)?,
        summary: row.get(3)?,
        cover_url: row.get(4)?,
      });
    }
    Ok(())
  })();

  if let Err(e) = result {
    return db_error(e);
  }

  ok(ListResponse { items, total })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentGetByIdArgs {
  id: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseDetail {
  id: String,
  title: String,
  scene: String,
  summary: String,
  cover_url: Option<String>,
  body: String,
  violation: Option<String>,
  correct_action: Option<String>,
}

#[tauri::command]
fn content_get_case(
  state: tauri::State<'_, AppState>,
  args: ContentGetByIdArgs,
) -> ApiResponse<CaseDetail> {
  let conn = match lock_content_db::<CaseDetail>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let res: rusqlite::Result<CaseDetail> = conn.query_row(
    "SELECT id, title, scene, summary, cover_url, body, violation, correct_action FROM cases WHERE id = ?1",
    rusqlite::params![args.id],
    |row| {
      Ok(CaseDetail {
        id: row.get(0)?,
        title: row.get(1)?,
        scene: row.get(2)?,
        summary: row.get(3)?,
        cover_url: row.get(4)?,
        body: row.get(5)?,
        violation: row.get(6)?,
        correct_action: row.get(7)?,
      })
    },
  );

  match res {
    Ok(v) => ok(v),
    Err(rusqlite::Error::QueryReturnedNoRows) => ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "NOT_FOUND".to_string(),
        message: "case not found".to_string(),
        details: serde_json::Value::Null,
      }),
    },
    Err(e) => db_error(e),
  }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RegulationListItem {
  id: String,
  title: String,
  level: String,
  published_at: Option<String>,
  cover_url: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentListRegulationsArgs {
  keyword: Option<String>,
  level: Option<String>,
  limit: i64,
  offset: i64,
}

#[tauri::command]
fn content_list_regulations(
  state: tauri::State<'_, AppState>,
  args: ContentListRegulationsArgs,
) -> ApiResponse<ListResponse<RegulationListItem>> {
  let ContentListRegulationsArgs {
    keyword,
    level,
    limit,
    offset,
  } = args;

  let limit = limit.clamp(1, 100);
  let offset = offset.max(0);
  let like = keyword.as_ref().map(|k| format!("%{}%", k.trim()));

  let conn = match lock_content_db::<ListResponse<RegulationListItem>>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let (count_sql, list_sql, params_count, params_list): (
    &str,
    &str,
    Vec<rusqlite::types::Value>,
    Vec<rusqlite::types::Value>,
  ) = match (like.as_ref(), level.as_ref()) {
    (Some(k), Some(lv)) => (
      "SELECT COUNT(1) FROM regulations WHERE title LIKE ?1 AND level = ?2",
      "SELECT id, title, level, published_at, cover_url FROM regulations WHERE title LIKE ?1 AND level = ?2 ORDER BY updated_at DESC LIMIT ?3 OFFSET ?4",
      vec![k.clone().into(), lv.clone().into()],
      vec![k.clone().into(), lv.clone().into(), limit.into(), offset.into()],
    ),
    (Some(k), None) => (
      "SELECT COUNT(1) FROM regulations WHERE title LIKE ?1",
      "SELECT id, title, level, published_at, cover_url FROM regulations WHERE title LIKE ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
      vec![k.clone().into()],
      vec![k.clone().into(), limit.into(), offset.into()],
    ),
    (None, Some(lv)) => (
      "SELECT COUNT(1) FROM regulations WHERE level = ?1",
      "SELECT id, title, level, published_at, cover_url FROM regulations WHERE level = ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
      vec![lv.clone().into()],
      vec![lv.clone().into(), limit.into(), offset.into()],
    ),
    (None, None) => (
      "SELECT COUNT(1) FROM regulations",
      "SELECT id, title, level, published_at, cover_url FROM regulations ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2",
      vec![],
      vec![limit.into(), offset.into()],
    ),
  };

  let total: i64 = match conn.query_row(count_sql, rusqlite::params_from_iter(params_count), |row| row.get(0)) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  let mut items: Vec<RegulationListItem> = Vec::new();
  let result = (|| -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(list_sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params_list))?;
    while let Some(row) = rows.next()? {
      items.push(RegulationListItem {
        id: row.get(0)?,
        title: row.get(1)?,
        level: row.get(2)?,
        published_at: row.get(3)?,
        cover_url: row.get(4)?,
      });
    }
    Ok(())
  })();

  if let Err(e) = result {
    return db_error(e);
  }

  ok(ListResponse { items, total })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RegulationSection {
  id: String,
  chapter: Option<String>,
  article_no: Option<String>,
  title: Option<String>,
  body: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RegulationDetail {
  id: String,
  title: String,
  level: String,
  cover_url: Option<String>,
  source: Option<String>,
  published_at: Option<String>,
  sections: Vec<RegulationSection>,
}

#[tauri::command]
fn content_get_regulation(
  state: tauri::State<'_, AppState>,
  args: ContentGetByIdArgs,
) -> ApiResponse<RegulationDetail> {
  let conn = match lock_content_db::<RegulationDetail>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let header: rusqlite::Result<(String, String, String, Option<String>, Option<String>, Option<String>)> =
    conn.query_row(
      "SELECT id, title, level, cover_url, source, published_at FROM regulations WHERE id = ?1",
      rusqlite::params![args.id.clone()],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    );

  let (id, title, level, cover_url, source, published_at) = match header {
    Ok(v) => v,
    Err(rusqlite::Error::QueryReturnedNoRows) => {
      return ApiResponse {
        ok: false,
        data: None,
        error: Some(ApiError {
          code: "NOT_FOUND".to_string(),
          message: "regulation not found".to_string(),
          details: serde_json::Value::Null,
        }),
      };
    }
    Err(e) => return db_error(e),
  };

  let mut sections: Vec<RegulationSection> = Vec::new();
  let result = (|| -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
      "SELECT id, chapter, article_no, title, body FROM regulation_sections WHERE regulation_id = ?1 ORDER BY updated_at ASC",
    )?;
    let mut rows = stmt.query(rusqlite::params![args.id])?;
    while let Some(row) = rows.next()? {
      sections.push(RegulationSection {
        id: row.get(0)?,
        chapter: row.get(1)?,
        article_no: row.get(2)?,
        title: row.get(3)?,
        body: row.get(4)?,
      });
    }
    Ok(())
  })();

  if let Err(e) = result {
    return db_error(e);
  }

  ok(RegulationDetail {
    id,
    title,
    level,
    cover_url,
    source,
    published_at,
    sections,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StoryListItem {
  id: String,
  title: String,
  source: Option<String>,
  day_of_year: Option<i64>,
  cover_url: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentListStoriesArgs {
  keyword: Option<String>,
  limit: i64,
  offset: i64,
}

#[tauri::command]
fn content_list_stories(
  state: tauri::State<'_, AppState>,
  args: ContentListStoriesArgs,
) -> ApiResponse<ListResponse<StoryListItem>> {
  let ContentListStoriesArgs {
    keyword,
    limit,
    offset,
  } = args;

  let limit = limit.clamp(1, 100);
  let offset = offset.max(0);
  let keyword_fts = keyword
    .as_ref()
    .map(|k| format!("\"{}\"", k.trim().replace('"', "\"\"")));

  let conn = match lock_content_db::<ListResponse<StoryListItem>>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let (count_sql, list_sql, params_count, params_list): (
    &str,
    &str,
    Vec<rusqlite::types::Value>,
    Vec<rusqlite::types::Value>,
  ) = if let Some(k) = keyword_fts.as_ref() {
    (
      "SELECT COUNT(1) FROM stories s JOIN stories_fts f ON f.rowid = s.rowid WHERE f MATCH ?1",
      "SELECT s.id, s.title, s.source, s.day_of_year, s.cover_url FROM stories s JOIN stories_fts f ON f.rowid = s.rowid WHERE f MATCH ?1 ORDER BY s.updated_at DESC LIMIT ?2 OFFSET ?3",
      vec![k.clone().into()],
      vec![k.clone().into(), limit.into(), offset.into()],
    )
  } else {
    (
      "SELECT COUNT(1) FROM stories",
      "SELECT id, title, source, day_of_year, cover_url FROM stories ORDER BY updated_at DESC LIMIT ?1 OFFSET ?2",
      vec![],
      vec![limit.into(), offset.into()],
    )
  };

  let total: i64 = match conn.query_row(count_sql, rusqlite::params_from_iter(params_count), |row| row.get(0)) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  let mut items: Vec<StoryListItem> = Vec::new();
  let result = (|| -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(list_sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params_list))?;
    while let Some(row) = rows.next()? {
      items.push(StoryListItem {
        id: row.get(0)?,
        title: row.get(1)?,
        source: row.get(2)?,
        day_of_year: row.get(3)?,
        cover_url: row.get(4)?,
      });
    }
    Ok(())
  })();

  if let Err(e) = result {
    return db_error(e);
  }

  ok(ListResponse { items, total })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StoryDetail {
  id: String,
  title: String,
  cover_url: Option<String>,
  body: String,
  source: Option<String>,
  day_of_year: Option<i64>,
}

#[tauri::command]
fn content_get_story(
  state: tauri::State<'_, AppState>,
  args: ContentGetByIdArgs,
) -> ApiResponse<StoryDetail> {
  let conn = match lock_content_db::<StoryDetail>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let res: rusqlite::Result<StoryDetail> = conn.query_row(
    "SELECT id, title, cover_url, body, source, day_of_year FROM stories WHERE id = ?1",
    rusqlite::params![args.id],
    |row| {
      Ok(StoryDetail {
        id: row.get(0)?,
        title: row.get(1)?,
        cover_url: row.get(2)?,
        body: row.get(3)?,
        source: row.get(4)?,
        day_of_year: row.get(5)?,
      })
    },
  );

  match res {
    Ok(v) => ok(v),
    Err(rusqlite::Error::QueryReturnedNoRows) => ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "NOT_FOUND".to_string(),
        message: "story not found".to_string(),
        details: serde_json::Value::Null,
      }),
    },
    Err(e) => db_error(e),
  }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentGetTodayStoryArgs {
  #[serde(rename = "yyyyMMdd")]
  yyyy_mm_dd: String,
}

#[tauri::command]
fn content_get_today_story(
  state: tauri::State<'_, AppState>,
  args: ContentGetTodayStoryArgs,
) -> ApiResponse<StoryDetail> {
  let day = match day_of_year_from_yyyymmdd(&args.yyyy_mm_dd) {
    Some(v) => v,
    None => 1,
  };

  let conn = match lock_content_db::<StoryDetail>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let res: rusqlite::Result<StoryDetail> = conn.query_row(
    "SELECT id, title, cover_url, body, source, day_of_year FROM stories WHERE day_of_year = ?1 ORDER BY updated_at DESC LIMIT 1",
    rusqlite::params![day],
    |row| {
      Ok(StoryDetail {
        id: row.get(0)?,
        title: row.get(1)?,
        cover_url: row.get(2)?,
        body: row.get(3)?,
        source: row.get(4)?,
        day_of_year: row.get(5)?,
      })
    },
  );

  match res {
    Ok(v) => ok(v),
    Err(rusqlite::Error::QueryReturnedNoRows) => {
      let fallback: rusqlite::Result<StoryDetail> = conn.query_row(
        "SELECT id, title, cover_url, body, source, day_of_year FROM stories WHERE id = ?1",
        rusqlite::params!["story_demo"],
        |row| {
          Ok(StoryDetail {
            id: row.get(0)?,
            title: row.get(1)?,
            cover_url: row.get(2)?,
            body: row.get(3)?,
            source: row.get(4)?,
            day_of_year: row.get(5)?,
          })
        },
      );

      match fallback {
        Ok(v) => ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => ApiResponse {
          ok: false,
          data: None,
          error: Some(ApiError {
            code: "NOT_FOUND".to_string(),
            message: "story not found".to_string(),
            details: serde_json::Value::Null,
          }),
        },
        Err(e) => db_error(e),
      }
    }
    Err(e) => db_error(e),
  }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentResolveEntitiesArgs {
  items: Vec<ResolveEntityRef>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveEntityRef {
  entity_type: String,
  entity_id: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedEntity {
  entity_type: String,
  entity_id: String,
  exists: bool,
  title: Option<String>,
  subtitle: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveEntitiesResult {
  items: Vec<ResolvedEntity>,
}

#[tauri::command]
fn content_resolve_entities(
  state: tauri::State<'_, AppState>,
  args: ContentResolveEntitiesArgs,
) -> ApiResponse<ResolveEntitiesResult> {
  use rusqlite::OptionalExtension;

  let conn = match lock_content_db::<ResolveEntitiesResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let mut items: Vec<ResolvedEntity> = Vec::with_capacity(args.items.len());
  for r in args.items {
    let et = r.entity_type.as_str();
    let (exists, title, subtitle) = match et {
      "venue" => {
        let row: rusqlite::Result<Option<(String, Option<String>)>> = conn
          .query_row(
            "SELECT name, location FROM venues WHERE id = ?1",
            rusqlite::params![&r.entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
          )
          .optional();
        match row {
          Ok(Some((t, s))) => (true, Some(t), s),
          Ok(None) => (false, None, None),
          Err(e) => return db_error(e),
        }
      }
      "case" => {
        let row: rusqlite::Result<Option<(String, String)>> = conn
          .query_row(
            "SELECT title, scene FROM cases WHERE id = ?1",
            rusqlite::params![&r.entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
          )
          .optional();
        match row {
          Ok(Some((t, s))) => (true, Some(t), Some(s)),
          Ok(None) => (false, None, None),
          Err(e) => return db_error(e),
        }
      }
      "regulation" => {
        let row: rusqlite::Result<Option<(String, String)>> = conn
          .query_row(
            "SELECT title, level FROM regulations WHERE id = ?1",
            rusqlite::params![&r.entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
          )
          .optional();
        match row {
          Ok(Some((t, s))) => (true, Some(t), Some(s)),
          Ok(None) => (false, None, None),
          Err(e) => return db_error(e),
        }
      }
      "story" => {
        let row: rusqlite::Result<Option<(String, Option<String>)>> = conn
          .query_row(
            "SELECT title, source FROM stories WHERE id = ?1",
            rusqlite::params![&r.entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
          )
          .optional();
        match row {
          Ok(Some((t, s))) => (true, Some(t), s),
          Ok(None) => (false, None, None),
          Err(e) => return db_error(e),
        }
      }
      _ => (false, None, None),
    };

    items.push(ResolvedEntity {
      entity_type: r.entity_type,
      entity_id: r.entity_id,
      exists,
      title,
      subtitle,
    });
  }

  ok(ResolveEntitiesResult { items })
}

fn day_of_year_from_yyyymmdd(s: &str) -> Option<i64> {
  let digits = s.replace('-', "");
  if digits.len() != 8 {
    return None;
  }
  let y: i32 = digits[0..4].parse().ok()?;
  let m: i32 = digits[4..6].parse().ok()?;
  let d: i32 = digits[6..8].parse().ok()?;
  if m < 1 || m > 12 || d < 1 || d > 31 {
    return None;
  }
  let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
  let month_days = [31, 28 + if leap { 1 } else { 0 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  let mut sum = 0;
  for i in 0..(m - 1) {
    sum += month_days[i as usize];
  }
  sum += d;
  Some(sum as i64)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QuizQuestionOption {
  key: String,
  text: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QuizQuestion {
  id: String,
  stem: String,
  r#type: String,
  options: Vec<QuizQuestionOption>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuizStartSessionArgs {
  mode: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QuizStartSessionResult {
  session_id: String,
  questions: Vec<QuizQuestion>,
}

#[tauri::command]
fn quiz_start_session(
  state: tauri::State<'_, AppState>,
  args: QuizStartSessionArgs,
) -> ApiResponse<QuizStartSessionResult> {
  let conn_content = match lock_content_db::<QuizStartSessionResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let limit = if args.mode == "daily" { 5 } else { 10 };
  let mut qids: Vec<String> = Vec::new();
  let r = (|| -> rusqlite::Result<()> {
    let mut stmt = conn_content.prepare(
      "SELECT id FROM questions ORDER BY updated_at DESC LIMIT ?1",
    )?;
    let mut rows = stmt.query(rusqlite::params![limit])?;
    while let Some(row) = rows.next()? {
      qids.push(row.get(0)?);
    }
    Ok(())
  })();

  if let Err(e) = r {
    return db_error(e);
  }

  let mut questions: Vec<QuizQuestion> = Vec::new();
  for qid in &qids {
    let q: rusqlite::Result<(String, String, String)> = conn_content.query_row(
      "SELECT id, stem, type FROM questions WHERE id = ?1",
      rusqlite::params![qid],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    );
    let (id, stem, qtype) = match q {
      Ok(v) => v,
      Err(e) => return db_error(e),
    };

    let mut options: Vec<QuizQuestionOption> = Vec::new();
    let rr = (|| -> rusqlite::Result<()> {
      let mut stmt = conn_content.prepare(
        "SELECT opt_key, opt_text FROM question_options WHERE question_id = ?1 ORDER BY sort_order ASC",
      )?;
      let mut rows = stmt.query(rusqlite::params![qid])?;
      while let Some(row) = rows.next()? {
        options.push(QuizQuestionOption {
          key: row.get(0)?,
          text: row.get(1)?,
        });
      }
      Ok(())
    })();
    if let Err(e) = rr {
      return db_error(e);
    }

    questions.push(QuizQuestion {
      id,
      stem,
      r#type: qtype,
      options,
    });
  }

  drop(conn_content);
  let mut conn_user = match lock_user_db::<QuizStartSessionResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let session_id = uuid::Uuid::new_v4().to_string();
  let now = unix_ms();
  let mode = args.mode;
  let mode_for_db = mode.clone();
  let session_id_for_db = session_id.clone();
  let tx = match conn_user.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };
  let res = tx.execute(
    "INSERT INTO quiz_sessions(session_id, mode, started_at) VALUES (?1, ?2, ?3)",
    rusqlite::params![session_id_for_db, mode_for_db, now],
  );
  if let Err(e) = res {
    let _ = tx.rollback();
    return db_error(e);
  }

  let payload = serde_json::json!({
    "sessionId": session_id.clone(),
    "mode": mode,
    "startedAt": now
  });
  if let Err(e) = enqueue_outbox(
    &tx,
    &state.device_id,
    "quiz_session_started",
    "quiz_session",
    &session_id,
    &payload,
    now,
  ) {
    let _ = tx.rollback();
    return db_error(e);
  }
  if let Err(e) = tx.commit() {
    return db_error(e);
  }

  ok(QuizStartSessionResult { session_id, questions })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuizSubmitAnswerArgs {
  session_id: String,
  question_id: String,
  answer: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QuizSubmitAnswerResult {
  is_correct: bool,
  correct_answer: String,
  explanation: Option<String>,
  points_delta: i64,
  total_points: i64,
}

#[tauri::command]
fn quiz_submit_answer(
  state: tauri::State<'_, AppState>,
  args: QuizSubmitAnswerArgs,
) -> ApiResponse<QuizSubmitAnswerResult> {
  let session_id = args.session_id;
  let question_id = args.question_id;
  let answer = args.answer;
  let conn_content = match lock_content_db::<QuizSubmitAnswerResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let q: rusqlite::Result<(String, Option<String>)> = conn_content.query_row(
    "SELECT answer_key, analysis FROM questions WHERE id = ?1",
    rusqlite::params![question_id.clone()],
    |row| Ok((row.get(0)?, row.get(1)?)),
  );

  let (answer_key, analysis) = match q {
    Ok(v) => v,
    Err(rusqlite::Error::QueryReturnedNoRows) => {
      return ApiResponse {
        ok: false,
        data: None,
        error: Some(ApiError {
          code: "NOT_FOUND".to_string(),
          message: "question not found".to_string(),
          details: serde_json::Value::Null,
        }),
      };
    }
    Err(e) => return db_error(e),
  };

  let is_correct = answer.trim().eq_ignore_ascii_case(answer_key.trim());
  let points_delta: i64 = if is_correct { 10 } else { 0 };
  drop(conn_content);

  let mut conn_user = match lock_user_db::<QuizSubmitAnswerResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let now = unix_ms();
  let ans_id = uuid::Uuid::new_v4().to_string();
  let ans_id_for_db = ans_id.clone();
  let tx = match conn_user.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };

  let res = tx.execute(
    "INSERT INTO quiz_answers(id, session_id, question_id, answer, is_correct, answered_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    rusqlite::params![ans_id_for_db, session_id.clone(), question_id.clone(), answer.clone(), if is_correct { 1 } else { 0 }, now],
  );
  if let Err(e) = res {
    let _ = tx.rollback();
    return db_error(e);
  }

  let ans_payload = serde_json::json!({
    "answerId": ans_id.clone(),
    "sessionId": session_id,
    "questionId": question_id.clone(),
    "answer": answer,
    "isCorrect": is_correct,
    "answeredAt": now
  });
  if let Err(e) = enqueue_outbox(
    &tx,
    &state.device_id,
    "quiz_answer_submitted",
    "quiz_answer",
    ans_payload["answerId"].as_str().unwrap_or(""),
    &ans_payload,
    now,
  ) {
    let _ = tx.rollback();
    return db_error(e);
  }

  if points_delta != 0 {
    let pid = uuid::Uuid::new_v4().to_string();
    let pid_for_db = pid.clone();
    let res = tx.execute(
      "INSERT INTO points_ledger(id, reason, delta, occurred_at, ref_type, ref_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      rusqlite::params![pid_for_db, "QUIZ_CORRECT", points_delta, now, "question", question_id.clone()],
    );
    if let Err(e) = res {
      let _ = tx.rollback();
      return db_error(e);
    }

    let p_payload = serde_json::json!({
      "id": pid.clone(),
      "reason": "QUIZ_CORRECT",
      "delta": points_delta,
      "occurredAt": now,
      "refType": "question",
      "refId": question_id
    });
    if let Err(e) = enqueue_outbox(
      &tx,
      &state.device_id,
      "points_add",
      "points_ledger",
      p_payload["id"].as_str().unwrap_or(""),
      &p_payload,
      now,
    ) {
      let _ = tx.rollback();
      return db_error(e);
    }
  }

  if let Err(e) = tx.commit() {
    return db_error(e);
  }

  let total_points: i64 = match conn_user.query_row(
    "SELECT COALESCE(SUM(delta), 0) FROM points_ledger",
    [],
    |row| row.get(0),
  ) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  ok(QuizSubmitAnswerResult {
    is_correct,
    correct_answer: answer_key,
    explanation: analysis,
    points_delta,
    total_points,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct QuizProgress {
  total_points: i64,
}

#[tauri::command]
fn quiz_get_progress(state: tauri::State<'_, AppState>) -> ApiResponse<QuizProgress> {
  let conn_user = match lock_user_db::<QuizProgress>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let _ = conn_user.execute(
    "CREATE TABLE IF NOT EXISTS points_ledger (id TEXT PRIMARY KEY, reason TEXT NOT NULL, delta INTEGER NOT NULL, occurred_at INTEGER NOT NULL, ref_type TEXT, ref_id TEXT)",
    [],
  );

  let total_points: i64 = match conn_user.query_row(
    "SELECT COALESCE(SUM(delta), 0) FROM points_ledger",
    [],
    |row| row.get(0),
  ) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  ok(QuizProgress { total_points })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FavoriteItem {
  entity_type: String,
  entity_id: String,
  created_at: i64,
}

#[tauri::command]
fn user_list_favorites(
  state: tauri::State<'_, AppState>,
  args: UserListFavoritesArgs,
) -> ApiResponse<ListResponse<FavoriteItem>> {
  let UserListFavoritesArgs {
    entity_type,
    limit,
    offset,
  } = args;

  let conn = match lock_user_db::<ListResponse<FavoriteItem>>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let mut items: Vec<FavoriteItem> = Vec::new();
  let total: i64 = match conn
    .query_row("SELECT COUNT(1) FROM favorites", [], |row| row.get(0))
  {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  let sql = if entity_type.is_some() {
    "SELECT entity_type, entity_id, created_at FROM favorites WHERE entity_type = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
  } else {
    "SELECT entity_type, entity_id, created_at FROM favorites ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
  };

  let result = (|| -> rusqlite::Result<()> {
    if let Some(et) = entity_type.as_deref() {
      let mut stmt = conn.prepare(sql)?;
      let mut rows = stmt.query(rusqlite::params![et, limit, offset])?;
      while let Some(row) = rows.next()? {
        items.push(FavoriteItem {
          entity_type: row.get(0)?,
          entity_id: row.get(1)?,
          created_at: row.get(2)?,
        });
      }
    } else {
      let mut stmt = conn.prepare(sql)?;
      let mut rows = stmt.query(rusqlite::params![limit, offset])?;
      while let Some(row) = rows.next()? {
        items.push(FavoriteItem {
          entity_type: row.get(0)?,
          entity_id: row.get(1)?,
          created_at: row.get(2)?,
        });
      }
    }
    Ok(())
  })();

  if let Err(e) = result {
    return db_error(e);
  }

  ok(ListResponse { items, total })
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserListFavoritesArgs {
  entity_type: Option<String>,
  limit: i64,
  offset: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserSetFavoriteArgs {
  entity_type: String,
  entity_id: String,
  is_favorite: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSetFavoriteResult {
  is_favorite: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserIsFavoriteArgs {
  entity_type: String,
  entity_id: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UserIsFavoriteResult {
  is_favorite: bool,
}

#[tauri::command]
fn user_is_favorite(state: tauri::State<'_, AppState>, args: UserIsFavoriteArgs) -> ApiResponse<UserIsFavoriteResult> {
  use rusqlite::OptionalExtension;
  let conn = match lock_user_db::<UserIsFavoriteResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let v: Option<i64> = match conn
    .query_row(
      "SELECT 1 FROM favorites WHERE entity_type = ?1 AND entity_id = ?2",
      rusqlite::params![args.entity_type, args.entity_id],
      |row| row.get(0),
    )
    .optional()
  {
    Ok(x) => x,
    Err(e) => return db_error(e),
  };
  ok(UserIsFavoriteResult { is_favorite: v.is_some() })
}

#[tauri::command]
fn user_set_favorite(
  state: tauri::State<'_, AppState>,
  args: UserSetFavoriteArgs,
) -> ApiResponse<UserSetFavoriteResult> {
  let now = unix_ms();
  let entity_type = args.entity_type;
  let entity_id = args.entity_id;
  let is_favorite = args.is_favorite;
  let mut conn = match lock_user_db::<UserSetFavoriteResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let tx = match conn.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };

  let res: rusqlite::Result<()> = if is_favorite {
    tx.execute(
      "INSERT OR REPLACE INTO favorites(entity_type, entity_id, created_at) VALUES (?1, ?2, ?3)",
      rusqlite::params![entity_type, entity_id, now],
    )
    .map(|_| ())
  } else {
    tx.execute(
      "DELETE FROM favorites WHERE entity_type = ?1 AND entity_id = ?2",
      rusqlite::params![entity_type, entity_id],
    )
    .map(|_| ())
  };
  if let Err(e) = res {
    let _ = tx.rollback();
    return db_error(e);
  }

  let payload = serde_json::json!({ "isFavorite": is_favorite });
  if let Err(e) = enqueue_outbox(
    &tx,
    &state.device_id,
    "favorite_set",
    &entity_type,
    &entity_id,
    &payload,
    now,
  ) {
    let _ = tx.rollback();
    return db_error(e);
  }

  if let Err(e) = tx.commit() {
    return db_error(e);
  }

  ok(UserSetFavoriteResult {
    is_favorite,
  })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSettings {
  items: std::collections::BTreeMap<String, String>,
}

fn user_get_settings_inner(
  conn: &rusqlite::Connection,
) -> Result<std::collections::BTreeMap<String, String>, rusqlite::Error> {
  let mut items: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
  let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key ASC")?;
  let mut rows = stmt.query([])?;
  while let Some(row) = rows.next()? {
    let k: String = row.get(0)?;
    let v: String = row.get(1)?;
    items.insert(k, v);
  }
  Ok(items)
}

#[tauri::command]
fn user_get_settings(state: tauri::State<'_, AppState>) -> ApiResponse<UserSettings> {
  let conn = match lock_user_db::<UserSettings>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let items = match user_get_settings_inner(&conn) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  ok(UserSettings { items })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserUpdateSettingsArgs {
  patch: std::collections::BTreeMap<String, String>,
}

#[tauri::command]
fn user_update_settings(
  state: tauri::State<'_, AppState>,
  args: UserUpdateSettingsArgs,
) -> ApiResponse<UserSettings> {
  let now = unix_ms();
  let mut conn = match lock_user_db::<UserSettings>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let tx = match conn.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };

  let patch_for_outbox = args.patch.clone();
  for (k, v) in args.patch {
    let k = k.trim().to_string();
    if k.is_empty() {
      let _ = tx.rollback();
      return invalid_argument("settings key 不能为空");
    }
    let res = tx.execute(
      "INSERT OR REPLACE INTO settings(key, value, updated_at) VALUES (?1, ?2, ?3)",
      rusqlite::params![k, v, now],
    );
    if let Err(e) = res {
      let _ = tx.rollback();
      return db_error(e);
    }
  }

  let mut patch_obj = serde_json::Map::new();
  for (k, v) in patch_for_outbox {
    patch_obj.insert(k, serde_json::Value::String(v));
  }
  let payload = serde_json::json!({ "patch": serde_json::Value::Object(patch_obj) });
  if let Err(e) = enqueue_outbox(
    &tx,
    &state.device_id,
    "settings_patch",
    "settings",
    "settings",
    &payload,
    now,
  ) {
    let _ = tx.rollback();
    return db_error(e);
  }

  if let Err(e) = tx.commit() {
    return db_error(e);
  }

  let items = match user_get_settings_inner(&conn) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  ok(UserSettings { items })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UserProfile {
  nickname: String,
  avatar_color: String,
  avatar_image_data_url: String,
}

fn user_get_profile_inner(conn: &rusqlite::Connection) -> rusqlite::Result<UserProfile> {
  use rusqlite::OptionalExtension;
  let nickname: Option<String> = conn
    .query_row(
      "SELECT value FROM profile WHERE key = 'nickname'",
      [],
      |row| row.get(0),
    )
    .optional()?;
  let avatar_color: Option<String> = conn
    .query_row(
      "SELECT value FROM profile WHERE key = 'avatarColor'",
      [],
      |row| row.get(0),
    )
    .optional()?;
  let avatar_image_data_url: Option<String> = conn
    .query_row(
      "SELECT value FROM profile WHERE key = 'avatarImageDataUrl'",
      [],
      |row| row.get(0),
    )
    .optional()?;
  Ok(UserProfile {
    nickname: nickname.unwrap_or_default(),
    avatar_color: avatar_color.unwrap_or_default(),
    avatar_image_data_url: avatar_image_data_url.unwrap_or_default(),
  })
}

#[tauri::command]
fn user_get_profile(state: tauri::State<'_, AppState>) -> ApiResponse<UserProfile> {
  let conn = match lock_user_db::<UserProfile>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let p = match user_get_profile_inner(&conn) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };
  ok(p)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserUpdateProfileArgs {
  nickname: Option<String>,
  avatar_color: Option<String>,
  avatar_image_data_url: Option<String>,
}

#[tauri::command]
fn user_update_profile(state: tauri::State<'_, AppState>, args: UserUpdateProfileArgs) -> ApiResponse<UserProfile> {
  let now = unix_ms();
  let mut conn = match lock_user_db::<UserProfile>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let tx = match conn.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };

  let mut patch_obj = serde_json::Map::new();

  if let Some(v) = args.nickname {
    let vv = v.trim().to_string();
    let _ = tx.execute(
      "INSERT OR REPLACE INTO profile(key, value, updated_at) VALUES ('nickname', ?1, ?2)",
      rusqlite::params![vv.clone(), now],
    );
    patch_obj.insert("nickname".to_string(), serde_json::Value::String(vv));
  }
  if let Some(v) = args.avatar_color {
    let vv = v.trim().to_string();
    let _ = tx.execute(
      "INSERT OR REPLACE INTO profile(key, value, updated_at) VALUES ('avatarColor', ?1, ?2)",
      rusqlite::params![vv.clone(), now],
    );
    patch_obj.insert("avatarColor".to_string(), serde_json::Value::String(vv));
  }
  if let Some(v) = args.avatar_image_data_url {
    let vv = v.trim().to_string();
    let _ = tx.execute(
      "INSERT OR REPLACE INTO profile(key, value, updated_at) VALUES ('avatarImageDataUrl', ?1, ?2)",
      rusqlite::params![vv, now],
    );
  }

  if !patch_obj.is_empty() {
    let payload = serde_json::json!({ "patch": serde_json::Value::Object(patch_obj) });
    if let Err(e) = enqueue_outbox(&tx, &state.device_id, "profile_patch", "profile", "profile", &payload, now) {
      let _ = tx.rollback();
      return db_error(e);
    }
  }

  if let Err(e) = tx.commit() {
    return db_error(e);
  }

  let p = match user_get_profile_inner(&conn) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };
  ok(p)
}

const SETTINGS_SYNC_BASE_URL: &str = "syncBaseUrl";
const SETTINGS_SYNC_ACCESS_TOKEN: &str = "syncAccessToken";
const SETTINGS_SYNC_USERNAME: &str = "syncUsername";
const DEFAULT_SYNC_BASE_URL: &str = "http://127.0.0.1:8788";

fn enqueue_outbox(
  conn: &rusqlite::Connection,
  device_id: &str,
  event_type: &str,
  entity_type: &str,
  entity_id: &str,
  payload: &serde_json::Value,
  occurred_at: i64,
) -> rusqlite::Result<String> {
  let event_id = uuid::Uuid::new_v4().to_string();
  let payload_json = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
  conn.execute(
    "INSERT INTO sync_outbox(event_id, device_id, event_type, entity_type, entity_id, payload_json, occurred_at, sent_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
    rusqlite::params![
      event_id,
      device_id,
      event_type,
      entity_type,
      entity_id,
      payload_json,
      occurred_at
    ],
  )?;
  Ok(event_id)
}

fn ensure_device_row(conn: &rusqlite::Connection, device_id: &str, now: i64) -> rusqlite::Result<()> {
  conn.execute(
    "INSERT OR IGNORE INTO device(device_id, created_at, last_seen_at) VALUES (?1, ?2, ?2)",
    rusqlite::params![device_id, now],
  )?;
  conn.execute(
    "UPDATE device SET last_seen_at = ?2 WHERE device_id = ?1",
    rusqlite::params![device_id, now],
  )?;
  Ok(())
}

fn get_setting(conn: &rusqlite::Connection, key: &str) -> rusqlite::Result<Option<(String, i64)>> {
  use rusqlite::OptionalExtension;
  conn
    .query_row(
      "SELECT value, updated_at FROM settings WHERE key = ?1",
      rusqlite::params![key],
      |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

fn set_setting(
  conn: &rusqlite::Connection,
  key: &str,
  value: &str,
  now: i64,
) -> rusqlite::Result<()> {
  conn.execute(
    "INSERT OR REPLACE INTO settings(key, value, updated_at) VALUES (?1, ?2, ?3)",
    rusqlite::params![key, value, now],
  )?;
  Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthState {
  is_logged_in: bool,
  base_url: Option<String>,
  username: Option<String>,
}

#[tauri::command]
fn auth_get_state(state: tauri::State<'_, AppState>) -> ApiResponse<AuthState> {
  let conn = match lock_user_db::<AuthState>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };

  let base_url = match get_setting(&conn, SETTINGS_SYNC_BASE_URL) {
    Ok(v) => Some(v.map(|x| x.0).unwrap_or_else(|| DEFAULT_SYNC_BASE_URL.to_string())),
    Err(e) => return db_error(e),
  };
  let token = match get_setting(&conn, SETTINGS_SYNC_ACCESS_TOKEN) {
    Ok(v) => v.map(|x| x.0),
    Err(e) => return db_error(e),
  };
  let username = match get_setting(&conn, SETTINGS_SYNC_USERNAME) {
    Ok(v) => v.map(|x| x.0),
    Err(e) => return db_error(e),
  };

  ok(AuthState {
    is_logged_in: token.as_ref().is_some_and(|t| !t.trim().is_empty()),
    base_url,
    username,
  })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthLoginArgs {
  base_url: Option<String>,
  username: Option<String>,
  password: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthLoginResponse {
  access_token: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthLoginResult {
  access_token: String,
}

#[tauri::command]
fn auth_login(state: tauri::State<'_, AppState>, args: AuthLoginArgs) -> ApiResponse<AuthLoginResult> {
  let conn_for_base = match lock_user_db::<AuthLoginResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let mut base = args
    .base_url
    .or_else(|| get_setting(&conn_for_base, SETTINGS_SYNC_BASE_URL).ok().flatten().map(|x| x.0))
    .unwrap_or_else(|| DEFAULT_SYNC_BASE_URL.to_string());
  base = base.trim().trim_end_matches('/').to_string();
  drop(conn_for_base);

  let username_for_server = args.username.clone().unwrap_or_else(|| "user".to_string());
  let url = format!("{}/v1/auth/login", base);
  let payload = serde_json::json!({
    "username": username_for_server,
    "password": args.password.unwrap_or_default(),
    "deviceId": state.device_id.clone()
  });

  let resp = match http_agent()
    .post(&url)
    .header("Content-Type", "application/json")
    .send_json(payload)
  {
    Ok(r) => r,
    Err(e) => match e {
      ureq::Error::StatusCode(401) => return invalid_argument("用户名或密码错误"),
      _ => return invalid_argument(&format!("登录失败：{}", e)),
    },
  };

  let text = match resp.into_body().read_to_string() {
    Ok(s) => s,
    Err(e) => return invalid_argument(&format!("登录响应读取失败：{}", e)),
  };
  let parsed: AuthLoginResponse = match serde_json::from_str(&text) {
    Ok(v) => v,
    Err(e) => return invalid_argument(&format!("登录响应解析失败：{}", e)),
  };

  let now = unix_ms();
  let mut conn = match lock_user_db::<AuthLoginResult>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let tx = match conn.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };
  if let Err(e) = set_setting(&tx, SETTINGS_SYNC_BASE_URL, &base, now) {
    let _ = tx.rollback();
    return db_error(e);
  }
  if let Err(e) = set_setting(&tx, SETTINGS_SYNC_ACCESS_TOKEN, &parsed.access_token, now) {
    let _ = tx.rollback();
    return db_error(e);
  }
  if let Err(e) = set_setting(&tx, SETTINGS_SYNC_USERNAME, &username_for_server, now) {
    let _ = tx.rollback();
    return db_error(e);
  }
  if let Err(e) = tx.commit() {
    return db_error(e);
  }

  ok(AuthLoginResult {
    access_token: parsed.access_token,
  })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthSetServerArgs {
  base_url: String,
}

#[tauri::command]
fn auth_set_server(state: tauri::State<'_, AppState>, args: AuthSetServerArgs) -> ApiResponse<AuthState> {
  let now = unix_ms();
  let v = args.base_url.trim().trim_end_matches('/').to_string();
  let mut conn = match lock_user_db::<AuthState>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let tx = match conn.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };
  if v.is_empty() {
    if let Err(e) = tx.execute(
      "DELETE FROM settings WHERE key = ?1",
      rusqlite::params![SETTINGS_SYNC_BASE_URL],
    ) {
      let _ = tx.rollback();
      return db_error(e);
    }
  } else if let Err(e) = set_setting(&tx, SETTINGS_SYNC_BASE_URL, &v, now) {
    let _ = tx.rollback();
    return db_error(e);
  }
  if let Err(e) = tx.commit() {
    return db_error(e);
  }
  drop(conn);
  auth_get_state(state)
}

#[tauri::command]
fn auth_logout(state: tauri::State<'_, AppState>) -> ApiResponse<bool> {
  let mut conn = match lock_user_db::<bool>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let tx = match conn.transaction() {
    Ok(t) => t,
    Err(e) => return db_error(e),
  };
  if let Err(e) = tx.execute(
    "DELETE FROM settings WHERE key = ?1",
    rusqlite::params![SETTINGS_SYNC_ACCESS_TOKEN],
  ) {
    let _ = tx.rollback();
    return db_error(e);
  }
  let _ = tx.execute(
    "DELETE FROM settings WHERE key = ?1",
    rusqlite::params![SETTINGS_SYNC_USERNAME],
  );
  if let Err(e) = tx.execute(
    "UPDATE device SET sync_cursor = NULL, last_sync_at = NULL WHERE device_id = ?1",
    rusqlite::params![state.device_id.clone()],
  ) {
    let _ = tx.rollback();
    return db_error(e);
  }
  if let Err(e) = tx.commit() {
    return db_error(e);
  }
  ok(true)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncState {
  pending_count: i64,
  last_sync_at: Option<i64>,
  cursor: Option<String>,
}

#[tauri::command]
fn sync_get_state(state: tauri::State<'_, AppState>) -> ApiResponse<SyncState> {
  let now = unix_ms();
  let conn = match lock_user_db::<SyncState>(&state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  if let Err(e) = ensure_device_row(&conn, &state.device_id, now) {
    return db_error(e);
  }

  let pending_count: i64 = match conn.query_row(
    "SELECT COUNT(1) FROM sync_outbox WHERE sent_at IS NULL",
    [],
    |row| row.get(0),
  ) {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  let device_row: rusqlite::Result<(Option<i64>, Option<String>)> = conn.query_row(
    "SELECT last_sync_at, sync_cursor FROM device WHERE device_id = ?1",
    rusqlite::params![state.device_id.clone()],
    |row| Ok((row.get(0)?, row.get(1)?)),
  );
  let (last_sync_at, cursor) = match device_row {
    Ok(v) => v,
    Err(e) => return db_error(e),
  };

  ok(SyncState {
    pending_count,
    last_sync_at,
    cursor,
  })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRunArgs {
  mode: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncRunResult {
  pushed: i64,
  pulled: i64,
  pending_count: i64,
  last_sync_at: Option<i64>,
  cursor: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SyncEventWire {
  event_id: String,
  device_id: String,
  event_type: String,
  entity_type: String,
  entity_id: String,
  payload: serde_json::Value,
  occurred_at: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncPushRequest {
  events: Vec<SyncEventWire>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPushResponse {
  acked: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPullResponse {
  events: Vec<SyncEventWire>,
  next_cursor: String,
}

fn apply_sync_event(
  conn: &rusqlite::Connection,
  ev: &SyncEventWire,
  applied_at: i64,
) -> rusqlite::Result<bool> {
  use rusqlite::OptionalExtension;
  let already: Option<i64> = conn
    .query_row(
      "SELECT 1 FROM sync_applied WHERE event_id = ?1",
      rusqlite::params![ev.event_id.clone()],
      |row| row.get(0),
    )
    .optional()?;
  if already.is_some() {
    return Ok(false);
  }

  match ev.event_type.as_str() {
    "favorite_set" => {
      let is_favorite = ev
        .payload
        .get("isFavorite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
      if is_favorite {
        conn.execute(
          "INSERT OR REPLACE INTO favorites(entity_type, entity_id, created_at) VALUES (?1, ?2, ?3)",
          rusqlite::params![ev.entity_type.clone(), ev.entity_id.clone(), ev.occurred_at],
        )?;
      } else {
        conn.execute(
          "DELETE FROM favorites WHERE entity_type = ?1 AND entity_id = ?2",
          rusqlite::params![ev.entity_type.clone(), ev.entity_id.clone()],
        )?;
      }
    }
    "points_add" => {
      let id = ev
        .payload
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(ev.entity_id.as_str())
        .to_string();
      let reason = ev
        .payload
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("SYNC")
        .to_string();
      let delta = ev.payload.get("delta").and_then(|v| v.as_i64()).unwrap_or(0);
      let occurred_at = ev
        .payload
        .get("occurredAt")
        .and_then(|v| v.as_i64())
        .unwrap_or(ev.occurred_at);
      let ref_type = ev.payload.get("refType").and_then(|v| v.as_str()).map(|s| s.to_string());
      let ref_id = ev.payload.get("refId").and_then(|v| v.as_str()).map(|s| s.to_string());
      conn.execute(
        "INSERT OR IGNORE INTO points_ledger(id, reason, delta, occurred_at, ref_type, ref_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, reason, delta, occurred_at, ref_type, ref_id],
      )?;
    }
    "settings_patch" => {
      if let Some(patch) = ev.payload.get("patch").and_then(|v| v.as_object()) {
        for (k, vv) in patch {
          let Some(vs) = vv.as_str() else { continue };
          let existing: Option<i64> = conn
            .query_row(
              "SELECT updated_at FROM settings WHERE key = ?1",
              rusqlite::params![k.clone()],
              |row| row.get(0),
            )
            .optional()?;
          if existing.is_none() || existing.unwrap_or(0) <= ev.occurred_at {
            conn.execute(
              "INSERT OR REPLACE INTO settings(key, value, updated_at) VALUES (?1, ?2, ?3)",
              rusqlite::params![k.clone(), vs.to_string(), ev.occurred_at],
            )?;
          }
        }
      }
    }
    "profile_patch" => {
      if let Some(patch) = ev.payload.get("patch").and_then(|v| v.as_object()) {
        for (k, vv) in patch {
          let Some(vs) = vv.as_str() else { continue };
          let existing: Option<i64> = conn
            .query_row(
              "SELECT updated_at FROM profile WHERE key = ?1",
              rusqlite::params![k.clone()],
              |row| row.get(0),
            )
            .optional()?;
          if existing.is_none() || existing.unwrap_or(0) <= ev.occurred_at {
            conn.execute(
              "INSERT OR REPLACE INTO profile(key, value, updated_at) VALUES (?1, ?2, ?3)",
              rusqlite::params![k.clone(), vs.to_string(), ev.occurred_at],
            )?;
          }
        }
      }
    }
    "quiz_session_started" => {
      let sid = ev
        .payload
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or(ev.entity_id.as_str())
        .to_string();
      let mode = ev
        .payload
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("daily")
        .to_string();
      let started_at = ev
        .payload
        .get("startedAt")
        .and_then(|v| v.as_i64())
        .unwrap_or(ev.occurred_at);
      conn.execute(
        "INSERT OR IGNORE INTO quiz_sessions(session_id, mode, started_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![sid, mode, started_at],
      )?;
    }
    "quiz_answer_submitted" => {
      let aid = ev
        .payload
        .get("answerId")
        .and_then(|v| v.as_str())
        .unwrap_or(ev.entity_id.as_str())
        .to_string();
      let sid = ev
        .payload
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
      let qid = ev
        .payload
        .get("questionId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
      let ans = ev
        .payload
        .get("answer")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
      let is_correct = ev
        .payload
        .get("isCorrect")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
      let answered_at = ev
        .payload
        .get("answeredAt")
        .and_then(|v| v.as_i64())
        .unwrap_or(ev.occurred_at);
      conn.execute(
        "INSERT OR IGNORE INTO quiz_answers(id, session_id, question_id, answer, is_correct, answered_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![aid, sid, qid, ans, if is_correct { 1 } else { 0 }, answered_at],
      )?;
    }
    _ => {}
  }

  conn.execute(
    "INSERT OR IGNORE INTO sync_applied(event_id, applied_at) VALUES (?1, ?2)",
    rusqlite::params![ev.event_id.clone(), applied_at],
  )?;

  Ok(true)
}

#[tauri::command]
async fn sync_run(
  state: tauri::State<'_, AppState>,
  args: SyncRunArgs,
) -> Result<ApiResponse<SyncRunResult>, ()> {
  let app = state.inner().clone();
  let r = match tauri::async_runtime::spawn_blocking(move || sync_run_inner(&app, args)).await {
    Ok(v) => v,
    Err(e) => ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "INTERNAL".to_string(),
        message: format!("sync_run join failed: {}", e),
        details: serde_json::Value::Null,
      }),
    },
  };
  Ok(r)
}

fn lock_user_db_app<'a, T>(state: &'a AppState) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, ApiResponse<T>>
where
  T: serde::Serialize,
{
  state.user_db.lock().map_err(|_| {
    ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "INTERNAL".to_string(),
        message: "DB lock poisoned".to_string(),
        details: serde_json::Value::Null,
      }),
    }
  })
}

fn sync_run_inner(state: &AppState, args: SyncRunArgs) -> ApiResponse<SyncRunResult> {
  let now = unix_ms();
  let mode = args.mode.trim().to_lowercase();
  if mode != "push" && mode != "pull" && mode != "both" {
    return invalid_argument("mode 必须是 push / pull / both");
  }

  let conn = match lock_user_db_app::<SyncRunResult>(state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  if let Err(e) = ensure_device_row(&conn, &state.device_id, now) {
    return db_error(e);
  }

  let base_url = match get_setting(&conn, SETTINGS_SYNC_BASE_URL) {
    Ok(v) => v.map(|x| x.0).unwrap_or_else(|| DEFAULT_SYNC_BASE_URL.to_string()),
    Err(e) => return db_error(e),
  };
  let token = match get_setting(&conn, SETTINGS_SYNC_ACCESS_TOKEN) {
    Ok(v) => v.map(|x| x.0).unwrap_or_default(),
    Err(e) => return db_error(e),
  };
  if token.trim().is_empty() {
    return invalid_argument("请先登录");
  }
  drop(conn);

  let mut pushed: i64 = 0;
  let mut pulled: i64 = 0;
  let mut last_sync_at: Option<i64> = None;
  let mut cursor: Option<String> = None;

  if mode == "push" || mode == "both" {
    let mut batch_guard: i32 = 0;
    loop {
      batch_guard += 1;
      if batch_guard > 20 {
        break;
      }

      let conn = match lock_user_db_app::<SyncRunResult>(state) {
        Ok(c) => c,
        Err(e) => return e,
      };
      let mut events: Vec<SyncEventWire> = Vec::new();
      let res = (|| -> rusqlite::Result<()> {
        let mut stmt = conn.prepare(
          "SELECT event_id, device_id, event_type, entity_type, entity_id, payload_json, occurred_at FROM sync_outbox WHERE sent_at IS NULL ORDER BY occurred_at ASC LIMIT 200",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
          let payload_json: String = row.get(5)?;
          let payload: serde_json::Value =
            serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
          events.push(SyncEventWire {
            event_id: row.get(0)?,
            device_id: row.get(1)?,
            event_type: row.get(2)?,
            entity_type: row.get(3)?,
            entity_id: row.get(4)?,
            payload,
            occurred_at: row.get(6)?,
          });
        }
        Ok(())
      })();
      if let Err(e) = res {
        return db_error(e);
      }
      drop(conn);

      if events.is_empty() {
        break;
      }

      let url = format!("{}/v1/sync/push", base_url.trim_end_matches('/'));
      let req = SyncPushRequest { events };
      let resp = match http_agent()
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", token))
        .send_json(req)
      {
        Ok(r) => r,
        Err(e) => match e {
          ureq::Error::StatusCode(401) => {
            let _ = clear_sync_auth(state, now);
            return invalid_argument("登录已失效，请重新登录");
          }
          _ => return invalid_argument(&format!("push 失败：{}", e)),
        },
      };

      let text = match resp.into_body().read_to_string() {
        Ok(s) => s,
        Err(e) => return invalid_argument(&format!("push 响应读取失败：{}", e)),
      };
      let parsed: SyncPushResponse = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return invalid_argument(&format!("push 响应解析失败：{}", e)),
      };

      let mut conn = match lock_user_db_app::<SyncRunResult>(state) {
        Ok(c) => c,
        Err(e) => return e,
      };
      let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return db_error(e),
      };
      for eid in &parsed.acked {
        let _ = tx.execute(
          "UPDATE sync_outbox SET sent_at = ?2 WHERE event_id = ?1 AND sent_at IS NULL",
          rusqlite::params![eid.clone(), now],
        );
      }
      let _ = tx.execute(
        "UPDATE device SET last_sync_at = ?2 WHERE device_id = ?1",
        rusqlite::params![state.device_id.clone(), now],
      );
      if let Err(e) = tx.commit() {
        return db_error(e);
      }
      pushed += parsed.acked.len() as i64;
      last_sync_at = Some(now);
      if parsed.acked.is_empty() {
        break;
      }
    }
  }

  if mode == "pull" || mode == "both" {
    let conn = match lock_user_db_app::<SyncRunResult>(state) {
      Ok(c) => c,
      Err(e) => return e,
    };
    let device_row: rusqlite::Result<Option<String>> = conn.query_row(
      "SELECT sync_cursor FROM device WHERE device_id = ?1",
      rusqlite::params![state.device_id.clone()],
      |row| row.get(0),
    );
    let cursor_local = match device_row {
      Ok(v) => v.unwrap_or_else(|| "0".to_string()),
      Err(_) => "0".to_string(),
    };
    drop(conn);

    let mut cursor_local = cursor_local;
    let mut batch_guard: i32 = 0;
    loop {
      batch_guard += 1;
      if batch_guard > 20 {
        break;
      }
      let url = format!(
        "{}/v1/sync/pull?cursor={}",
        base_url.trim_end_matches('/'),
        cursor_local
      );
      let resp = match http_agent()
        .get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .call()
      {
        Ok(r) => r,
        Err(e) => match e {
          ureq::Error::StatusCode(401) => {
            let _ = clear_sync_auth(state, now);
            return invalid_argument("登录已失效，请重新登录");
          }
          _ => return invalid_argument(&format!("pull 失败：{}", e)),
        },
      };

      let text = match resp.into_body().read_to_string() {
        Ok(s) => s,
        Err(e) => return invalid_argument(&format!("pull 响应读取失败：{}", e)),
      };
      let parsed: SyncPullResponse = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return invalid_argument(&format!("pull 响应解析失败：{}", e)),
      };

      let mut conn = match lock_user_db_app::<SyncRunResult>(state) {
        Ok(c) => c,
        Err(e) => return e,
      };
      let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => return db_error(e),
      };
      for ev in &parsed.events {
        if let Err(e) = apply_sync_event(&tx, ev, now) {
          let _ = tx.rollback();
          return db_error(e);
        }
      }
      if let Err(e) = tx.execute(
        "UPDATE device SET last_sync_at = ?2, sync_cursor = ?3 WHERE device_id = ?1",
        rusqlite::params![state.device_id.clone(), now, parsed.next_cursor.clone()],
      ) {
        let _ = tx.rollback();
        return db_error(e);
      }
      if let Err(e) = tx.commit() {
        return db_error(e);
      }

      pulled += parsed.events.len() as i64;
      cursor = Some(parsed.next_cursor.clone());
      last_sync_at = Some(now);

      if parsed.events.is_empty() {
        break;
      }
      if parsed.next_cursor.trim() == cursor_local.trim() {
        break;
      }
      cursor_local = parsed.next_cursor.clone();
    }
  }

  let conn = match lock_user_db_app::<SyncRunResult>(state) {
    Ok(c) => c,
    Err(e) => return e,
  };
  let pending_count: i64 = conn
    .query_row("SELECT COUNT(1) FROM sync_outbox WHERE sent_at IS NULL", [], |row| row.get(0))
    .unwrap_or(0);
  let device_row: rusqlite::Result<(Option<i64>, Option<String>)> = conn.query_row(
    "SELECT last_sync_at, sync_cursor FROM device WHERE device_id = ?1",
    rusqlite::params![state.device_id.clone()],
    |row| Ok((row.get(0)?, row.get(1)?)),
  );
  let (ls, cur) = device_row.unwrap_or((None, None));
  if cursor.is_none() {
    cursor = cur;
  }
  if last_sync_at.is_none() {
    last_sync_at = ls;
  }

  ok(SyncRunResult {
    pushed,
    pulled,
    pending_count,
    last_sync_at,
    cursor,
  })
}

fn clear_sync_auth(state: &AppState, now: i64) -> rusqlite::Result<()> {
  let mut conn = match state.user_db.lock() {
    Ok(c) => c,
    Err(_) => return Ok(()),
  };
  let tx = conn.transaction()?;
  let _ = tx.execute(
    "DELETE FROM settings WHERE key = ?1",
    rusqlite::params![SETTINGS_SYNC_ACCESS_TOKEN],
  );
  let _ = tx.execute(
    "DELETE FROM settings WHERE key = ?1",
    rusqlite::params![SETTINGS_SYNC_USERNAME],
  );
  let _ = tx.execute(
    "UPDATE device SET sync_cursor = NULL, last_sync_at = NULL, last_seen_at = ?2 WHERE device_id = ?1",
    rusqlite::params![state.device_id.clone(), now],
  );
  tx.commit()?;
  Ok(())
}

fn unix_ms() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  let d = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
  d.as_millis() as i64
}

fn lock_user_db<'a, T>(
  state: &'a tauri::State<'a, AppState>,
) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, ApiResponse<T>>
where
  T: serde::Serialize,
{
  state.user_db.lock().map_err(|_| {
    ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "INTERNAL".to_string(),
        message: "DB lock poisoned".to_string(),
        details: serde_json::Value::Null,
      }),
    }
  })
}

fn lock_content_db<'a, T>(
  state: &'a tauri::State<'a, AppState>,
) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, ApiResponse<T>>
where
  T: serde::Serialize,
{
  state.content_db.lock().map_err(|_| {
    ApiResponse {
      ok: false,
      data: None,
      error: Some(ApiError {
        code: "INTERNAL".to_string(),
        message: "DB lock poisoned".to_string(),
        details: serde_json::Value::Null,
      }),
    }
  })
}

fn db_error<T: serde::Serialize>(e: rusqlite::Error) -> ApiResponse<T> {
  ApiResponse {
    ok: false,
    data: None,
    error: Some(ApiError {
      code: "DB_ERROR".to_string(),
      message: e.to_string(),
      details: serde_json::Value::Null,
    }),
  }
}

fn io_error<T: serde::Serialize>(e: std::io::Error) -> ApiResponse<T> {
  ApiResponse {
    ok: false,
    data: None,
    error: Some(ApiError {
      code: "IO_ERROR".to_string(),
      message: e.to_string(),
      details: serde_json::Value::Null,
    }),
  }
}

fn invalid_argument<T: serde::Serialize>(message: &str) -> ApiResponse<T> {
  ApiResponse {
    ok: false,
    data: None,
    error: Some(ApiError {
      code: "INVALID_ARGUMENT".to_string(),
      message: message.to_string(),
      details: serde_json::Value::Null,
    }),
  }
}
