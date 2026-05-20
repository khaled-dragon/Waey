import type { ReactNode } from "react";
import type { AppSettings } from "../shared/types";

interface SettingsPanelProps {
  errorMessage: string | null;
  isLoading: boolean;
  onChangeSettings: (settings: AppSettings) => Promise<void>;
  settings: AppSettings;
}

export function SettingsPanel({
  errorMessage,
  isLoading,
  onChangeSettings,
  settings,
}: SettingsPanelProps) {
  function updateSettings<Key extends keyof AppSettings>(key: Key, value: AppSettings[Key]) {
    void onChangeSettings({ ...settings, [key]: value });
  }

  return (
    <section className="min-h-0 flex-1 overflow-y-auto rounded-3xl border border-white/10 bg-black/30 p-4">
      <div>
        <p className="text-sm font-semibold text-waey-coral">Settings</p>
        <h2 className="mt-1 text-xl font-semibold">App preferences</h2>
      </div>

      {errorMessage ? (
        <p className="mt-4 rounded-2xl border border-waey-coral/40 bg-waey-bright/10 px-4 py-3 text-sm text-waey-coral">
          {errorMessage}
        </p>
      ) : null}

      <div className="mt-4 grid gap-4">
        <SettingField label="Overlay hotkey">
          <input
            className="w-full rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none focus:border-waey-coral"
            disabled={isLoading}
            onChange={(event) => updateSettings("hotkeyOverlay", event.currentTarget.value)}
            value={settings.hotkeyOverlay}
          />
        </SettingField>

        <SettingField label="Smart Crop hotkey">
          <input
            className="w-full rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none focus:border-waey-coral"
            disabled={isLoading}
            onChange={(event) => updateSettings("hotkeyRegion", event.currentTarget.value)}
            value={settings.hotkeyRegion}
          />
        </SettingField>

        <div className="grid gap-4 sm:grid-cols-2">
          <SettingField label="Theme">
            <select
              className="w-full rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none focus:border-waey-coral"
              disabled={isLoading}
              onChange={(event) =>
                updateSettings("theme", event.currentTarget.value as AppSettings["theme"])
              }
              value={settings.theme}
            >
              <option value="dark">Dark</option>
              <option value="light">Light</option>
              <option value="system">System</option>
            </select>
          </SettingField>

          <SettingField label="Language">
            <select
              className="w-full rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none focus:border-waey-coral"
              disabled={isLoading}
              onChange={(event) =>
                updateSettings("language", event.currentTarget.value as AppSettings["language"])
              }
              value={settings.language}
            >
              <option value="en">English</option>
              <option value="ar">Arabic</option>
            </select>
          </SettingField>
        </div>

        <label className="flex items-center justify-between gap-4 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3">
          <span>
            <span className="block text-sm font-semibold">Auto capture on overlay</span>
            <span className="mt-1 block text-xs text-white/45">
              Capture the screen automatically when Waey opens.
            </span>
          </span>
          <input
            checked={settings.autoCaptureOnOverlay}
            className="size-5 accent-waey-bright"
            disabled={isLoading}
            onChange={(event) =>
              updateSettings("autoCaptureOnOverlay", event.currentTarget.checked)
            }
            type="checkbox"
          />
        </label>

        <p className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-xs leading-5 text-white/45">
          Hotkey changes are saved now. Live re-registration will be wired after Rust runtime testing
          is available.
        </p>
      </div>
    </section>
  );
}

function SettingField({ children, label }: { children: ReactNode; label: string }) {
  return (
    <label className="grid gap-2">
      <span className="text-sm font-semibold text-white/75">{label}</span>
      {children}
    </label>
  );
}
