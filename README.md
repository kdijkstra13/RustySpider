# RustySpider
RustySpider is a small, configurable crawler pipeline with an optional companion UI. It takes a list of content queries, runs them through one or more crawlers, and sends results to one or more fetchers. Configuration lives in TOML files so the crawler and UI can share the same settings.

**What’s included**
- `spider`: CLI that loads contents, crawlers, and fetchers and executes the crawl pipeline.
- `spider_app`: Web UI for editing `contents.toml`, `crawlers.toml`, and `fetchers.toml`, plus viewing logs and triggering a run.

## Install
Requirements: Rust toolchain (stable) with Cargo.

Build the binaries:
```bash
cargo build --release
```

Run them from the build output:
```bash
./target/release/spider
./target/release/spider_app ./spider.toml
```

The UI listens on port `7878` by default. Override with:
```bash
SPIDER_APP_PORT=9000 ./target/release/spider_app ./spider.toml
```

## Configure
Configuration is split into four TOML files. The UI uses `spider.toml` as the entry point, and all other file paths are resolved from it.

### `spider.toml`
Example from `spider.example.toml`:
```toml
spider_executable = "./spider"
contents = "./contents.toml"
crawlers = "./crawlers.toml"
fetchers = "./fetchers.toml"
log_file = "./rusty_spider.log"
```

Field details:
- `spider_executable`: Path to the `spider` binary the UI should launch when you click Run.
- `contents`: Path to the contents file (see below).
- `crawlers`: Path to the crawlers file (see below).
- `fetchers`: Path to the fetchers file (see below).
- `log_file`: Path to the log file written by `spider`.

### `contents.toml`
Defines the set of queries to run. Example from `contents.example.toml`:
```toml
[[content]]
prefix = ""
title = ""
first_prefix = ""
first = 1
second_prefix = ""
second = 2
digits = 2
postfix = ""
```

Field details:
- `prefix`: Optional text prepended before the title, followed by a space when non-empty.
- `title`: The main query title (usually the content name).
- `first_prefix`: Prefix for the first counter.
- `first`: First counter value.
- `second_prefix`: Prefix for the second counter.
- `second`: Second counter value.
- `digits`: Zero padding width applied to both `first` and `second` (e.g. `2` yields `01`, `02`).
- `postfix`: Optional text appended after the counters, preceded by a space when non-empty.

Query format:
```
{prefix} {title} {first_prefix}{first:0digits}{second_prefix}{second:0digits} {postfix}
```

After a successful fetch, RustySpider predicts the next query by trying:
1. The next `second` value.
1. The next `first` value with `second = 1`.

### `crawlers.toml`
Defines where and how to search. Example from `crawlers.example.toml`:
```toml
[[crawlers]]
type = "twostageweb"
categories = [""]
categories_get_name = ""
url = ""
search_page = ""
search_get_name = ""
user_agent = "Mozilla/5.0 (compatible; RustySpider/1.0; +https://example.com)"
limit = 10
wait = 5
first_stage_match = ''
second_stage_match = ''
```

Field details:
- `type`: Crawler type. Currently only `twostageweb` is supported.
- `categories`: List of category values to include in the search query.
- `categories_get_name`: Query parameter name for categories (appended once per entry in `categories`).
- `url`: Base URL for the site (used to resolve relative links).
- `search_page`: Path appended to `url` for the search request.
- `search_get_name`: Query parameter name for the search string.
- `user_agent`: User-Agent header sent with requests.
- `limit`: Intended result limit (parsed but not currently enforced).
- `wait`: Seconds to sleep between first-stage and second-stage requests. Default is `5`.
- `first_stage_match`: CSS selector used to find result links on the first page.
- `second_stage_match`: CSS selector used to find the final link on the second page.

### `fetchers.toml`
Defines how to deliver results. Example from `fetchers.example.toml`:
```toml
[[fetchers]]
type = "qbfetcher"
url = ""
add_url = ""
login_url = ""
username = ""
password = ""
save_path = ""
```

Field details:
- `type`: Fetcher type. Currently only `qbfetcher` is supported.
- `url`: Base URL of the Web UI (no trailing slash required).
- `add_url`: API path for adding URLs (appended to `url`).
- `login_url`: API path for login (appended to `url`).
- `username`: username. Leave empty for no login.
- `password`: password.
- `save_path`: Save path passed to. The final path is `save_path + title`.

## Run Spider
1. Create copies of the example files and fill them in.
2. Run the Spider:
```bash
./spider -l ./spider.log
```
3. Optionally add a call to ./spider to crontab.

## Run Web UI
1. Update `spider.toml` to point at your actual file paths.
2. Run the UI:
```bash
./spider_app ./spider.toml
```
3. Use the UI to edit configs, then start a run.
4. Optionally add ./spider_app to systemctl.