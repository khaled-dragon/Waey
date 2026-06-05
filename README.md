# Waey

Waey is a Tauri 2 desktop assistant that opens over any app, captures screen context,
and lets the user ask an LLM about what they are seeing.

## Stack

- Tauri 2 + Rust for the desktop shell, hotkeys, capture, packaging, and storage bridge
- React + TypeScript for the UI
- Tailwind CSS for styling
- SQLite for local conversations, providers, personas, and settings

## Scripts

- `npm run dev` starts the Vite renderer.
- `npm run build` builds the renderer.
- `npm run app:dev` starts the Tauri app.
- `npm run app:build` builds desktop bundles.

Rust is required for `app:dev` and `app:build`.

## Provider URLs

Waey uses OpenAI-compatible chat completions endpoints. Save the provider base URL
without `/chat/completions`; the Rust client appends it automatically.

- OpenRouter: `https://openrouter.ai/api/v1`
- Ollama: `http://localhost:11434/v1`
