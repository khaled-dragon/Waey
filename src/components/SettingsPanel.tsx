import { useState, type FormEvent } from "react";
import type { AppSettings, LlmProvider, Persona, ProviderDraft, PersonaDraft, ProviderKind } from "../shared/types";

interface SettingsPanelProps {
  errorMessage: string | null;
  isLoading: boolean;
  onChangeSettings: (settings: AppSettings) => Promise<void>;
  settings: AppSettings;
  providers: LlmProvider[];
  personas: Persona[];
  selectedProviderId: string;
  selectedPersonaId: string;
  onSelectProvider: (id: string) => void;
  onSelectPersona: (id: string) => void;
  onSaveProvider: (draft: ProviderDraft) => Promise<void>;
  onDeleteProvider: (id: string) => Promise<void>;
  onSavePersona: (draft: PersonaDraft) => Promise<void>;
  onDeletePersona: (id: string) => Promise<void>;
  isRtl: boolean;
}

type Tab = "general" | "providers" | "personas";

function defaultBaseUrl(kind: ProviderKind) {
  if (kind === "ollama") return "http://localhost:11434/v1";
  if (kind === "openrouter") return "https://openrouter.ai/api/v1";
  return "https://api.openai.com/v1";
}

const initialProviderDraft: ProviderDraft = { name: "", kind: "openrouter", baseUrl: "https://openrouter.ai/api/v1", apiKey: "", model: "" };
const initialPersonaDraft: PersonaDraft = { name: "", prompt: "" };

export function SettingsPanel({ errorMessage, isLoading, onChangeSettings, settings, providers, personas, selectedProviderId, selectedPersonaId, onSelectProvider, onSelectPersona, onSaveProvider, onDeleteProvider, onSavePersona, onDeletePersona, isRtl }: SettingsPanelProps) {
  const [tab, setTab] = useState<Tab>("general");
  const [providerDraft, setProviderDraft] = useState<ProviderDraft>(initialProviderDraft);
  const [personaDraft, setPersonaDraft] = useState<PersonaDraft>(initialPersonaDraft);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [personaError, setPersonaError] = useState<string | null>(null);
  const [savingProvider, setSavingProvider] = useState(false);
  const [savingPersona, setSavingPersona] = useState(false);

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    void onChangeSettings({ ...settings, [key]: value });
  }

  function updateProviderDraft<K extends keyof ProviderDraft>(key: K, value: ProviderDraft[K]) {
    setProviderDraft((draft) => ({ ...draft, [key]: value }));
  }

  function updatePersonaDraft<K extends keyof PersonaDraft>(key: K, value: PersonaDraft[K]) {
    setPersonaDraft((draft) => ({ ...draft, [key]: value }));
  }

  async function handleSaveProvider(e: FormEvent) {
    e.preventDefault();
    setSavingProvider(true); setProviderError(null);
    try { await onSaveProvider(providerDraft); setProviderDraft(initialProviderDraft); }
    catch (err) { setProviderError(err instanceof Error ? err.message : String(err)); }
    finally { setSavingProvider(false); }
  }

  async function handleSavePersona(e: FormEvent) {
    e.preventDefault();
    setSavingPersona(true); setPersonaError(null);
    try { await onSavePersona(personaDraft); setPersonaDraft(initialPersonaDraft); }
    catch (err) { setPersonaError(err instanceof Error ? err.message : String(err)); }
    finally { setSavingPersona(false); }
  }

  function handleSelectProvider(provider: LlmProvider) {
    onSelectProvider(provider.id);
    setProviderError(null);
    setProviderDraft({
      id: provider.id,
      name: provider.name,
      kind: provider.kind,
      baseUrl: provider.baseUrl,
      apiKey: provider.apiKey,
      model: provider.model,
    });
  }

  function handleSelectPersona(persona: Persona) {
    onSelectPersona(persona.id);
    setPersonaError(null);
    setPersonaDraft({
      id: persona.id,
      name: persona.name,
      prompt: persona.prompt,
    });
  }

  const tabs: { id: Tab; label: string; labelAr: string }[] = [
    { id: "general", label: "General", labelAr: "عام" },
    { id: "providers", label: "Providers", labelAr: "مزودون" },
    { id: "personas", label: "Personas", labelAr: "شخصيات" },
  ];

  return (
    <div className="panel">
      <div className="panel-header">
        <div className="panel-label">{isRtl ? "التفضيلات" : "Preferences"}</div>
        <div className="panel-title">{isRtl ? "الإعدادات" : "Settings"}</div>
      </div>

      <div style={{display:"flex",gap:"4px",background:"var(--bg-surface)",borderRadius:"8px",padding:"3px"}}>
        {tabs.map((t) => (
          <button key={t.id} type="button"
            onClick={() => setTab(t.id)}
            style={{flex:1,padding:"5px 8px",borderRadius:"6px",border:"none",cursor:"pointer",fontSize:"11.5px",fontFamily:"inherit",fontWeight:tab===t.id?"600":"400",background:tab===t.id?"var(--accent)":"transparent",color:tab===t.id?"white":"var(--text-muted)",transition:"all 0.15s"}}>
            {isRtl ? t.labelAr : t.label}
          </button>
        ))}
      </div>

      {errorMessage && <div className="error-msg">{errorMessage}</div>}

      {tab === "general" && (
        <div className="settings-group">
          <div>
            <span className="field-label">{isRtl ? "المظهر" : "Theme"}</span>
            <div className="theme-toggle">
              {(["dark","light","system"] as const).map((t) => (
                <button key={t} type="button" className={`theme-btn ${settings.theme===t?"theme-btn--active":""}`}
                  disabled={isLoading} onClick={() => update("theme", t)}>
                  {t === "dark" ? (isRtl ? "داكن" : "Dark") : t === "light" ? (isRtl ? "فاتح" : "Light") : (isRtl ? "تلقائي" : "System")}
                </button>
              ))}
            </div>
          </div>

          <div>
            <span className="field-label">{isRtl ? "اللغة والاتجاه" : "Language"}</span>
            <div className="lang-toggle">
              <button type="button" className={`lang-btn ${settings.language==="en"?"lang-btn--active":""}`}
                disabled={isLoading} onClick={() => update("language", "en")}>English</button>
              <button type="button" className={`lang-btn ${settings.language==="ar"?"lang-btn--active":""}`}
                disabled={isLoading} onClick={() => update("language", "ar")}>عربي</button>
            </div>
          </div>

          <label className="toggle-row">
            <div>
              <div className="toggle-title">{isRtl ? "التقاط تلقائي عند الفتح" : "Auto capture on open"}</div>
              <div className="toggle-desc">{isRtl ? "التقاط الشاشة تلقائياً عند فتح Waey" : "Capture screen automatically when Waey opens"}</div>
            </div>
            <input type="checkbox" checked={settings.autoCaptureOnOverlay} disabled={isLoading}
              onChange={(e) => update("autoCaptureOnOverlay", e.currentTarget.checked)}
              style={{accentColor:"var(--accent)",width:15,height:15}} />
          </label>

          <div className="settings-note">
            {isRtl ? "Alt+Space لفتح Waey. Ctrl+Space لتحديد منطقة." : "Alt+Space opens Waey. Ctrl+Space selects a region."}
          </div>
        </div>
      )}

      {tab === "providers" && (
        <div style={{display:"flex",flexDirection:"column",gap:"10px"}}>
          <form className="panel-form" onSubmit={handleSaveProvider}>
            <div className="form-row">
              <input className="form-input" placeholder={isRtl ? "اسم المزود" : "Provider name"} value={providerDraft.name} onChange={(e) => updateProviderDraft("name", e.currentTarget.value)} />
              <select className="form-select" value={providerDraft.kind} onChange={(e) => { const kind = e.currentTarget.value as ProviderKind; setProviderDraft((draft) => ({...draft, kind, baseUrl:defaultBaseUrl(kind)})); }}>
                <option value="openrouter">OpenRouter</option>
                <option value="ollama">Ollama</option>
                <option value="custom">OpenAI / Custom</option>
              </select>
            </div>
            <input className="form-input" placeholder="Base URL" value={providerDraft.baseUrl} onChange={(e) => updateProviderDraft("baseUrl", e.currentTarget.value)} />
            <div className="form-row">
              <input className="form-input" placeholder={isRtl ? "نموذج" : "Model ID"} value={providerDraft.model} onChange={(e) => updateProviderDraft("model", e.currentTarget.value)} />
              <input className="form-input" placeholder="API Key" type="password" autoComplete="off" value={providerDraft.apiKey} onChange={(e) => updateProviderDraft("apiKey", e.currentTarget.value)} />
            </div>
            {providerError && <div className="error-inline">{providerError}</div>}
            <button className="btn-primary" disabled={savingProvider} type="submit">{savingProvider ? "..." : (isRtl ? "حفظ المزود" : "Save Provider")}</button>
          </form>
          <div className="item-list">
            {providers.length === 0 ? <div className="empty-list">{isRtl ? "لا يوجد مزودون بعد" : "No providers yet"}</div> : providers.map((p) => (
              <div key={p.id} className={`list-item ${selectedProviderId===p.id?"list-item--active":""}`}>
                <button className="list-item-info" onClick={() => handleSelectProvider(p)} type="button">
                  <div className="list-item-name">{p.name}</div>
                  <div className="list-item-sub">{p.model}</div>
                </button>
                <div className="list-item-actions">
                  {selectedProviderId===p.id && <span className="badge-active">{isRtl ? "نشط" : "Active"}</span>}
                  <button className="btn-secondary" onClick={() => { setProviderDraft(initialProviderDraft); void onDeleteProvider(p.id); }} type="button">{isRtl ? "حذف" : "Delete"}</button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {tab === "personas" && (
        <div style={{display:"flex",flexDirection:"column",gap:"10px"}}>
          <form className="panel-form" onSubmit={handleSavePersona}>
            <input className="form-input" placeholder={isRtl ? "اسم الشخصية" : "Persona name"} value={personaDraft.name} onChange={(e) => updatePersonaDraft("name", e.currentTarget.value)} />
            <textarea className="form-textarea" placeholder={isRtl ? "موجه النظام..." : "System prompt..."} value={personaDraft.prompt} onChange={(e) => updatePersonaDraft("prompt", e.currentTarget.value)} />
            {personaError && <div className="error-inline">{personaError}</div>}
            <button className="btn-primary" disabled={savingPersona} type="submit">{savingPersona ? "..." : (isRtl ? "حفظ الشخصية" : "Save Persona")}</button>
          </form>
          <div className="item-list">
            <div className={`list-item ${selectedPersonaId===""?"list-item--active":""}`}>
              <button className="list-item-info" onClick={() => { onSelectPersona(""); setPersonaDraft(initialPersonaDraft); }} type="button">
                <div className="list-item-name">{isRtl ? "Waey الافتراضي" : "Default Waey"}</div>
                <div className="list-item-sub">{isRtl ? "مساعد واعٍ بالشاشة" : "Screen-aware assistant"}</div>
              </button>
              {selectedPersonaId==="" && <span className="badge-active">{isRtl ? "نشط" : "Active"}</span>}
            </div>
            {personas.map((p) => (
              <div key={p.id} className={`list-item ${selectedPersonaId===p.id?"list-item--active":""}`}>
                <button className="list-item-info" onClick={() => handleSelectPersona(p)} type="button">
                  <div className="list-item-name">{p.name}</div>
                  <div className="list-item-sub" style={{overflow:"hidden",display:"-webkit-box",WebkitLineClamp:2,WebkitBoxOrient:"vertical"}}>{p.prompt}</div>
                </button>
                <div className="list-item-actions">
                  {selectedPersonaId===p.id && <span className="badge-active">{isRtl ? "نشط" : "Active"}</span>}
                  <button className="btn-secondary" onClick={() => { setPersonaDraft(initialPersonaDraft); void onDeletePersona(p.id); }} type="button">{isRtl ? "حذف" : "Delete"}</button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
