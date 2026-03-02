import { useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getApp, deleteApp } from "@/lib/api";
import { StatusBadge } from "@/components/StatusBadge";
import { AppStatusPanel } from "@/features/apps/AppStatusPanel";
import { AppLogsPanel } from "@/features/apps/AppLogsPanel";
import { DeployButton } from "@/features/apps/DeployButton";
import { DeploymentList } from "@/features/apps/DeploymentList";
import { EnvVarPanel } from "@/features/apps/EnvVarPanel";

const tabs = ["Status", "Logs", "Deployments", "Env Vars"] as const;
type Tab = (typeof tabs)[number];

export function AppDetail() {
  const { name } = useParams<{ name: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<Tab>("Status");
  const [confirmDelete, setConfirmDelete] = useState(false);

  const app = useQuery({
    queryKey: ["app", name],
    queryFn: () => getApp(name!),
    refetchInterval: 10_000,
    enabled: !!name,
  });

  const remove = useMutation({
    mutationFn: () => deleteApp(name!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["apps"] });
      navigate("/");
    },
  });

  if (app.isLoading) return <p className="text-sm text-neutral-500">Loading...</p>;
  if (app.error) return <p className="text-sm text-red-400">App not found</p>;
  if (!app.data) return null;

  const a = app.data;

  return (
    <div>
      {/* Header */}
      <div className="mb-6 flex items-start justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h2 className="font-mono text-xl font-bold">{a.name}</h2>
            <StatusBadge status={a.status} />
          </div>
          <p className="mt-1 text-sm text-neutral-400">{a.domain}</p>
          <p className="text-xs text-neutral-500">
            Port {a.port}
            {a.docker_image && <span className="ml-2">{a.docker_image}</span>}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <DeployButton appName={a.name} />
          {!confirmDelete ? (
            <button
              onClick={() => setConfirmDelete(true)}
              className="rounded-md border border-neutral-800 px-3 py-1.5 text-xs text-neutral-500 transition hover:border-red-800 hover:text-red-400"
            >
              Delete
            </button>
          ) : (
            <>
              <button
                onClick={() => setConfirmDelete(false)}
                className="rounded-md border border-neutral-800 px-3 py-1.5 text-xs text-neutral-500 transition hover:text-neutral-200"
              >
                Cancel
              </button>
              <button
                onClick={() => remove.mutate()}
                disabled={remove.isPending}
                className="rounded-md border border-red-800 bg-red-950 px-3 py-1.5 text-xs text-red-400 transition hover:bg-red-900"
              >
                Confirm Delete
              </button>
            </>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="mb-4 flex gap-1 border-b border-neutral-800">
        {tabs.map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm transition ${
              activeTab === tab
                ? "border-b-2 border-neutral-100 text-neutral-100"
                : "text-neutral-500 hover:text-neutral-300"
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === "Status" && <AppStatusPanel app={a} />}
      {activeTab === "Logs" && <AppLogsPanel appName={a.name} enabled={a.status === "running"} />}
      {activeTab === "Deployments" && <DeploymentList appName={a.name} />}
      {activeTab === "Env Vars" && <EnvVarPanel appName={a.name} />}
    </div>
  );
}
