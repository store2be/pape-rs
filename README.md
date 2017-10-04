# Papers

![Papers Logo](logo.png)

[![Build Status](https://travis-ci.org/store2be/pape-rs.svg?branch=master)](https://travis-ci.org/store2be/pape-rs)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/store2be/redux-belt/blob/master/CONTRIBUTING.md)
[![license](https://img.shields.io/github/license/mashape/apistatus.svg?maxAge=2592000)](https://github.com/store2be/pape-rs/blob/master/LICENSE)

A Latex template to PDF generation web service written in Rust. Papers is available as a docker image on [Docker Hub](https://hub.docker.com/r/store2be/pape-rs/). It relies on an installation of [xelatex](https://en.wikipedia.org/wiki/XeTeX).

**Papers uses semantic versioning. So until 1.0.0 is reached expect many breaking changes.**


## Examples / Documentation

* [Simple example](examples/simple)
* [Papers on Kubernetes](examples/kubernetes)
* [Merge documents with Papers](examples/concatenation)


## Security

**This service is not secure yet so it should not be publicly accessible.** An invader could create a template [that does bad things with Latex](http://www.lieberbiber.de/2017/03/05/arbitrary-code-execution-in-many-tex-distributions/). Therefore, it is recommended to set the `PAPERS_BEARER` environment variable to a long and arbitrary string and set this string in the `Authorization` header of every request.


## Endpoints

### GET /healthz

Returns 200. [For liveness probes in Kubernetes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-probes/).


### POST /submit

Headers:

```
Content-Type: application/json
```

Example body:

```json
{
  "template_url": "http://example.com/template.tex.tera",
  "asset_urls": [
    "http://example.com/logo.png"
  ],
  "variables": {
    "world": "World"
  },
  "callback_url": "http://example.com/callback"
}
```

* `template_url`: The Latex template as a downloadable URL.
* `asset_urls`: An array of asset URLs that are used in the Latex template. They are downloaded next to the Latex document.
* `variables`: The variables that are used in the Latex template.
* `callback_url`: The URL that the final PDF or the error will be sent to.


### POST /preview

Headers:

```
Content-Type: application/json
```

Example body:

```json
{
  "template_url": "http://example.com/template.tex.tera",
  "asset_urls": [
    "http://example.com/logo.png"
  ],
  "variables": {
    "world": "World"
  },
}
```

* `template_url`: The Latex template as a downloadable URL.
* `asset_urls`: An array of asset URLs that are used in the Latex template. They are downloaded next to the Latex document.
* `variables`: The variables that are used in the Latex template.


## Example Latex template

The templating language is [Tera](https://github.com/Keats/tera)

```latex
\documentclass{article}

\begin{document}
hello, {{world}}

\end{document}
```

## Local server

Papers ships with the `papers-local` executable that you can use to develop your templates locally. Just put your assets in a directory, name your template `template.tex.tera`, put variables in a `variables.json` and run the binary. You will get a rendered PDF that is produced by the same code that runs in the service.

Take a look at the [simple example](examples/simple) in the examples directory for a quick introduction.

## Explanation of the environment variables

### PAPERS_BEARER

The string that will be checked in the `Authorization` header.

```
Default: <empty string>
```

Example:
```
PAPERS_BEARER=secret-string
=> Authorization=Bearer secret-string
```

### PAPERS_LOG_LEVEL

The logger level for the Papers service.

```
Default: info
Other options: debug
```

### PAPERS_PORT

The port the Papers sever runs on.

```
Default: 8080
```

### PAPERS_MAX_ASSETS_PER_DOCUMENT

The maximum amount of assets per document/request accepted by Papers

```
Default: 20
```

### PAPERS_MAX_ASSET_SIZE

The maximum size of assets in bytes (or with K, M or G suffix).

```
Default: 10M
```

### PAPERS_AWS_ACCESS_KEY

The key will be used for the S3 uploads.

Required.

## PAPERS_AWS_SECRET_KEY

The secret key corresponding to `PAPERS_AWS_ACCESS_KEY`.

Required.

## PAPERS_S3_BUCKET

The S3 bucket where generated documents and debug output should be uploaded.

Required.

```
Example: my-company-name-papers
```

## PAPERS_AWS_REGION

The AWS region the bucket belongs to.

Required.

```
Example: eu-central-1
```

## PAPERS_S3_EXPIRATION_TIME

The expiration delay on the presigned urls returned to the callback url, in seconds.

```
Default: 86400
```
