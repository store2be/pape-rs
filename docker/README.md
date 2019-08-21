# Papers on Docker

## Build the image

```bash
export DATETIME=$(date -u +"%Y-%m-%dT%H-%M-%S")
export TAG=store2be/pape-rs
export TAG_DATETIME=$TAG:$DATETIME

docker build -t store2be/pape-rs-base -f docker/Dockerfile.base .
docker build -t store2be/pape-rs-test -f docker/Dockerfile.test .
docker run --rm -it -v `pwd`:/papers -v `pwd`/docker/target:/papers/target store2be/pape-rs-test:latest cargo build --release
docker build --no-cache -t $TAG_DATETIME -f docker/Dockerfile .
docker tag $TAG_DATETIME $TAG # Tag the image also as `latest`

docker push $TAG_DATETIME
docker push $TAG
```
