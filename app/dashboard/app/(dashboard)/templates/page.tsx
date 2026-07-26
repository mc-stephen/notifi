"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { useTemplates, type TemplateFilters } from "@/hooks/use-templates";
import { PageHeader } from "@/components/custom/page-header";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { format } from "date-fns";
import {
  Search,
  FileText,
  Plus,
  FolderOpen,
  Edit3,
  Eye,
  Clock,
  MoreHorizontal,
  Trash2,
  Copy,
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";

const FOLDERS = ["All", "Onboarding", "Security", "Commerce", "Messaging", "Marketing", "System"];

export default function TemplatesPage() {
  const router = useRouter();
  const [filters, setFilters] = useState<TemplateFilters>({});
  const [selectedFolder, setSelectedFolder] = useState("All");

  const templates = useTemplates({
    ...filters,
    folder: selectedFolder === "All" ? undefined : selectedFolder,
  });

  const folders = FOLDERS.map((f) => ({
    name: f,
    count: f === "All" ? templates.length : templates.filter((t) => t.folder === f).length,
  }));

  return (
    <div className="space-y-6">
      <PageHeader
        title="Templates"
        description={`${templates.length} templates across ${FOLDERS.length - 1} folders`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Templates" }]}
        actions={
          <Button size="sm" className="gap-1.5">
            <Plus className="size-3.5" /> New template
          </Button>
        }
      />

      <div className="grid gap-6 lg:grid-cols-[220px_1fr]">
        {/* Folder sidebar */}
        <Card className="h-fit">
          <CardContent className="pt-4 space-y-1">
            {folders.map((folder) => (
              <button
                key={folder.name}
                onClick={() => setSelectedFolder(folder.name)}
                className={`flex w-full items-center justify-between rounded-md px-3 py-2 text-sm transition-colors ${
                  selectedFolder === folder.name
                    ? "bg-muted font-medium"
                    : "text-muted-foreground hover:bg-muted/50"
                }`}
              >
                <div className="flex items-center gap-2">
                  <FolderOpen className="size-3.5" />
                  <span>{folder.name}</span>
                </div>
                <span className="text-xs text-muted-foreground">{folder.count}</span>
              </button>
            ))}
          </CardContent>
        </Card>

        {/* Template grid */}
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <div className="relative flex-1 max-w-sm">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
              <Input
                placeholder="Search templates..."
                value={filters.search ?? ""}
                onChange={(e) => setFilters((p) => ({ ...p, search: e.target.value || undefined }))}
                className="pl-8 h-8 text-sm"
              />
            </div>
          </div>

          {templates.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center text-muted-foreground">
                No templates found in this folder.
              </CardContent>
            </Card>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
              {templates.map((template) => (
                <Card
                  key={template.id}
                  className="cursor-pointer hover:bg-muted/30 transition-colors group"
                  onClick={() => router.push(`/templates/${template.id}`)}
                >
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-2 min-w-0">
                        <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted">
                          <FileText className="size-4 text-muted-foreground" />
                        </div>
                        <div className="min-w-0">
                          <CardTitle className="text-sm truncate">{template.name}</CardTitle>
                          <div className="flex items-center gap-1.5 mt-0.5">
                            <ChannelBadge channel={template.channel} showIcon />
                            {template.isDraft && (
                              <Badge variant="secondary" className="text-[10px] px-1 h-3.5">Draft</Badge>
                            )}
                          </div>
                        </div>
                      </div>
                      <DropdownMenu>
                        <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" className="opacity-0 group-hover:opacity-100" onClick={(e) => e.stopPropagation()} />}>
                          <MoreHorizontal className="size-3.5" />
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={(e) => { e.stopPropagation(); router.push(`/templates/${template.id}`); }}>
                            <Edit3 className="size-3.5" /> Edit
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={(e) => e.stopPropagation()}>
                            <Eye className="size-3.5" /> Preview
                          </DropdownMenuItem>
                          <DropdownMenuItem onClick={(e) => e.stopPropagation()}>
                            <Copy className="size-3.5" /> Duplicate
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem className="text-destructive" onClick={(e) => e.stopPropagation()}>
                            <Trash2 className="size-3.5" /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </CardHeader>
                  <CardContent className="pt-0">
                    <p className="text-xs text-muted-foreground line-clamp-2 mb-3">
                      {template.subject ?? template.body.slice(0, 100)}
                    </p>
                    <Separator className="mb-3" />
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                      <div className="flex items-center gap-1">
                        <Clock className="size-3" />
                        v{template.version}
                      </div>
                      <span>{format(new Date(template.updatedAt), "MMM d, yyyy")}</span>
                    </div>
                    <div className="flex flex-wrap gap-1 mt-2">
                      {template.variables.slice(0, 3).map((v) => (
                        <Badge key={v.name} variant="outline" className="text-[10px] px-1 h-3.5 font-mono">
                          {`{{${v.name}}}`}
                        </Badge>
                      ))}
                      {template.variables.length > 3 && (
                        <Badge variant="outline" className="text-[10px] px-1 h-3.5">
                          +{template.variables.length - 3}
                        </Badge>
                      )}
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
