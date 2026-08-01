"use client";

import { useState, useMemo } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { CodeBlock } from "@/components/custom/code-block";
import { env } from "@/lib/env";
import { format } from "date-fns";
import {
  Activity,
  Gauge,
  Search,
  Zap,
  Clock,
  ArrowUpRight,
  ArrowDownRight,
  ExternalLink,
  Play,
} from "lucide-react";

type ApiLog = {
  id: string;
  method: string;
  path: string;
  statusCode: number;
  latencyMs: number;
  timestamp: string;
};

function generateApiLogs(count: number): ApiLog[] {
  const methods = ["GET", "POST", "PUT", "DELETE"];
  const paths = ["/v1/notifications", "/v1/recipients", "/v1/templates", "/v1/webhooks", "/v1/campaigns", "/v1/api-keys"];
  return Array.from({ length: count }, (_, i) => {
    const ts = new Date();
    ts.setMinutes(ts.getMinutes() - i * 3);
    return {
      id: `req_${String(i + 1).padStart(5, "0")}`,
      method: methods[i % methods.length],
      path: paths[i % paths.length],
      statusCode: [200, 200, 200, 200, 201, 201, 204, 400, 401, 404, 429, 500][i % 12],
      latencyMs: Math.floor(20 + Math.random() * 500),
      timestamp: ts.toISOString(),
    };
  });
}

const API_LOGS = generateApiLogs(100);

const METHOD_COLORS: Record<string, string> = {
  GET: "bg-success/15 text-success",
  POST: "bg-info/15 text-info",
  PUT: "bg-warning/15 text-warning",
  DELETE: "bg-destructive/15 text-destructive",
};

export default function DevelopersPage() {
  const [search, setSearch] = useState("");
  const [selectedMethod, setSelectedMethod] = useState<string | null>(null);

  const filteredLogs = useMemo(() => {
    let result = [...API_LOGS];
    if (search) {
      const q = search.toLowerCase();
      result = result.filter((l) => l.path.toLowerCase().includes(q) || l.id.toLowerCase().includes(q));
    }
    if (selectedMethod) {
      result = result.filter((l) => l.method === selectedMethod);
    }
    return result;
  }, [search, selectedMethod]);

  const totalRequests = API_LOGS.length;
  const errorRate = ((API_LOGS.filter((l) => l.statusCode >= 400).length / totalRequests) * 100).toFixed(1);
  const avgLatency = Math.round(API_LOGS.reduce((acc, l) => acc + l.latencyMs, 0) / totalRequests);
  const rateLimited = API_LOGS.filter((l) => l.statusCode === 429).length;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Developer Console"
        description="API status, logs, and playground"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Developers" }]}
        actions={
          <div className="flex items-center gap-2">
            <Button size="sm" variant="outline" className="gap-1.5" render={<a href={env.docs()} target="_blank" rel="noopener noreferrer" />}>
              API docs <ExternalLink className="size-3 ml-1" />
            </Button>
          </div>
        }
      />

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total Requests</span>
              <Activity className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{totalRequests}</div>
            <p className="text-xs text-success mt-1 flex items-center gap-1">
              <ArrowUpRight className="size-3" /> 12% vs last hour
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Avg Latency</span>
              <Clock className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{avgLatency}ms</div>
            <p className="text-xs text-success mt-1 flex items-center gap-1">
              <ArrowDownRight className="size-3" /> -8ms vs last hour
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Error Rate</span>
              <Gauge className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{errorRate}%</div>
            <p className="text-xs text-muted-foreground mt-1">{API_LOGS.filter((l) => l.statusCode >= 400).length} errors</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Rate Limited</span>
              <Zap className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{rateLimited}</div>
            <p className="text-xs text-muted-foreground mt-1">429 responses</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="logs">
        <TabsList>
          <TabsTrigger value="logs">Request Logs</TabsTrigger>
          <TabsTrigger value="limits">Rate Limits</TabsTrigger>
          <TabsTrigger value="playground">Playground</TabsTrigger>
        </TabsList>

        <TabsContent value="logs" className="mt-4">
          <div className="flex items-center gap-3 mb-4">
            <div className="relative flex-1 max-w-sm">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
              <Input
                placeholder="Search by path or request ID..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="pl-8 h-8 text-sm"
              />
            </div>
            <div className="flex items-center gap-2">
              {(["GET", "POST", "PUT", "DELETE"] as const).map((m) => (
                <button
                  key={m}
                  onClick={() => setSelectedMethod(selectedMethod === m ? null : m)}
                  className={`rounded-md border px-2.5 py-1 text-xs transition-colors ${
                    selectedMethod === m ? "border-primary bg-primary/5" : "hover:bg-muted/50"
                  }`}
                >
                  {m}
                </button>
              ))}
            </div>
          </div>

          <Card>
            <CardContent className="p-0">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Request ID</TableHead>
                    <TableHead>Method</TableHead>
                    <TableHead>Path</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Latency</TableHead>
                    <TableHead>Timestamp</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredLogs.slice(0, 50).map((log) => (
                    <TableRow key={log.id}>
                      <TableCell>
                        <span className="font-mono text-xs">{log.id}</span>
                      </TableCell>
                      <TableCell>
                        <Badge variant="secondary" className={`text-[10px] font-mono ${METHOD_COLORS[log.method]}`}>
                          {log.method}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-mono text-xs">{log.path}</TableCell>
                      <TableCell>
                        <Badge
                          variant="secondary"
                          className={`text-[10px] font-mono ${
                            log.statusCode < 300 ? "bg-success/15 text-success" :
                            log.statusCode < 400 ? "bg-info/15 text-info" :
                            log.statusCode < 500 ? "bg-warning/15 text-warning" :
                            "bg-destructive/15 text-destructive"
                          }`}
                        >
                          {log.statusCode}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-xs">
                        <span className={log.latencyMs > 300 ? "text-warning" : ""}>
                          {log.latencyMs}ms
                        </span>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">
                        {format(new Date(log.timestamp), "MMM d, HH:mm:ss")}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="limits" className="mt-4">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle>Rate Limits</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                {[
                  { endpoint: "POST /v1/notifications", limit: "100 req/min", current: 42, color: "bg-success" },
                  { endpoint: "GET /v1/notifications", limit: "500 req/min", current: 280, color: "bg-success" },
                  { endpoint: "POST /v1/recipients", limit: "50 req/min", current: 45, color: "bg-warning" },
                  { endpoint: "POST /v1/webhooks", limit: "20 req/min", current: 8, color: "bg-success" },
                ].map((r) => (
                  <div key={r.endpoint} className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                      <code className="font-mono text-xs">{r.endpoint}</code>
                      <span className="text-xs text-muted-foreground">{r.limit}</span>
                    </div>
                    <div className="h-2 bg-muted rounded-full overflow-hidden">
                      <div
                        className={`h-full rounded-full ${r.color}`}
                        style={{ width: `${(r.current / parseInt(r.limit)) * 100}%` }}
                      />
                    </div>
                    <p className="text-xs text-muted-foreground">Current: {r.current} requests/min</p>
                  </div>
                ))}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Your Plan Limits</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {[
                  { label: "Notifications API", value: "10,000 / day", used: "6,234" },
                  { label: "Recipients API", value: "5,000 / day", used: "1,200" },
                  { label: "Templates API", value: "1,000 / day", used: "340" },
                  { label: "Webhooks API", value: "500 / day", used: "89" },
                  { label: "File Upload", value: "100 MB / day", used: "12 MB" },
                ].map((l) => (
                  <div key={l.label} className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">{l.label}</span>
                    <div className="text-right">
                      <span className="font-mono text-xs">{l.used}</span>
                      <span className="text-muted-foreground text-xs"> / {l.value}</span>
                    </div>
                  </div>
                ))}
              </CardContent>
            </Card>
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
