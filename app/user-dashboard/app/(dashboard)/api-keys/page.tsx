"use client";

import { useEffect, useMemo, useState, type ReactNode } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { format } from "date-fns";
import {
  KeyRound,
  Plus,
  Copy,
  Trash2,
  ShieldCheck,
  Loader2,
  Pencil,
  Check,
  X,
} from "lucide-react";
import { useApiKeys, useApiKeyActions, useIpAllowlist } from "@/hooks/use-api-keys";
import { ApiError } from "@/lib/api";
import type { ApiKey } from "@/lib/types";

const ENV_COLORS: Record<string, string> = {
  production: "bg-success/15 text-success border-success/20",
  development: "bg-info/15 text-info border-info/20",
};

function getStatus(key: ApiKey) {
  if (key.revokedAt) return "Revoked";
  return key.enabled ? "Active" : "Disabled";
}

function getStatusClass(key: ApiKey) {
  if (key.revokedAt) return "";
  return key.enabled ? "bg-success/15 text-success border-success/20" : "";
}

function getStatusVariant(key: ApiKey): "default" | "secondary" {
  if (key.revokedAt || !key.enabled) return "secondary";
  return "default";
}

// ---------------------------------------------------------------------------
// Right-side detail panel
// ---------------------------------------------------------------------------

function DetailRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <dt className="shrink-0 pt-px text-xs text-muted-foreground">{label}</dt>
      <dd className="min-w-0 text-right">{children}</dd>
    </div>
  );
}

function ApiKeyDetailPanel({
  apiKey,
  onChanged,
  onClose,
}: {
  apiKey: ApiKey;
  onChanged: () => void;
  onClose: () => void;
}) {
  const { update, revoke } = useApiKeyActions();
  const { entries, loading, error, add, remove } = useIpAllowlist(apiKey.id);

  const [editingName, setEditingName] = useState(false);
  const [name, setName] = useState(apiKey.name);
  const [actionError, setActionError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [revokeOpen, setRevokeOpen] = useState(false);

  const [newCidr, setNewCidr] = useState("");
  const [newLabel, setNewLabel] = useState("");
  const [ipError, setIpError] = useState<string | null>(null);
  const [ipSaving, setIpSaving] = useState(false);

  const revoked = !!apiKey.revokedAt;

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const handleRename = async () => {
    setSaving(true);
    setActionError(null);
    try {
      await update(apiKey.id, { name: name.trim() });
      setEditingName(false);
      onChanged();
    } catch (e) {
      setActionError(e instanceof ApiError ? e.message : "Failed to rename key");
    } finally {
      setSaving(false);
    }
  };

  const handleToggleEnabled = async (enabled: boolean) => {
    setSaving(true);
    setActionError(null);
    try {
      await update(apiKey.id, { enabled });
      onChanged();
    } catch (e) {
      setActionError(e instanceof ApiError ? e.message : "Failed to update key");
    } finally {
      setSaving(false);
    }
  };

  const handleRevoke = async () => {
    setSaving(true);
    setActionError(null);
    try {
      await revoke(apiKey.id);
      setRevokeOpen(false);
      onChanged();
    } catch (e) {
      setActionError(e instanceof ApiError ? e.message : "Failed to revoke key");
    } finally {
      setSaving(false);
    }
  };

  const handleAddIp = async () => {
    if (!newCidr.trim()) return;
    setIpSaving(true);
    setIpError(null);
    try {
      await add(newCidr.trim(), newLabel.trim() || undefined);
      setNewCidr("");
      setNewLabel("");
    } catch (e) {
      setIpError(e instanceof ApiError ? e.message : "Failed to add address");
    } finally {
      setIpSaving(false);
    }
  };

  const handleRemoveIp = async (entryId: string) => {
    setIpError(null);
    try {
      await remove(entryId);
    } catch (e) {
      setIpError(e instanceof ApiError ? e.message : "Failed to remove address");
    }
  };

  const statusDot = revoked
    ? "bg-destructive"
    : apiKey.enabled
      ? "bg-emerald-500"
      : "bg-muted-foreground/40";

  return (
    <aside className="flex min-h-0 w-96 shrink-0 flex-col overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm animate-in slide-in-from-right-3 fade-in duration-200">
      {/* Header — key identity */}
      <div className="flex items-start justify-between gap-2 border-b px-5 py-4">
        <div className="min-w-0 space-y-1.5">
          <div className="flex items-center gap-2">
            <span className={`size-2 shrink-0 rounded-full ${statusDot}`} />
            <h3 className="truncate font-semibold text-foreground">
              {apiKey.name}
            </h3>
          </div>
          <code className="block truncate font-mono text-xs text-muted-foreground">
            {apiKey.prefix}&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;
          </code>
        </div>
        <button
          type="button"
          aria-label="Close details"
          className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          onClick={onClose}
        >
          <X className="size-4" />
        </button>
      </div>

      {/* Scrollable body */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {actionError && (
          <p className="mx-5 mt-3 text-xs text-destructive">{actionError}</p>
        )}

        {/* Overview — definition-list style */}
        <div className="px-5 pt-4 pb-3">
          <h4 className="mb-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Overview
          </h4>
          <dl className="space-y-2">
            <DetailRow label="Name">
              {editingName ? (
                <div className="flex items-center gap-1.5">
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    className="h-7 flex-1 text-xs"
                    autoFocus
                  />
                  <Button
                    size="sm"
                    className="h-7 px-2"
                    disabled={saving || !name.trim()}
                    onClick={handleRename}
                  >
                    {saving ? <Loader2 className="size-3 animate-spin" /> : <Check className="size-3" />}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2"
                    onClick={() => { setEditingName(false); setName(apiKey.name); }}
                  >
                    <X className="size-3" />
                  </Button>
                </div>
              ) : (
                <div className="flex items-center gap-1">
                  <span className="text-xs font-medium">{apiKey.name}</span>
                  {!revoked && (
                    <button
                      type="button"
                      className="rounded p-0.5 text-muted-foreground hover:text-foreground"
                      onClick={() => setEditingName(true)}
                    >
                      <Pencil className="size-3" />
                    </button>
                  )}
                </div>
              )}
            </DetailRow>
            <DetailRow label="Environment">
              <Badge
                variant="secondary"
                className={`text-[10px] font-medium ${ENV_COLORS[apiKey.environment] ?? ""}`}
              >
                {apiKey.environment}
              </Badge>
            </DetailRow>
            <DetailRow label="Status">
              <Badge
                variant={getStatusVariant(apiKey)}
                className={`text-[10px] font-medium ${getStatusClass(apiKey)}`}
              >
                {getStatus(apiKey)}
              </Badge>
            </DetailRow>
            <DetailRow label="Permissions">
              <div className="flex flex-wrap gap-1">
                {apiKey.permissions.map((p) => (
                  <Badge key={p} variant="outline" className="text-[10px]">
                    {p}
                  </Badge>
                ))}
              </div>
            </DetailRow>
            {apiKey.createdAt && (
              <DetailRow label="Created">
                <span className="text-xs text-foreground">
                  {new Date(apiKey.createdAt).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}
                </span>
              </DetailRow>
            )}
            {apiKey.lastUsedAt && (
              <DetailRow label="Last used">
                <span className="text-xs text-foreground">
                  {new Date(apiKey.lastUsedAt).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}
                </span>
              </DetailRow>
            )}
          </dl>
        </div>

        <div className="mx-5 h-px bg-border" />

        {/* IP allowlist */}
        <div className="px-5 py-4">
          <h4 className="mb-2.5 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            IP allowlist
          </h4>

          <div className="mb-3 flex items-center gap-1.5 text-xs text-muted-foreground">
            <ShieldCheck className="size-3.5" />
            {entries.length === 0
              ? "Unrestricted — works from any IP"
              : `Restricted — ${entries.length} address${entries.length === 1 ? "" : "es"}`}
          </div>

          {error && <p className="mb-2 text-xs text-destructive">{error}</p>}
          {ipError && <p className="mb-2 text-xs text-destructive">{ipError}</p>}

          <div className="space-y-1.5">
            <Input
              placeholder="IP or CIDR (e.g. 203.0.113.0/24)"
              value={newCidr}
              onChange={(e) => setNewCidr(e.target.value)}
              className="h-8 font-mono text-xs"
            />
            <Input
              placeholder="Label (optional)"
              value={newLabel}
              onChange={(e) => setNewLabel(e.target.value)}
              className="h-8 text-xs"
            />
            <Button
              size="sm"
              className="h-8 w-full gap-1.5 text-xs"
              disabled={ipSaving || !newCidr.trim()}
              onClick={handleAddIp}
            >
              {ipSaving ? <Loader2 className="size-3 animate-spin" /> : <Plus className="size-3" />}
              Add address
            </Button>
          </div>

          {loading ? (
            <div className="flex items-center justify-center py-6">
              <Loader2 className="size-4 animate-spin text-muted-foreground" />
            </div>
          ) : entries.length > 0 ? (
            <div className="mt-3 space-y-1">
              {entries.map((entry) => (
                <div
                  key={entry.id}
                  className="group flex items-center gap-2 rounded-md bg-muted/50 px-2.5 py-1.5"
                >
                  <code className="flex-1 truncate font-mono text-xs">
                    {entry.cidr}
                  </code>
                  {entry.label && (
                    <span className="shrink-0 text-[10px] text-muted-foreground">
                      {entry.label}
                    </span>
                  )}
                  <button
                    type="button"
                    className="shrink-0 rounded p-0.5 text-muted-foreground opacity-0 transition-opacity hover:text-destructive group-hover:opacity-100"
                    onClick={() => handleRemoveIp(entry.id)}
                  >
                    <Trash2 className="size-3" />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <p className="py-4 text-center text-xs text-muted-foreground">
              No addresses yet.
            </p>
          )}
        </div>
      </div>

      {/* Sticky footer — actions */}
      <div className="flex items-center justify-between gap-3 border-t px-5 py-3">
        <div className="flex items-center gap-2">
          <Switch
            checked={apiKey.enabled}
            disabled={revoked || saving}
            onCheckedChange={handleToggleEnabled}
          />
          <span className="text-xs text-muted-foreground">Enabled</span>
        </div>
        {!revoked ? (
          <Button size="sm" variant="destructive" className="gap-1.5" onClick={() => setRevokeOpen(true)}>
            <Trash2 className="size-3.5" /> Revoke
          </Button>
        ) : (
          <span className="text-[11px] font-medium uppercase tracking-wider text-destructive">
            Revoked
          </span>
        )}
      </div>

      <Dialog open={revokeOpen} onOpenChange={setRevokeOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Revoke API key</DialogTitle>
            <DialogDescription>
              Permanently revoke {apiKey.name}? Applications using this key will stop working.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRevokeOpen(false)}>Cancel</Button>
            <Button variant="destructive" disabled={saving} onClick={handleRevoke}>
              {saving && <Loader2 className="size-3.5 animate-spin" />} Revoke
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </aside>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function ApiKeysPage() {
  const { keys, error, refresh } = useApiKeys();
  const { create } = useApiKeyActions();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState("");
  const [environment, setEnvironment] = useState("development");
  const [permissions, setPermissions] = useState<string[]>(["read"]);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const [freshSecret, setFreshSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const selected = useMemo(
    () => keys.find((k) => k.id === selectedId) ?? null,
    [keys, selectedId],
  );

  const stats = useMemo(() => {
    const total = keys.length;
    const active = keys.filter((k) => k.enabled && !k.revokedAt).length;
    const prod = keys.filter((k) => k.environment === "production").length;
    const dev = keys.filter((k) => k.environment === "development").length;
    const inactive = keys.filter((k) => !k.enabled || !!k.revokedAt).length;
    return { total, active, prod, dev, inactive };
  }, [keys]);

  const togglePermission = (perm: string) => {
    setPermissions((prev) =>
      prev.includes(perm) ? prev.filter((p) => p !== perm) : [...prev, perm],
    );
  };

  const handleCreate = async () => {
    setSubmitting(true);
    setSubmitError(null);
    try {
      const res = await create({
        name: name.trim(),
        environment,
        permissions,
      });
      setCreateOpen(false);
      setName("");
      setEnvironment("development");
      setPermissions(["read"]);
      setFreshSecret(res.secret);
      setCopied(false);
      setSelectedId(res.key.id);
      refresh();
    } catch (e) {
      setSubmitError(e instanceof ApiError ? e.message : "Failed to generate key");
    } finally {
      setSubmitting(false);
    }
  };

  const handleCopy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };

  const columns: ColumnDef<ApiKey, unknown>[] = [
    {
      accessorKey: "name",
      header: "Name",
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <KeyRound className="size-3.5 text-muted-foreground shrink-0" />
          <span className="text-sm font-medium">{row.original.name}</span>
        </div>
      ),
    },
    {
      id: "key",
      header: "Key",
      cell: ({ row }) => (
        <code className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded">
          {row.original.prefix}{"•".repeat(12)}
        </code>
      ),
    },
    {
      accessorKey: "environment",
      header: "Environment",
      cell: ({ row }) => (
        <Badge variant="secondary" className={`text-xs ${ENV_COLORS[row.original.environment] ?? ""}`}>
          {row.original.environment}
        </Badge>
      ),
    },
    {
      accessorKey: "permissions",
      header: "Permissions",
      cell: ({ row }) => (
        <div className="flex gap-1">
          {row.original.permissions.map((p) => (
            <Badge key={p} variant="outline" className="text-[10px]">{p}</Badge>
          ))}
        </div>
      ),
    },
    {
      accessorKey: "lastUsedAt",
      header: "Last Used",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {row.original.lastUsedAt
            ? format(new Date(row.original.lastUsedAt), "MMM d, HH:mm")
            : "Never"}
        </span>
      ),
    },
    {
      id: "status",
      header: "Status",
      cell: ({ row }) => (
        <Badge variant={getStatusVariant(row.original)} className={`text-xs ${getStatusClass(row.original)}`}>
          {getStatus(row.original)}
        </Badge>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="API Keys"
        description={`${keys.length} API keys`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "API Keys" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => { setSubmitError(null); setCreateOpen(true); }}>
            <Plus className="size-3.5" /> Generate key
          </Button>
        }
      />

      <div className="grid gap-4 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5">
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{stats.total}</div>
            <p className="text-xs text-muted-foreground mt-1">Total keys</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{stats.active}</div>
            <p className="text-xs text-muted-foreground mt-1">Active keys</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{stats.prod}</div>
            <p className="text-xs text-muted-foreground mt-1">Prod keys</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{stats.dev}</div>
            <p className="text-xs text-muted-foreground mt-1">Dev keys</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{stats.inactive}</div>
            <p className="text-xs text-muted-foreground mt-1">Inactive keys</p>
          </CardContent>
        </Card>
      </div>

      {error && <p className="text-sm text-destructive">{error}</p>}

      <div className="flex min-h-0 flex-1 items-stretch gap-6">
        <div className="min-w-0 flex-1 overflow-y-auto">
          <DataTable
            columns={columns}
            data={keys}
            searchKey="name"
            searchPlaceholder="Search keys…"
            pageSize={10}
            emptyMessage="No API keys yet. Generate one to get started."
            tableClassName="bg-card"
            stripeRows
            onRowClick={(row) => {
              setSelectedId((prev) => (prev === row.id ? null : row.id));
            }}
          />
        </div>

        {selected && (
          <ApiKeyDetailPanel
            key={selected.id}
            apiKey={selected}
            onChanged={refresh}
            onClose={() => setSelectedId(null)}
          />
        )}
      </div>

      {/* Create key dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Generate API key</DialogTitle>
            <DialogDescription>
              The secret is shown exactly once. Store it somewhere safe.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            {submitError && <p className="text-xs text-destructive">{submitError}</p>}
            <div className="space-y-1.5">
              <Label>Name</Label>
              <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Production API Key" />
            </div>
            <div className="space-y-1.5">
              <Label>Environment</Label>
              <Select value={environment} onValueChange={(v) => { if (v) setEnvironment(v); }}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="development">development</SelectItem>
                  <SelectItem value="production">production</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label>Permissions</Label>
              <div className="flex gap-4">
                {(["read", "write"] as const).map((perm) => (
                  <label key={perm} className="flex items-center gap-2 text-sm">
                    <Checkbox
                      checked={permissions.includes(perm)}
                      onCheckedChange={() => togglePermission(perm)}
                    />
                    {perm}
                  </label>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>Cancel</Button>
            <Button disabled={submitting || !name.trim() || permissions.length === 0} onClick={handleCreate}>
              {submitting && <Loader2 className="size-3.5 animate-spin" />} Generate
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Show-once secret dialog */}
      <Dialog open={!!freshSecret} onOpenChange={() => setFreshSecret(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Copy your secret now</DialogTitle>
            <DialogDescription>
              This is the only time the full key is visible. It is stored hashed and cannot be recovered.
            </DialogDescription>
          </DialogHeader>
          <div className="flex items-center gap-2 rounded-lg border bg-muted p-3">
            <code className="flex-1 break-all font-mono text-xs">{freshSecret}</code>
            <Button variant="ghost" size="icon-xs" onClick={() => freshSecret && handleCopy(freshSecret)}>
              {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
            </Button>
          </div>
          <DialogFooter>
            <Button onClick={() => setFreshSecret(null)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
