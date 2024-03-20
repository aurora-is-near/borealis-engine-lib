FROM rust as build

ARG ENV=mainnet
ARG SOURCE=nearcore
ARG RELEASE_TAG

RUN apt update && apt install make pkg-config libssl-dev clang -y  \
    && mkdir -p /src && cd /src  \
    && git clone https://github.com/aurora-is-near/borealis-engine-lib.git .  \
    && git fetch --tags  \
    && TAG=${RELEASE_TAG:-$(git describe --tags `git rev-list --tags --max-count=1`)} \
    && git checkout "$TAG"  \
    && mkdir -p output/refiner  \
    && cargo build --release  \
    && echo "branch: $(git rev-parse --abbrev-ref HEAD)" > target/release/build.info  \
    && echo "commit: $(git rev-parse HEAD)" >> target/release/build.info  \
    && echo "tag: $(git describe --tag --exact-match $(git rev-parse HEAD) 2>/dev/null)" >> target/release/build.info  \
    && sed -i "s/$TAG/$TAG/g" target/release/build.info

RUN mkdir -p /standalone-rpc && cd /standalone-rpc && git clone https://github.com/aurora-is-near/standalone-rpc.git .

FROM ubuntu:latest

COPY --from=build /src/target/release/build.info /version
COPY --from=build /src/target/release/aurora-refiner /usr/local/bin/
COPY --from=build /standalone-rpc/contrib/config/build/refiner.sh /docker-entrypoint.sh
COPY --from=build /standalone-rpc/contrib/config/build/common.sh /common.sh

RUN apt update  \
    && apt install ca-certificates curl jq -y \
    && mkdir -p /config /engine /near /data /log

VOLUME /config
VOLUME /engine
VOLUME /near
VOLUME /data
VOLUME /log

ENTRYPOINT ["/docker-entrypoint.sh"]
