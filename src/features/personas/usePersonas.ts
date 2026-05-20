import { useCallback, useEffect, useMemo, useState } from "react";
import type { Persona, PersonaDraft } from "../../shared/types";
import { deletePromptPersona, listPromptPersonas, savePromptPersona } from "./personaCommands";

export function usePersonas() {
  const [personas, setPersonas] = useState<Persona[]>([]);
  const [selectedPersonaId, setSelectedPersonaId] = useState<string>("");
  const [personaError, setPersonaError] = useState<string | null>(null);

  const selectedPersona = useMemo(
    () => personas.find((persona) => persona.id === selectedPersonaId) ?? null,
    [personas, selectedPersonaId],
  );

  const refreshPersonas = useCallback(async () => {
    setPersonaError(null);

    try {
      const nextPersonas = await listPromptPersonas();

      setPersonas(nextPersonas);
      setSelectedPersonaId((currentId) => {
        if (!currentId || nextPersonas.some((persona) => persona.id === currentId)) {
          return currentId;
        }

        return "";
      });
    } catch (error) {
      setPersonaError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const savePersona = useCallback(
    async (persona: PersonaDraft) => {
      const savedPersona = await savePromptPersona(persona);
      await refreshPersonas();
      setSelectedPersonaId(savedPersona.id);
    },
    [refreshPersonas],
  );

  const deletePersona = useCallback(
    async (personaId: string) => {
      await deletePromptPersona(personaId);
      await refreshPersonas();
    },
    [refreshPersonas],
  );

  useEffect(() => {
    void refreshPersonas();
  }, [refreshPersonas]);

  return {
    deletePersona,
    personaError,
    personas,
    refreshPersonas,
    savePersona,
    selectedPersona,
    selectedPersonaId,
    setSelectedPersonaId,
  };
}
