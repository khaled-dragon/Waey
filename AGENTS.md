# AGENTS.md

This file defines the engineering rules for Waey. Follow it for every change.

## Code Style

- Write clean, readable, production-oriented code.
- Keep one consistent pattern across files in the same layer.
- Prefer explicit names over generic names. Objects, functions, components, and modules must describe their exact responsibility.
- Keep components and modules focused. Split code when a file starts mixing unrelated responsibilities.
- Avoid clever code when straightforward code is easier to maintain.

## Comments

- Do not add comments by default.
- Add a comment only when the code protects an important invariant, handles a non-obvious platform behavior, or would be risky for a future maintainer to change without context.
- Never write comments that merely repeat what the code already says.

## Project Structure

- Keep the structure aligned with common Tauri, React, and TypeScript conventions.
- `src/components` is for reusable UI pieces.
- `src/features` is for product feature modules such as overlay, capture, providers, history, personas, and settings.
- `src/shared` is for shared types, constants, and pure utilities.
- `src/styles` is for global styles and design tokens.
- `src-tauri/src` is for Rust desktop capabilities, commands, plugins, and OS integration.

## Engineering Standard

- Treat every implementation as if another senior engineer will maintain it.
- Keep API boundaries clear between React UI and Tauri/Rust commands.
- Validate user-controlled inputs before they reach OS-level behavior.
- Prefer small, composable modules over large catch-all files.
