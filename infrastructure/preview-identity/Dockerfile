# Temporary loopback-only source build. Return to a qualified upstream digest
# when Keycloak ships the fix tracked in erp4us #121.
FROM registry.access.redhat.com/ubi9/ubi@sha256:6636b79fba9e5044360366f189333c30b5ce528efd92c40c68b074592b971fcd AS source
SHELL ["/bin/bash", "-e", "-u", "-o", "pipefail", "-c"]
RUN dnf install -y git-core tar gzip unzip findutils \
    && dnf clean all

RUN case "$(uname -m)" in \
        x86_64) arch=x64; jdk_sha=bb8a93f9ae6e7a693b2b721e85dbeff26ae1a77447985a8d7afcdd9c7cf6222e ;; \
        aarch64) arch=aarch64; jdk_sha=8c72816a6b308842d5ed1f13e5826b69c567da4784f7238d7e800efa71fe54b1 ;; \
        ppc64le) arch=ppc64le; jdk_sha=4d3d75690785cf593002446005427bb3c61de163f2469343572cdcb0d258ed62 ;; \
        s390x) arch=s390x; jdk_sha=cb4cc8f52538c9a0e2bccdab5ea5e7cb1b14c1731e752f1f764ccc376fcb2225 ;; \
        *) echo 'No verified build JDK for this architecture' >&2; exit 2 ;; \
    esac \
    && curl -fsSL --retry 3 -o /tmp/jdk.tgz "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.13.1%2B2/OpenJDK21U-jdk_${arch}_linux_hotspot_21.0.13.1_2.tar.gz" \
    && printf '%s  %s\n' "$jdk_sha" /tmp/jdk.tgz | sha256sum --check \
    && mkdir /opt/build-jdk \
    && tar -xzf /tmp/jdk.tgz --strip-components=1 -C /opt/build-jdk \
    && curl -fsSL --retry 3 -o /tmp/maven.zip https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.16/apache-maven-3.9.16-bin.zip \
    && printf '%s  %s\n' ed41650d42485cfc243fad22158caf9cbb5dc408ce7a09ddb94dd42a019de929ca43065bfa450612cf12bf78b5cafa3884b96c090de326ff590448c933454af3 /tmp/maven.zip | sha512sum --check \
    && unzip -q /tmp/maven.zip -d /opt \
    && rm /tmp/jdk.tgz /tmp/maven.zip
ENV JAVA_HOME=/opt/build-jdk \
    PATH=/opt/build-jdk/bin:$PATH \
    CI=true \
    WIREIT_PARALLEL=1 \
    MAVEN_OPTS="-Xmx1150m -XX:ActiveProcessorCount=2 -Djava.awt.headless=true" \
    NODE_OPTIONS=--max-old-space-size=1096

WORKDIR /src
RUN git init --quiet \
    && git fetch --depth=1 https://github.com/keycloak/keycloak.git refs/tags/26.8.2 \
    && test "$(git rev-parse FETCH_HEAD)" = e514445f9dcdabc5bdcf67cab56c1e49655de067 \
    && git checkout --quiet --detach FETCH_HEAD
COPY guava-dependency.patch /tmp/guava-dependency.patch
# Kiota generates the upstream TypeScript admin client and needs ICU at build time.
RUN dnf install -y libicu && dnf clean all
RUN git apply --check /tmp/guava-dependency.patch \
    && git apply /tmp/guava-dependency.patch \
    && /opt/apache-maven-3.9.16/bin/mvn --batch-mode --no-transfer-progress -T1 \
        -pl quarkus/deployment,quarkus/dist -am -DskipTests clean install

# Require the reviewed Guava bytes in the archive and reject old/extra versions.
RUN archive=/src/quarkus/dist/target/keycloak-26.8.2.tar.gz \
    && tar -tzf "$archive" > /tmp/dist-contents \
    && grep -E '(^|/)(com\.google\.guava\.)?guava-[0-9].*\.jar$' /tmp/dist-contents >/tmp/guava-entries \
    && while IFS= read -r jar; do \
         [[ "$jar" == *guava-33.8.0-jre.jar ]]; \
         digest=$(tar -xOzf "$archive" -- "$jar" | sha256sum); \
         test "${digest%% *}" = 97b79ec3f2c0a6313a0c2df51ce1abdf8907e6531803467b81efcaaf99b35161; \
         printf 'Verified Guava: %s\n' "$jar"; \
       done < /tmp/guava-entries

# Use the upstream runtime assembly without inheriting the affected server layers.
FROM registry.access.redhat.com/ubi9/ubi@sha256:6636b79fba9e5044360366f189333c30b5ce528efd92c40c68b074592b971fcd AS assemble
RUN dnf install -y tar gzip
COPY --from=source /src/quarkus/dist/target/keycloak-26.8.2.tar.gz /tmp/keycloak.tar.gz
COPY --from=source /src/quarkus/container/ubi-null.sh /tmp/ubi-null.sh
RUN mkdir /opt/keycloak \
    && tar -xzf /tmp/keycloak.tar.gz --strip-components=1 -C /opt/keycloak \
    && mkdir -p /opt/keycloak/data \
    && chmod -R g+rwX /opt/keycloak \
    && bash /tmp/ubi-null.sh java-21-openjdk-headless glibc-langpack-en findutils

FROM registry.access.redhat.com/ubi9/ubi-micro@sha256:dbd325b96bcd432dd42816d31412bdf1dc0fb973b79ed310344bc23c76c068db
ENV LANG=en_US.UTF-8 KC_RUN_IN_CONTAINER=true
COPY --from=assemble /tmp/null/rootfs/ /
COPY --from=assemble --chown=1000:0 /opt/keycloak /opt/keycloak
RUN echo "keycloak:x:0:root" >> /etc/group \
    && echo "keycloak:x:1000:0:keycloak user:/opt/keycloak:/sbin/nologin" >> /etc/passwd
LABEL org.opencontainers.image.title="erp4us loopback preview identity" \
      org.opencontainers.image.source="https://github.com/keycloak/keycloak" \
      org.opencontainers.image.revision="e514445f9dcdabc5bdcf67cab56c1e49655de067" \
      org.opencontainers.image.version="26.8.2-erp4us-guava33.8.0"
USER 1000
EXPOSE 8080 8443 9000
ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
