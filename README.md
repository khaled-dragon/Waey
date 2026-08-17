![App Screenshot](assets/Waey.png)

Waey is a screen-aware desktop AI assistant for people who do not want to pause their flow just to explain what is already visible on their screen.

Open it with a global shortcut, ask a question, and keep working from the same overlay. Waey can combine optional screenshots with fresh, readable screen context so a model can reason about the active app, visible controls, selected text, and the element under the cursor without asking the user to reconstruct the screen in prose.

## What Waey Does

Waey sits quietly on your desktop and opens as an always-on-top overlay when you need it. It streams model responses, keeps local chat history, remembers personas, works with OpenAI-compatible providers, and keeps screen context separate from image attachments so text-only models can still use the visible desktop structure.

The goal is simple: make AI feel closer to the work on your screen, not like a separate tab you have to keep feeding with context.

## Features

- Global overlay shortcut with `Alt+Space` by default
- Region selection shortcut with `Ctrl+Space` by default
- Full-screen capture when the overlay opens
- Up to 3 screenshots attached to the same message
- Fresh readable screen context on Windows for each prompt, including visible windows, active app, controls, bounds, focused or pointed elements, and selected text when available
- Screenshot and readable screen context routed independently, so removing an image does not remove text context
- Interactive Guide Mode that presents one verified step at a time, highlights a target when it is available, recaptures context after confirmation, and stays out of the user's way between steps
- OpenAI-compatible provider support
- OpenRouter, Ollama, Groq, and custom base URL support
- Managed default Waey provider bootstrap for first-time users
- Signed in-app updates with manual checks from Settings
- Streaming responses
- Local conversation history
- Rename, pin, search, and delete saved chats
- Edit the latest user message and regenerate the answer
- Cancel an active model response
- Personas for reusable system prompts
- Code block rendering with one-click copy
- Optional Developer Mode for focused local code and spreadsheet context from allowed workspaces
- Developer access levels for approved reads, edits, and new workspace files
- Structured spreadsheet summaries and bounded spreadsheet edits
- Worked timer for longer model responses
- Light, dark, and system themes
- English and Arabic UI direction support
- Optional launch on startup
- Local SQLite storage for settings, providers, personas, and chats
- Cross-platform Tauri bundle targets

## Product Shape

Waey is not a chatbot window with a screenshot button added later. The product is built around screen context from the start.

The overlay is compact, draggable, resizable, and meant to stay out of the way while still being ready for real work. A user can open Waey, ask about the screen, attach more context, cancel a bad request, edit the last prompt, or return to a pinned chat without leaving the desktop workflow.

Waey combines visual capture with structured desktop context. A screenshot helps a model understand layout, graphics, code, charts, images, and visual state. The readable UI structure gives it precision around visible controls, their bounds, the active application, selected text, and the element under the pointer. This hybrid approach makes UI-heavy screens easier to reason about, while still supporting text-only providers.

## Guide Mode

Guide Mode turns a multi-step desktop task into a focused, one-step interaction. After a user enables **Guide** in the composer, Waey asks the selected model for a structured guide response rather than a long set of instructions.

For each step, Waey shows a movable guide card with the Waey mascot, the current instruction, and a target highlight when the active screen has a reliable matching UI element. The user can confirm the step, adjust the request, or cancel. Confirmation captures fresh screen context before requesting the next step, so the guide can react to what actually changed instead of assuming a fixed path.

Guide Mode never clicks, types, or changes desktop settings on the user's behalf. It guides the user through the current screen and keeps the primary overlay hidden while a guide step is active.

## Developer Mode

Developer Mode is optional and off by default. It is built for quick coding help when a screenshot is not enough, such as asking about an error in the current editor, checking a component, or getting a focused fix for a file that is already open.

When enabled, the user chooses allowed workspace folders from the composer. Waey can then attach focused code, selected-text, file, or spreadsheet context from those folders to the next prompt, while ignoring heavy folders such as `node_modules`, `.git`, `target`, `dist`, and `build`. It also avoids secret-like files such as `.env`, `.pem`, and `.key`.

Access levels are available from the chat bar:

- `Ask for approval`: asks before reading workspace files or applying edits.
- `Approve for me`: reads allowed workspaces and asks before file edits.
- `Full access`: can apply validated Waey edit blocks inside allowed workspaces.

For edits, Waey resolves every target beneath an allowed workspace, blocks secret-like files and path escapes, verifies an existing file has not changed since it was read, and applies only validated structured file or spreadsheet actions. It can create new files only inside an allowed workspace. The feature is intended for fast, focused help, not for replacing a full coding agent.

## Tech Stack

- Tauri 2
- Rust
- React
- TypeScript
- Tailwind CSS
- SQLite
- Vite

Rust owns the desktop layer, capture flow, readable UI context, storage bridge, tray behavior, startup behavior, shortcut registration, and model streaming. React owns the overlay interface, chat workflow, settings, history, and interaction states.

## Getting Started

### Use Waey

You do not need Node.js, Rust, or a local development setup to use Waey.

Download the latest installer from the GitHub Releases page, install it, and open Waey. The desktop app is already bundled and ready to run.

First-time users can start with the managed Waey provider preset. Users who prefer their own models can add an OpenRouter, Ollama, Groq, or custom OpenAI-compatible provider from Settings.

Waey checks for signed application updates when it starts. Users can also check from Settings and choose when to install an available update.

### Develop Locally

You only need the following tools if you want to run the source code, change Waey, or build the desktop bundles yourself:

- Node.js
- Rust
- Tauri prerequisites for your target OS

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

API keys added by the user are stored locally so Waey can work as a normal desktop app. The managed Waey provider is available immediately for first-run use while its key remains hidden from Settings.

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
- `src-tauri/src` contains Rust commands, persistence, capture, Windows UI Automation context, guide-window coordination, developer workspace actions, spreadsheets, providers, settings, shortcuts, and LLM streaming.
- `.github/workflows` builds and releases the app through GitHub Actions.

## Keyboard Shortcuts

```text
Alt+Space   Open Waey overlay by default
Ctrl+Space  Open region selector by default
Esc         Close the active overlay flow
```

## Privacy Notes

Waey only sends the prompt, selected conversation context, optional screenshots, and optional readable screen structure to the provider selected by the user. Local history and settings stay on the device.

Readable screen structure is designed to be conservative. It is optional, refreshed for each enabled prompt, and filters sensitive password fields before anything reaches the selected model provider. Screenshots remain separately optional. On non-Windows systems, Waey falls back to screenshot-based context without breaking the app.

Developer Mode can send selected local code context from user-approved workspace folders to the chosen model provider. If the user allows a model to read or edit local files, the user is responsible for reviewing the output, choosing a trusted provider, and understanding any impact of those actions. Use it carefully and only with folders you are comfortable exposing to the selected model.

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

Waey is in active development. The current release line focuses on reliable Windows UI Automation, interactive screen guidance, controlled local developer workflows, provider flexibility, and a fast screen-aware overlay experience.
