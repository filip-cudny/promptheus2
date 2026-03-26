# Promptheus

Desktop application built with Tauri 2, Svelte 5, and Rust.

## Prerequisites

| Tool | Version | Check |
|------|---------|-------|
| Node.js | >= 18 | `node --version` |
| pnpm | >= 9 | `pnpm --version` |
| Rust | latest stable | `rustc --version` |

### Install Node.js

```bash
# Option 1: nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
nvm install 22

# Option 2: fnm
curl -fsSL https://fnm.vercel.app/install | bash
fnm install 22
```

### Install pnpm

```bash
corepack enable
corepack prepare pnpm@latest --activate
```

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Restart your terminal or run:
source "$HOME/.cargo/env"
```

## System dependencies

### macOS

```bash
xcode-select --install
```

### Linux (Debian / Ubuntu)

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev
```

### Linux (Fedora)

```bash
sudo dnf install -y \
  gcc-c++ \
  openssl-devel \
  gtk3-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  webkit2gtk4.1-devel \
  javascriptcoregtk4.1-devel \
  libsoup3-devel
```

### Linux (Arch)

```bash
sudo pacman -S --needed \
  base-devel \
  openssl \
  gtk3 \
  libappindicator-gtk3 \
  librsvg \
  webkit2gtk-4.1 \
  libsoup3
```

## Getting started

```bash
# 1. Install frontend dependencies
pnpm install

# 2. Run in development mode (opens the app window)
pnpm tauri dev
```

The first run will compile the Rust backend — this takes a few minutes. Subsequent runs are fast thanks to incremental compilation.

## Build for production

```bash
pnpm tauri build
```

The bundled application will be in `src-tauri/target/release/bundle/`.

## Project structure

```
promptheus-tauri/
├── src/                  # Svelte 5 frontend (TypeScript)
│   ├── components/       # UI components
│   ├── services/         # Frontend service layer
│   ├── stores/           # State management (Svelte 5 runes)
│   └── main.ts           # App entry point
├── src-tauri/            # Rust backend
│   ├── src/
│   │   ├── commands/     # Tauri command handlers
│   │   ├── models/       # Data models
│   │   ├── services/     # Backend business logic
│   │   └── lib.rs        # App setup
│   ├── Cargo.toml        # Rust dependencies
│   └── tauri.conf.json   # Tauri configuration
├── package.json          # Frontend dependencies & scripts
├── vite.config.ts        # Vite bundler config
└── svelte.config.js      # Svelte preprocessor config
```

## Available scripts

| Command | Description |
|---------|-------------|
| `pnpm dev` | Start Vite dev server (frontend only) |
| `pnpm build` | Build frontend for production |
| `pnpm tauri dev` | Run full app in development mode |
| `pnpm tauri build` | Build production desktop app |
