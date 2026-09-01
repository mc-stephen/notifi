"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
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
  FolderOpen,
  Plus,
  Globe,
  Key,
  Settings,
  Trash2,
  Copy,
} from "lucide-react";

type ProjectEntry = {
  id: string;
  name: string;
  slug: string;
  description?: string;
  environments: {
    name: string;
    apiKeyPrefix: string;
    notificationsToday: number;
  }[];
  createdAt: string;
};

const MOCK_PROJECTS: ProjectEntry[] = [
  {
    id: "proj_1",
    name: "Acme Web App",
    slug: "acme-web",
    description: "Main web application notifications",
    environments: [
      { name: "Production", apiKeyPrefix: "napi_prod_", notificationsToday: 12847 },
      { name: "Development", apiKeyPrefix: "napi_dev_", notificationsToday: 89 },
    ],
    createdAt: "2025-01-10T00:00:00Z",
  },
  {
    id: "proj_2",
    name: "Mobile App",
    slug: "mobile-app",
    description: "iOS and Android push notifications",
    environments: [
      { name: "Production", apiKeyPrefix: "napi_prod_m_", notificationsToday: 8420 },
      { name: "Development", apiKeyPrefix: "napi_dev_m_", notificationsToday: 120 },
    ],
    createdAt: "2025-02-15T00:00:00Z",
  },
  {
    id: "proj_3",
    name: "Internal Tools",
    slug: "internal",
    description: "Internal team notifications and alerts",
    environments: [
      { name: "Production", apiKeyPrefix: "napi_prod_i_", notificationsToday: 245 },
      { name: "Development", apiKeyPrefix: "napi_dev_i_", notificationsToday: 0 },
    ],
    createdAt: "2025-03-20T00:00:00Z",
  },
];

export default function ProjectsPage() {
  const [createDialog, setCreateDialog] = useState(false);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Projects"
        description={`${MOCK_PROJECTS.length} projects`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Projects" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setCreateDialog(true)}>
            <Plus className="size-3.5" /> New project
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total Projects</span>
              <FolderOpen className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{MOCK_PROJECTS.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Environments</span>
              <Globe className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">
              {MOCK_PROJECTS.reduce((acc, p) => acc + p.environments.length, 0)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Notifications Today</span>
              <Key className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">
              {MOCK_PROJECTS.reduce((acc, p) => acc + p.environments.reduce((a, e) => a + e.notificationsToday, 0), 0).toLocaleString()}
            </div>
          </CardContent>
        </Card>
      </div>

      {MOCK_PROJECTS.map((project) => (
        <Card key={project.id}>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="flex size-10 items-center justify-center rounded-lg bg-muted text-sm font-bold">
                  {project.name.charAt(0)}
                </div>
                <div>
                  <CardTitle className="text-base">{project.name}</CardTitle>
                  <p className="text-xs text-muted-foreground">{project.description}</p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Button variant="ghost" size="icon-xs">
                  <Settings className="size-3.5" />
                </Button>
                <Button variant="ghost" size="icon-xs" className="text-destructive">
                  <Trash2 className="size-3.5" />
                </Button>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="text-xs text-muted-foreground mb-3">
              Created {format(new Date(project.createdAt), "MMM d, yyyy")} · Slug: <code className="font-mono">{project.slug}</code>
            </div>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Environment</TableHead>
                  <TableHead>API Key Prefix</TableHead>
                  <TableHead>Notifications Today</TableHead>
                  <TableHead className="w-[80px]"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {project.environments.map((env) => (
                  <TableRow key={env.name}>
                    <TableCell>
                      <Badge
                        variant="secondary"
                        className={`text-xs ${
                          env.name === "Production" ? "bg-success/15 text-success" :
                          env.name === "Staging" ? "bg-warning/15 text-warning" :
                          "bg-info/15 text-info"
                        }`}
                      >
                        {env.name}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <code className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded">{env.apiKeyPrefix}••••••••</code>
                        <Button variant="ghost" size="icon-xs">
                          <Copy className="size-3" />
                        </Button>
                      </div>
                    </TableCell>
                    <TableCell className="text-sm">{env.notificationsToday.toLocaleString()}</TableCell>
                    <TableCell>
                      <Button variant="ghost" size="icon-xs">
                        <Settings className="size-3.5" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      ))}

      <Dialog open={createDialog} onOpenChange={setCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create project</DialogTitle>
            <DialogDescription>Create a new project to organize notifications.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Project name</label>
              <Input placeholder="e.g. My Web App" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Slug</label>
              <Input placeholder="my-web-app" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Description</label>
              <Input placeholder="Brief description of the project" />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateDialog(false)}>Cancel</Button>
            <Button onClick={() => setCreateDialog(false)}>Create project</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
