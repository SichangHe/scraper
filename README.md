# Steven Hé (Sīchàng)'s recursive scraper

The scraper scrapes *recursively*,
at *constant frequency* (requests per time),
*asynchronously*,
and write the results continuously to disk.
It is simple to use,
while providing flexible configurations.

This scraper is an open source rewrite
of the proprietary scraper used for SSO in DKU.

## Objective

The scraper is designed for *recursive* scraping,
that is,
by default,
the scraper process every `href` and `img`
from the HTML it gets to know even more URLs,
and then process those URLs as well.
One obvious usage of *recursive* scraping is full site scraping.

If you want to scrape *only* the URLs you provide,
just provide a tricky `filter` such as `"#"`
and it will function as a *non-recursive* scraper.
One usage of *non-recursive* scraping is bulk image scraping.

## Installation

Use Cargo to install recursive_scraper:

```shell
cargo install recursive_scraper
```

## Features

### Constant frequency

The scraper guarantees that eventually the number of requests sent
per time is constant.
This constant depends on the `delay` set between each request.

The `delay` needs to be set in milliseconds.
It has a default value of `500`.

### Regex filter and blacklist

The scraper does not process any new URLs
that does not match given `filter` regex
or does match given `blacklist` regex.

Any urls specified by the user are not checked.
If not specified,
`filter` defaults to `".*"` to match any URLs,
and `blacklist` defaults to `"#"` to match no URLs.
(URLs processed do not include `#` because the scraper strips it to avoid repetition.)

### Adjustable connection timeout

The scraper times out a request if it fails to connect after `10` seconds.
You can set custom timeout in milliseconds.

Under the hood,
the scraper also uses a timeout eight times as long as the connection timeout
for the request and response to finish.

### Continuously-updating record

The scrape record is written to disk as `summary.toml` in the log directory.
The record is updated once in a while as the scraper goes.

In `[urls]`,
each URL is mapped to an id based on their order of discovery.
`[scrapes]` records the ids to the URLs that are scraped.
`[fails]` records the the ids to the URLs that the scraper failed to process.
`[redirections]` records if one URL (whose id is on the left)
was redirected to another URL (on the right).

### Rings

The URLs that does not match `filter` are URLs that are in the outer rings.
To be rigorous, these URLs also need to not match `blacklist`.
When scraping,
the scraper would encounter hrefs that do not match `filter`,
if `number_of_rings` is set,
the scraper append these hrefs to the "next" pending list.
When the scraper runs out of tasks,
it takes the "next" pending list as the pending list
and continue scraping
if `number_of_rings` is set and the current ring is less than it.

## Usage

```shell
$ recursive_scraper --help
Scrapes given urls (separated by commas) recursively.
Saves the results to `html/` and `other/`, the log to `log/`,
or other directories if specified.
See <https://github.com/SichangHe/scraper> for more instructions.

Usage: recursive_scraper [OPTIONS] <START_URLS>

Arguments:
  <START_URLS>  The URLs to start scraping from, separated by commas.

Options:
  -b, --blacklist <BLACKLIST>
          Regex to match URLs that should be excluded.
  -c, --connection-timeout <CONNECTION_TIMEOUT>
          Connection timeout for each request in integer milliseconds.
  -d, --delay <DELAY>
          Delay between each request in integer milliseconds
  -f, --filter <FILTER>
          Regex to match URLs that should be included.
  -i, --disregard-html
          Do not save HTMLs.
  -l, --log-dir <LOG_DIR>
          Directory to output the log.
  -o, --other-dir <OTHER_DIR>
          Directory to save non-HTMLs.
  -r, --number-of-rings <NUMBER_OF_RINGS>
          Set the number of rings for the URLs outside the filter.
  -s, --disregard-other
          Do not save non-HTMLs.
  -t, --html-dir <HTML_DIR>
          Directory to save HTMLs.
  -h, --help
          Print help information
  -V, --version
          Print version information
```

Recursively scrape the whole `https://example.com/`:

```shell
recursive_scraper -f "https://example.com/.*" https://example.com/
```

Same as above except I don't want images:

```shell
recursive_scraper -f "https://example.com/.*" -s https://example.com/
```

Only scrape the URLs I provide (separated by commas):

```shell
recursive_scraper -f "#" https://example.com/blah,https://example.com/blahblah,https://example.com/bla
```

Scrape everything into one folder `result/`:

```shell
recursive_scraper -f "https://example.com/.*" -l result/ -o result/ -t result/ https://example.com/
```

### Environment variable

recursive_scraper uses [env_logger](https://docs.rs/env_logger/)
for logging,
so you can set `RUST_LOG` to control the log level.

For example, if you want to do the same as the first example above with
the log level set to `info`:

```shell
RUST_LOG=recursive_scraper=info recursive_scraper -f "https://example.com/.*" https://example.com/
```

On fish shell, you would instead do:

```fish
env RUST_LOG=recursive_scraper=info recursive_scraper -f "https://example.com/.*" https://example.com/
```

The log level is by default `error`.
Other options include `warn`, `info`, and `debug`.

For more instruction, see the
[Enable Logging](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)
section from env_logger.
