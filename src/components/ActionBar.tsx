import { captureCurrentScreen, showRegionSelector } from "../features/capture";
import type { LlmProvider, Persona } from "../shared/types";

interface ActionBarProps {
  onOpenPersonas?: () => void;
  providers: LlmProvider[];
  personas: Persona[];
  onOpenHistory?: () => void;
  onOpenSettings?: () => void;
  selectedPersonaId: string;
  selectedProviderId: string;
  onOpenProviders?: () => void;
  onSelectPersona: (personaId: string) => void;
  onSelectProvider: (providerId: string) => void;
}

export function ActionBar({ providers, personas, selectedPersonaId, selectedProviderId, onSelectPersona, onSelectProvider }: ActionBarProps) {
  return (
    <div className="action-bar">
      <div className="action-bar-buttons">
        <button className="action-btn-sm" onClick={() => void captureCurrentScreen()} type="button">📷 Screenshot</button>
        <button className="action-btn-sm" onClick={() => void showRegionSelector()} type="button">✂️ Smart Crop</button>
      </div>
      <div className="action-bar-selects">
        <select className="action-select" onChange={(e) => onSelectPersona(e.currentTarget.value)} value={selectedPersonaId}>
          <option value="">Default</option>
          {personas.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
        <select className="action-select" onChange={(e) => onSelectProvider(e.currentTarget.value)} value={selectedProviderId}>
          {providers.length === 0 ? <option value="">No provider</option> : null}
          {providers.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
      </div>
    </div>
  );
}
