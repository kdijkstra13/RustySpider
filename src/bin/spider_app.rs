use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, put};
use axum::Json;
use axum::Router;
use clap::{CommandFactory, Parser};
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use spider::modules::crawlers::{CrawlersConfig, CrawlersConfigs};
use spider::modules::fetchers::{FetchersConfig, FetchersConfigs};
use spider::modules::serialize::{
    ContentFile,
    load_contents_file,
    load_crawlers_file,
    load_fetchers_file,
    load_spider_run_config,
    save_crawlers,
    save_fetchers,
    save_contents,
    SpiderRunConfig,
};
use spider::modules::types::Content;

#[derive(Clone)]
struct AppState {
    contents_path: PathBuf,
    crawlers_path: PathBuf,
    fetchers_path: PathBuf,
    log_path: PathBuf,
    spider_config_path: PathBuf,
}

#[derive(Parser)]
#[command(
    name = "spider_app",
    version,
    about = "RustySpider companion UI",
    long_about = None
)]
struct Cli {
    #[arg(short = 'l', long = "log-file", required = true)]
    log_file: String,

    #[arg(short = 'c', long = "contents", default_value = "./contents.toml")]
    contents: String,

    #[arg(short = 'r', long = "crawlers", default_value = "./crawlers.toml")]
    crawlers: String,

    #[arg(short = 'f', long = "fetchers", default_value = "./fetchers.toml")]
    fetchers: String,

    #[arg(
        short = 's',
        long = "spider-toml",
        alias = "spider-config",
        value_name = "FILE",
        help = "Path to spider.toml run configuration",
        default_value = "./spider.toml"
    )]
    spider_config: String,
}

#[tokio::main]
async fn main() {
    if std::env::args_os().len() == 1 {
        let mut cmd = Cli::command();
        cmd.print_long_help().expect("help output failed");
        println!();
        return;
    }

    let cli = Cli::parse();
    let state = AppState {
        contents_path: PathBuf::from(cli.contents),
        crawlers_path: PathBuf::from(cli.crawlers),
        fetchers_path: PathBuf::from(cli.fetchers),
        log_path: PathBuf::from(cli.log_file),
        spider_config_path: PathBuf::from(cli.spider_config),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/contents", get(list_contents).post(add_content))
        .route(
            "/api/contents/:idx",
            put(update_content).delete(delete_content),
        )
        .route("/api/crawlers", get(list_crawlers).post(add_crawler))
        .route(
            "/api/crawlers/:idx",
            put(update_crawler).delete(delete_crawler),
        )
        .route("/api/fetchers", get(list_fetchers).post(add_fetcher))
        .route(
            "/api/fetchers/:idx",
            put(update_fetcher).delete(delete_fetcher),
        )
        .route("/api/run", axum::routing::post(run_spider))
        .route("/api/log", get(get_log))
        .with_state(state);

    let port = env::var("SPIDER_APP_PORT")
        .ok()
        .and_then(|val| val.parse::<u16>().ok())
        .unwrap_or(7878);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Spider app running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind port");
    axum::serve(listener, app)
        .await
        .expect("server error");
}

async fn index() -> Html<String> {
    Html(index_html())
}

async fn list_contents(State(state): State<AppState>) -> Result<Json<Vec<Content>>, ApiError> {
    let file = read_contents(&state.contents_path)?;
    Ok(Json(file.content))
}

async fn add_content(
    State(state): State<AppState>,
    Json(payload): Json<Content>,
) -> Result<Json<Vec<Content>>, ApiError> {
    let mut file = read_contents(&state.contents_path)?;
    file.content.push(payload);
    write_contents(&state.contents_path, &file)?;
    Ok(Json(file.content))
}

async fn update_content(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
    Json(payload): Json<Content>,
) -> Result<Json<Vec<Content>>, ApiError> {
    let mut file = read_contents(&state.contents_path)?;
    if idx >= file.content.len() {
        return Err(ApiError::not_found("content index out of range"));
    }
    file.content[idx] = payload;
    write_contents(&state.contents_path, &file)?;
    Ok(Json(file.content))
}

async fn delete_content(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Result<Json<Vec<Content>>, ApiError> {
    let mut file = read_contents(&state.contents_path)?;
    if idx >= file.content.len() {
        return Err(ApiError::not_found("content index out of range"));
    }
    file.content.remove(idx);
    write_contents(&state.contents_path, &file)?;
    Ok(Json(file.content))
}

async fn list_crawlers(State(state): State<AppState>) -> Result<Json<Vec<CrawlersConfig>>, ApiError> {
    let file = read_crawlers(&state.crawlers_path)?;
    Ok(Json(file.crawlers))
}

async fn add_crawler(
    State(state): State<AppState>,
    Json(payload): Json<CrawlersConfig>,
) -> Result<Json<Vec<CrawlersConfig>>, ApiError> {
    let mut file = read_crawlers(&state.crawlers_path)?;
    file.crawlers.push(payload);
    write_crawlers(&state.crawlers_path, &file)?;
    Ok(Json(file.crawlers))
}

async fn update_crawler(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
    Json(payload): Json<CrawlersConfig>,
) -> Result<Json<Vec<CrawlersConfig>>, ApiError> {
    let mut file = read_crawlers(&state.crawlers_path)?;
    if idx >= file.crawlers.len() {
        return Err(ApiError::not_found("crawler index out of range"));
    }
    file.crawlers[idx] = payload;
    write_crawlers(&state.crawlers_path, &file)?;
    Ok(Json(file.crawlers))
}

async fn delete_crawler(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Result<Json<Vec<CrawlersConfig>>, ApiError> {
    let mut file = read_crawlers(&state.crawlers_path)?;
    if idx >= file.crawlers.len() {
        return Err(ApiError::not_found("crawler index out of range"));
    }
    file.crawlers.remove(idx);
    write_crawlers(&state.crawlers_path, &file)?;
    Ok(Json(file.crawlers))
}

async fn list_fetchers(State(state): State<AppState>) -> Result<Json<Vec<FetchersConfig>>, ApiError> {
    let file = read_fetchers(&state.fetchers_path)?;
    Ok(Json(file.fetchers))
}

async fn add_fetcher(
    State(state): State<AppState>,
    Json(payload): Json<FetchersConfig>,
) -> Result<Json<Vec<FetchersConfig>>, ApiError> {
    let mut file = read_fetchers(&state.fetchers_path)?;
    file.fetchers.push(payload);
    write_fetchers(&state.fetchers_path, &file)?;
    Ok(Json(file.fetchers))
}

async fn update_fetcher(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
    Json(payload): Json<FetchersConfig>,
) -> Result<Json<Vec<FetchersConfig>>, ApiError> {
    let mut file = read_fetchers(&state.fetchers_path)?;
    if idx >= file.fetchers.len() {
        return Err(ApiError::not_found("fetcher index out of range"));
    }
    file.fetchers[idx] = payload;
    write_fetchers(&state.fetchers_path, &file)?;
    Ok(Json(file.fetchers))
}

async fn delete_fetcher(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Result<Json<Vec<FetchersConfig>>, ApiError> {
    let mut file = read_fetchers(&state.fetchers_path)?;
    if idx >= file.fetchers.len() {
        return Err(ApiError::not_found("fetcher index out of range"));
    }
    file.fetchers.remove(idx);
    write_fetchers(&state.fetchers_path, &file)?;
    Ok(Json(file.fetchers))
}

async fn get_log(State(state): State<AppState>) -> Result<String, ApiError> {
    let text = match fs::read_to_string(&state.log_path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(ApiError::internal(err.to_string())),
    };
    Ok(limit_tail(&text, 20000))
}

async fn run_spider(State(state): State<AppState>) -> Result<Json<RunResponse>, ApiError> {
    let config = read_spider_config(&state.spider_config_path)?;
    let mut cmd = tokio::process::Command::new(&config.spider_executable);
    cmd.arg("-l")
        .arg(&config.log_file)
        .arg("-c")
        .arg(&config.contents)
        .arg("-r")
        .arg(&config.crawlers)
        .arg("-f")
        .arg(&config.fetchers);

    let mut child = cmd
        .spawn()
        .map_err(|err| ApiError::internal(format!("failed to start spider: {err}")))?;
    tokio::spawn(async move {
        let _ = child.wait().await;
    });

    Ok(Json(RunResponse {
        status: "started".to_string(),
    }))
}

fn write_crawlers(path: &FsPath, data: &CrawlersConfigs) -> Result<(), ApiError> {
    save_crawlers(
        path.to_str().ok_or_else(|| ApiError::internal("invalid crawlers path".to_string()))?,
        data,
    )
    .map_err(|err| ApiError::internal(err.to_string()))
}

fn write_fetchers(path: &FsPath, data: &FetchersConfigs) -> Result<(), ApiError> {
    save_fetchers(
        path.to_str().ok_or_else(|| ApiError::internal("invalid fetchers path".to_string()))?,
        data,
    )
    .map_err(|err| ApiError::internal(err.to_string()))
}

fn write_contents(path: &FsPath, data: &ContentFile) -> Result<(), ApiError> {
    save_contents(
        path.to_str().ok_or_else(|| ApiError::internal("invalid contents path".to_string()))?,
        &data.content,
    )
    .map_err(|err| ApiError::internal(err.to_string()))
}

fn read_contents(path: &FsPath) -> Result<ContentFile, ApiError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| ApiError::internal("invalid contents path".to_string()))?;
    match load_contents_file(path_str) {
        Ok(file) => Ok(file),
        Err(err) if is_not_found(&err) => Ok(ContentFile::default()),
        Err(err) => Err(ApiError::internal(err.to_string())),
    }
}

fn read_crawlers(path: &FsPath) -> Result<CrawlersConfigs, ApiError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| ApiError::internal("invalid crawlers path".to_string()))?;
    match load_crawlers_file(path_str) {
        Ok(file) => Ok(file),
        Err(err) if is_not_found(&err) => Ok(CrawlersConfigs {
            crawlers: Vec::new(),
        }),
        Err(err) => Err(ApiError::internal(err.to_string())),
    }
}

fn read_fetchers(path: &FsPath) -> Result<FetchersConfigs, ApiError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| ApiError::internal("invalid fetchers path".to_string()))?;
    match load_fetchers_file(path_str) {
        Ok(file) => Ok(file),
        Err(err) if is_not_found(&err) => Ok(FetchersConfigs {
            fetchers: Vec::new(),
        }),
        Err(err) => Err(ApiError::internal(err.to_string())),
    }
}

fn read_spider_config(path: &FsPath) -> Result<SpiderRunConfig, ApiError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| ApiError::internal("invalid spider config path".to_string()))?;
    match load_spider_run_config(path_str) {
        Ok(config) => Ok(config),
        Err(err) if is_not_found(&err) => Err(ApiError::not_found("spider config not found")),
        Err(err) => Err(ApiError::internal(err.to_string())),
    }
}

fn is_not_found(err: &Box<dyn std::error::Error>) -> bool {
    err.downcast_ref::<std::io::Error>()
        .map(|io_err| io_err.kind() == std::io::ErrorKind::NotFound)
        .unwrap_or(false)
}

fn limit_tail(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let start = text.len() - max_chars;
    text[start..].to_string()
}

#[derive(Debug)]
struct ApiError {
    code: StatusCode,
    message: String,
}

#[derive(serde::Serialize)]
struct RunResponse {
    status: String,
}

impl ApiError {
    fn not_found(message: &str) -> Self {
        Self {
            code: StatusCode::NOT_FOUND,
            message: message.to_string(),
        }
    }

    fn internal(message: String) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.code, self.message).into_response()
    }
}

fn index_html() -> String {
    let html = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>RustySpider App</title>
  <style>
    :root {
      --bg: #0f172a;
      --panel: #0b1324;
      --panel-2: #111827;
      --card: #0f1c33;
      --accent: #f59e0b;
      --accent-2: #22c55e;
      --text: #e2e8f0;
      --muted: #94a3b8;
      --border: rgba(148, 163, 184, 0.2);
      --danger: #ef4444;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      font-family: "Trebuchet MS", "Verdana", "Geneva", sans-serif;
      color: var(--text);
      background: radial-gradient(circle at top, #1e293b, #0b1020 55%, #090c18);
      min-height: 100vh;
    }

    header {
      padding: 24px 20px 12px;
      text-align: left;
    }

    header h1 {
      margin: 0 0 6px;
      font-size: 28px;
      letter-spacing: 0.5px;
    }

    header p {
      margin: 0;
      color: var(--muted);
      font-size: 14px;
    }

    .shell {
      padding: 0 16px 32px;
      max-width: 1200px;
      margin: 0 auto;
    }

    .tabs {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      margin-bottom: 16px;
    }

    .tab {
      border: 1px solid var(--border);
      padding: 10px 16px;
      border-radius: 999px;
      background: rgba(15, 23, 42, 0.6);
      color: var(--text);
      cursor: pointer;
      transition: transform 0.2s ease, background 0.2s ease;
    }

    .tab.active {
      background: var(--accent);
      color: #1f2937;
      border-color: transparent;
    }

    .tab:hover {
      transform: translateY(-1px);
    }

    .panel {
      background: linear-gradient(145deg, rgba(15, 23, 42, 0.9), rgba(17, 24, 39, 0.95));
      border: 1px solid var(--border);
      border-radius: 18px;
      padding: 20px;
      box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
    }

    .panel-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 16px;
    }

    .panel-header h2 {
      margin: 0;
      font-size: 20px;
    }

    .panel-header small {
      color: var(--muted);
    }

    .grid {
      display: grid;
      gap: 16px;
    }

    .card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 16px;
    }

    .card h3 {
      margin: 0 0 12px;
      font-size: 16px;
      color: var(--accent);
    }

    .fields {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 12px;
    }

    .field label {
      display: block;
      font-size: 12px;
      color: var(--muted);
      margin-bottom: 6px;
    }

    .field input,
    .field textarea,
    .field select {
      width: 100%;
      padding: 10px 12px;
      border-radius: 10px;
      border: 1px solid transparent;
      background: var(--panel-2);
      color: var(--text);
    }

    .field input:focus,
    .field textarea:focus,
    .field select:focus {
      outline: none;
      border-color: var(--accent);
    }

    .actions {
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
      margin-top: 14px;
    }

    .btn {
      border: 1px solid transparent;
      padding: 10px 16px;
      border-radius: 12px;
      cursor: pointer;
      font-weight: 600;
      background: var(--accent);
      color: #1f2937;
      transition: transform 0.2s ease, box-shadow 0.2s ease;
    }

    .btn.secondary {
      background: transparent;
      color: var(--text);
      border-color: var(--border);
    }

    .btn.danger {
      background: var(--danger);
      color: #111827;
    }

    .btn:active {
      transform: translateY(1px);
    }

    .notice {
      padding: 10px 14px;
      border-radius: 12px;
      background: rgba(34, 197, 94, 0.15);
      color: #bbf7d0;
      border: 1px solid rgba(34, 197, 94, 0.3);
      display: none;
      margin-bottom: 14px;
    }

    .notice.error {
      background: rgba(239, 68, 68, 0.15);
      color: #fecaca;
      border-color: rgba(239, 68, 68, 0.3);
    }

    .log-box {
      background: #0b1020;
      border-radius: 12px;
      padding: 14px;
      border: 1px solid var(--border);
      color: #d1d5db;
      font-family: "Courier New", monospace;
      font-size: 12px;
      white-space: pre-wrap;
      max-height: 460px;
      overflow-y: auto;
    }

    @media (max-width: 640px) {
      header {
        padding: 20px 16px 8px;
      }

      header h1 {
        font-size: 22px;
      }

      .panel {
        padding: 16px;
      }

      .fields {
        grid-template-columns: 1fr;
      }

      .tab {
        width: 100%;
        justify-content: center;
      }
    }
  </style>
</head>
<body>
  <header>
    <div class="shell">
      <h1>RustySpider Companion</h1>
      <p>Manage contents, crawlers, fetchers, and review logs.</p>
    </div>
  </header>
  <div class="shell">
    <div class="tabs" id="tabs">
      <button class="tab active" data-tab="contents">Contents</button>
      <button class="tab" data-tab="advanced">Advanced</button>
      <button class="tab" data-tab="log">Log</button>
    </div>

    <div class="notice" id="notice"></div>

    <div class="panel" id="panel">
      <div class="panel-header">
        <h2 id="panel-title">Contents</h2>
        <small id="panel-subtitle">Edit contents.toml entries</small>
      </div>
      <div id="panel-body"></div>
    </div>
  </div>

  <script>
    const notice = document.getElementById("notice");
    const panelBody = document.getElementById("panel-body");
    const panelTitle = document.getElementById("panel-title");
    const panelSubtitle = document.getElementById("panel-subtitle");
    let logTimer = null;
    let autoScrollEnabled = true;
    const state = {
      contents: [],
      crawlers: [],
      fetchers: [],
      log: "",
      tab: "contents",
      advancedTab: "crawlers"
    };

    const schemas = {
      contents: [
        { name: "prefix", label: "Prefix", type: "text" },
        { name: "title", label: "Title", type: "text" },
        { name: "first_prefix", label: "First prefix", type: "text" },
        { name: "first", label: "First", type: "number" },
        { name: "second_prefix", label: "Second prefix", type: "text" },
        { name: "second", label: "Second", type: "number" },
        { name: "digits", label: "Digits", type: "number" },
        { name: "postfix", label: "Postfix", type: "text" }
      ],
      crawlers: [
        { name: "url", label: "Base URL", type: "text" },
        { name: "search_page", label: "Search page", type: "text" },
        { name: "search_get_name", label: "Search query param", type: "text" },
        { name: "categories", label: "Categories (comma separated)", type: "text" },
        { name: "categories_get_name", label: "Category param", type: "text" },
        { name: "user_agent", label: "User agent", type: "text" },
        { name: "limit", label: "Limit", type: "number" },
        { name: "first_stage_match", label: "First stage selector", type: "text" },
        { name: "second_stage_match", label: "Second stage selector", type: "text" }
      ],
      fetchers: [
        { name: "url", label: "Base URL", type: "text" },
        { name: "add_url", label: "Add URL", type: "text" },
        { name: "login_url", label: "Login URL", type: "text" },
        { name: "username", label: "Username", type: "text" },
        { name: "password", label: "Password", type: "password" },
        { name: "save_path", label: "Save path", type: "text" }
      ]
    };

    const templates = {
      contents: {
        prefix: "",
        title: "",
        first_prefix: "S",
        first: 0,
        second_prefix: "E",
        second: 0,
        digits: 2,
        postfix: ""
      },
      crawlers: {
        type: "twostageweb",
        url: "",
        search_page: "/search/",
        search_get_name: "search",
        categories: [],
        categories_get_name: "category[]",
        user_agent: "Mozilla/5.0 (compatible; RustySpider/1.0)",
        limit: 10,
        first_stage_match: "",
        second_stage_match: ""
      },
      fetchers: {
        type: "qbfetcher",
        url: "",
        add_url: "/api/v2/torrents/add",
        login_url: "/api/v2/auth/login",
        username: "",
        password: "",
        save_path: ""
      }
    };

    function showNotice(message, isError = false) {
      notice.textContent = message;
      notice.classList.toggle("error", isError);
      notice.style.display = "block";
      setTimeout(() => {
        notice.style.display = "none";
      }, 3500);
    }

    async function apiGet(path) {
      const res = await fetch(path);
      if (!res.ok) {
        throw new Error(await res.text());
      }
      return res.json();
    }

    async function apiSend(path, method, payload) {
      const res = await fetch(path, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });
      if (!res.ok) {
        throw new Error(await res.text());
      }
      return res.json();
    }

    async function loadAll() {
      state.contents = await apiGet("/api/contents");
      state.crawlers = await apiGet("/api/crawlers");
      state.fetchers = await apiGet("/api/fetchers");
      state.log = await fetchLog();
    }

    function scrollLogToBottom() {
      const box = document.getElementById("log-box");
      if (!box) return;
      box.scrollTop = box.scrollHeight;
    }

    function isNearLogBottom(box) {
      const threshold = 12;
      return box.scrollHeight - box.scrollTop - box.clientHeight <= threshold;
    }

    function attachLogBoxHandlers() {
      const box = document.getElementById("log-box");
      if (!box) return;
      box.addEventListener("scroll", () => {
        autoScrollEnabled = isNearLogBottom(box);
      });
      if (autoScrollEnabled) {
        scrollLogToBottom();
      }
    }

    function startLogPolling() {
      if (logTimer) return;
      logTimer = setInterval(async () => {
        if (state.tab !== "log") return;
        const next = await fetchLog();
        if (next !== state.log) {
          state.log = next;
          const box = document.getElementById("log-box");
          if (box) {
            box.textContent = state.log || "No log entries yet.";
            if (autoScrollEnabled) {
              scrollLogToBottom();
            }
          } else {
            render();
          }
        } else {
          if (autoScrollEnabled) {
            scrollLogToBottom();
          }
        }
      }, 3000);
    }

    function stopLogPolling() {
      if (!logTimer) return;
      clearInterval(logTimer);
      logTimer = null;
    }

    function setTab(tab) {
      state.tab = tab;
      document.querySelectorAll(".tab").forEach(btn => {
        btn.classList.toggle("active", btn.dataset.tab === tab);
      });
      if (tab === "log") {
        startLogPolling();
      } else {
        stopLogPolling();
      }
      render();
    }

    function fieldValue(item, name) {
      if (name === "categories") {
        return (item.categories || []).join(", ");
      }
      return item[name] ?? "";
    }

    function renderEntries(items, kind) {
      if (!items.length) {
        return `<div class="card"><h3>No entries yet</h3><p>Add a new one below.</p></div>`;
      }
      return items
        .map((item, idx) => {
          const fields = schemas[kind]
            .map(field => {
              const value = fieldValue(item, field.name);
              const type = field.type || "text";
              return `
                <div class="field">
                  <label>${field.label}</label>
                  <input data-field="${field.name}" data-index="${idx}" data-kind="${kind}" type="${type}" value="${escapeHtml(value)}" />
                </div>
              `;
            })
            .join("");
          const badge = kind === "contents" ? "content" : (item.type || "");
          return `
            <div class="card" data-card="${kind}-${idx}">
              <h3>${badge}</h3>
              <div class="fields">${fields}</div>
              <div class="actions">
                <button class="btn" data-action="save" data-kind="${kind}" data-index="${idx}">Save</button>
                <button class="btn danger" data-action="delete" data-kind="${kind}" data-index="${idx}">Delete</button>
              </div>
            </div>
          `;
        })
        .join("");
    }

    function renderAddCard(kind) {
      const defaults = kind === "contents" ? templates.contents : {};
      return `
        <div class="card">
          <h3>Add new ${kind.slice(0, -1)}</h3>
          <div class="fields">
            ${schemas[kind]
              .map(field => {
                const type = field.type || "text";
                const value = defaults[field.name] ?? "";
                return `
                  <div class="field">
                    <label>${field.label}</label>
                    <input data-field="${field.name}" data-kind="${kind}" data-new="true" type="${type}" value="${escapeHtml(value)}" />
                  </div>
                `;
              })
              .join("")}
          </div>
          <div class="actions">
            <button class="btn secondary" data-action="add" data-kind="${kind}">Add</button>
            <button class="btn secondary" data-action="reset" data-kind="${kind}">Reset</button>
          </div>
        </div>
      `;
    }

    function renderLog() {
      return `
        <div class="card">
          <div class="actions">
            <button class="btn secondary" data-action="run-spider">Run spider</button>
            <button class="btn" data-action="refresh-log">Refresh log</button>
          </div>
          <div class="log-box" id="log-box">${escapeHtml(state.log || "No log entries yet.")}</div>
        </div>
      `;
    }

    function render() {
      if (state.tab === "contents") {
        panelTitle.textContent = "Contents";
        panelSubtitle.textContent = "Edit contents.toml entries";
        panelBody.innerHTML = `<div class="grid">${renderEntries(state.contents, "contents")}${renderAddCard("contents")}</div>`;
      } else if (state.tab === "advanced") {
        panelTitle.textContent = "Advanced";
        panelSubtitle.textContent = "Crawler and fetcher configuration";
        panelBody.innerHTML = `
          <div class="tabs" id="advanced-tabs">
            <button class="tab ${state.advancedTab === "crawlers" ? "active" : ""}" data-advanced="crawlers">Crawlers</button>
            <button class="tab ${state.advancedTab === "fetchers" ? "active" : ""}" data-advanced="fetchers">Fetchers</button>
          </div>
          <div class="grid">
            ${state.advancedTab === "crawlers"
              ? `${renderEntries(state.crawlers, "crawlers")}${renderAddCard("crawlers")}`
              : `${renderEntries(state.fetchers, "fetchers")}${renderAddCard("fetchers")}`
            }
          </div>
        `;
      } else if (state.tab === "log") {
        panelTitle.textContent = "Log";
        panelSubtitle.textContent = "View rusty_spider.log";
        panelBody.innerHTML = renderLog();
        attachLogBoxHandlers();
      }
    }

    function entryLabel(kind, item, index) {
      if (!item) {
        return `${kind.slice(0, -1)} #${index + 1}`;
      }
      if (kind === "contents") {
        const title = (item.title || "").trim();
        const prefix = (item.prefix || "").trim();
        if (title) return title;
        if (prefix) return prefix;
        return `content #${index + 1}`;
      }
      if (kind === "crawlers" || kind === "fetchers") {
        const url = (item.url || "").trim();
        if (url) return url;
      }
      return `${kind.slice(0, -1)} #${index + 1}`;
    }

    function collectItem(kind, container, defaults) {
      const item = { ...defaults };
      const inputs = container.querySelectorAll(`input[data-kind="${kind}"]`);
      inputs.forEach(input => {
        const field = input.dataset.field;
        if (!field) return;
        if (field === "categories") {
          item[field] = input.value
            .split(",")
            .map(s => s.trim())
            .filter(Boolean);
          return;
        }
        if (input.type === "number") {
          item[field] = Number.parseInt(input.value || "0", 10);
          return;
        }
        item[field] = input.value;
      });
      return item;
    }

    function escapeHtml(value) {
      const text = String(value ?? "");
      return text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/\"/g, "&quot;")
        .replace(/'/g, "&#39;");
    }

    async function fetchLog() {
      const res = await fetch("/api/log");
      if (!res.ok) {
        return "";
      }
      return res.text();
    }

    document.getElementById("tabs").addEventListener("click", event => {
      const button = event.target.closest(".tab");
      if (!button) return;
      setTab(button.dataset.tab);
    });

    document.addEventListener("click", event => {
      const button = event.target.closest("button[data-advanced]");
      if (!button) return;
      state.advancedTab = button.dataset.advanced;
      render();
    });

    document.addEventListener("click", async event => {
      const button = event.target.closest("button[data-action]");
      if (!button) return;

      const action = button.dataset.action;
      const kind = button.dataset.kind;
      const index = Number.parseInt(button.dataset.index || "-1", 10);

      try {
        if (action === "save") {
          const card = button.closest(".card");
          const payload = collectItem(kind, card, templates[kind]);
          if (kind === "crawlers" || kind === "fetchers") {
            payload.type = templates[kind].type;
          }
          const data = await apiSend(`/api/${kind}/${index}`, "PUT", payload);
          state[kind] = data;
          showNotice("Saved entry.");
          render();
        } else if (action === "delete") {
          const item = state[kind] ? state[kind][index] : null;
          const label = entryLabel(kind, item, index);
          const ok = window.confirm(`Delete ${label}? This cannot be undone.`);
          if (!ok) return;
          const data = await apiSend(`/api/${kind}/${index}`, "DELETE", {});
          state[kind] = data;
          showNotice("Deleted entry.");
          render();
        } else if (action === "add") {
          const card = button.closest(".card");
          const payload = collectItem(kind, card, templates[kind]);
          if (kind === "crawlers" || kind === "fetchers") {
            payload.type = templates[kind].type;
          }
          const data = await apiSend(`/api/${kind}`, "POST", payload);
          state[kind] = data;
          showNotice("Added entry.");
          render();
        } else if (action === "reset") {
          render();
        } else if (action === "refresh-log") {
          state.log = await fetchLog();
          render();
        } else if (action === "run-spider") {
          const res = await fetch("/api/run", { method: "POST" });
          if (!res.ok) {
            throw new Error(await res.text());
          }
          showNotice("Spider run started.");
        }
      } catch (err) {
        showNotice(err.message || "Request failed.", true);
      }
    });

    async function boot() {
      try {
        await loadAll();
        render();
      } catch (err) {
        showNotice(err.message || "Failed to load data.", true);
      }
    }

    boot();
  </script>
</body>
</html>"#;

    html.to_string()
}
