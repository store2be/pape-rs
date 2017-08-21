# Release Checklist

## Bump the version in [Cargo.toml](Cargo.toml)


## Update the [changelog](CHANGELOG.md)

Add a new heading for the unreleased changes.


## Build binaries

```
$ cargo build --release
```

* `x86_64-linux-gnu`
* `x86_64-apple-darwin`


## Build the Docker image

```
$ docker build --no-cache=true -f docker/Dockerfile -t store2be/pape-rs:<version> .
```


## Upload the Docker image to Docker Hub

```
$ docker push store2be/pape-rs:<version>
$ docker push store2be/pape-rs  # for the 'latest' tag
```


## Git tag and push

```
$ git tag x.x.x && git push origin --tags
```


## Create a Github release

* Change the release title to `x.x.x - YYYY-MM-DD`
* Add the entries from the [changelog](CHANGELOG.md)
* Upload the new binaries
* Add the checksums of the binaries (`$ md5sum target/release/papers-local`)
