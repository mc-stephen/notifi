"use client";

import { use, useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useTemplate, useTemplateActions } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import {
  TEMPLATE_CHANNELS,
  templateChannelModel,
  coveredChannels,
  getChannelField,
  setChannelField,
  emptyChannelFields,
  type TemplateField,
} from "@/lib/template-content";
import type { TemplateAttachment, TemplateContent } from "@/lib/types";
import { ApiError } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { format } from "date-fns";
import {
  ArrowLeft,
  Save,
  Trash2,
  Loader2,
  AlertTriangle,
  FileText,
  Paperclip,
  Plus,
  Clock,
  Settings2,
} from "lucide-react";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function AttachmentRow({
  attachment,
  onRemove,
}: {
  attachment: { id?: string; name: string; mimeType: string; sizeBytes: number };
  onRemove: () => void;
}) {
  return (
    <div className="flex items-center gap-3 rounded-md border px-3 py-2">
      <FileText className="size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{attachment.name}</div>
        <div className="text-xs text-muted-foreground">
          {attachment.mimeType} · {formatBytes(attachment.sizeBytes)}
        </div>
      </div>
      <Button variant="ghost" size="icon-xs" onClick={onRemove}>
        <Trash2 className="size-3.5" />
      </Button>
    </div>
  );
}

function ChannelTab({
  channel,
  active,
  onClick,
}: {
  channel: string;
  active: boolean;
  onClick: () => void;
}) {
  const model = templateChannelModel(channel);
  const Icon = model.icon;
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
        active
          ? "bg-foreground text-background"
          : "text-muted-foreground hover:bg-muted",
      )}
    >
      <Icon className="size-3.5" />
      {model.label}
    </button>
  );
}

function ChannelFieldInput({
  id,
  field,
  value,
  onChange,
}: {
  id: string;
  field: TemplateField;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{field.label}</Label>
      {field.multiline ? (
        <textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.placeholder}
          className="w-full min-h-[160px] rounded-md border bg-transparent px-3 py-2 text-sm font-mono placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring resize-y"
        />
      ) : (
        <Input
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.placeholder}
        />
      )}
    </div>
  );
}

function EmailFormatTabs({
  value,
  onChange,
}: {
  value: "html" | "text";
  onChange: (value: "html" | "text") => void;
}) {
  return (
    <div className="inline-flex items-center gap-1 rounded-lg border bg-muted/40 p-1">
      {(
        [
          { value: "html", label: "HTML" },
          { value: "text", label: "Plain text" },
        ] as const
      ).map((opt) => (
        <button
          key={opt.value}
          type="button"
          onClick={() => onChange(opt.value)}
          className={cn(
            "rounded-md px-3 py-1 text-xs font-medium transition-colors",
            value === opt.value
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}

function TemplateEditor({ id }: { id: string }) {
  const router = useRouter();
  const { template, loading, error, refresh } = useTemplate(id);
  const { update, remove } = useTemplateActions();

  // Draft: the full per-channel content blob { email: {...}, sms: {...} }.
  const [draft, setDraft] = useState<TemplateContent>({});
  const [activeChannel, setActiveChannel] = useState<string>("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  // Email sub-tab: which body format (HTML or plain text) is shown for email.
  const [emailFormat, setEmailFormat] = useState<"html" | "text">("html");

  // Attachment drafts (metadata only; existing ones keep ids by url).
  const [drafts, setDrafts] = useState<TemplateAttachment[]>([]);
  const [addOpen, setAddOpen] = useState(false);
  const [addName, setAddName] = useState("");
  const [addMime, setAddMime] = useState("");
  const [addUrl, setAddUrl] = useState("");
  const [addSize, setAddSize] = useState("");

  // Channel management + delete.
  const [manageOpen, setManageOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  // Seed drafts once (never again for the same id, so a post-save reload
  // doesn't wipe in-progress edits).
  const initializedFor = useRef<string | null>(null);
  useEffect(() => {
    if (!template) return;
    if (initializedFor.current === template.id) return;
    initializedFor.current = template.id;
    setDraft({ ...(template.content ?? {}) } as TemplateContent);
    setDrafts(template.attachments ?? []);
  }, [template]);

  const covered = coveredChannels(draft);
  const active = activeChannel || (covered.length > 0 ? covered[0] : "");
  const activeModel = active ? templateChannelModel(active) : null;

  const setField = useCallback(
    (channel: string, key: string, value: string) => {
      setDraft((prev) => setChannelField(prev, channel, key, value));
    },
    [],
  );

  const toggleChannel = useCallback((channel: string, checked: boolean) => {
    setDraft((prev) => {
      if (checked) {
        return { ...prev, [channel]: emptyChannelFields(channel, false) };
      }
      const next = { ...prev };
      delete next[channel];
      return next;
    });
    setActiveChannel((current) => {
      if (current === channel && !checked) {
        return covered.filter((c) => c !== channel)[0] ?? "";
      }
      return current;
    });
  }, [covered]);

  const handleSave = useCallback(async () => {
    if (!template) return;
    setSaving(true);
    setSaveError(null);
    try {
      const attachments = drafts.map((a) => ({
        name: a.name,
        mimeType: a.mimeType,
        sizeBytes: a.sizeBytes,
        url: a.url,
      }));
      await update(template.id, {
        name: template.name,
        description: template.description,
        channel: template.channel,
        content: draft,
        attachments,
      });
      await refresh();
    } catch (e) {
      setSaveError(
        e instanceof ApiError ? e.message : "Failed to save template",
      );
    } finally {
      setSaving(false);
    }
  }, [template, draft, drafts, update, refresh]);

  const handleAddAttachment = useCallback(() => {
    const size = Number(addSize);
    if (!addName.trim() || !addUrl.trim()) return;
    setDrafts((prev) => [
      ...prev,
      {
        id: `draft_${Date.now()}`,
        name: addName.trim(),
        mimeType: addMime.trim() || "application/octet-stream",
        sizeBytes: isNaN(size) || size < 0 ? 0 : size,
        url: addUrl.trim(),
      },
    ]);
    setAddName("");
    setAddMime("");
    setAddUrl("");
    setAddSize("");
    setAddOpen(false);
  }, [addName, addMime, addUrl, addSize]);

  const handleDelete = useCallback(async () => {
    if (!template) return;
    setDeleting(true);
    try {
      await remove(template.id);
      router.push("/templates");
    } finally {
      setDeleting(false);
    }
  }, [template, remove, router]);

  if (loading) {
    return (
      <div className="overflow-hidden rounded-lg border bg-white">
        <div className="flex items-center justify-center py-20 text-muted-foreground">
          <Loader2 className="size-5 animate-spin mr-2" /> Loading template…
        </div>
      </div>
    );
  }

  if (error || !template) {
    return (
      <div className="flex flex-col items-center justify-center py-20">
        <AlertTriangle className="size-8 text-warning" />
        <h2 className="text-lg font-medium mt-3">Template not found</h2>
        <p className="text-sm text-muted-foreground mt-1">
          {error ?? `The template ${id} does not exist.`}
        </p>
        <Button
          variant="outline"
          className="mt-4"
          onClick={() => router.push("/templates")}
        >
          <ArrowLeft className="size-3.5 mr-1" /> Back to templates
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title={template.name}
        description={`v${template.version} · ${covered.length} channel${covered.length === 1 ? "" : "s"}`}
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
            <div className="flex flex-wrap items-center gap-1">
              {covered.map((c) => {
                const model = templateChannelModel(c);
                const Icon = model.icon;
                return (
                  <Badge key={c} variant="outline" className="gap-1">
                    <Icon className="size-3" /> {model.label}
                  </Badge>
                );
              })}
            </div>
            <Button
              size="sm"
              variant="outline"
              className="gap-1.5 text-destructive"
              onClick={() => setConfirmDelete(true)}
            >
              <Trash2 className="size-3.5" /> Delete
            </Button>
            <Button
              size="sm"
              className="gap-1.5"
              onClick={handleSave}
              disabled={saving}
            >
              {saving ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <Save className="size-3.5" />
              )}
              {saving ? "Saving…" : "Save changes"}
            </Button>
          </div>
        }
      />

      <div className="flex items-start gap-2">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted">
          <FileText className="size-4 text-muted-foreground" />
        </div>
        <div className="min-w-0">
          <p className="text-sm font-medium">{template.name}</p>
          <p className="text-sm text-muted-foreground">
            {template.description || "One template, multiple channels"}
          </p>
        </div>
      </div>

      {/* Channel tabs */}
      <div className="flex items-center gap-2">
        <div className="flex flex-wrap items-center gap-1 rounded-lg border bg-card p-1">
          {covered.length === 0 && (
            <span className="px-3 py-1.5 text-sm text-muted-foreground">
              No channels yet — add one below.
            </span>
          )}
          {covered.map((c) => (
            <ChannelTab
              key={c}
              channel={c}
              active={active === c}
              onClick={() => setActiveChannel(c)}
            />
          ))}
        </div>
        <Button
          variant="outline"
          size="sm"
          className="gap-1"
          onClick={() => setManageOpen(true)}
        >
          <Settings2 className="size-3.5" /> Manage channels
        </Button>
      </div>

      {!activeModel ? (
        <div className="overflow-hidden rounded-lg border bg-white">
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
            <Settings2 className="size-8" />
            <p className="text-sm">
              This template doesn&apos;t cover any channels yet. Add channels to
              start authoring content.
            </p>
          </div>
        </div>
      ) : (
        <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                {activeModel.icon && (
                  <activeModel.icon className="size-4" />
                )}
                {activeModel.label} content
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {active === "email" ? (
                <>
                  <ChannelFieldInput
                    id="email-subject"
                    field={activeModel.fields.find((f) => f.key === "subject")!}
                    value={getChannelField(draft, active, "subject")}
                    onChange={(v) => setField(active, "subject", v)}
                  />
                  <div className="space-y-2">
                    <Label>Content format</Label>
                    <EmailFormatTabs value={emailFormat} onChange={setEmailFormat} />
                  </div>
                  {(() => {
                    const field = activeModel.fields.find(
                      (f) => f.key === emailFormat,
                    );
                    return field ? (
                      <ChannelFieldInput
                        id={`email-${emailFormat}`}
                        field={field}
                        value={getChannelField(draft, active, emailFormat)}
                        onChange={(v) => setField(active, emailFormat, v)}
                      />
                    ) : null;
                  })()}
                </>
              ) : (
                activeModel.fields.map((field) => (
                  <ChannelFieldInput
                    key={field.key}
                    id={`field-${field.key}`}
                    field={field}
                    value={getChannelField(draft, active, field.key)}
                    onChange={(v) => setField(active, field.key, v)}
                  />
                ))
              )}
              {saveError && <p className="text-sm text-destructive">{saveError}</p>}
            </CardContent>
          </Card>

          <div className="space-y-4">
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Paperclip className="size-3.5" /> Attachments
                  </CardTitle>
                  <Button
                    variant="outline"
                    size="sm"
                    className="gap-1"
                    onClick={() => setAddOpen(true)}
                  >
                    <Plus className="size-3.5" /> Add
                  </Button>
                </div>
              </CardHeader>
              <CardContent>
                {drafts.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    No attachments. Attachments are metadata only (name, type,
                    size, URL) — no file upload yet.
                  </p>
                ) : (
                  <div className="space-y-2">
                    {drafts.map((a, i) => (
                      <AttachmentRow
                        key={a.id ?? `${a.name}-${i}`}
                        attachment={a}
                        onRemove={() =>
                          setDrafts((prev) => prev.filter((_, j) => j !== i))
                        }
                      />
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-sm">Details</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3 text-sm">
                <div>
                  <dt className="text-muted-foreground">ID</dt>
                  <dd className="mt-0.5 font-mono text-xs break-all">
                    {template.id}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Project</dt>
                  <dd className="mt-0.5 font-mono text-xs break-all">
                    {template.projectId}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Created</dt>
                  <dd className="mt-0.5">
                    {format(new Date(template.createdAt), "MMM d, yyyy HH:mm")}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Updated</dt>
                  <dd className="mt-0.5">
                    {format(new Date(template.updatedAt), "MMM d, yyyy HH:mm")}
                  </dd>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      )}

      {/* Manage channels */}
      <Dialog open={manageOpen} onOpenChange={setManageOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Manage channels</DialogTitle>
            <DialogDescription>
              Choose which channels this template covers. You can add them now
              or later — each tab is authored separately.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-1">
            {TEMPLATE_CHANNELS.map((c) => {
              const checked = covered.includes(c.value);
              const Icon = c.icon;
              return (
                <label
                  key={c.value}
                  className="flex items-center gap-3 rounded-md border px-3 py-2 cursor-pointer"
                  onClick={(e) => e.stopPropagation()}
                >
                  <Checkbox
                    checked={checked}
                    onCheckedChange={(value) => toggleChannel(c.value, !!value)}
                  />
                  <Icon className="size-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{c.label}</span>
                </label>
              );
            })}
          </div>
          <Separator />
          <DialogFooter>
            <Button onClick={() => setManageOpen(false)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={addOpen} onOpenChange={setAddOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add attachment</DialogTitle>
            <DialogDescription>
              Attach a file reference (metadata only) to this template.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="att-name">Name</Label>
              <Input
                id="att-name"
                value={addName}
                onChange={(e) => setAddName(e.target.value)}
                placeholder="invoice.pdf"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="att-mime">MIME type</Label>
              <Input
                id="att-mime"
                value={addMime}
                onChange={(e) => setAddMime(e.target.value)}
                placeholder="application/pdf"
              />
            </div>
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <Label htmlFor="att-size">Size (bytes)</Label>
                <Input
                  id="att-size"
                  value={addSize}
                  onChange={(e) => setAddSize(e.target.value)}
                  placeholder="1024"
                  inputMode="numeric"
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="att-url">URL</Label>
              <Input
                id="att-url"
                value={addUrl}
                onChange={(e) => setAddUrl(e.target.value)}
                placeholder="https://cdn.example.com/files/invoice.pdf"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setAddOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleAddAttachment}
              disabled={!addName.trim() || !addUrl.trim()}
            >
              Add attachment
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete template</DialogTitle>
            <DialogDescription>
              Permanently delete {template.name} ({template.id})? This cannot be
              undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmDelete(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              disabled={deleting}
            >
              {deleting ? "Deleting…" : "Delete"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default function TemplateDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  return <TemplateEditor id={id} />;
}
