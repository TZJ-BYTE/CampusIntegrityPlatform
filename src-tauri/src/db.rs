use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

pub const USER_SCHEMA_VERSION: i64 = 2;
pub const CONTENT_SCHEMA_VERSION: i64 = 1;

pub fn open_user_db(path: &Path) -> rusqlite::Result<Connection> {
  let conn = Connection::open(path)?;
  conn.execute_batch(
    r#"
    PRAGMA journal_mode = WAL;
    PRAGMA foreign_keys = ON;
  "#,
  )?;
  migrate_user_db(&conn)?;
  Ok(conn)
}

pub fn open_content_db(path: &Path) -> rusqlite::Result<Connection> {
  let conn = Connection::open(path)?;
  conn.execute_batch(
    r#"
    PRAGMA journal_mode = WAL;
    PRAGMA foreign_keys = ON;
  "#,
  )?;
  migrate_content_db(&conn)?;
  Ok(conn)
}

fn migrate_user_db(conn: &Connection) -> rusqlite::Result<()> {
  conn.execute_batch(
    r#"
    CREATE TABLE IF NOT EXISTS schema_meta (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS favorites (
      entity_type TEXT NOT NULL,
      entity_id TEXT NOT NULL,
      created_at INTEGER NOT NULL,
      PRIMARY KEY (entity_type, entity_id)
    );

    CREATE INDEX IF NOT EXISTS idx_favorites_created_at
      ON favorites(created_at);

    CREATE TABLE IF NOT EXISTS quiz_sessions (
      session_id TEXT PRIMARY KEY,
      mode TEXT NOT NULL,
      started_at INTEGER NOT NULL,
      finished_at INTEGER,
      summary_json TEXT
    );

    CREATE TABLE IF NOT EXISTS quiz_answers (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL,
      question_id TEXT NOT NULL,
      answer TEXT NOT NULL,
      is_correct INTEGER NOT NULL,
      answered_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_quiz_answers_session
      ON quiz_answers(session_id, answered_at);

    CREATE TABLE IF NOT EXISTS points_ledger (
      id TEXT PRIMARY KEY,
      reason TEXT NOT NULL,
      delta INTEGER NOT NULL,
      occurred_at INTEGER NOT NULL,
      ref_type TEXT,
      ref_id TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_points_ledger_time
      ON points_ledger(occurred_at);

    CREATE TABLE IF NOT EXISTS settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
  "#,
  )?;

  let current: Option<i64> = conn
    .query_row(
      "SELECT CAST(value AS INTEGER) FROM schema_meta WHERE key = 'user_schema_version'",
      [],
      |row| row.get(0),
    )
    .optional()?;

  if current.is_none() {
    conn.execute(
      "INSERT INTO schema_meta(key, value) VALUES('user_schema_version', ?1)",
      params![USER_SCHEMA_VERSION.to_string()],
    )?;
  }

  Ok(())
}

fn migrate_content_db(conn: &Connection) -> rusqlite::Result<()> {
  conn.execute_batch(
    r#"
    CREATE TABLE IF NOT EXISTS meta (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS venues (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      type TEXT NOT NULL,
      location TEXT,
      description TEXT,
      contact TEXT,
      open_hours TEXT,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_venues_type
      ON venues(type);

    CREATE INDEX IF NOT EXISTS idx_venues_updated_at
      ON venues(updated_at);

    CREATE TABLE IF NOT EXISTS cases (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      scene TEXT NOT NULL,
      summary TEXT NOT NULL,
      body TEXT NOT NULL,
      violation TEXT,
      correct_action TEXT,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_cases_scene
      ON cases(scene);

    CREATE INDEX IF NOT EXISTS idx_cases_updated_at
      ON cases(updated_at);

    CREATE VIRTUAL TABLE IF NOT EXISTS cases_fts USING fts5(
      id UNINDEXED,
      title,
      summary,
      body
    );

    CREATE TRIGGER IF NOT EXISTS trg_cases_ai AFTER INSERT ON cases BEGIN
      INSERT INTO cases_fts(rowid, id, title, summary, body)
      VALUES (new.rowid, new.id, new.title, new.summary, new.body);
    END;

    CREATE TRIGGER IF NOT EXISTS trg_cases_ad AFTER DELETE ON cases BEGIN
      DELETE FROM cases_fts WHERE rowid = old.rowid;
    END;

    CREATE TRIGGER IF NOT EXISTS trg_cases_au AFTER UPDATE ON cases BEGIN
      DELETE FROM cases_fts WHERE rowid = old.rowid;
      INSERT INTO cases_fts(rowid, id, title, summary, body)
      VALUES (new.rowid, new.id, new.title, new.summary, new.body);
    END;

    INSERT INTO cases_fts(rowid, id, title, summary, body)
    SELECT rowid, id, title, summary, body FROM cases
    WHERE NOT EXISTS (SELECT 1 FROM cases_fts LIMIT 1);

    CREATE TABLE IF NOT EXISTS regulations (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      level TEXT NOT NULL,
      source TEXT,
      published_at TEXT,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_regulations_level
      ON regulations(level);

    CREATE TABLE IF NOT EXISTS regulation_sections (
      id TEXT PRIMARY KEY,
      regulation_id TEXT NOT NULL,
      chapter TEXT,
      article_no TEXT,
      title TEXT,
      body TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_regulation_sections_regulation
      ON regulation_sections(regulation_id, updated_at);

    CREATE TABLE IF NOT EXISTS stories (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      body TEXT NOT NULL,
      source TEXT,
      day_of_year INTEGER,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_stories_day
      ON stories(day_of_year);

    CREATE VIRTUAL TABLE IF NOT EXISTS stories_fts USING fts5(
      id UNINDEXED,
      title,
      body,
      source
    );

    CREATE TRIGGER IF NOT EXISTS trg_stories_ai AFTER INSERT ON stories BEGIN
      INSERT INTO stories_fts(rowid, id, title, body, source)
      VALUES (new.rowid, new.id, new.title, new.body, new.source);
    END;

    CREATE TRIGGER IF NOT EXISTS trg_stories_ad AFTER DELETE ON stories BEGIN
      DELETE FROM stories_fts WHERE rowid = old.rowid;
    END;

    CREATE TRIGGER IF NOT EXISTS trg_stories_au AFTER UPDATE ON stories BEGIN
      DELETE FROM stories_fts WHERE rowid = old.rowid;
      INSERT INTO stories_fts(rowid, id, title, body, source)
      VALUES (new.rowid, new.id, new.title, new.body, new.source);
    END;

    INSERT INTO stories_fts(rowid, id, title, body, source)
    SELECT rowid, id, title, body, source FROM stories
    WHERE NOT EXISTS (SELECT 1 FROM stories_fts LIMIT 1);

    CREATE TABLE IF NOT EXISTS questions (
      id TEXT PRIMARY KEY,
      module TEXT NOT NULL,
      stem TEXT NOT NULL,
      type TEXT NOT NULL,
      difficulty INTEGER NOT NULL DEFAULT 1,
      answer_key TEXT NOT NULL,
      analysis TEXT,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_questions_module
      ON questions(module);

    CREATE TABLE IF NOT EXISTS question_options (
      id TEXT PRIMARY KEY,
      question_id TEXT NOT NULL,
      opt_key TEXT NOT NULL,
      opt_text TEXT NOT NULL,
      sort_order INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX IF NOT EXISTS idx_question_options_qid
      ON question_options(question_id, sort_order);
  "#,
  )?;

  let schema: Option<i64> = conn
    .query_row(
      "SELECT CAST(value AS INTEGER) FROM meta WHERE key = 'content_schema_version'",
      [],
      |row| row.get(0),
    )
    .optional()?;

  if schema.is_none() {
    conn.execute(
      "INSERT INTO meta(key, value) VALUES('content_schema_version', ?1)",
      params![CONTENT_SCHEMA_VERSION.to_string()],
    )?;
  }

  let version: Option<String> = conn
    .query_row(
      "SELECT value FROM meta WHERE key = 'content_version'",
      [],
      |row| row.get(0),
    )
    .optional()?;

  if version.is_none() {
    conn.execute(
      "INSERT INTO meta(key, value) VALUES('content_version', ?1)",
      params!["0".to_string()],
    )?;
  }

  let has_any_venue: bool = conn
    .query_row("SELECT EXISTS(SELECT 1 FROM venues LIMIT 1)", [], |row| row.get(0))
    .unwrap_or(false);

  if !has_any_venue {
    let now = unix_ms();
    conn.execute(
      "INSERT INTO venues(id, name, type, location, description, contact, open_hours, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        "venue_demo",
        "廉洁文化园（示例）",
        "文化展示",
        "图书馆东侧广场",
        "示例数据：用于验证离线内容库读取与列表展示。",
        "张老师 138****1234",
        "周一至周五 8:00-17:30",
        now
      ],
    )?;
  }

  let has_any_case: bool = conn
    .query_row("SELECT EXISTS(SELECT 1 FROM cases LIMIT 1)", [], |row| row.get(0))
    .unwrap_or(false);

  if !has_any_case {
    let now = unix_ms();
    conn.execute(
      "INSERT INTO cases(id, title, scene, summary, body, violation, correct_action, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        "case_demo",
        "班费收支不公开（示例）",
        "班级管理",
        "班费使用与报销不透明，容易引发误解甚至违纪风险。",
        "示例情景：班级收取班费后，班干部未按月公示收支明细，报销票据也未留存。",
        "收支不公开、票据不齐全、挪用风险",
        "建立班费台账与月度公示制度；大额支出需集体讨论并留存票据。",
        now
      ],
    )?;
  }

  let has_any_reg: bool = conn
    .query_row(
      "SELECT EXISTS(SELECT 1 FROM regulations LIMIT 1)",
      [],
      |row| row.get(0),
    )
    .unwrap_or(false);

  if !has_any_reg {
    let now = unix_ms();
    conn.execute(
      "INSERT INTO regulations(id, title, level, source, published_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![
        "reg_demo",
        "校园学生行为规范（节选·示例）",
        "校内制度",
        "校纪校规（示例）",
        "2026-01-01",
        now
      ],
    )?;

    conn.execute(
      "INSERT INTO regulation_sections(id, regulation_id, chapter, article_no, title, body, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        "reg_demo_s1",
        "reg_demo",
        "第一章",
        "第一条",
        "诚信与公正",
        "示例条款：学生应当诚信学习，公平参与评优评先，不得弄虚作假。",
        now
      ],
    )?;
  }

  let has_any_story: bool = conn
    .query_row("SELECT EXISTS(SELECT 1 FROM stories LIMIT 1)", [], |row| row.get(0))
    .unwrap_or(false);

  if !has_any_story {
    let now = unix_ms();
    conn.execute(
      "INSERT INTO stories(id, title, body, source, day_of_year, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![
        "story_demo",
        "悬鱼拒贿（示例）",
        "示例故事：古人以清廉自守，婉拒馈赠，留下“悬鱼拒贿”的佳话。",
        "古代典故（示例）",
        1,
        now
      ],
    )?;
  }

  let has_any_question: bool = conn
    .query_row(
      "SELECT EXISTS(SELECT 1 FROM questions LIMIT 1)",
      [],
      |row| row.get(0),
    )
    .unwrap_or(false);

  if !has_any_question {
    let now = unix_ms();
    conn.execute(
      "INSERT INTO questions(id, module, stem, type, difficulty, answer_key, analysis, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        "q_demo_1",
        "story",
        "以下哪种做法更符合廉洁要求？",
        "single",
        1,
        "B",
        "示例解析：面对馈赠应当婉拒或按规定处理。",
        now
      ],
    )?;

    conn.execute(
      "INSERT INTO question_options(id, question_id, opt_key, opt_text, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
      params!["q_demo_1_A", "q_demo_1", "A", "私下收下礼物不告诉任何人", 1],
    )?;
    conn.execute(
      "INSERT INTO question_options(id, question_id, opt_key, opt_text, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
      params!["q_demo_1_B", "q_demo_1", "B", "礼貌拒绝并说明规定", 2],
    )?;
    conn.execute(
      "INSERT INTO question_options(id, question_id, opt_key, opt_text, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
      params!["q_demo_1_C", "q_demo_1", "C", "等事情过去再决定是否退还", 3],
    )?;
  }

  Ok(())
}

fn unix_ms() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  let d = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
  d.as_millis() as i64
}
