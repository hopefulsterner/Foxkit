# 🦊 Foxkit

### **Theia × Zed = Foxkit**
> A next-generation, AI-native monorepo development platform

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript"/>
  <img src="https://img.shields.io/badge/AI_Native-FF6B6B?style=for-the-badge" alt="AI Native"/>
  <img src="https://img.shields.io/badge/License-Apache_2.0-blue?style=for-the-badge" alt="License"/>
</p>

---

## 🧬 DNA

Foxkit is a **hybrid breed** combining the best of two worlds:

| Parent | Contribution |
|--------|-------------|
| **[Theia](https://github.com/eclipse-theia/theia)** | Cloud-native architecture, VS Code extension compatibility, modular design |
| **[Zed](https://github.com/zed-industries/zed)** | Rust performance, GPUI rendering, built-in collaboration, AI assistant |

The result? **Foxkit** - a unified, intelligent software engineering platform.

---

## ✨ Key Features

### 🧠 AI-Native by Design
AI is not a plugin - it's a **core system layer**:
- Multi-provider support (Anthropic, OpenAI, Azure, Ollama)
- Monorepo-aware context building
- Autonomous agent mode for complex tasks
- Built-in tools: read/write files, search, run commands

### 📦 Monorepo Intelligence
Foxkit's unique superpower - understanding entire codebases:
- Automatic package detection (npm, Cargo, Go, Python, Java, etc.)
- Dependency graph visualization
- Impact analysis ("what breaks if I change this?")
- Optimal build ordering
- Cross-package navigation

### ⚡ Blazing Performance
Built in Rust with GPU-accelerated UI:
- Handles massive monorepos without lag
- Instant file switching
- Real-time syntax highlighting
- Native performance on desktop, optimized WASM for web

### 🤝 Real-time Collaboration
Built-in, not bolted-on:
- CRDT-based real-time editing
- Multi-cursor support
- Shared terminals and debugging sessions
- Presence indicators

### 🔌 Universal Extension System
Best of both worlds:
- VS Code extension compatibility (via Theia DNA)
- Native Rust plugins for performance
- WASM sandboxed extensions
- Secure permission model

---

## 📁 Project Structure

```
foxkit-core/              # The hybrid core
├── Cargo.toml            # Workspace manifest
├── crates/
│   ├── foxkit/           # Main application
│   ├── foxkit-core/      # Core foundation (DI, events, settings)
│   ├── foxkit-gpui/      # GPU-accelerated UI (planned)
│   │
│   ├── monorepo/         # 🦊 Monorepo intelligence
│   │   ├── detector.rs   # Multi-language package detection
│   │   ├── graph.rs      # Dependency graph
│   │   ├── impact.rs     # Impact analysis
│   │   └── package.rs    # Package model
│   │
│   ├── ai-core/          # 🧠 AI native layer
│   │   ├── agent.rs      # Autonomous AI agent
│   │   ├── context.rs    # Monorepo-aware context builder
│   │   ├── providers.rs  # LLM providers (Anthropic, OpenAI, etc.)
│   │   └── tools.rs      # AI-callable tools
│   │
│   ├── editor/           # Editor core (planned)
│   ├── terminal/         # Terminal emulator (planned)
│   ├── collab/           # Collaboration (planned)
│   └── extension-host/   # Extension system (planned)
│
├── theia-base/           # Reference: Theia source
└── zed-base/             # Reference: Zed source
```

---

## 🚀 Getting Started

### Prerequisites
- Rust 1.75+ (edition 2024)
- Node.js 20+ (for Theia reference)

### Build

```bash
cd foxkit-core
cargo build
```

### Run

```bash
cargo run --bin foxkit
```

---

## 🗺️ Roadmap

### Phase 1: Foundation ✅
- [x] Core architecture (DI, events, settings)
- [x] Monorepo intelligence engine
- [x] AI core with multi-provider support
- [x] Package detection for all major languages

### Phase 2: Editor Core 🚧
- [ ] GPU-accelerated text rendering (GPUI)
- [ ] Rope-based text buffer
- [ ] Tree-sitter syntax highlighting
- [ ] LSP client integration

### Phase 3: Platform
- [ ] Terminal emulator
- [ ] Extension host (VS Code compat)
- [ ] Native desktop app (Electron alternative)
- [ ] Web version (WASM)

### Phase 4: Collaboration
- [ ] CRDT implementation
- [ ] Real-time presence
- [ ] Shared debugging

### Phase 5: Ecosystem
- [ ] Marketplace
- [ ] Cloud workspaces
- [ ] Enterprise features

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    FOXKIT APPLICATION                        │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   AI LAYER  │  │   COLLAB    │  │  MONOREPO INTEL     │  │
│  │  (Agents)   │  │   (CRDT)    │  │  (Dependency Graph) │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐│
│  │              WORKSPACE & EDITOR CORE                    ││
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐           ││
│  │  │ Buffer │ │ Editor │ │Terminal│ │  Task  │           ││
│  │  │ (Rope) │ │ (View) │ │ (PTY)  │ │ Runner │           ││
│  │  └────────┘ └────────┘ └────────┘ └────────┘           ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐│
│  │              EXTENSION SYSTEM                           ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐                ││
│  │  │ VS Code  │ │   WASM   │ │  Native  │                ││
│  │  │  Compat  │ │  Plugins │ │  Plugins │                ││
│  │  └──────────┘ └──────────┘ └──────────┘                ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    FOXKIT GPUI                          ││
│  │         GPU-Accelerated + Web-Compatible UI             ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Native    │  │     Web     │  │      Remote         │  │
│  │  (Desktop)  │  │  (Browser)  │  │   (Cloud/SSH)       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone with submodules
git clone https://github.com/scrapyfox/Foxkit.git
cd Foxkit

# Study the reference implementations
ls theia-base/packages/     # Theia's modular architecture
ls zed-base/crates/         # Zed's Rust implementation

# Build Foxkit
cd foxkit-core
cargo build
```

---

## 📜 License

Apache 2.0 - See [LICENSE](LICENSE) for details.

---

## 🦊 Philosophy

> "This is not another VS Code. It's a unified, intelligent software engineering platform built for the future of large-scale development."

Foxkit is designed to:
- **Reduce cognitive load** - AI handles complexity
- **Replace fragmented tooling** - One platform for everything  
- **Make large codebases understandable** - Monorepo intelligence
- **Turn developers into system architects** - Focus on creativity, not mechanics

---

<p align="center">
  <b>Built with 🦊 by the Foxkit Team</b>
</p>
