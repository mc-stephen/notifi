"use client";

import { use, useState } from "react";
import { useRouter } from "next/navigation";
import { useTemplate } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { CodeBlock } from "@/components/custom/code-block";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Separator } from "@/components/ui/separator";
import { format } from "date-fns";
import {
  ArrowLeft,
  Save,
  Eye,
  Copy,
  RotateCcw,
  Trash2,
  Variable,
  Clock,
  CheckCircle2,
  FileText,
} from "lucide-react";

function NewTemplateForm() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [channel, setChannel] = useState("email");
  const [body, setBody] = useState("");

  return (
    <div className="space-y-6">
      <PageHeader
        title="Create Template"
        description="Design a new notification template"
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Templates", href: "/templates" },
          { label: "New" },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Button size="sm" variant="outline" onClick={() => router.push("/templates")}>
              <ArrowLeft className="size-3.5 mr-1" /> Cancel
            </Button>
            <Button size="sm" className="gap-1.5" onClick={() => router.push("/templates")}>
              <Save className="size-3.5" /> Create template
            </Button>
          </div>
        }
      />

      <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="flex items-center gap-2">
                <FileText className="size-4" /> Template Details
              </CardTitle>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Template name</label>
              <Input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. Welcome Email"
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Channel</label>
              <select
                value={channel}
                onChange={(e) => setChannel(e.target.value)}
                className="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
              >
                <option value="email">Email</option>
                <option value="sms">SMS</option>
                <option value="push-android">Android Push</option>
                <option value="push-ios">Apple Push</option>
                <option value="web-push">Web Push</option>
                <option value="slack">Slack</option>
                <option value="discord">Discord</option>
                <option value="webhook">Webhook</option>
              </select>
            </div>
            {channel === "email" && (
              <div className="space-y-2">
                <label className="text-sm font-medium">Subject</label>
                <Input placeholder="Email subject line" />
              </div>
            )}
            <div className="space-y-2">
              <label className="text-sm font-medium">Body</label>
              <textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                placeholder="Write your template body here. Use {{variable_name}} for dynamic values."
                className="w-full min-h-[300px] rounded-md border bg-transparent px-3 py-2 text-sm font-mono placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring resize-y"
              />
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Variables</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-xs text-muted-foreground">
                Variables are detected automatically when you use {"{{variable_name}}"} syntax in the body.
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Preview</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-xs text-muted-foreground">
                Preview will be available after creating the template.
              </p>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

function TemplateEditor({ template }: { template: NonNullable<ReturnType<typeof useTemplate>> }) {
  const [body, setBody] = useState(template.body);
  const [subject, setSubject] = useState(template.subject ?? "");

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Editor</CardTitle>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="outline" className="gap-1.5">
                <Eye className="size-3.5" /> Preview
              </Button>
              <Button size="sm" className="gap-1.5">
                <Save className="size-3.5" /> Save
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {template.channel === "email" && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Subject</label>
              <Input
                value={subject}
                onChange={(e) => setSubject(e.target.value)}
                placeholder="Email subject line"
              />
            </div>
          )}
          <div className="space-y-2">
            <label className="text-sm font-medium">Body</label>
            <textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              className="w-full min-h-[300px] rounded-md border bg-transparent px-3 py-2 text-sm font-mono placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring resize-y"
            />
          </div>
        </CardContent>
      </Card>

      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-sm flex items-center gap-2">
              <Variable className="size-3.5" /> Variables
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {template.variables.map((v) => (
              <div key={v.name} className="space-y-1">
                <div className="flex items-center justify-between">
                  <code className="text-xs font-mono bg-muted px-1.5 py-0.5 rounded">{`{{${v.name}}}`}</code>
                  <Badge variant="secondary" className="text-[10px] px-1 h-3.5">{v.type}</Badge>
                </div>
                {v.description && (
                  <p className="text-xs text-muted-foreground">{v.description}</p>
                )}
                <Input
                  placeholder={v.defaultValue ?? `Enter ${v.name}`}
                  className="h-7 text-xs"
                />
              </div>
            ))}
            {template.variables.length === 0 && (
              <p className="text-xs text-muted-foreground">No variables in this template.</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-sm">Preview Data</CardTitle>
          </CardHeader>
          <CardContent>
            <CodeBlock
              language="JSON"
              code={JSON.stringify(
                Object.fromEntries(template.variables.map((v) => [v.name, v.defaultValue ?? ""])),
                null,
                2,
              )}
            />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function VersionHistory({ template }: { template: NonNullable<ReturnType<typeof useTemplate>> }) {
  const versions = Array.from({ length: template.version }, (_, i) => ({
    version: template.version - i,
    updatedAt: new Date(new Date(template.updatedAt).getTime() - i * 86400000 * 7).toISOString(),
    author: ["Alice Chen", "Bob Kim", "Carol Wu"][i % 3],
    changes: i === 0 ? "Latest version" : `Updated ${["body", "subject", "variables"][i % 3]}`,
  }));

  return (
    <Card>
      <CardHeader>
        <CardTitle>Version History</CardTitle>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Version</TableHead>
              <TableHead>Changes</TableHead>
              <TableHead>Author</TableHead>
              <TableHead>Date</TableHead>
              <TableHead className="w-[80px]"></TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {versions.map((v) => (
              <TableRow key={v.version}>
                <TableCell>
                  <Badge variant={v.version === template.version ? "default" : "secondary"}>
                    v{v.version}
                  </Badge>
                </TableCell>
                <TableCell className="text-sm">{v.changes}</TableCell>
                <TableCell className="text-sm text-muted-foreground">{v.author}</TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {format(new Date(v.updatedAt), "MMM d, yyyy")}
                </TableCell>
                <TableCell>
                  {v.version !== template.version && (
                    <Button variant="ghost" size="icon-xs">
                      <RotateCcw className="size-3" />
                    </Button>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

function TemplateDetail({ id }: { id: string }) {
  const template = useTemplate(id);
  const router = useRouter();

  if (id === "new") {
    return <NewTemplateForm />;
  }

  if (!template) {
    return (
      <div className="flex flex-col items-center justify-center py-20">
        <h2 className="text-lg font-medium">Template not found</h2>
        <p className="text-sm text-muted-foreground mt-1">The template {id} does not exist.</p>
        <Button variant="outline" className="mt-4" onClick={() => router.push("/templates")}>
          <ArrowLeft className="size-3.5 mr-1" /> Back to templates
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title={template.name}
        description={template.subject ?? template.body.slice(0, 60)}
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Templates", href: "/templates" },
          { label: template.name },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Badge variant="secondary" className="gap-1">
              <Clock className="size-3" /> v{template.version}
            </Badge>
            {template.isDraft && (
              <Badge variant="outline" className="gap-1 text-warning border-warning/30">
                Draft
              </Badge>
            )}
            <ChannelBadge channel={template.channel} showIcon />
            <Button size="sm" variant="outline" className="gap-1.5">
              <Copy className="size-3.5" /> Duplicate
            </Button>
            <Button size="sm" variant="outline" className="gap-1.5 text-destructive">
              <Trash2 className="size-3.5" /> Delete
            </Button>
          </div>
        }
      />

      <Tabs defaultValue="editor">
        <TabsList>
          <TabsTrigger value="editor">Editor</TabsTrigger>
          <TabsTrigger value="versions">Version History</TabsTrigger>
          <TabsTrigger value="usage">Usage</TabsTrigger>
        </TabsList>

        <TabsContent value="editor" className="mt-4">
          <TemplateEditor template={template} />
        </TabsContent>

        <TabsContent value="versions" className="mt-4">
          <VersionHistory template={template} />
        </TabsContent>

        <TabsContent value="usage" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle>Template Usage</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-3">
                <div className="text-center p-4 rounded-lg bg-muted/50">
                  <div className="text-2xl font-bold">1,247</div>
                  <div className="text-xs text-muted-foreground mt-1">Total sends</div>
                </div>
                <div className="text-center p-4 rounded-lg bg-muted/50">
                  <div className="text-2xl font-bold">98.2%</div>
                  <div className="text-xs text-muted-foreground mt-1">Delivery rate</div>
                </div>
                <div className="text-center p-4 rounded-lg bg-muted/50">
                  <div className="text-2xl font-bold">34.5%</div>
                  <div className="text-xs text-muted-foreground mt-1">Open rate</div>
                </div>
              </div>
              <Separator />
              <div>
                <h4 className="text-sm font-medium mb-2">Last 5 sends</h4>
                <div className="space-y-2">
                  {[1, 2, 3, 4, 5].map((i) => (
                    <div key={i} className="flex items-center justify-between text-sm">
                      <div className="flex items-center gap-2">
                        <CheckCircle2 className="size-3.5 text-success" />
                        <span className="font-mono text-xs">ntf_{String(i).padStart(4, "0")}</span>
                      </div>
                      <span className="text-xs text-muted-foreground">
                        {format(new Date(new Date(template.updatedAt).getTime() - i * 3600000), "MMM d, HH:mm")}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default function TemplateDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  return <TemplateDetail id={id} />;
}
