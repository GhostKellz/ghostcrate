# 👻️ GhostCrate

> Self-hosted Rust crate registry & package server – fast, simple, Proxmox-ready, and designed for modern devops and the GhostMesh ecosystem.

---

## 🌟 **Features**

* ⚡ **Easy Self-Hosting**: One-command Docker Compose & systemd helpers
* 🖥️ **Modern Web UI**: Browse/search, manage orgs, see crate stats
* 🚀 **Proxmox-Optimized**: Helper script for instant LXC/VM deployment
* 🛡️ **SSO Authentication**: Azure Entra, GitHub, or OIDC
* 🗄️ **S3/MinIO Storage**: Bring your own storage backend
* 🔄 **Optional Crates.io Mirroring**: Mirror/backup public crates locally
* 🔑 **Fine-Grained Access Control**: Invite codes, org roles, 2FA
* 📦 **Cargo Compatible**: Works out of the box with `cargo publish`/`cargo install`
* 🔗 **API-First**: Full API for automation, admin, and CI/CD
* 📜 **Clear Docs & Scripts**: Dead-simple setup and usage

---

## 🚀 **Quick Start**

### **Docker Compose**

```bash
git clone https://github.com/ghostkellz/ghostcrate
cd ghostcrate
cp .env.example .env      # Edit your settings
docker compose up -d
```

Open [http://localhost:8080](http://localhost:8080) for the Web UI!

### **Proxmox LXC Deployment**

```bash
# One-liner for Proxmox shell
wget https://raw.githubusercontent.com/ghostkellz/ghostcrate/main/scripts/proxmox-lxc.sh -O - | bash
```

---

## 🔐 **Authentication & Security**

* OIDC support (Azure, GitHub, custom)
* Fine-grained permissions: org, user, crate, admin
* 2FA support (optional)
* API tokens for CI/CD & automation

---

## ☁️ **Storage**

* S3-compatible (MinIO, AWS S3, Wasabi, etc.)
* Local disk fallback for quick demo
* Encrypted crate storage
* Custom retention & mirroring

---

## 🔄 **Mirroring / Federation**

* (Optional) mirror crates.io for offline/corporate environments
* Full or selective sync
* Future: Federation with other GhostCrate servers (peer-to-peer registry mesh)

---

## 🖥️ **Web UI**

* Blazing fast Svelte (or React) SPA
* Browse/search, view crate stats, manage organizations
* Admin dashboard: users, orgs, logs
* Token management, invite system, pending approvals

---

## ⚡ **CLI & API**

* 100% Cargo-compatible
* REST/JSON & gRPC APIs for full automation
* CLI helpers for publish, search, admin
* Example usage:

```bash
# Publish a crate
cargo publish --registry ghostcrate

# Install from GhostCrate
cargo install --registry ghostcrate crate_name
```

---

## 🤝 **Proxmox & DevOps**

* One-liner LXC/VM deployment
* Docker & Podman support
* Scripts for HA, backup, auto-upgrade
* Integrates with CI/CD (GitHub Actions, GitLab CI)

---

## 🛠️ **Extensibility**

* Plug-in architecture for custom auth/storage
* Webhooks, audit log streaming
* Future: S3 bucket encryption, audit trails, IPFS integration

---

## 📑 **Documentation**

* [Getting Started](./docs/getting-started.md)
* [Authentication](./docs/authentication.md)
* [Storage Setup](./docs/storage.md)
* [API Reference](./docs/api.md)
* [Mirroring/Federation](./docs/mirroring.md)
* [Proxmox Deployment](./docs/proxmox.md)

---

## 📜 **License**

GhostCrate is released under the MIT License.

---

## 🚧 **Roadmap & Contributing**

* Federation between GhostCrate instances
* Advanced metrics, org billing, and audit
* Custom indexers (for Zig, Python, Go, etc.)
* More SSO providers (GitLab, Okta)
* Native Proxmox UI plugin
* **PRs and ideas welcome!**

