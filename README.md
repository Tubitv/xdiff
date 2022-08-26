# HTTP request and diff tools

There're two separate CLIs provided:

    - xdiff: A diff tool for comparing HTTP requests. It could be used to compare the difference between production staging or two versions of the same API.
    - xreq: A tool to build HTTP requests based on predefined profiles. It could be used to replace curl/httpie for building complicated HTTP requests.

## xdiff

### Configuration

You can configure multiple profiles for xdiff. Each profile is identified by a name. Inside a profile you can define the details of the two requests (method, url, query params, request headers, request body), and also what part of the response should be skipped for comparison (currently only headers could be skipped).

```yaml
rust:
  request1:
    method: GET
    url: https://www.rust-lang.org/
    headers:
        user-agent: Aloha
    params:
      hello: world
  request2:
    method: GET
    url: https://www.rust-lang.org/
    params: {}
  response:
    skip_headers:
      - set-cookie
      - date
      - via
      - x-amz-cf-id
```

You could put the configuration in `~/.config/xdiff.yml`, or `/etc/xdiff.yml`, or `~/xdiff.yml`. The xdiff CLI will look for configuration from these paths.

### How to use?

You can use `cargo install xdiff` to install it. Once finished you shall be able to use it.

```bash
➜ xdiff --help
xdiff 0.1.0
Diff API response

USAGE:
    xdiff [OPTIONS] --profile <PROFILE>

OPTIONS:
    -c, --config <CONFIG>      Path to the config file
    -e <EXTRA_PARAMS>          Extra parameters to pass to the API
    -h, --help                 Print help information
    -p, --profile <PROFILE>    API profile to use
    -V, --version              Print version information
```

An example:

```bash
xdiff -p todo -c requester/fixtures/diff.yml -e a=1 -e b=2
```

This will use the todo profile in the diff.yml defined in `requester/fixtures`, and add extra params for query string with a=1, b=2. Output look like this:

![screenshot](docs/images/screenshot1.png)
