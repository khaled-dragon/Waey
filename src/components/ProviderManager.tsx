import { useState, type FormEvent } from "react";
import type { LlmProvider, ProviderDraft, ProviderKind } from "../shared/types";

interface ProviderManagerProps {
  providers: LlmProvider[];
  selectedProviderId: string;
  onDeleteProvider: (providerId: string) => Promise<void>;
  onSaveProvider: (provider: ProviderDraft) => Promise<void>;
  onSelectProvider: (providerId: string) => void;
}

const initialDraft: ProviderDraft = { name: "", kind: "openrouter", baseUrl: "https://openrouter.ai/api/v1", apiKey: "", model: "" };

function defaultBaseUrl(kind: ProviderKind) {
  if (kind === "ollama") return "http://localhost:11434/v1";
  if (kind === "openrouter") return "https://openrouter.ai/api/v1";
  if (kind === "custom") return "https://api.openai.com/v1";
  return "";
}

export function ProviderManager({ providers, selectedProviderId, onDeleteProvider, onSaveProvider, onSelectProvider }: ProviderManagerProps) {
  const [draft, setDraft] = useState<ProviderDraft>(initialDraft);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setErrorMessage(null);
    try {
      await onSaveProvider(draft);
      setDraft(initialDraft);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  }

  function update(field: keyof ProviderDraft, value: string) {
    setDraft((d) => ({ ...d, [field]: value }));
  }

  function updateKind(kind: ProviderKind) {
    setDraft((d) => ({ ...d, kind, baseUrl: defaultBaseUrl(kind), apiKey: kind === "ollama" ? "" : d.apiKey }));
  }

  return (
    <div className="panel">
      <div className="panel-header">
        <div className="panel-title-label">API Manager</div>
        <div className="panel-title">Providers</div>
      </div>

      <form className="panel-form" onSubmit={handleSubmit}>
        <div className="form-row">
          <input className="form-input" placeholder="Provider name" value={draft.name} onChange={(e) => update("name", e.currentTarget.value)} />
          <select className="form-select" value={draft.kind} onChange={(e) => updateKind(e.currentTarget.value as ProviderKind)}>
            <option value="openrouter">OpenRouter</option>
            <option value="ollama">Ollama</option>
            <option value="custom">OpenAI / Custom</option>
          </select>
        </div>
        <input className="form-input" placeholder="Base URL" value={draft.baseUrl} onChange={(e) => update("baseUrl", e.currentTarget.value)} />
        <div className="form-row">
          <input className="form-input" placeholder="Model ID" value={draft.model} onChange={(e) => update("model", e.currentTarget.value)} />
          <input className="form-input" placeholder="API Key" type="password" value={draft.apiKey} onChange={(e) => update("apiKey", e.currentTarget.value)} />
        </div>
        {errorMessage && <div className="error-inline">{errorMessage}</div>}
        <button className="btn-primary" disabled={isSaving} type="submit">{isSaving ? "Saving..." : "Save Provider"}</button>
      </form>

      <div className="item-list">
        {providers.length === 0 ? (
          <div className="empty-list">No providers yet — add one above to get started</div>
        ) : (
          providers.map((p) => (
            <div key={p.id} className={`list-item ${selectedProviderId === p.id ? "list-item--active" : ""}`}>
              <button className="list-item-info" onClick={() => onSelectProvider(p.id)} type="button" style={{ background: "none", border: "none", cursor: "pointer", textAlign: "left" }}>
                <div className="list-item-name">{p.name}</div>
                <div className="list-item-sub">{p.model}</div>
              </button>
              <div className="list-item-actions">
                {selectedProviderId === p.id && <span className="badge-active">Active</span>}
                <button className="btn-secondary" onClick={() => void onDeleteProvider(p.id)} type="button">Delete</button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
