"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { CodeBlock } from "@/components/custom/code-block";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { env } from "@/lib/env";
import { format } from "date-fns";
import {
  KeyRound,
  Plus,
  Eye,
  EyeOff,
  Copy,
  Trash2,
  Play,
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

const MOCK_KEYS: ApiKeyEntry[] = [
  { id: "key_1", name: "Production API Key", prefix: "napi_prod_", environment: "production", permissions: ["read", "write"], lastUsedAt: "2025-06-25T10:00:00Z", enabled: true, createdAt: "2025-01-15T00:00:00Z" },
  { id: "key_3", name: "Development Key", prefix: "napi_dev_", environment: "development", permissions: ["read"], lastUsedAt: "2025-06-20T09:00:00Z", enabled: true, createdAt: "2025-03-10T00:00:00Z" },
  { id: "key_4", name: "Legacy Integration", prefix: "napi_leg_", environment: "production", permissions: ["read"], enabled: false, createdAt: "2024-06-01T00:00:00Z" },
];

const ENV_COLORS: Record<string, string> = {
  production: "bg-success/15 text-success border-success/20",
  development: "bg-info/15 text-info border-info/20",
};

export default function ApiKeysPage() {
  const [keys] = useState(MOCK_KEYS);
  const [visibleKeys, setVisibleKeys] = useState<Record<string, boolean>>({});
  const [deleteDialog, setDeleteDialog] = useState<ApiKeyEntry | null>(null);

  const toggleVisibility = (id: string) => {
    setVisibleKeys((prev) => ({ ...prev, [id]: !prev[id] }));
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
          <TabsTrigger value="playground">Playground</TabsTrigger>
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

        <TabsContent value="playground" className="mt-4">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>API Playground</CardTitle>
                <Button size="sm" className="gap-1.5">
                  <Play className="size-3.5" /> Send request
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-2">
                <select className="rounded-md border bg-transparent px-3 py-2 text-sm font-mono">
                  <option>POST</option>
                  <option>GET</option>
                  <option>PUT</option>
                  <option>DELETE</option>
                </select>
                <Input
                  defaultValue={`${env.apiBase}/v1/notifications`}
                  className="flex-1 font-mono text-sm"
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Request Body</label>
                <CodeBlock
                  language="JSON"
                  code={JSON.stringify({
                    recipient_id: "rcp_0001",
                    channel: "email",
                    subject: "Hello from playground",
                    body: "This is a test notification sent from the developer console.",
                    priority: "normal",
                  }, null, 2)}
                />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Response</label>
                <CodeBlock
                  language="JSON"
                  code={JSON.stringify({
                    id: "ntf_0042",
                    status: "queued",
                    channel: "email",
                    recipient_id: "rcp_0001",
                    created_at: new Date().toISOString(),
                  }, null, 2)}
                />
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
