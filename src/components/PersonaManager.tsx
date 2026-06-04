import { useState, type FormEvent } from "react";
import type { Persona, PersonaDraft } from "../shared/types";

interface PersonaManagerProps {
  personas: Persona[];
  selectedPersonaId: string;
  onDeletePersona: (personaId: string) => Promise<void>;
  onSavePersona: (persona: PersonaDraft) => Promise<void>;
  onSelectPersona: (personaId: string) => void;
}

const initialDraft: PersonaDraft = { name: "", prompt: "" };

export function PersonaManager({ personas, selectedPersonaId, onDeletePersona, onSavePersona, onSelectPersona }: PersonaManagerProps) {
  const [draft, setDraft] = useState<PersonaDraft>(initialDraft);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setErrorMessage(null);
    try {
      await onSavePersona(draft);
      setDraft(initialDraft);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="panel">
      <div className="panel-header">
        <div className="panel-title-label">Custom Prompts</div>
        <div className="panel-title">Personas</div>
      </div>

      <form className="panel-form" onSubmit={handleSubmit}>
        <input className="form-input" placeholder="Persona name, e.g. Arabic Explainer" value={draft.name} onChange={(e) => { const value = e.currentTarget.value; setDraft((d) => ({ ...d, name: value })); }} />
        <textarea className="form-textarea" placeholder="System prompt, e.g. Always explain in Arabic..." value={draft.prompt} onChange={(e) => { const value = e.currentTarget.value; setDraft((d) => ({ ...d, prompt: value })); }} />
        {errorMessage && <div className="error-inline">{errorMessage}</div>}
        <button className="btn-primary" disabled={isSaving} type="submit">{isSaving ? "Saving..." : "Save Persona"}</button>
      </form>

      <div className="item-list">
        <div className={`list-item ${selectedPersonaId === "" ? "list-item--active" : ""}`}>
          <button className="list-item-info" onClick={() => onSelectPersona("")} type="button" style={{ background: "none", border: "none", cursor: "pointer", textAlign: "left" }}>
            <div className="list-item-name">Default Waey</div>
            <div className="list-item-sub">Concise screen-aware assistant</div>
          </button>
          {selectedPersonaId === "" && <span className="badge-active">Active</span>}
        </div>

        {personas.map((p) => (
          <div key={p.id} className={`list-item ${selectedPersonaId === p.id ? "list-item--active" : ""}`}>
            <button className="list-item-info" onClick={() => onSelectPersona(p.id)} type="button" style={{ background: "none", border: "none", cursor: "pointer", textAlign: "left" }}>
              <div className="list-item-name">{p.name}</div>
              <div className="list-item-sub" style={{ display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden" }}>{p.prompt}</div>
            </button>
            <div className="list-item-actions">
              {selectedPersonaId === p.id && <span className="badge-active">Active</span>}
              <button className="btn-secondary" onClick={() => void onDeletePersona(p.id)} type="button">Delete</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
