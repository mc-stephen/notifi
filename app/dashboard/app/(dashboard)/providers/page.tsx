"use client";

import { useProviders } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { HealthIndicator } from "@/components/custom/health-indicator";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  Plus,
  RotateCcw,
  Settings,
  Zap,
  Clock,
  Shield,
} from "lucide-react";

export default function ProvidersPage() {
  const providers = useProviders();

  const healthyCount = providers.filter((p) => p.health === "healthy").length;
  const avgLatency = Math.round(providers.reduce((acc, p) => acc + p.latencyMs, 0) / providers.length);
  const totalQuotaUsed = providers.reduce((acc, p) => acc + p.quotaUsed, 0);
  const totalQuotaLimit = providers.reduce((acc, p) => acc + p.quotaLimit, 0);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Providers"
        description={`${providers.length} configured providers`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Providers" }]}
        actions={
          <Button size="sm" className="gap-1.5">
            <Plus className="size-3.5" /> Add provider
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Healthy</span>
              <Zap className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{healthyCount}/{providers.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Avg Latency</span>
              <Clock className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{avgLatency}ms</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total Quota</span>
              <Shield className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{((totalQuotaUsed / totalQuotaLimit) * 100).toFixed(0)}%</div>
            <p className="text-xs text-muted-foreground mt-1">{totalQuotaUsed.toLocaleString()} / {totalQuotaLimit.toLocaleString()}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Degraded</span>
              <RotateCcw className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{providers.filter((p) => p.health === "degraded").length}</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>All Providers</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Health</TableHead>
                <TableHead>Latency</TableHead>
                <TableHead>Success Rate</TableHead>
                <TableHead>Quota</TableHead>
                <TableHead>Priority</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="w-[60px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {providers.map((provider) => {
                const quotaPercent = Math.round((provider.quotaUsed / provider.quotaLimit) * 100);
                return (
                  <TableRow key={provider.id}>
                    <TableCell>
                      <div>
                        <div className="text-sm font-medium">{provider.name}</div>
                        <div className="text-xs text-muted-foreground">{provider.region}</div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="text-xs">{provider.type}</Badge>
                    </TableCell>
                    <TableCell>
                      <HealthIndicator status={provider.health} />
                    </TableCell>
                    <TableCell className="text-sm">{provider.latencyMs}ms</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <div className="w-16 h-1.5 bg-muted rounded-full overflow-hidden">
                          <div
                            className={`h-full rounded-full ${
                              provider.successRate >= 99 ? "bg-success" :
                              provider.successRate >= 97 ? "bg-warning" : "bg-destructive"
                            }`}
                            style={{ width: `${provider.successRate}%` }}
                          />
                        </div>
                        <span className="text-xs">{provider.successRate}%</span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="text-xs">
                        <span>{quotaPercent}%</span>
                        <div className="w-16 h-1 bg-muted rounded-full overflow-hidden mt-1">
                          <div
                            className={`h-full rounded-full ${
                              quotaPercent >= 80 ? "bg-destructive" :
                              quotaPercent >= 60 ? "bg-warning" : "bg-primary"
                            }`}
                            style={{ width: `${quotaPercent}%` }}
                          />
                        </div>
                      </div>
                    </TableCell>
                    <TableCell className="text-sm">{provider.priority}</TableCell>
                    <TableCell>
                      <Badge variant={provider.enabled ? "default" : "secondary"} className={provider.enabled ? "bg-success/15 text-success border-success/20" : ""}>
                        {provider.enabled ? "Active" : "Disabled"}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Button variant="ghost" size="icon-xs">
                        <Settings className="size-3.5" />
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
