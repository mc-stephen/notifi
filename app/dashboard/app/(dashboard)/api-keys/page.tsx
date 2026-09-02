"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
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
  Eye,
  EyeOff,
  Copy,
  Trash2,
  ShieldCheck,
  MapPin,
  Lock,
} from "lucide-react";

type ApiKeyEntry = {
  id: string;
  name: string;
  prefix: string;
  environment: string;
  permissions: string[];
  expiresAt?: string;
  lastUsedAt?: string;
  enabled: boolean;
  createdAt: string;
};

type WhitelistEntry = {
  id: string;
  address: string;
  label: string;
  lastUsedAt?: string;
  addedAt: string;
};

const MOCK_KEYS: ApiKeyEntry[] = [
  { id: "key_1", name: "Production API Key", prefix: "napi_prod_", environment: "production", permissions: ["read", "write"], lastUsedAt: "2025-06-25T10:00:00Z", enabled: true, createdAt: "2025-01-15T00:00:00Z" },
  { id: "key_3", name: "Development Key", prefix: "napi_dev_", environment: "development", permissions: ["read"], lastUsedAt: "2025-06-20T09:00:00Z", enabled: true, createdAt: "2025-03-10T00:00:00Z" },
  { id: "key_4", name: "Legacy Integration", prefix: "napi_leg_", environment: "production", permissions: ["read"], enabled: false, createdAt: "2024-06-01T00:00:00Z" },
];

const MOCK_WHITELIST: WhitelistEntry[] = [
  { id: "ip_1", address: "203.0.113.10", label: "Office network", lastUsedAt: "2025-06-25T09:15:00Z", addedAt: "2025-01-10T00:00:00Z" },
  { id: "ip_2", address: "198.51.100.42", label: "CI/CD server", lastUsedAt: "2025-06-24T22:00:00Z", addedAt: "2025-03-02T00:00:00Z" },
  { id: "ip_3", address: "192.0.2.77", label: "Staging", addedAt: "2024-11-20T00:00:00Z" },
];

const ENV_COLORS: Record<string, string> = {
  production: "bg-success/15 text-success border-success/20",
  development: "bg-info/15 text-info border-info/20",
};

export default function ApiKeysPage() {
  const [keys] = useState(MOCK_KEYS);
  const [visibleKeys, setVisibleKeys] = useState<Record<string, boolean>>({});
  const [deleteDialog, setDeleteDialog] = useState<ApiKeyEntry | null>(null);
  const [enforce, setEnforce] = useState(false);
  const [whitelist, setWhitelist] = useState(MOCK_WHITELIST);
  const [newAddress, setNewAddress] = useState("");
  const [newLabel, setNewLabel] = useState("");

  const toggleVisibility = (id: string) => {
    setVisibleKeys((prev) => ({ ...prev, [id]: !prev[id] }));
  };

  const addAddress = () => {
    if (!newAddress.trim()) return;
    setWhitelist((prev) => [
      ...prev,
      {
        id: `ip_${Date.now()}`,
        address: newAddress.trim(),
        label: newLabel.trim() || "Untitled",
        addedAt: new Date().toISOString(),
      },
    ]);
    setNewAddress("");
    setNewLabel("");
  };

  const removeAddress = (id: string) => {
    setWhitelist((prev) => prev.filter((entry) => entry.id !== id));
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="API Keys"
        description={`${keys.length} API keys`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "API Keys" }]}
        actions={
          <Button size="sm" className="gap-1.5">
            <Plus className="size-3.5" /> Generate key
          </Button>
        }
      />

      <Tabs defaultValue="keys">
        <TabsList>
          <TabsTrigger value="keys">API Keys</TabsTrigger>
          <TabsTrigger value="whitelist">IP Whitelisting</TabsTrigger>
        </TabsList>

        <TabsContent value="keys" className="mt-4">
          <div className="space-y-6">
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{keys.filter((k) => k.enabled).length}</div>
            <p className="text-xs text-muted-foreground mt-1">Active keys</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">{keys.filter((k) => k.environment === "production").length}</div>
            <p className="text-xs text-muted-foreground mt-1">Production keys</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="text-2xl font-bold">
              {keys.filter((k) => k.lastUsedAt).length > 0
                ? format(new Date(Math.max(...keys.filter((k) => k.lastUsedAt).map((k) => new Date(k.lastUsedAt!).getTime()))), "MMM d")
                : "—"}
            </div>
            <p className="text-xs text-muted-foreground mt-1">Last used</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Key</TableHead>
                <TableHead>Environment</TableHead>
                <TableHead>Permissions</TableHead>
                <TableHead>Last Used</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="w-[80px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {keys.map((key) => (
                <TableRow key={key.id}>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <KeyRound className="size-3.5 text-muted-foreground" />
                      <span className="text-sm font-medium">{key.name}</span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <code className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded">
                        {visibleKeys[key.id] ? `${key.prefix}sk_live_...` : `${key.prefix}${"•".repeat(12)}`}
                      </code>
                      <Button variant="ghost" size="icon-xs" onClick={() => toggleVisibility(key.id)}>
                        {visibleKeys[key.id] ? <EyeOff className="size-3" /> : <Eye className="size-3" />}
                      </Button>
                      <Button variant="ghost" size="icon-xs">
                        <Copy className="size-3" />
                      </Button>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary" className={`text-xs ${ENV_COLORS[key.environment] ?? ""}`}>
                      {key.environment}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      {key.permissions.map((p) => (
                        <Badge key={p} variant="outline" className="text-[10px]">{p}</Badge>
                      ))}
                    </div>
                  </TableCell>
                  <TableCell>
                    <span className="text-xs text-muted-foreground">
                      {key.lastUsedAt ? format(new Date(key.lastUsedAt), "MMM d, HH:mm") : "Never"}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={key.enabled ? "default" : "secondary"} className={key.enabled ? "bg-success/15 text-success border-success/20" : ""}>
                      {key.enabled ? "Active" : "Revoked"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Button variant="ghost" size="icon-xs" className="text-destructive" onClick={() => setDeleteDialog(key)}>
                      <Trash2 className="size-3" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog open={!!deleteDialog} onOpenChange={() => setDeleteDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Revoke API key</DialogTitle>
            <DialogDescription>
              Permanently revoke {deleteDialog?.name}? Applications using this key will stop working.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialog(null)}>Cancel</Button>
            <Button variant="destructive" onClick={() => setDeleteDialog(null)}>Revoke</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
          </div>
        </TabsContent>

        <TabsContent value="whitelist" className="mt-4">
          <div className="space-y-6">
            <Card>
              <CardContent className="pt-5">
                <div className="flex flex-wrap items-center justify-between gap-4">
                  <div className="flex items-start gap-3">
                    <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
                      <ShieldCheck className="size-5" />
                    </div>
                    <div>
                      <div className="text-sm font-semibold">Enforce IP allowlist</div>
                      <p className="max-w-md text-xs text-muted-foreground">
                        When enabled, API requests are only accepted from the IP addresses below.
                        Requests from any other address will be rejected.
                      </p>
                    </div>
                  </div>
                  <Switch size="default" checked={enforce} onCheckedChange={setEnforce} />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <MapPin className="size-4 text-muted-foreground" /> Allowed addresses
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex flex-wrap items-end gap-2">
                  <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground">IP or CIDR</label>
                    <Input
                      placeholder="203.0.113.0/24"
                      value={newAddress}
                      onChange={(e) => setNewAddress(e.target.value)}
                      className="w-52 font-mono text-sm"
                    />
                  </div>
                  <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground">Label</label>
                    <Input
                      placeholder="Office network"
                      value={newLabel}
                      onChange={(e) => setNewLabel(e.target.value)}
                      className="w-48 text-sm"
                    />
                  </div>
                  <Button size="sm" className="gap-1.5" onClick={addAddress}>
                    <Plus className="size-3.5" /> Add address
                  </Button>
                </div>

                <div className="rounded-lg border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Address</TableHead>
                        <TableHead>Label</TableHead>
                        <TableHead>Last used</TableHead>
                        <TableHead className="w-[60px]"></TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {whitelist.map((entry) => (
                        <TableRow key={entry.id}>
                          <TableCell>
                            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                              {entry.address}
                            </code>
                          </TableCell>
                          <TableCell>
                            <span className="text-sm">{entry.label}</span>
                          </TableCell>
                          <TableCell>
                            <span className="text-xs text-muted-foreground">
                              {entry.lastUsedAt ? format(new Date(entry.lastUsedAt), "MMM d, HH:mm") : "Never"}
                            </span>
                          </TableCell>
                          <TableCell>
                            <Button variant="ghost" size="icon-xs" className="text-destructive" onClick={() => removeAddress(entry.id)}>
                              <Trash2 className="size-3.5" />
                            </Button>
                          </TableCell>
                        </TableRow>
                      ))}
                      {whitelist.length === 0 && (
                        <TableRow>
                          <TableCell colSpan={4} className="py-8 text-center text-sm text-muted-foreground">
                            No allowed addresses yet. Add one above to get started.
                          </TableCell>
                        </TableRow>
                      )}
                    </TableBody>
                  </Table>
                </div>

                {!enforce && whitelist.length > 0 && (
                  <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Lock className="size-3.5" />
                    The allowlist is not being enforced until you toggle the setting above.
                  </p>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
