import { useCallback, useEffect, useMemo, useState } from "react";
import type { LlmProvider, ManagedProviderUpdate, ProviderDraft } from "../../shared/types";
import {
  applyManagedProviderUpdate,
  bootstrapManagedProvider,
  checkManagedProviderUpdate,
  deleteLlmProvider,
  listLlmProviders,
  saveLlmProvider,
} from "./providerCommands";

const DISMISSED_MANAGED_UPDATE_KEY = "waey.dismissedManagedProviderUpdate";

export function useProviders() {
  const [providers, setProviders] = useState<LlmProvider[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState<string>("");
  const [isLoadingProviders, setIsLoadingProviders] = useState(true);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [managedProviderChecked, setManagedProviderChecked] = useState(false);
  const [pendingManagedProviderUpdate, setPendingManagedProviderUpdate] =
    useState<ManagedProviderUpdate | null>(null);

  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.id === selectedProviderId) ?? providers[0] ?? null,
    [providers, selectedProviderId],
  );

  const refreshProviders = useCallback(async () => {
    setIsLoadingProviders(true);
    setProviderError(null);

    try {
      if (!managedProviderChecked) {
        await bootstrapManagedProvider().catch(() => null);
        const update = await checkManagedProviderUpdate().catch(() => null);

        if (update && !isManagedUpdateDismissed(update)) {
          setPendingManagedProviderUpdate(update);
        }

        setManagedProviderChecked(true);
      }

      const nextProviders = await listLlmProviders();

      setProviders(nextProviders);
      setSelectedProviderId((currentId) => {
        if (nextProviders.some((provider) => provider.id === currentId)) {
          return currentId;
        }

        return nextProviders[0]?.id ?? "";
      });
    } catch (error) {
      setProviderError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsLoadingProviders(false);
    }
  }, [managedProviderChecked]);

  const saveProvider = useCallback(
    async (provider: ProviderDraft) => {
      const savedProvider = await saveLlmProvider(provider);
      await refreshProviders();
      setSelectedProviderId(savedProvider.id);
      return savedProvider;
    },
    [refreshProviders],
  );

  const deleteProvider = useCallback(
    async (providerId: string) => {
      await deleteLlmProvider(providerId);
      await refreshProviders();
    },
    [refreshProviders],
  );

  const applyManagedUpdate = useCallback(async () => {
    const provider = await applyManagedProviderUpdate();

    setPendingManagedProviderUpdate(null);
    clearDismissedManagedUpdate();
    await refreshProviders();
    return provider;
  }, [refreshProviders]);

  const dismissManagedUpdate = useCallback(() => {
    if (pendingManagedProviderUpdate) {
      dismissManagedUpdateSignature(pendingManagedProviderUpdate);
    }

    setPendingManagedProviderUpdate(null);
  }, [pendingManagedProviderUpdate]);

  useEffect(() => {
    void refreshProviders();
  }, [refreshProviders]);

  return {
    applyManagedUpdate,
    deleteProvider,
    dismissManagedUpdate,
    isLoadingProviders,
    pendingManagedProviderUpdate,
    providerError,
    providers,
    refreshProviders,
    saveProvider,
    selectedProvider,
    selectedProviderId,
    setSelectedProviderId,
  };
}

function managedUpdateSignature(update: ManagedProviderUpdate) {
  return `${update.revision ?? "runtime"}|${update.provider.baseUrl}|${update.provider.model}`;
}

function isManagedUpdateDismissed(update: ManagedProviderUpdate) {
  try {
    return localStorage.getItem(DISMISSED_MANAGED_UPDATE_KEY) === managedUpdateSignature(update);
  } catch {
    return false;
  }
}

function dismissManagedUpdateSignature(update: ManagedProviderUpdate) {
  try {
    localStorage.setItem(DISMISSED_MANAGED_UPDATE_KEY, managedUpdateSignature(update));
  } catch {
  }
}

function clearDismissedManagedUpdate() {
  try {
    localStorage.removeItem(DISMISSED_MANAGED_UPDATE_KEY);
  } catch {
  }
}
