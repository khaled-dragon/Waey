import type { ReactNode } from "react";
import type { AppSettings } from "../shared/types";

interface SettingsPanelProps {
  errorMessage: string | null;
  isLoading: boolean;
  onChangeSettings: (settings: AppSettings) => Promise<void>;
  settings: AppSettings;
}

export function SettingsPanel({ errorMessage, isLoading, onChangeSettings, settings }: SettingsPanelProps) {
  function update<Key extends keyof AppSettings>(key: Key, value: AppSettings[Key]) {
    void onChangeSettings({ ...settings, [key]: value });
  }

  return (
    <div className="panel">
      <div className="panel-header">
        <div className="panel-title-label">Preferences</div>
        <div className="panel-title">Settings</div>
      </div>

      {errorMessage && <div className="error-msg">{errorMessage}</div>}

      <div className="settings-group">
        <Field label="Overlay Hotkey">
          <input className="form-input" disabled={isLoading} value={settings.hotkeyOverlay} onChange={(e) => update("hotkeyOverlay", e.currentTarget.value)} />
        </Field>
        <Field label="Smart Crop Hotkey">
          <input className="form-input" disabled={isLoading} value={settings.hotkeyRegion} onChange={(e) => update("hotkeyRegion", e.currentTarget.value)} />
        </Field>
      </div>

      <div className="form-row">
        <Field label="Theme">
          <select className="form-select" disabled={isLoading} value={settings.theme} onChange={(e) => update("theme", e.currentTarget.value as AppSettings["theme"])}>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="system">System</option>
          </select>
        </Field>
        <Field label="Language">
          <select className="form-select" disabled={isLoading} value={settings.language} onChange={(e) => update("language", e.currentTarget.value as AppSettings["language"])}>
            <option value="en">English</option>
            <option value="ar">Arabic</option>
          </select>
        </Field>
      </div>

      <label className="toggle-row">
        <div className="toggle-info">
          <div className="toggle-title">Auto capture on open</div>
          <div className="toggle-desc">Capture screen automatically when Waey opens</div>
        </div>
        <input type="checkbox" checked={settings.autoCaptureOnOverlay} disabled={isLoading} onChange={(e) => update("autoCaptureOnOverlay", e.currentTarget.checked)} style={{ accentColor: "#ef3f42", width: 16, height: 16 }} />
      </label>

      <div className="settings-note">
        Hotkey changes are saved immediately. Live re-registration will be available in a future update.
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span className="settings-label">{label}</span>
      {children}
    </label>
  );
}
