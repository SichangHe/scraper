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

## Usage

```shell
$ ./scraper --help
recursive_scraper 0.2.1
Steven Hé (Sīchàng)
Scrapes given urls (separated by commas) recursively. Saves the results to 
`html/` and `other/`, the log to `log/`, or other directories if specified.

USAGE:
    recursive_scraper [OPTIONS] <START_URLS>

ARGS:
    <START_URLS>

OPTIONS:
    -b, --blacklist <BLACKLIST>
    -c, --connection-timeout <CONNECTION_TIMEOUT>
    -d, --delay <DELAY>
    -f, --filter <FILTER>
    -h, --help                                       Print help information
    -i, --disregard-html
    -l, --log-dir <LOG_DIR>
    -o, --other-dir <OTHER_DIR>
    -s, --disregard-other
    -t, --html-dir <HTML_DIR>
    -V, --version                                    Print version information
```

Recursively scrape the whole `https://example.com/`:

```shell
./scraper -f "https://example.com/.*" https://example.com/
```

Same as above except I don't want images:

```shell
./scraper -f "https://example.com/.*" -s https://example.com/
```

Only scrape the URLs I provide (separated by commas):

```shell
./scraper -f "#" https://example.com/blah,https://example.com/blahblah,https://example.com/bla
```

Scrape everything into one folder `result/`:

```shell
./scraper -f "https://example.com/.*" -l result/ -o result/ -t result/ https://example.com/
```
