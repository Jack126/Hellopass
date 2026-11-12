# Hellopass — Secure Password Manager

Hellopass is a cross-platform password manager built with Vue 3 + Rust + Tauri 2.
All passwords are encrypted with **AES-256-GCM**, and the master key is derived
from your master password using **PBKDF2-HMAC-SHA256 (600,000 iterations)**.

Current version: **v1.0.1**

> [!IMPORTANT]
> - 🤖 **AI-Generated Project**: All code in this project was generated with AI
>   assistance, for learning, reference, and secondary development only.
> - 💾 **Local-Only Storage**: All data is encrypted with AES-256-GCM and stored
>   **only on your local device** — nothing is uploaded to any server.

---

## 1. Quick Start (Windows portable build)

1. Unzip `Hellopass-<version>-win64.zip`.
2. Double-click **`Hellopass.exe`**.
   - Requires the system **WebView2 Runtime** (built into Windows 11 and recent
     Windows 10). If you use the offline build with a bundled runtime, run
     **`Hellopass.cmd`** instead.
   - The app runs in the **system tray**; no main window pops up.
3. On first launch you'll be asked to set a **master password**:
   - At least **12 characters**, with upper- and lower-case letters, digits, and
     a special character.
4. Press `Ctrl+Alt+Space` to open the search window and start using it.

> The build is **not code-signed**. Windows SmartScreen may show an "unknown
> publisher" warning — click **More info → Run anyway** to proceed.

---

## 2. Basic Usage

### Hotkeys (default)

| Key | Action |
| --- | --- |
| `Ctrl+Alt+Space` | Open the search window |
| `Ctrl+Shift+L` | Lock immediately |
| `↑` / `↓` | Move selection |
| `Enter` | Copy password and collapse (clipboard auto-clears after 30s) |
| `Ctrl+Enter` | Copy username |
| `→` | View entry details |
| `Ctrl+N` | Create a new entry |

- **Search as you type.** Supports pinyin initials, e.g. `wx` → 微信 (WeChat).
- Both hotkeys can be **rebound** in the Settings window (they are temporarily
  released while recording a new combination).

### Tray menu

Right-click the tray icon for: **Search / Lock now / Settings… / Quit**.
Import and export are handled inside the Settings window.

### Settings window

Open it from the tray menu. It includes:

- Window size (auto / compact / standard / large)
- Result count (auto-fill / fixed N)
- Auto-lock duration
- Change master password
- Import / export
- Backup / restore
- Integrity check and audit log

---

## 3. Screenshots

![Launcher](screenshots/2026-11-10_10-34-49.jpg)
![New entry](screenshots/2026-11-10_10-35-10.jpg)
![Settings](screenshots/2026-11-10_10-35-43.jpg)

---

## 4. Security at a glance

- **Encryption:** AES-256-GCM (authenticated, tamper-resistant)
- **Key derivation:** PBKDF2-HMAC-SHA256, 600,000 iterations, 256-bit key
- **Master password:** ≥12 chars, mixed case + digits + special char
- **Session:** idle auto-lock (default 30s, up to 5 min); 5 failed attempts →
  30-minute lockout
- **Sensitive memory:** master key and decrypted buffers are zeroized on free
- **Backups:** encrypted `.spb` export / self-contained encrypted backup

If you forget your master password, the data **cannot be recovered** — export an
encrypted `.spb` backup regularly.

---

## 5. Completed

- **CSV import**: supports this app's export format and browser exports using
  `name, url, username, password, note` headers (case-insensitive, BOM / CRLF /
  quoted newlines accepted). Entries are matched by **name + URL + username**,
  so a site can keep multiple accounts; existing entries are overwritten with
  the new password.

## 6. Planned (not yet implemented)

- **Fixed-format Excel (.xlsx) import (later)**
  - Import password entries from a **fixed template** (columns such as: site
    name / URL / username / password / notes; column names and order to be
    spec'd up front).
  - Reuses the existing "name + URL + username" matching: existing entries are
    overwritten, and added / updated / skipped counts are returned.
  - Plaintext Excel is an import-only convenience channel — **no bulk plaintext
    export** (keeps the current security policy).

---

## License

This project is licensed under the **BSD 3-Clause License** — see
[LICENSE](LICENSE). Third-party licenses are listed in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

---
Last updated: 2026-11-12
