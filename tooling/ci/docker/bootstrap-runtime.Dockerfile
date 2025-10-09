# syntax=docker/dockerfile:1.6

FROM --platform=linux/amd64 ubuntu:22.04

LABEL maintainer="Reml Project" \
      org.opencontainers.image.source="https://github.com/dolphilia/kestrel" \
      org.opencontainers.image.description="Reml Phase 1 bootstrap toolchain for x86_64 Linux" \
      org.opencontainers.image.licenses="MIT"

ARG DEBIAN_FRONTEND=noninteractive
ARG LLVM_VERSION=18
ARG LLVM_APT_URL="https://apt.llvm.org/llvm.sh"
ARG USERNAME=reml
ARG USER_UID=1000
ARG USER_GID=1000

ENV TZ=Etc/UTC \
    OPAMYES=1 \
    OPAMROOT=/home/${USERNAME}/.opam \
    OPAMCOLOR=never \
    OPAMNOENVNOTICE=1

# --- 基本パッケージと LLVM リポジトリの追加 ---
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      gnupg \
      software-properties-common \
      build-essential \
      wget \
      git \
      rsync \
      pkg-config \
      unzip \
      xz-utils \
      m4 \
      libgmp-dev \
      libffi-dev \
      libunwind-dev \
      python3 \
      python3-pip \
      lsb-release \
      valgrind \
      zlib1g-dev \
      libedit-dev \
      cmake \
      tzdata && \
    rm -rf /var/lib/apt/lists/*

# apt.llvm.org のスクリプトを利用して LLVM を導入
RUN curl -fsSL "${LLVM_APT_URL}" -o /tmp/llvm.sh && \
    chmod +x /tmp/llvm.sh && \
    /tmp/llvm.sh ${LLVM_VERSION} all && \
    rm /tmp/llvm.sh && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
      "llvm-${LLVM_VERSION}-dev" \
      libzstd-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# LLVM バイナリを汎用名で参照できるようシンボリックリンクを作成
RUN ln -sf /usr/bin/clang-${LLVM_VERSION} /usr/local/bin/clang && \
    ln -sf /usr/bin/clang++-${LLVM_VERSION} /usr/local/bin/clang++ && \
    ln -sf /usr/bin/llvm-config-${LLVM_VERSION} /usr/local/bin/llvm-config && \
    ln -sf /usr/bin/llvm-ar-${LLVM_VERSION} /usr/local/bin/llvm-ar && \
    ln -sf /usr/bin/llvm-as-${LLVM_VERSION} /usr/local/bin/llvm-as && \
    ln -sf /usr/bin/llvm-nm-${LLVM_VERSION} /usr/local/bin/llvm-nm && \
    ln -sf /usr/bin/llvm-objdump-${LLVM_VERSION} /usr/local/bin/llvm-objdump && \
    ln -sf /usr/bin/llc-${LLVM_VERSION} /usr/local/bin/llc && \
    ln -sf /usr/bin/opt-${LLVM_VERSION} /usr/local/bin/opt

# 非特権ユーザーを作成
RUN groupadd --gid ${USER_GID} ${USERNAME} && \
    useradd --uid ${USER_UID} --gid ${USER_GID} --home-dir /home/${USERNAME} --create-home --shell /bin/bash ${USERNAME}

# opam, dune 等をインストール
RUN apt-get update && \
    apt-get install -y --no-install-recommends opam bubblewrap && \
    rm -rf /var/lib/apt/lists/*

USER ${USERNAME}
WORKDIR /home/${USERNAME}

# opam 初期化とスイッチ作成
RUN opam init --bare --disable-sandboxing && \
    opam switch create 5.2.1 ocaml-base-compiler.5.2.1 && \
    eval "$(opam env --switch 5.2.1 --set-switch)" && \
    opam install -y dune menhir llvm odoc

ENV PATH="/home/${USERNAME}/.opam/5.2.1/bin:${PATH}" \
    LLVM_CONFIG=/usr/local/bin/llvm-config

# コンテナ内の作業ディレクトリを準備
USER root
RUN mkdir -p /workspace && \
    mkdir -p /workspace/_docker_cache && \
    chmod -R 775 /workspace && \
    chown -R ${USER_UID}:${USER_GID} /workspace

USER ${USERNAME}
WORKDIR /workspace

# キャッシュ用ディレクトリ（opam/ccache 等）の権限調整
VOLUME ["/workspace/_docker_cache"]

CMD ["bash"]
