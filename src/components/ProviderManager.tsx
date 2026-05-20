import { useState, type FormEvent } from "react";
import type { LlmProvider, ProviderDraft, ProviderKind } from "../shared/types";

interface ProviderManagerProps {
  providers: LlmProvider[];
  selectedProviderId: string;
  onDeleteProvider: (providerId: string) => Promise<void>;
  onSaveProvider: (provider: ProviderDraft) => Promise<void>;
  onSelectProvider: (providerId: string) => void;
}

const initialProviderDraft: ProviderDraft = {
  name: "",
  kind: "openrouter",
  baseUrl: "https://openrouter.ai/api/v1",
  apiKey: "",
  model: "",
};

export function ProviderManager({
  providers,
  selectedProviderId,
  onDeleteProvider,
  onSaveProvider,
  onSelectProvider,
}: ProviderManagerProps) {
  const [draft, setDraft] = useState<ProviderDraft>(initialProviderDraft);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setErrorMessage(null);

    try {
      await onSaveProvider(draft);
      setDraft(initialProviderDraft);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  }

  function updateDraft(field: keyof ProviderDraft, value: string) {
    setDraft((currentDraft) => ({ ...currentDraft, [field]: value }));
  }

  function updateProviderKind(kind: ProviderKind) {
    setDraft((currentDraft) => ({
      ...currentDraft,
      kind,
      baseUrl: defaultBaseUrl(kind),
      apiKey: kind === "ollama" ? "" : currentDraft.apiKey,
    }));
  }

  return (
    <section className="rounded-3xl border border-white/10 bg-black/30 p-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-sm font-semibold text-waey-coral">API Manager</p>
          <h2 className="mt-1 text-xl font-semibold">Providers</h2>
        </div>
        <p className="max-w-72 text-xs leading-5 text-white/45">
          Add any OpenAI-compatible endpoint. OpenRouter and Ollama work through the same
          chat completions format.
        </p>
      </div>

      <form className="mt-4 grid gap-3" onSubmit={handleSubmit}>
        <div className="grid gap-3 sm:grid-cols-[1fr_150px]">
          <input
            className="rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none placeholder:text-white/35 focus:border-waey-coral"
            onChange={(event) => updateDraft("name", event.currentTarget.value)}
            placeholder="Provider name, e.g. Sonnet 4.6"
            value={draft.name}
          />
          <select
            className="rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none focus:border-waey-coral"
            onChange={(event) => updateProviderKind(event.currentTarget.value as ProviderKind)}
            value={draft.kind}
          >
            <option value="openrouter">OpenRouter</option>
            <option value="ollama">Ollama</option>
            <option value="custom">Custom</option>
          </select>
        </div>

        <input
          className="rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none placeholder:text-white/35 focus:border-waey-coral"
          onChange={(event) => updateDraft("baseUrl", event.currentTarget.value)}
          placeholder="Base URL, e.g. https://openrouter.ai/api/v1"
          value={draft.baseUrl}
        />

        <div className="grid gap-3 sm:grid-cols-2">
          <input
            className="rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none placeholder:text-white/35 focus:border-waey-coral"
            onChange={(event) => updateDraft("model", event.currentTarget.value)}
            placeholder="Model, e.g. anthropic/claude-sonnet-4.5"
            value={draft.model}
          />
          <input
            className="rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none placeholder:text-white/35 focus:border-waey-coral"
            onChange={(event) => updateDraft("apiKey", event.currentTarget.value)}
            placeholder="API key, optional for local Ollama"
            type="password"
            value={draft.apiKey}
          />
        </div>

        {errorMessage ? <p className="text-sm text-waey-coral">{errorMessage}</p> : null}

        <button
          className="rounded-2xl bg-waey-bright px-4 py-3 text-sm font-semibold transition hover:bg-waey-red disabled:cursor-not-allowed disabled:opacity-50"
          disabled={isSaving}
          type="submit"
        >
          {isSaving ? "Saving..." : "Save Provider"}
        </button>
      </form>

      <div className="mt-4 grid gap-2">
        {providers.length === 0 ? (
          <p className="rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3 text-sm text-white/50">
            No providers saved yet.
          </p>
        ) : (
          providers.map((provider) => (
            <div
              className="flex items-center justify-between gap-3 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3"
              key={provider.id}
            >
              <button
                className="min-w-0 text-left"
                onClick={() => onSelectProvider(provider.id)}
                type="button"
              >
                <p className="truncate text-sm font-semibold">{provider.name}</p>
                <p className="truncate text-xs text-white/45">{provider.model}</p>
              </button>
              <div className="flex items-center gap-2">
                {selectedProviderId === provider.id ? (
                  <span className="rounded-full bg-waey-bright/20 px-3 py-1 text-xs text-waey-coral">
                    Active
                  </span>
                ) : null}
                <button
                  className="rounded-full border border-white/10 px-3 py-1 text-xs text-white/55 hover:border-waey-coral hover:text-white"
                  onClick={() => void onDeleteProvider(provider.id)}
                  type="button"
                >
                  Delete
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

function defaultBaseUrl(kind: ProviderKind) {
  if (kind === "ollama") {
    return "http://localhost:11434/v1";
  }

  if (kind === "openrouter") {
    return "https://openrouter.ai/api/v1";
  }

  return "";
}
