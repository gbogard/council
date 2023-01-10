FROM rust:1.66.0-slim-bullseye

# accelerate file sharing in vscode MAC/WINDOS
ENV CARGO_TARGET_DIR="/tmp/target"

# OS dependencies
RUN apt update
RUN apt install curl -y

RUN curl -fsSL https://deb.nodesource.com/setup_17.x | bash -
RUN apt update
RUN apt install -y \
    git \
    zsh

# Install the GDB Debugger
RUN apt install build-essential -y
RUN apt install gdb -y

#################################################################
# Create the user
ARG USERNAME=devcontainer-user
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && apt install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

RUN mkdir -p /home/devcontainer-user/.cargo && chown -R devcontainer-user /home/devcontainer-user/.cargo

# Set the default user. Omit if you want to keep the default as root.
USER $USERNAME

#################################################################

# Install Oh-my-ZSH
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended

RUN rustup component add clippy
RUN rustup toolchain install nightly --component rustfmt  

RUN cargo install mdbook
RUN cargo install mdbook-mermaid
RUN cargo install mdbook-admonish
RUN cargo install just
RUN cargo install cargo-deny
