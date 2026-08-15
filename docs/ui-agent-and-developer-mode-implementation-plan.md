# Waey Screen Intelligence and Developer Agent Plan

## Delivery Tracker

- [x] Task 1: Establish the versioned Screen Intelligence contracts, diagnostics, and rollout modes. Commit: `3faf5ad`.
- [x] Task 2: Rebuild the bounded Windows UIA collector with normalization, sensitive-data filtering, and richer screen metadata.
- [x] Task 3: Route fresh screen context and screenshots independently through the provider request pipeline.
- [x] Task 4: Build the stateful Guide Mode overlay, mascot targeting, confirmation, and recapture loop.
- [x] Task 5: Harden Developer Mode context, selected-text capture, workspace actions, and cross-platform fallback behavior.

## Purpose

Build a reliable, privacy-aware screen intelligence and developer workflow for Waey. The result should let supported models understand the active desktop through both a readable accessibility tree and an optional screenshot, guide a user through multi-step tasks, and safely read or edit explicitly approved local workspaces.

This is a clean-room implementation plan. It reproduces product capabilities and reliability patterns, not another project's source code, prompts, assets, or proprietary implementation details.

## Product Outcomes

- Every prompt can carry a fresh, compact, readable description of the active screen when Screen Context is enabled, whether or not the user attaches an image.
- Vision-capable models receive screenshots plus semantic UI context. Text-only and local models still receive useful UI context.
- Developer Mode can inspect approved workspaces, understand the active file and selected text when available, propose narrow changes, create files, and apply changes according to the selected approval level.
- Guided tasks are stateful. The model proposes one step, the user confirms completion, Waey captures fresh context, and only then requests the next step.
- The overlay stays responsive. Screen collection, filesystem reads, image preparation, and network requests never block the UI thread.

## Non-Negotiable Boundaries

- Do not put API keys, provider secrets, raw clipboard data, full disk paths outside approved workspaces, or unredacted sensitive field values in logs, history, or screen context.
- Never grant filesystem access from a model instruction. Access is determined locally from user-selected workspaces and the active access level.
- Never execute shell commands as part of this feature. File reads and writes remain explicit, validated local operations.
- Never let a model select arbitrary files by absolute path. Resolve paths beneath a canonical approved workspace and reject traversal, symlinks that escape it, device paths, and protected roots.
- Preserve current chat, provider, settings, update, capture, and history behavior while the new pipeline is introduced behind feature flags.

## Target Architecture

### 1. Screen Intelligence Domain

Create a dedicated Rust module group under `src-tauri/src/screen_intelligence/`:

- `mod.rs`: public commands and shared orchestration.
- `uia_windows.rs`: Windows UI Automation collection. This is the only platform-specific collector in phase one.
- `model.rs`: typed snapshots, elements, bounds, collection diagnostics, and redaction metadata.
- `normalizer.rs`: removes empty scaffold nodes, normalizes names, caps values, and canonicalizes bounds.
- `selector.rs`: determines the element under the cursor, focused element, active window, and selected-text candidates.
- `serializer.rs`: converts a snapshot into the bounded model-facing screen brief.
- `redaction.rs`: filters likely password fields, token-looking values, and configured sensitive application fields.

Keep `src-tauri/src/ui_context.rs` as a temporary compatibility facade during migration. It should delegate to the new domain until all callers move, then be removed in a separate cleanup release.

### 2. Snapshot Contract

Replace the current flat context with a versioned snapshot that has explicit collection quality:

```text
ScreenContextSnapshot
  version
  capturedAt
  platform
  activeWindow
  cursor
  focusedElement
  selectedText
  pointedElement
  visibleWindows
  activeWindowTree
  diagnostics
```

Each `UiElement` should include only model-useful fields:

```text
role, name, value, automationId, className, bounds, depth,
isEnabled, isFocused, isOffscreen, parentTrail, childCount
```

`diagnostics` must report timeout, truncation, inaccessible controls, and source availability. This lets Waey say that context is partial instead of implying that the model saw something it did not receive.

### 3. Windows UI Automation Collection

The Windows collector should use `System.Windows.Automation` through a hidden PowerShell process or a native Rust-compatible alternative after a feasibility spike. Start with a hardened hidden PowerShell path because it is proven in the current product.

Collection flow:

1. Resolve the foreground window and its HWND before showing the overlay when possible.
2. Capture cursor coordinates and use `AutomationElement.FromPoint` for the pointed element.
3. Resolve the active window by HWND and traverse its Raw View tree.
4. Capture focused element and selection-capable text patterns where exposed by the application.
5. Collect a short list of visible top-level windows for multi-app awareness.
6. Return JSON through a unique temporary output file, then read and delete it from Rust.

Traversal guardrails:

- Maximum depth: 15.
- Maximum elements: 2,000.
- Collection budget: 1.5 seconds for normal prompts, 3 seconds for explicit refresh or guide recapture.
- Skip empty `Pane`, `Group`, and `Custom` scaffold nodes while retaining meaningful `Document`, `Edit`, `Text`, `Button`, `MenuItem`, `TabItem`, `TreeItem`, `DataItem`, and `ListItem` nodes.
- Treat tables and virtualized grids as terminal summaries instead of recursively enumerating thousands of cells.
- Prefer `ValuePattern`, then `TextPattern`, then name. Cap deep document text and mark it truncated.
- Give pointed and focused elements priority even when the tree budget is exhausted.

Chromium, canvas, and custom-rendered applications will remain partially opaque to UIA. The collector must state this through diagnostics; screenshot grounding handles the visual gap.

### 4. Context Freshness and Caching

Use a `ScreenContextCoordinator` with an in-memory snapshot cache. It owns freshness without touching React state during collection.

- Capture fresh context before every submitted prompt when Screen Context is enabled.
- Reuse a snapshot only when it is less than 750 ms old, belongs to the same foreground HWND, and the cursor has moved less than a small threshold.
- Start UIA collection and screenshot capture concurrently.
- If UIA misses its budget, send the last valid snapshot with `stale=true` only when it belongs to the same window; otherwise send no UI tree and explain that it was unavailable.
- Never make a prompt wait for a screenshot if the selected provider has no vision capability or the user removed all attachments.

This makes context always available to local text models without forcing unnecessary image latency.

### 5. Model-Facing Screen Brief

Do not send raw JSON tree dumps as the primary prompt format. Generate a compact, stable text brief:

```text
SCREEN CONTEXT (captured 180 ms ago)
ACTIVE WINDOW: Visual Studio Code | file: src/App.tsx
CURSOR: 1092, 538
FOCUSED: Editor "App.tsx"
SELECTED TEXT: "const result = ..."
POINTED ELEMENT:
  Button "Run" id="workbench.action.debug.run" @(1021, 24, 66x28)
VISIBLE WINDOWS: ...
ACTIVE WINDOW LAYOUT:
  Toolbar "..."
  Editor "App.tsx"
    Text "..."
```

Budget rules:

- Reserve context in this order: diagnostics, active window, selected text, pointed element, focused element, visible windows, then layout tree.
- Cap normal values to 200 characters and document/editor excerpts to 8,000 characters.
- Cap the entire screen brief at 30,000 characters for regular chat and 45,000 for Developer Mode.
- Use explicit truncation markers. Never silently cut a value in a way that changes its meaning.

Add one stable system-prompt clause for all providers: Screen Context is trusted local metadata, can be used even without an image, and must not be confused with a user request. The user prompt remains separate.

### 6. Screenshot Grounding

Keep screenshots optional, user-visible, and limited to three manual attachments. Add a dedicated ephemeral screenshot path for Guide Mode only.

- Vision providers: send the model-facing brief plus the screenshot image(s).
- Non-vision providers: send the exact same screen brief but omit image content.
- Attach a coordinate grid only for guide screenshots, not normal chat screenshots, to avoid visual clutter and token waste.
- Convert all guide bounds to physical screen pixels and maintain monitor origin and DPI metadata. The overlay renderer consumes the same coordinate system.
- Do not persist automatic guide screenshots in conversation history unless the user explicitly attaches them.

### 7. Provider Capability Model

Extend provider metadata with explicit capabilities instead of inferring from provider name:

```text
supportsVision
supportsReasoning
supportsStreaming
supportsDeveloperEdits
maxContextHint
```

Defaults remain conservative. Users may override capabilities for custom OpenAI-compatible endpoints. Screen Context is always permitted; only images are gated by `supportsVision`.

## Guided Task Mode

### 8. Guide State Machine

Create `src-tauri/src/guide/` and `src/features/guide/`.

The Rust controller owns state and emits typed events. React only renders the current state.

```text
idle -> offered -> step_active -> waiting_for_confirmation
     -> recapturing -> requesting_next_step -> step_active
     -> completed | cancelled | failed
```

Each guide session stores a random session ID, generation number, current step, last screen snapshot metadata, and elapsed time. A response from an older generation is discarded.

### 9. Structured Guide Protocol

Use a strict model output marker, parsed locally after streaming:

```json
{"kind":"guide_step","title":"Open Settings","instruction":"Select Settings in the sidebar.","target":{"uiaBounds":{"x":0,"y":0,"width":0,"height":0},"guessBounds":null},"confirmation":"done"}
```

Supported markers: `guide_offer`, `guide_step`, `guide_complete`, and `guide_abort`.

The guide system must accept `target: null` for steps that have no safe exact target. It must never invent a click target just to show an arrow.

### 10. Guide Overlay and Mascot

Use a dedicated transparent, always-on-top Tauri window only during an active guide step. It must be click-through except for its own explicit controls.

- Render the Waey mascot, pointer, short step label, and Done / Cancel controls in the existing brand system.
- Place the mascot on the side with more free screen space.
- Clamp overlay geometry to the visible monitor work area.
- Animate only transforms and opacity.
- Support multi-monitor coordinates and DPI conversion as a first-class contract.
- When a target is from UIA, label it as a precise target internally. When it is screenshot-derived, label it as estimated internally and render a softer pointer treatment.

### 11. Step Completion and Recapture

Do not rely on a global mouse hook to guess when the user completed a step. It creates race conditions and accidental capture of Waey controls.

Instead:

1. User performs the instruction.
2. User confirms Done in the guide overlay.
3. Waey hides its windows, restores focus to the original app when safe, waits a short configurable settle interval, then captures new UIA context and a fresh guide screenshot in parallel.
4. Waey submits only the fresh guide context with the prior step and guide session state.
5. The model returns exactly one next step or completion.

Add a five-minute inactivity timeout and an explicit Cancel action. Never automatically click external UI elements.

## Developer Mode

### 12. Workspace Trust Boundary

Keep Developer Mode opt-in. Workspaces are selected with the native folder picker and stored as canonical paths.

- Allow up to eight workspace roots.
- Resolve every read and write using canonical paths after validating the requested relative path.
- Disallow writing outside a selected workspace, path traversal, protected system paths, symlink escapes, and hidden credential files by default.
- Add a per-workspace ignore policy for `.git`, dependency directories, build outputs, secrets, binary blobs, and user-configurable exclusions.
- Show the active workspace and current access level in the composer, not in Settings. Settings only manages Developer Mode enablement and the persisted workspace list.

### 13. Developer Context Builder

Replace ad hoc file scanning with `DeveloperContextBuilder` under `src-tauri/src/developer_context/`.

Responsibilities:

- Resolve files from active UI details, explicit user paths, and selected workspace context.
- Read the active file first, then only the smallest supporting set of related files needed to answer the request.
- Preserve code line numbers and language identity.
- Include the UIA selected text as a high-priority excerpt when available.
- Include a bounded repository map: root files, source folders, manifests, and relevant neighboring files.
- Skip binary files except supported spreadsheet workflows.
- Return an invisible `DeveloperAttachment` separate from the user message, so internal context never appears in the chat bubble or pollutes history.

Suggested per-request limits:

- One focused file: up to 120 KB, with a line-window fallback for larger files.
- Supporting files: up to five, 40 KB each.
- Repository map: 8 KB.
- Whole developer attachment: 180 KB before provider-specific token budgeting.

If the requested scope exceeds the budget, include the most relevant excerpts and state exactly what was omitted in private diagnostic metadata, not in the user-facing message.

### 14. Selected Text Capture

UIA selection is opportunistic because different editors expose it differently. Implement a layered strategy:

1. Read UIA `TextPattern` selection from the focused element.
2. Read `ValuePattern` or document text and identify selection metadata where the application exposes it.
3. For approved Developer Mode only, offer a focused-window copy fallback that temporarily copies selection, restores the clipboard exactly, and runs only after local user consent.
4. If no reliable selection is available, send the active file context and say internally that selection was unavailable.

Never use clipboard fallback outside Developer Mode. Never leave copied code in the clipboard or log it.

### 15. File Read, Create, and Edit Protocol

The model does not receive direct filesystem tools. It returns typed fenced action blocks that Waey validates locally.

```text
waey-file-edit
  path: workspace-relative path
  expectedSha256: current content hash
  edits: replace exact ranges or unified diff hunks

waey-file-create
  path: workspace-relative path
  content: file content
  overwrite: false
```

Local validation must verify:

- action schema and size limits
- workspace ownership and path safety
- expected file hash before applying an edit
- exact patch applicability
- no binary target unless a dedicated feature handles it
- no overwrite without explicit access and user intent

Write atomically through a temporary sibling file, retain a short-lived backup, and report a structured result to the conversation. Auto mode can apply only validated, non-destructive edits. Ask and Assist modes always require explicit approval before writes.

### 16. Spreadsheet Support

Keep spreadsheet handling in a dedicated module with its current structured sheet-edit protocol. Expand it only after the core file-edit protocol is stable.

- Read a workbook summary, sheet names, used ranges, and selected values through the spreadsheet module.
- Apply only bounded structured actions such as cell updates, formulas, rows, and sheets.
- Do not route `.xlsx` files through generic text-file actions.

## Performance Plan

### 17. Keep the Overlay Fast

- Move UIA collection, PowerShell invocation, image encoding, workspace scanning, hashing, and diff validation to `tauri::async_runtime::spawn_blocking` or dedicated worker tasks.
- The overlay show hotkey must only show and focus the window. It should not wait for screen intelligence.
- Emit context readiness asynchronously. The composer can show a small neutral status while a fresh snapshot is being prepared.
- Debounce settings writes and never reinitialize desktop integrations for a simple visual setting change.
- Cache canonical workspace paths, manifests, and compact repository maps with file modification-time invalidation.
- Use bounded channels and cancellation tokens to stop stale capture, guide, and developer-context work when a new user action supersedes it.
- Add timing telemetry locally: UIA elapsed time, screenshot elapsed time, context bytes, prompt bytes, provider first-token time, and completion time. Do not upload it by default.

Target budgets:

| Operation | Target | Hard ceiling |
| --- | ---: | ---: |
| Overlay visible after hotkey | 120 ms | 250 ms |
| Normal UIA context | 350 ms | 1.5 s |
| Explicit guide refresh | 750 ms | 3 s |
| Developer context build | 500 ms | 2 s |
| UI-thread blocking work | 0 ms | 0 ms |

## Migration Phases

### Phase 0: Foundation and Tests

1. Add feature flags: `screen_intelligence_v2`, `guide_mode`, and `developer_context_v2`.
2. Define versioned Rust and TypeScript contracts.
3. Add fixture-based tests for normalization, redaction, serialization, path validation, patch application, and guide marker parsing.
4. Record baseline timing from the existing UI context path.

Exit criteria: legacy functionality unchanged, new code unreachable by default, tests pass on Windows, macOS, and Linux builds.

### Phase 1: Screen Intelligence v2 on Windows

1. Implement the collector, normalizer, and serializer.
2. Run it in shadow mode beside the current collector and compare output and timing in local diagnostic logs.
3. Move regular chat to one fresh `ScreenContextSnapshot` per prompt.
4. Ensure the context is sent independently from image attachments.
5. Add partial-context diagnostics and a compact developer-only debug view.

Exit criteria: text-only providers answer questions about visible buttons, fields, selected text, and active window without an attached screenshot.

### Phase 2: Provider and Prompt Routing

1. Add provider capabilities and manual overrides.
2. Build a provider-neutral request envelope with separate user text, screen brief, developer attachment, and images.
3. Apply provider-specific image formatting only at the final HTTP adapter.
4. Add regression tests for OpenAI-compatible, OpenRouter, Ollama, and managed Waey provider request shapes.

Exit criteria: removing a screenshot never removes screen context, and non-vision providers never receive image payloads.

### Phase 3: Guided Tasks

1. Implement guide protocol parser and Rust state machine.
2. Build the Tauri guide overlay window and coordinate conversion contract.
3. Add confirm-based recapture, cancellation, inactivity timeout, and stale-generation protection.
4. Add visual tests for pointer placement at multiple DPI scales and monitor layouts.

Exit criteria: a guide reliably advances exactly one confirmed step at a time and never performs clicks on the user's behalf.

### Phase 4: Developer Context v2

1. Implement canonical workspace registry and policy layer.
2. Build focused-file and repository-map collection.
3. Add invisible developer attachments to LLM request and history boundaries.
4. Implement selected-text collection with UIA first and consented clipboard fallback.
5. Introduce typed file creation and patch action blocks with Ask, Assist, and Auto validation.

Exit criteria: an approved local model can inspect a selected code region, edit one validated file, create a new workspace file, and never access outside the workspace.

### Phase 5: Platform Support and Hardening

1. Keep Windows as full UIA support.
2. Add macOS Accessibility API collector behind the same snapshot contract.
3. Add Linux AT-SPI collector behind the same snapshot contract.
4. Gracefully degrade to screenshots and active-window metadata where accessibility access is denied or unavailable.
5. Add test matrices for DPI, focus changes, Chromium, VS Code, Office apps, Arabic RTL UI, local text models, and vision models.

Exit criteria: no platform emits a crash or broken overlay when its semantic screen collector is unavailable.

## Improvements Beyond the Reference Pattern

1. **Context quality score**: attach a local `high`, `partial`, or `unavailable` rating so Waey can avoid overconfident answers.
2. **Delta context**: for follow-up messages in the same foreground window, send a compact semantic diff instead of the full layout every time, with periodic full snapshots to prevent drift.
3. **Privacy classes**: detect and redact password inputs, OTP fields, tokens, and credential-like values before a provider request.
4. **Provider-aware budgeting**: scale context and file excerpts to the selected provider's configured context limit instead of applying one global cap.
5. **Dry-run edit preview**: in Ask and Assist modes, show a semantic summary and a compact diff before applying changes.
6. **Recoverable edits**: keep a local edit journal with one-click undo for a short retention period.
7. **Guide confidence fallback**: when UIA and screenshot targets disagree, ask the user to confirm rather than drawing a misleading pointer.

## Definition of Done

The work is complete only when all of these are true:

- Regular chat sends a fresh readable screen brief on every prompt when enabled, with or without images.
- UI context is never rendered inside the user's message or stored as user-authored content.
- A local non-vision model can answer from UI context and approved developer files.
- Vision models receive aligned screenshot and UI coordinates.
- Guide sessions return one safe next step only after confirmation and fresh recapture.
- Developer actions remain inside canonical approved workspaces and respect Ask, Assist, and Auto policies.
- All expensive operations run off the UI thread and meet the performance budgets.
- Windows, macOS, and Linux builds succeed; unsupported accessibility APIs degrade gracefully.
- Security, redaction, path traversal, symlink, malformed action, timeout, cancellation, and stale-response tests pass.

## Delivery Checklist

The work ships in five focused Git commits. Each task is pushed after its local checks pass. No release is created until all five are integrated and tested together.

- [ ] **Task 1: Screen Intelligence v2 foundation**
  - Add versioned Rust and TypeScript contracts, feature flags, diagnostics, timing interfaces, and fixture-based unit tests.
  - Preserve the existing UI context behavior behind a compatibility adapter.
  - Suggested commit: `feat: establish screen intelligence foundation`

- [ ] **Task 2: Windows UIA and prompt context pipeline**
  - Implement bounded Windows UIA collection, normalization, redaction, compact model serialization, freshness cache, and provider-aware image routing.
  - Ensure screen context reaches every enabled prompt independently from screenshot attachments.
  - Suggested commit: `feat: add reliable screen context pipeline`

- [ ] **Task 3: Interactive guide mode**
  - Add the guide state machine, one-step structured protocol, transparent mascot overlay, safe target pointing, Done and Cancel handling, and fresh context recapture after confirmation.
  - The mascot moves to the current target, shows one instruction, then waits for confirmation before Waey asks the model for the next instruction.
  - Suggested commit: `feat: add interactive guided tasks`

- [ ] **Task 4: Secure developer agent workflow**
  - Add workspace policy enforcement, invisible developer attachments, focused file and selected-text context, typed file-create and file-edit actions, atomic writes, previews, and approval-level enforcement.
  - Suggested commit: `feat: strengthen developer workspace agent`

- [ ] **Task 5: Hardening and release readiness**
  - Add performance cancellation, platform fallbacks, local telemetry, security regression tests, end-to-end test scenarios, documentation, and final integration verification.
  - Suggested commit: `chore: harden screen intelligence workflows`
