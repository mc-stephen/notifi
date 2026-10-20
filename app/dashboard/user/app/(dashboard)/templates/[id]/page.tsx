"use client";

import { use, useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useTemplate, useTemplateActions, useProviderConfigs } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import {
  templateChannelModel,
  coveredChannels,
  getChannelField,
  setChannelField,
  getVariables,
  setVariables,
  extractRefs,
  undeclaredRefs,
  pruneEmptyChannels,
  DEFAULT_CONTENT_KEY,
  RECIPIENT_REF_PREFIX,
  type TemplateField,
} from "@/lib/template-content";
import type { TemplateAttachment, TemplateContent } from "@/lib/types";
import { ApiError } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
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
  Check,
  Trash2,
  Loader2,
  AlertTriangle,
  FileText,
  Paperclip,
  Plus,
  Clock,
  Braces,
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
  covered,
  active,
  onClick,
}: {
  channel: string;
  covered: boolean;
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
        // Channels without content yet sit dimmed but stay editable.
        !covered && !active && "opacity-50",
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
        <Textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.placeholder}
          className="min-h-[160px] resize-y font-mono"
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
  // Configured channels = the providers wired up for this project.
  const { configs } = useProviderConfigs();

  // Draft: the full per-channel content blob { email: {...}, sms: {...} }.
  const [draft, setDraft] = useState<TemplateContent>({});
  const [activeChannel, setActiveChannel] = useState<string>("");
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  // Success feedback without a toast system: the Save button flips to
  // "Saved" (check icon) for ~2s after a successful PUT.
  const [justSaved, setJustSaved] = useState(false);
  // Email sub-tab: which body format (HTML or plain text) is shown for email.
  const [emailFormat, setEmailFormat] = useState<"html" | "text">("html");

  // Attachment drafts (metadata only; existing ones keep ids by url).
  const [drafts, setDrafts] = useState<TemplateAttachment[]>([]);
  const [addOpen, setAddOpen] = useState(false);
  const [addName, setAddName] = useState("");
  const [addMime, setAddMime] = useState("");
  const [addUrl, setAddUrl] = useState("");
  const [addSize, setAddSize] = useState("");

  // Dynamic data add row (staged; committed to the draft on Add).
  const [varKey, setVarKey] = useState("");
  const [varDesc, setVarDesc] = useState("");
  const [varError, setVarError] = useState<string | null>(null);

  // Delete.
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

  // Coverage derives from the pruned draft: a channel counts as covered only
  // once it carries at least one non-empty field — an empty slice would
  // render the Default fallback.
  const base = pruneEmptyChannels(draft);
  const covered = coveredChannels(base);
  const hasDefault = DEFAULT_CONTENT_KEY in base;
  const active = activeChannel || covered[0] || DEFAULT_CONTENT_KEY;
  const activeModel = templateChannelModel(active);
  // {{refs}} used by the visible channel, declared → shown as OK.
  const activeRefs = Array.from(
    new Set(
      activeModel.fields.flatMap((f) => extractRefs(getChannelField(draft, active, f.key))),
    ),
  );
  const declared = new Set(getVariables(draft).map((v) => v.key));

  // Tab strip: the pinned Default tab first, then the channels that have
  // at least one enabled provider config for this project. No config →
  // Default only. Dimmed = no own content, so it would render Default.
  const configuredChannels = [
    ...new Set(configs.filter((c) => c.enabled).map((c) => c.channel_id)),
  ];
  const tabs = [DEFAULT_CONTENT_KEY, ...configuredChannels];

  const setField = useCallback(
    (channel: string, key: string, value: string) => {
      setDraft((prev) => setChannelField(prev, channel, key, value));
    },
    [],
  );

  // Saves serialize: overlapping auto-saves and manual saves would race.
  const savingRef = useRef(false);
  const savedFlashRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastAutoSig = useRef<string | null>(null);

  // One save path for the whole template: the button commits content
  // fields; Dynamic data and attachment rows commit themselves ~0.9s after
  // they change (effect below). Every save is one PUT of the template as
  // it stands — what you see is what is stored.
  const handleSave = useCallback(async () => {
    if (!template || savingRef.current) return;
    // Undeclared {{refs}} must not reach the server: dynamic data is the
    // single source of truth for what any channel field may reference.
    const missing = undeclaredRefs(draft);
    if (missing.length > 0) {
      const unique = [...new Set(missing.map((m) => `{{${m.ref}}}`))];
      setSaveError(
        `Not saved — ${unique.join(", ")} ${unique.length === 1 ? "is" : "are"} not declared in Dynamic data.`,
      );
      return;
    }
    savingRef.current = true;
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
        content: pruneEmptyChannels(draft),
        attachments,
      });
      await refresh();
      setJustSaved(true);
      if (savedFlashRef.current) clearTimeout(savedFlashRef.current);
      savedFlashRef.current = setTimeout(() => setJustSaved(false), 2000);
    } catch (e) {
      setSaveError(
        e instanceof ApiError ? e.message : "Failed to save template",
      );
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  }, [template, draft, drafts, update, refresh]);

  // Auto-save: Dynamic data and attachment rows commit themselves shortly
  // after they change; content typing stays manual (no PUT per keystroke).
  // The signature string scopes the effect to those two sections only.
  const autoSig = JSON.stringify(getVariables(draft)) + JSON.stringify(drafts);
  useEffect(() => {
    if (!template) return;
    const current = autoSig;
    if (lastAutoSig.current === null) {
      lastAutoSig.current = current; // first render: nothing changed yet
      return;
    }
    if (current === lastAutoSig.current) return;
    const t = setTimeout(() => {
      // A save already in flight skips this one; the next change (or the
      // button) commits it.
      if (savingRef.current) return;
      lastAutoSig.current = current;
      void handleSave();
    }, 900);
    return () => clearTimeout(t);
  }, [autoSig, template, handleSave]);

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

  // Keys are validated at Add time — same charset extractRefs() matches,
  // so a declared ref is always usable and never ambiguous.
  const addVariable = useCallback(() => {
    const key = varKey.trim();
    if (!key) return;
    if (!/^[a-zA-Z0-9_.-]+$/.test(key)) {
      setVarError("Use letters, digits, dots, dashes or underscores only.");
      return;
    }
    const existing = getVariables(draft);
    if (existing.some((v) => v.key === key)) {
      setVarError(`{{${key}}} is already declared.`);
      return;
    }
    setDraft((prev) =>
      setVariables(prev, [
        ...existing,
        { key, description: varDesc.trim() || undefined },
      ]),
    );
    setVarKey("");
    setVarDesc("");
    setVarError(null);
  }, [varKey, varDesc, draft]);

  const removeVariable = useCallback((key: string) => {
    setDraft((prev) =>
      setVariables(prev, getVariables(prev).filter((v) => v.key !== key)),
    );
  }, []);

  const setVarDescription = useCallback(
    (key: string, description: string) => {
      setDraft((prev) =>
        setVariables(
          prev,
          getVariables(prev).map((v) =>
            v.key === key ? { ...v, description: description || undefined } : v,
          ),
        ),
      );
    },
    [],
  );

  if (loading) {
    return (
      <div className="overflow-hidden rounded-lg border bg-card">
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
        description={template.description}
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
          </div>
        }
      />

      {/* Channel tabs — the pinned Default tab first, then the project's
          enabled channels; covered ones are normal, the rest dimmed (they
          would render Default). One scrollable row, no wrap. */}
      <div className="flex items-center">
        <div className="flex items-center gap-1 overflow-x-auto rounded-lg border bg-card p-1">
          {tabs.map((t) => (
            <span key={t} className="shrink-0">
              <ChannelTab
                channel={t}
                covered={t === DEFAULT_CONTENT_KEY ? hasDefault : covered.includes(t)}
                active={active === t}
                onClick={() => setActiveChannel(t)}
              />
            </span>
          ))}
        </div>
      </div>

      {/* Authoring row — content beside the declared vocabulary: both can
          grow tall, so they share the row as peers; declaring a ref and
          using it in a field are one glance apart. */}
      <div className="grid gap-6 lg:grid-cols-[3fr_2fr]">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2">
              {activeModel.icon && <activeModel.icon className="size-4" />}
              {activeModel.label} content
            </CardTitle>
            {/* Content's save — typing must not PUT per keystroke. Local
                feedback only: Saving… spinner, then a brief ✓ Saved flash. */}
            <Button
              size="sm"
              className="shrink-0 gap-1.5"
              onClick={handleSave}
              disabled={saving}
            >
              {saving ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : justSaved ? (
                <Check className="size-3.5" />
              ) : null}
              {saving ? "Saving…" : justSaved ? "Saved" : "Save changes"}
            </Button>
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
                  const field = activeModel.fields.find((f) => f.key === emailFormat);
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

            {/* Live check of {{refs}} used here against Dynamic data. */}
            {activeRefs.length > 0 && (
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="text-xs text-muted-foreground">Refs</span>
                {activeRefs.map((ref) => (
                  <Badge
                    key={ref}
                    variant={
                      ref.startsWith(RECIPIENT_REF_PREFIX + ".")
                        ? "secondary"
                        : declared.has(ref)
                          ? "outline"
                          : "destructive"
                    }
                    className="font-mono text-[10px] font-normal"
                  >
                    {`{{${ref}}}`}
                  </Badge>
                ))}
              </div>
            )}
            {saveError && <p className="text-sm text-destructive">{saveError}</p>}
          </CardContent>
        </Card>

        {/* Dynamic data — the template's declared {{refs}}. Declared here,
            a ref may be used in any channel field; nothing else may. */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-sm flex items-center gap-2">
                <Braces className="size-3.5" /> Dynamic data
              </CardTitle>
              {getVariables(draft).length > 0 && (
                <Badge variant="outline" className="font-mono font-normal">
                  {getVariables(draft).length}
                </Badge>
              )}
            </div>
          </CardHeader>
          <CardContent className="space-y-2">
            {getVariables(draft).length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No variables declared. Declare one below to use it as{" "}
                <span className="font-mono text-xs">{"{{ref}}"}</span> in
                channel fields.
              </p>
            ) : (
              <div className="space-y-1.5">
                {getVariables(draft).map((v) => (
                  <div
                    key={v.key}
                    className="flex items-center gap-2.5 rounded-md border px-3 py-2"
                  >
                    <Badge
                      variant="outline"
                      className="shrink-0 px-1.5 font-mono font-normal"
                    >
                      {`{{${v.key}}}`}
                    </Badge>
                    <Input
                      value={v.description ?? ""}
                      onChange={(e) => setVarDescription(v.key, e.target.value)}
                      placeholder="What fills it (optional)"
                      aria-label={`Description for ${v.key}`}
                      className="h-7 min-w-0 flex-1 border-transparent px-1 text-xs md:text-xs hover:border-input focus-visible:border-ring focus-visible:ring-0 dark:bg-transparent"
                    />
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      aria-label={`Remove ${v.key}`}
                      onClick={() => removeVariable(v.key)}
                    >
                      <Trash2 className="size-3" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
            <div className="rounded-lg bg-muted/30 p-2">
              <div className="flex items-center gap-2">
                <Input
                  value={varKey}
                  onChange={(e) => {
                    setVarKey(e.target.value);
                    setVarError(null);
                  }}
                  placeholder="variable_name"
                  aria-label="New variable name"
                  className="h-7 flex-1 font-mono text-xs"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") addVariable();
                  }}
                />
                <Input
                  value={varDesc}
                  onChange={(e) => setVarDesc(e.target.value)}
                  placeholder="Description (optional)"
                  aria-label="New variable description"
                  className="h-7 flex-1 text-xs"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") addVariable();
                  }}
                />
                <Button size="sm" variant="outline" onClick={addVariable}>
                  <Plus className="size-3.5" /> Add
                </Button>
              </div>
            </div>
            {varError && <p className="text-xs text-destructive">{varError}</p>}
            <p className="text-xs text-muted-foreground">
              Refs like{" "}
              <span className="font-mono">{"{{recipient.username}}"}</span>{" "}
              resolve from the recipient at send time. Everything else must
              be declared here first — Save is blocked otherwise.
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Attachments — full width; the long metadata rows get the room. */}
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

      {/* Static facts as a slim strip — not worth a card, let alone a column. */}
      <div className="flex flex-wrap items-center gap-x-5 gap-y-1 rounded-lg border bg-card px-4 py-2.5 text-xs text-muted-foreground">
        <span className="whitespace-nowrap">
          ID <span className="font-mono">{template.id}</span>
        </span>
        <span className="whitespace-nowrap">
          Project <span className="font-mono">{template.projectId}</span>
        </span>
        <span className="whitespace-nowrap">
          Created {format(new Date(template.createdAt), "MMM d, yyyy HH:mm")}
        </span>
        <span className="whitespace-nowrap">
          Updated {format(new Date(template.updatedAt), "MMM d, yyyy HH:mm")}
        </span>
      </div>

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
