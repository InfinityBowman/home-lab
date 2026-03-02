import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listEnvVars, bulkSetEnvVars, deleteEnvVar } from "@/lib/api";

export function EnvVarPanel({ appName }: { appName: string }) {
  const queryClient = useQueryClient();
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");

  const envVars = useQuery({
    queryKey: ["env", appName],
    queryFn: () => listEnvVars(appName),
  });

  const addVar = useMutation({
    mutationFn: () => bulkSetEnvVars(appName, { [newKey]: newValue }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["env", appName] });
      setNewKey("");
      setNewValue("");
    },
  });

  const removeVar = useMutation({
    mutationFn: (key: string) => deleteEnvVar(appName, key),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["env", appName] }),
  });

  if (envVars.isLoading) return <p className="text-sm text-neutral-500">Loading...</p>;
  if (envVars.error) return <p className="text-sm text-red-400">Failed to load env vars: {(envVars.error as Error).message}</p>;

  return (
    <div className="space-y-4">
      {/* Existing vars */}
      {envVars.data && envVars.data.length > 0 && (
        <div className="space-y-1">
          {envVars.data.map((v) => (
            <div
              key={v.key}
              className="flex items-center justify-between rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
            >
              <div className="flex gap-3 font-mono text-xs">
                <span className="text-neutral-200">{v.key}</span>
                <span className="text-neutral-500">{v.value}</span>
              </div>
              <button
                onClick={() => removeVar.mutate(v.key)}
                disabled={removeVar.isPending}
                className="text-xs text-neutral-600 transition hover:text-red-400"
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}

      {envVars.data?.length === 0 && (
        <p className="text-sm text-neutral-500">No environment variables set.</p>
      )}

      {/* Add new */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (newKey) addVar.mutate();
        }}
        className="flex gap-2"
      >
        <input
          type="text"
          value={newKey}
          onChange={(e) => setNewKey(e.target.value.toUpperCase())}
          placeholder="KEY"
          className="flex-1 rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-neutral-500 focus:outline-none"
        />
        <input
          type="text"
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          placeholder="value"
          className="flex-1 rounded-md border border-neutral-700 bg-neutral-900 px-2.5 py-1.5 font-mono text-xs text-neutral-100 placeholder:text-neutral-600 focus:border-neutral-500 focus:outline-none"
        />
        <button
          type="submit"
          disabled={addVar.isPending || !newKey}
          className="rounded-md border border-neutral-700 px-3 py-1.5 text-xs font-medium text-neutral-300 transition hover:border-neutral-500 hover:text-neutral-100 disabled:opacity-50"
        >
          Add
        </button>
      </form>

      {addVar.error && (
        <p className="text-xs text-red-400">{(addVar.error as Error).message}</p>
      )}
    </div>
  );
}
