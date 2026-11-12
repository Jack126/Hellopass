# Hellopass — Supplemental Documentation

This file collects the detailed content beyond the main README: where your data
lives, building from source, license, and a docs index.

For quick usage, security parameters, screenshots, and planned items, see
[README_EN.md](README_EN.md).

## Where your data lives

```
%APPDATA%\com.hellopass.app\
├── hellopass.db                              # encrypted vault
└── config\
    ├── master_password_config.json           # salt + verification token
    └── hotkey.json                           # hotkey bindings
```

The database file is stored with `0600` permissions. Site names, URLs, and
usernames are stored in plaintext; **passwords and notes are encrypted**.

> Deleting `Hellopass.exe` does **not** delete your data. To fully wipe, remove
> the `%APPDATA%\com.hellopass.app` folder.

---

## Build from source (developers)

### Requirements

- Rust 1.70+
- Node.js 18+
- Tauri 2 CLI
- Linux: `webkit2gtk-4.1` and `libdbus-1-dev`

### Run in development

```bash
npm install --prefix frontend
cargo install tauri-cli --version ^2.0.0
cargo tauri dev
```

### Headless core tests (no GUI)

```bash
cargo test --bin hellopass_cli --no-default-features
```

### Package the Windows portable build (from Linux/WSL)

```bash
./scripts/package-win.sh                 # depends on system WebView2
./scripts/package-win.sh --with-webview2 # bundles an offline WebView2 runtime
```

---

## License

This project is licensed under the **BSD 3-Clause License** — see
[LICENSE](LICENSE). Third-party licenses are listed in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

---

## Docs

- [Security architecture](security-architecture.md)
- [Deployment guide](DEPLOYMENT.md)
- [Project summary](PROJECT_SUMMARY.md)
- [Development task list](todo.md)

---
Last updated: 2026-11-12
