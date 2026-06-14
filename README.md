![App Screenshot](assets/logo.svg)

# Waey

Waey is a screen-aware desktop AI assistant built for people who do not want to stop their flow just to explain what is already visible on their screen.

Open it with a global shortcut, let it capture the current screen, ask your question, and keep working from the same overlay. Waey is designed for quick visual context, coding help, research, debugging, reading, and everyday desktop tasks where switching between apps slows the whole conversation down.

## What Waey Does

Waey sits quietly on your desktop and opens as an always-on-top overlay when you need it. It can attach screenshots, stream model responses, keep local chat history, remember personas, and work with OpenAI-compatible providers.

The goal is simple: make AI feel closer to the work on your screen, not like a separate tab you have to keep feeding with context.

## Features

- Global overlay shortcut with `Alt+Space`
- Region selection shortcut with `Ctrl+Space`
- Full-screen capture when the overlay opens
- Up to 3 screenshots attached to the same message
- OpenAI-compatible provider support
- OpenRouter, Ollama, Groq, and custom base URL support
- Managed default Waey provider bootstrap for first-time users
- Streaming responses
- Local conversation history
- Rename, pin, search, and delete saved chats
- Edit the latest user message and regenerate the answer
- Cancel an active model response
- Personas for reusable system prompts
- Code block rendering with one-click copy
- Light, dark, and system themes
- English and Arabic UI direction support
- Optional launch on startup
- Local SQLite storage for settings, providers, personas, and chats
- Cross-platform Tauri bundle targets

## Product Shape

Waey is not a chatbot window with a screenshot button added later. The product is built around screen context from the start.

The overlay is compact, draggable, resizable, and meant to stay out of the way while still being ready for real work. A user can open Waey, ask about the screen, attach more context, cancel a bad request, edit the last prompt, or return to a pinned chat without leaving the desktop workflow.

## Tech Stack

- Tauri 2
- Rust
- React
- TypeScript
- Tailwind CSS
- SQLite
- Vite

Rust owns the desktop layer, capture flow, storage bridge, tray behavior, startup behavior, and model streaming. React owns the overlay interface, chat workflow, settings, history, and interaction states.

## Getting Started

### Requirements

- Node.js
- Rust
- Tauri prerequisites for your target OS
- A supported LLM provider key, unless you use the managed Waey provider preset

### Install Dependencies

```bash
npm install
```

### Run The App In Development

```bash
npm run app:dev
```

### Build The Renderer

```bash
npm run build
```

### Build Desktop Bundles

```bash
npm run app:build
```

Platform-specific bundle helpers are also available:

```bash
npm run release:windows
npm run release:mac
npm run release:linux
```

## Provider Setup

Waey talks to OpenAI-compatible chat completions endpoints. Add a provider from Settings, then choose it from the overlay.

Use the base URL without `/chat/completions`. Waey appends that path internally.

Common examples:

```text
OpenRouter: https://openrouter.ai/api/v1
Ollama:     http://localhost:11434/v1
Groq:       https://api.groq.com/openai/v1
Custom:     any OpenAI-compatible base URL
```

Each provider stores:

- Name
- Provider type
- Base URL
- Model ID
- API key

Users can keep multiple providers and switch between them without editing config files.

## Local Data

Waey stores user data locally in SQLite:

- Conversations
- Messages
- Providers
- Personas
- Settings

API keys added by the user are stored locally so Waey can work as a normal desktop app. The managed Waey provider is hidden from the settings list to keep first-run usage simple.

## Current Architecture

```text
React overlay
  -> Tauri commands
  -> Rust services
  -> SQLite, screen capture, startup integration, LLM streaming
  -> streamed events back to React
```

The app is split around clear responsibilities:

- `src/components` contains UI surfaces.
- `src/features` contains frontend feature workflows.
- `src/shared` contains shared TypeScript types and defaults.
- `src-tauri/src` contains Rust commands, persistence, capture, providers, settings, and LLM streaming.
- `.github/workflows` builds and releases the app through GitHub Actions.

## Keyboard Shortcuts

```text
Alt+Space   Open Waey overlay
Ctrl+Space  Open region selector
Esc         Close the active overlay flow
```

## Privacy Notes

Waey only sends the prompt, selected conversation context, and attached screenshots to the provider selected by the user. Local history and settings stay on the device.

Because providers are configurable, privacy and retention behavior also depend on the provider used. For sensitive work, choose a provider you trust or use a local provider such as Ollama.

## Development Notes

Useful scripts:

```bash
npm run dev
npm run build
npm run app:dev
npm run app:build
```

The frontend build runs before the Tauri bundle step through `tauri.conf.json`.

## Project Status

Waey is in active development. The current release line focuses on stability, polished settings behavior, local-first history, provider flexibility, and a fast screen-aware overlay experience.
