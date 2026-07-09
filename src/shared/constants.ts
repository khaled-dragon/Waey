import type { AppSettings } from "./types";

export const APP_NAME = "Waey";

export const DEFAULT_SETTINGS: AppSettings = {
  hotkeyOverlay: "Alt+Space",
  hotkeyRegion: "Ctrl+Space",
  theme: "dark",
  language: "en",
  autoCaptureOnOverlay: true,
  attachUiContext: true,
  launchOnStartup: false,
  selectedProviderId: "",
  selectedPersonaId: "",
  developerModeEnabled: false,
  developerAccessLevel: "assist",
  developerWorkspaces: [],
};
