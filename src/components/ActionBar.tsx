import { captureCurrentScreen, showRegionSelector } from "../features/capture";
import type { LlmProvider, Persona } from "../shared/types";

interface ActionBarProps {
  onOpenPersonas: () => void;
  providers: LlmProvider[];
  personas: Persona[];
  onOpenHistory: () => void;
  onOpenSettings: () => void;
  selectedPersonaId: string;
  selectedProviderId: string;
  onOpenProviders: () => void;
  onSelectPersona: (personaId: string) => void;
  onSelectProvider: (providerId: string) => void;
}

export function ActionBar({
  onOpenPersonas,
  providers,
  personas,
  onOpenHistory,
  onOpenSettings,
  selectedPersonaId,
  selectedProviderId,
  onOpenProviders,
  onSelectPersona,
  onSelectProvider,
}: ActionBarProps) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-white/10 bg-black/20 p-3">
      <div className="flex flex-wrap gap-2">
        <button
          className="rounded-full border border-white/10 px-3 py-2 text-sm text-white/80 transition hover:border-waey-coral hover:text-white"
          onClick={() => void captureCurrentScreen()}
          type="button"
        >
          Screenshot
        </button>
        <button
          className="rounded-full border border-white/10 px-3 py-2 text-sm text-white/80 transition hover:border-waey-coral hover:text-white"
          onClick={() => void showRegionSelector()}
          type="button"
        >
          Smart Crop
        </button>
        <button
          className="rounded-full border border-white/10 px-3 py-2 text-sm text-white/80 transition hover:border-waey-coral hover:text-white"
          onClick={onOpenHistory}
          type="button"
        >
          History
        </button>
        <button
          className="rounded-full border border-white/10 px-3 py-2 text-sm text-white/80 transition hover:border-waey-coral hover:text-white"
          onClick={onOpenProviders}
          type="button"
        >
          Providers
        </button>
        <button
          className="rounded-full border border-white/10 px-3 py-2 text-sm text-white/80 transition hover:border-waey-coral hover:text-white"
          onClick={onOpenPersonas}
          type="button"
        >
          Personas
        </button>
        <button
          className="rounded-full border border-white/10 px-3 py-2 text-sm text-white/80 transition hover:border-waey-coral hover:text-white"
          onClick={onOpenSettings}
          type="button"
        >
          Settings
        </button>
      </div>

      <div className="flex flex-wrap gap-2">
        <select
          className="max-w-48 rounded-full border border-white/10 bg-waey-panel px-3 py-2 text-sm text-white outline-none focus:border-waey-coral"
          onChange={(event) => onSelectPersona(event.currentTarget.value)}
          value={selectedPersonaId}
        >
          <option value="">Default Waey</option>
          {personas.map((persona) => (
            <option key={persona.id} value={persona.id}>
              {persona.name}
            </option>
          ))}
        </select>
        <select
          className="max-w-48 rounded-full border border-white/10 bg-waey-panel px-3 py-2 text-sm text-white outline-none focus:border-waey-coral"
          onChange={(event) => onSelectProvider(event.currentTarget.value)}
          value={selectedProviderId}
        >
          {providers.length === 0 ? <option value="">No provider</option> : null}
          {providers.map((provider) => (
            <option key={provider.id} value={provider.id}>
              {provider.name}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
