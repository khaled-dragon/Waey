import { useCallback, useEffect, useState } from "react";
import { DEFAULT_SETTINGS } from "../../shared/constants";
import type { AppSettings } from "../../shared/types";
import { getAppSettings, saveAppSettings } from "./settingsCommands";

export function useAppSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [isLoadingSettings, setIsLoadingSettings] = useState(true);

  const refreshSettings = useCallback(async () => {
    setIsLoadingSettings(true);
    setSettingsError(null);

    try {
      setSettings(await getAppSettings());
    } catch (error) {
      setSettingsError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsLoadingSettings(false);
    }
  }, []);

  const updateSettings = useCallback(async (nextSettings: AppSettings) => {
    setSettingsError(null);

    try {
      const savedSettings = await saveAppSettings(nextSettings);
      setSettings(savedSettings);
    } catch (error) {
      setSettingsError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => {
    void refreshSettings();
  }, [refreshSettings]);

  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme;
    document.documentElement.dir = settings.language === "ar" ? "rtl" : "ltr";
    document.documentElement.lang = settings.language;
  }, [settings.language, settings.theme]);

  return {
    isLoadingSettings,
    refreshSettings,
    settings,
    settingsError,
    updateSettings,
  };
}
