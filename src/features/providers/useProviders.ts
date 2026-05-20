import { useCallback, useEffect, useMemo, useState } from "react";
import type { LlmProvider, ProviderDraft } from "../../shared/types";
import { deleteLlmProvider, listLlmProviders, saveLlmProvider } from "./providerCommands";

export function useProviders() {
  const [providers, setProviders] = useState<LlmProvider[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState<string>("");
  const [isLoadingProviders, setIsLoadingProviders] = useState(true);
  const [providerError, setProviderError] = useState<string | null>(null);

  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.id === selectedProviderId) ?? providers[0] ?? null,
    [providers, selectedProviderId],
  );

  const refreshProviders = useCallback(async () => {
    setIsLoadingProviders(true);
    setProviderError(null);

    try {
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
  }, []);

  const saveProvider = useCallback(
    async (provider: ProviderDraft) => {
      const savedProvider = await saveLlmProvider(provider);
      await refreshProviders();
      setSelectedProviderId(savedProvider.id);
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

  useEffect(() => {
    void refreshProviders();
  }, [refreshProviders]);

  return {
    deleteProvider,
    isLoadingProviders,
    providerError,
    providers,
    refreshProviders,
    saveProvider,
    selectedProvider,
    selectedProviderId,
    setSelectedProviderId,
  };
}
