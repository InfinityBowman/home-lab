import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { createApp } from "@/lib/api";

export function CreateApp() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [port, setPort] = useState("3000");

  const mutation = useMutation({
    mutationFn: () => createApp({ name, port: Number(port) }),
    onSuccess: (app) => {
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      navigate(`/apps/${app.name}`);
    },
  });

  return (
    <div className="mx-auto max-w-md">
      <h2 className="mb-6 text-lg font-semibold">Create App</h2>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          mutation.mutate();
        }}
        className="space-y-4"
      >
        <div>
          <label className="mb-1 block text-sm text-neutral-400">App Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="my-app"
            required
            pattern="[a-z][a-z0-9-]*[a-z0-9]"
            className="w-full rounded-md border border-neutral-700 bg-neutral-900 px-3 py-2 font-mono text-sm text-neutral-100 placeholder:text-neutral-600 focus:border-neutral-500 focus:outline-none"
          />
          <p className="mt-1 text-xs text-neutral-500">
            Lowercase, alphanumeric and hyphens. Becomes {name || "my-app"}.jacobmaynard.dev
          </p>
        </div>
        <div>
          <label className="mb-1 block text-sm text-neutral-400">Port</label>
          <input
            type="number"
            value={port}
            onChange={(e) => setPort(e.target.value)}
            min={1}
            max={65535}
            className="w-full rounded-md border border-neutral-700 bg-neutral-900 px-3 py-2 font-mono text-sm text-neutral-100 focus:border-neutral-500 focus:outline-none"
          />
          <p className="mt-1 text-xs text-neutral-500">
            Internal container port your app listens on
          </p>
        </div>

        {mutation.error && (
          <p className="text-sm text-red-400">{(mutation.error as Error).message}</p>
        )}

        <button
          type="submit"
          disabled={mutation.isPending || !name || !Number.isInteger(Number(port)) || Number(port) < 1 || Number(port) > 65535}
          className="w-full rounded-md bg-neutral-100 px-4 py-2 text-sm font-medium text-neutral-900 transition hover:bg-neutral-200 disabled:opacity-50"
        >
          {mutation.isPending ? "Creating..." : "Create App"}
        </button>
      </form>
    </div>
  );
}
