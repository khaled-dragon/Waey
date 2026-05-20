import { useState, type FormEvent } from "react";
import type { Persona, PersonaDraft } from "../shared/types";

interface PersonaManagerProps {
  personas: Persona[];
  selectedPersonaId: string;
  onDeletePersona: (personaId: string) => Promise<void>;
  onSavePersona: (persona: PersonaDraft) => Promise<void>;
  onSelectPersona: (personaId: string) => void;
}

const initialPersonaDraft: PersonaDraft = {
  name: "",
  prompt: "",
};

export function PersonaManager({
  personas,
  selectedPersonaId,
  onDeletePersona,
  onSavePersona,
  onSelectPersona,
}: PersonaManagerProps) {
  const [draft, setDraft] = useState<PersonaDraft>(initialPersonaDraft);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setErrorMessage(null);

    try {
      await onSavePersona(draft);
      setDraft(initialPersonaDraft);
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  }

  function updateDraft(field: keyof PersonaDraft, value: string) {
    setDraft((currentDraft) => ({ ...currentDraft, [field]: value }));
  }

  return (
    <section className="min-h-0 flex-1 overflow-y-auto rounded-3xl border border-white/10 bg-black/30 p-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-sm font-semibold text-waey-coral">Custom Prompts</p>
          <h2 className="mt-1 text-xl font-semibold">Personas</h2>
        </div>
        <p className="max-w-72 text-xs leading-5 text-white/45">
          Save reusable behavior presets, then pick one before sending a prompt.
        </p>
      </div>

      <form className="mt-4 grid gap-3" onSubmit={handleSubmit}>
        <input
          className="rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm outline-none placeholder:text-white/35 focus:border-waey-coral"
          onChange={(event) => updateDraft("name", event.currentTarget.value)}
          placeholder="Persona name, e.g. Arabic Explainer"
          value={draft.name}
        />
        <textarea
          className="min-h-32 resize-none rounded-2xl border border-white/10 bg-waey-panel px-4 py-3 text-sm leading-6 outline-none placeholder:text-white/35 focus:border-waey-coral"
          onChange={(event) => updateDraft("prompt", event.currentTarget.value)}
          placeholder="Prompt, e.g. Always explain in Arabic, keep examples practical, and ask before making assumptions."
          value={draft.prompt}
        />

        {errorMessage ? <p className="text-sm text-waey-coral">{errorMessage}</p> : null}

        <button
          className="rounded-2xl bg-waey-bright px-4 py-3 text-sm font-semibold transition hover:bg-waey-red disabled:cursor-not-allowed disabled:opacity-50"
          disabled={isSaving}
          type="submit"
        >
          {isSaving ? "Saving..." : "Save Persona"}
        </button>
      </form>

      <div className="mt-4 grid gap-2">
        <button
          className={`rounded-2xl border px-4 py-3 text-left ${
            selectedPersonaId === ""
              ? "border-waey-coral/40 bg-waey-bright/10"
              : "border-white/10 bg-white/[0.04]"
          }`}
          onClick={() => onSelectPersona("")}
          type="button"
        >
          <p className="text-sm font-semibold">Default Waey</p>
          <p className="mt-1 text-xs text-white/45">Concise screen-aware assistant.</p>
        </button>

        {personas.map((persona) => (
          <div
            className="flex items-start justify-between gap-3 rounded-2xl border border-white/10 bg-white/[0.04] px-4 py-3"
            key={persona.id}
          >
            <button
              className="min-w-0 text-left"
              onClick={() => onSelectPersona(persona.id)}
              type="button"
            >
              <p className="truncate text-sm font-semibold">{persona.name}</p>
              <p className="mt-1 line-clamp-2 text-xs leading-5 text-white/45">{persona.prompt}</p>
            </button>
            <div className="flex shrink-0 items-center gap-2">
              {selectedPersonaId === persona.id ? (
                <span className="rounded-full bg-waey-bright/20 px-3 py-1 text-xs text-waey-coral">
                  Active
                </span>
              ) : null}
              <button
                className="rounded-full border border-white/10 px-3 py-1 text-xs text-white/55 hover:border-waey-coral hover:text-white"
                onClick={() => void onDeletePersona(persona.id)}
                type="button"
              >
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
