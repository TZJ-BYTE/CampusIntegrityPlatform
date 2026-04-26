use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::params;
use app_lib::db;

fn main() -> Result<(), String> {
  let out_dir = std::env::args()
    .nth(1)
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("../server-go/content-repo"));
  let version = std::env::args().nth(2).unwrap_or_else(|| "1".to_string());

  fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

  let tmp_dir = out_dir.join(format!("tmp-pack-{}", unix_ms()));
  fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

  let content_db_path = tmp_dir.join("content.db");
  let conn = db::open_content_db(&content_db_path).map_err(|e| e.to_string())?;

  seed_more_content(&conn).map_err(|e| e.to_string())?;

  conn
    .execute(
      "INSERT OR REPLACE INTO meta(key, value) VALUES('content_version', ?1)",
      params![version.clone()],
    )
    .map_err(|e| e.to_string())?;

  let manifest = serde_json::json!({
    "contentVersion": version.clone(),
    "minAppVersion": null
  });
  let manifest_path = tmp_dir.join("manifest.json");
  fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?)
    .map_err(|e| e.to_string())?;

  let pack_path = out_dir.join("content-pack.zip");
  build_zip(&pack_path, &manifest_path, &content_db_path)?;

  let notes = format!(
    "seeded dataset: venues/cases/regulations/stories/questions; v{}",
    version
  );
  let versions_json = serde_json::json!({
    "latest": {
      "version": version,
      "url": "/content-pack.zip",
      "notes": notes
    }
  });
  let versions_path = out_dir.join("versions.json");
  fs::write(&versions_path, serde_json::to_vec_pretty(&versions_json).map_err(|e| e.to_string())?)
    .map_err(|e| e.to_string())?;

  drop(conn);
  let _ = fs::remove_dir_all(&tmp_dir);
  Ok(())
}

fn seed_more_content(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
  let now = unix_ms();

  let venue_count: i64 = conn.query_row("SELECT COUNT(1) FROM venues", [], |row| row.get(0))?;
  if venue_count < 16 {
    for i in 1..=16 {
      let id = format!("venue_{:03}", i);
      let name = format!("廉洁文化点位 {:02}", i);
      let t = if i % 3 == 0 {
        "红色教育"
      } else if i % 3 == 1 {
        "文化展示"
      } else {
        "实践体验"
      };
      let location = format!("校园区域 {}", if i <= 8 { "A" } else { "B" });
      let description = format!(
        "该点位用于廉洁文化学习与主题活动组织（示例数据）。建议参观时间 15-30 分钟，可配套任务打卡与答题。编号 {}。",
        id
      );
      let contact = "校内管理办公室".to_string();
      let open_hours = "每日 8:00-20:00".to_string();
      conn.execute(
        "INSERT OR IGNORE INTO venues(id, name, type, location, description, contact, open_hours, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, name, t, location, description, contact, open_hours, now],
      )?;
    }
  }

  let case_count: i64 = conn.query_row("SELECT COUNT(1) FROM cases", [], |row| row.get(0))?;
  if case_count < 24 {
    let scenes = ["班级管理", "评优评先", "社团活动", "考试管理", "采购报销", "学生干部"];
    for i in 1..=24 {
      let id = format!("case_{:03}", i);
      let scene = scenes[(i as usize) % scenes.len()].to_string();
      let title = format!("典型情境 {:02}：{}中的风险点", i, scene);
      let summary = "以真实工作/学习流程为镜，识别风险点并给出可执行的规范做法（示例数据）。".to_string();
      let body = format!(
        "情境描述：在{}相关流程中，出现了信息不透明、规则不清、审批留痕不足等问题。\n\n风险提示：一旦形成习惯，容易导致误解、投诉或纪律风险。\n\n建议：明确规则、公开流程、保留凭证、分权制衡、定期复盘。",
        scene
      );
      let violation = "流程不透明、利益冲突、留痕不足、随意变更规则".to_string();
      let correct_action = "建立清单与台账；关键节点双人复核；对外公开关键规则与结果；异常情况留痕说明".to_string();
      conn.execute(
        "INSERT OR IGNORE INTO cases(id, title, scene, summary, body, violation, correct_action, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, title, scene, summary, body, violation, correct_action, now],
      )?;
    }
  }

  let reg_count: i64 = conn.query_row("SELECT COUNT(1) FROM regulations", [], |row| row.get(0))?;
  if reg_count < 10 {
    for i in 1..=10 {
      let id = format!("reg_{:03}", i);
      let title = format!("廉洁与诚信规范要点（示例）{:02}", i);
      let level = if i % 2 == 0 { "校内制度" } else { "学习指引" };
      let source = "综合整理（示例数据）";
      let published_at = format!("2026-{:02}-{:02}", ((i - 1) % 12) + 1, ((i - 1) % 28) + 1);
      conn.execute(
        "INSERT OR IGNORE INTO regulations(id, title, level, source, published_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, title, level, source, published_at, now],
      )?;

      for s in 1..=6 {
        let sid = format!("{}_s{:02}", id, s);
        let chapter = format!("第{}章", ((s - 1) / 3) + 1);
        let article_no = format!("第{}条", s);
        let stitle = if s % 3 == 0 {
          "公正与回避"
        } else if s % 3 == 1 {
          "诚信与公开"
        } else {
          "规范与留痕"
        };
        let body = format!(
          "要点：{}。\n\n建议做法：\n- 明确规则与边界\n- 过程公开可追溯\n- 关键节点留痕\n- 回避利益冲突\n\n示例：对评选标准、评分过程与结果进行公开说明。",
          stitle
        );
        conn.execute(
          "INSERT OR IGNORE INTO regulation_sections(id, regulation_id, chapter, article_no, title, body, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
          params![sid, id, chapter, article_no, stitle, body, now],
        )?;
      }
    }
  }

  let story_count: i64 = conn.query_row("SELECT COUNT(1) FROM stories", [], |row| row.get(0))?;
  if story_count < 40 {
    let sources = [
      "校园读书班（示例）",
      "学习进行时（示例）",
      "廉洁文化故事汇（示例）",
      "校园动态（示例）",
    ];
    for i in 1..=40 {
      let id = format!("story_{:03}", i);
      let title = format!("每日廉洁故事 {:03}", i);
      let source = sources[(i as usize) % sources.len()].to_string();
      let day = ((i - 1) % 365) as i64 + 1;
      let body = format!(
        "故事摘要：本故事通过一个小场景，强调“守住底线、公开透明、按规办事”的重要性。\n\n反思：\n1) 如果你处在相同情境，哪些细节最容易被忽略？\n2) 你会如何留痕并与同伴/老师沟通？\n\n行动建议：把规则写在前面，把记录留在过程中，把复盘放在结束后。"
      );
      conn.execute(
        "INSERT OR IGNORE INTO stories(id, title, body, source, day_of_year, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, title, body, source, day, now],
      )?;
    }

    let tute_titles = [
      ("tute_news_20260422_1", "推动全民阅读，建设书香社会", "2026-04-22"),
      ("tute_news_20260416_1", "校党委举办树立和践行正确政绩观学习教育读书班", "2026-04-16"),
      ("tute_news_20260415_1", "鲁班工坊建设团队先进群体荣获“天津楷模”称号", "2026-04-15"),
      ("tute_news_20260424_1", "书香润桃李 红韵启新程", "2026-04-24"),
      ("tute_news_20260424_2", "理学院举办2026年春季拔河比赛", "2026-04-24"),
    ];
    for (id, title, date) in tute_titles {
      let body = format!(
        "来源：学校官网公开信息（标题与日期）。\n\n摘要：本文围绕“{}”主题，强调学习风气建设与校园文化氛围营造。\n\n日期：{}。",
        title, date
      );
      conn.execute(
        "INSERT OR IGNORE INTO stories(id, title, body, source, day_of_year, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, title, body, "天津职业技术师范大学官网", 0i64, now],
      )?;
    }
  }

  let q_count: i64 = conn.query_row("SELECT COUNT(1) FROM questions", [], |row| row.get(0))?;
  if q_count < 60 {
    for i in 1..=60 {
      let id = format!("q_{:03}", i);
      let module = if i % 3 == 0 { "regulation" } else if i % 3 == 1 { "case" } else { "story" };
      let stem = format!("第 {:03} 题：在日常学习工作中，哪项做法更符合廉洁与诚信要求？", i);
      let qtype = "single";
      let difficulty = ((i - 1) % 3) as i64 + 1;
      let answer_key = "B";
      let analysis = "提示：遇到不确定情况，优先选择公开透明、按规办理、可追溯的做法。";
      conn.execute(
        "INSERT OR IGNORE INTO questions(id, module, stem, type, difficulty, answer_key, analysis, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, module, stem, qtype, difficulty, answer_key, analysis, now],
      )?;
      conn.execute(
        "INSERT OR IGNORE INTO question_options(id, question_id, opt_key, opt_text, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![format!("{}_A", id), id, "A", "只要没有人发现就可以", 1],
      )?;
      conn.execute(
        "INSERT OR IGNORE INTO question_options(id, question_id, opt_key, opt_text, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![format!("{}_B", id), id, "B", "按规定办理并留痕说明", 2],
      )?;
      conn.execute(
        "INSERT OR IGNORE INTO question_options(id, question_id, opt_key, opt_text, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![format!("{}_C", id), id, "C", "先做了再说，之后再补材料", 3],
      )?;
    }
  }

  Ok(())
}

fn build_zip(zip_path: &Path, manifest_path: &Path, content_db_path: &Path) -> Result<(), String> {
  let file = fs::File::create(zip_path).map_err(|e| e.to_string())?;
  let mut zip = zip::ZipWriter::new(file);
  let options: zip::write::FileOptions<'_, ()> =
    zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

  zip.start_file("manifest.json", options).map_err(|e| e.to_string())?;
  let mut mf = fs::File::open(manifest_path).map_err(|e| e.to_string())?;
  let mut buf = Vec::new();
  mf.read_to_end(&mut buf).map_err(|e| e.to_string())?;
  zip.write_all(&buf).map_err(|e| e.to_string())?;

  zip.start_file("content.db", options).map_err(|e| e.to_string())?;
  let mut cf = fs::File::open(content_db_path).map_err(|e| e.to_string())?;
  buf.clear();
  cf.read_to_end(&mut buf).map_err(|e| e.to_string())?;
  zip.write_all(&buf).map_err(|e| e.to_string())?;

  zip.finish().map_err(|e| e.to_string())?;
  Ok(())
}

fn unix_ms() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  d.as_millis() as i64
}
