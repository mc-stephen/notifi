"use client";

import { useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import {
  useTemplates,
  useTemplateActions,
} from "@/hooks/use-templates";
import {
  templateChannelModel,
  TEMPLATE_CHANNELS,
  coveredChannels,
} from "@/lib/template-content";
import type { Template } from "@/lib/types";
import { ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
  Pencil,
  Trash2,
  AlertTriangle,
  Loader2,
  Plus,
  FileText,
} from "lucide-react";

function ChannelBadgesCell({ content }: { content: Record<string, unknown> }) {
  const covered = coveredChannels(content).map((c) => templateChannelModel(c));
  return (
    <div className="flex flex-wrap gap-1">
      {covered.length === 0 ? (
        <span className="text-xs text-muted-foreground">—</span>
      ) : (
        covered.map((m) => {
          const Icon = m.icon;
          return (
            <Badge key={m.value} variant="outline" className="gap-1">
              <Icon className="size-3" /> {m.label}
            </Badge>
          );
        })
      )}
    </div>
  );
}

export default function TemplatesPage() {
  const router = useRouter();
  const { templates, loading, error, refresh } = useTemplates();
  const { create, update, remove } = useTemplateActions();

  // Create dialog state.
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [channels, setChannels] = useState<string[]>(["email"]);
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  // Edit dialog state.
  const [editTemplate, setEditTemplate] = useState<Template | null>(null);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editChannels, setEditChannels] = useState<string[]>([]);
  const [editSubmitting, setEditSubmitting] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);

  // Delete dialogs.
  const [deleteDialog, setDeleteDialog] = useState<Template | null>(null);
  const [bulkDelete, setBulkDelete] = useState<Template[] | null>(null);

  const resetForm = useCallback(() => {
    setName("");
    setDescription("");
    setChannels(["email"]);
    setSubmitError(null);
  }, []);

  const handleCreate = useCallback(async () => {
    setSubmitting(true);
    setSubmitError(null);
    try {
      const content: Record<string, unknown> = {};
      for (const channel of channels) {
        const model = templateChannelModel(channel);
        const fields: Record<string, string> = {};
        for (const field of model.fields) {
          fields[field.key] = "";
        }
        content[channel] = fields;
      }
      await create({
        name: name.trim(),
        description: description.trim() || undefined,
        channel: "multi",
        content,
      });
      setCreateOpen(false);
      resetForm();
      refresh();
    } catch (e) {
      setSubmitError(
        e instanceof ApiError ? e.message : "Failed to create template",
      );
    } finally {
      setSubmitting(false);
    }
  }, [create, name, description, channels, refresh, resetForm]);

  const openEdit = useCallback((template: Template) => {
    setEditName(template.name);
    setEditDescription(template.description ?? "");
    setEditChannels(coveredChannels(template.content));
    setEditError(null);
    setEditTemplate(template);
  }, []);

  const handleEditSave = useCallback(async () => {
    if (!editTemplate) return;
    setEditSubmitting(true);
    setEditError(null);
    try {
      // Preserve existing per-channel content; add empty slices for newly
      // selected channels, drop slices for deselected ones.
      const content: Record<string, unknown> = {
        ...(editTemplate.content ?? {}),
      };
      for (const channel of editChannels) {
        if (!Object.prototype.hasOwnProperty.call(content, channel)) {
          const model = templateChannelModel(channel);
          const fields: Record<string, string> = {};
          for (const field of model.fields) {
            fields[field.key] = "";
          }
          content[channel] = fields;
        }
      }
      for (const key of Object.keys(content)) {
        if (!editChannels.includes(key)) {
          delete content[key];
        }
      }
      await update(editTemplate.id, {
        name: editName.trim(),
        description: editDescription.trim() || undefined,
        channel: "multi",
        content,
      });
      setEditTemplate(null);
      refresh();
    } catch (e) {
      setEditError(
        e instanceof ApiError ? e.message : "Failed to update template",
      );
    } finally {
      setEditSubmitting(false);
    }
  }, [editTemplate, editName, editDescription, editChannels, update, refresh]);

  const handleDelete = useCallback(
    async (targets: Template[]) => {
      setDeleteDialog(null);
      setBulkDelete(null);
      try {
        for (const template of targets) {
          await remove(template.id);
        }
        refresh();
      } catch {
        // refresh will surface the source of truth; leave silently
      }
    },
    [remove, refresh],
  );

  const columns: ColumnDef<Template, unknown>[] = [
    {
      id: "select",
      header: ({ table }) => (
        <Checkbox
          checked={table.getIsAllPageRowsSelected()}
          onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
          onClick={(e) => e.stopPropagation()}
        />
      ),
      enableSorting: false,
      enableHiding: false,
      size: 40,
    },
    {
      accessorKey: "name",
      header: "Name",
      cell: ({ row }) => {
        const t = row.original;
        const count = coveredChannels(t.content).length;
        return (
          <div className="flex items-center gap-2">
            <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-muted">
              <FileText className="size-3.5 text-muted-foreground" />
            </div>
            <div className="min-w-0">
              <div className="text-sm font-medium truncate">{t.name}</div>
              <div className="text-xs text-muted-foreground truncate">
                {t.description || `${count} channel${count === 1 ? "" : "s"}`}
              </div>
            </div>
          </div>
        );
      },
    },
    {
      id: "channel",
      header: "Channels",
      cell: ({ row }) => (
        <ChannelBadgesCell content={row.original.content} />
      ),
    },
    {
      id: "version",
      accessorKey: "version",
      header: "Version",
      cell: ({ row }) => (
        <span className="font-mono text-xs">v{row.original.version}</span>
      ),
    },
    {
      id: "attachments",
      header: "Attachments",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground">
          {row.original.attachments.length} file
          {row.original.attachments.length === 1 ? "" : "s"}
        </span>
      ),
    },
    {
      id: "updatedAt",
      accessorKey: "updatedAt",
      header: "Updated",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {format(new Date(row.original.updatedAt), "MMM d, yyyy")}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Templates"
        description={`${templates.length} templates`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Templates" }]}
        actions={
          <Button size="sm" onClick={() => setCreateOpen(true)}>
            <Plus className="size-3.5 mr-1" /> New template
          </Button>
        }
      />

      {loading ? (
        <div className="overflow-hidden rounded-lg border bg-white">
          <div className="flex items-center justify-center py-16 text-muted-foreground">
            <Loader2 className="size-5 animate-spin mr-2" /> Loading templates…
          </div>
        </div>
      ) : error ? (
        <div className="overflow-hidden rounded-lg border bg-white">
          <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
            <AlertTriangle className="size-8 text-warning" />
            <p className="text-sm text-muted-foreground">{error}</p>
            <Button size="sm" variant="outline" onClick={refresh}>
              Retry
            </Button>
          </div>
        </div>
      ) : (
        <DataTable
          columns={columns}
          data={templates}
          searchKey="name"
          searchPlaceholder="Search templates..."
          pageSize={10}
          tableClassName="bg-white"
          stripeRows
          onRowClick={(t) => router.push(`/templates/${t.id}`)}
          rowActions={(t) => (
            <div className="flex items-center gap-1" onClick={(e) => e.stopPropagation()}>
              <Button
                variant="ghost"
                size="icon-xs"
                title="Edit"
                onClick={() => openEdit(t)}
              >
                <Pencil className="size-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon-xs"
                title="Delete"
                className="text-destructive hover:text-destructive"
                onClick={() => setDeleteDialog(t)}
              >
                <Trash2 className="size-3.5" />
              </Button>
            </div>
          )}
          bulkActions={(selected) => (
            <Button
              variant="destructive"
              size="sm"
              onClick={() => setBulkDelete(selected)}
            >
              <Trash2 className="size-3.5 mr-1" /> Delete ({selected.length})
            </Button>
          )}
        />
      )}

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>New template</DialogTitle>
            <DialogDescription>
              Create one template that covers multiple channels. You can edit
              each channel&apos;s content after creation, and add more channels
              later.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="template-name">Name</Label>
              <Input
                id="template-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. Welcome Email"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="template-description">Description</Label>
              <Input
                id="template-description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Optional short description"
              />
            </div>
            <div className="space-y-1.5">
              <Label>Channels</Label>
              <div className="grid gap-1 rounded-md border p-2">
                {TEMPLATE_CHANNELS.map((c) => {
                  const checked = channels.includes(c.value);
                  const Icon = c.icon;
                  return (
                    <label
                      key={c.value}
                      className="flex items-center gap-3 rounded px-2 py-1.5 cursor-pointer hover:bg-muted"
                    >
                      <Checkbox
                        checked={checked}
                        onCheckedChange={(value) =>
                          setChannels((prev) =>
                            value
                              ? prev.includes(c.value)
                                ? prev
                                : [...prev, c.value]
                              : prev.filter((x) => x !== c.value),
                          )
                        }
                      />
                      <Icon className="size-4 text-muted-foreground" />
                      <span className="text-sm font-medium">{c.label}</span>
                    </label>
                  );
                })}
              </div>
            </div>
            {submitError && <p className="text-sm text-destructive">{submitError}</p>}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={submitting || !name.trim() || channels.length === 0}
            >
              {submitting ? "Creating…" : "Create template"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={!!editTemplate}
        onOpenChange={(open) => !open && setEditTemplate(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit template</DialogTitle>
            <DialogDescription>
              Update the name, description, and the channels this template
              covers. Channel content is edited from the template&apos;s page.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="edit-template-name">Name</Label>
              <Input
                id="edit-template-name"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="edit-template-description">Description</Label>
              <Input
                id="edit-template-description"
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Channels</Label>
              <div className="grid gap-1 rounded-md border p-2">
                {TEMPLATE_CHANNELS.map((c) => {
                  const checked = editChannels.includes(c.value);
                  const Icon = c.icon;
                  return (
                    <label
                      key={c.value}
                      className="flex items-center gap-3 rounded px-2 py-1.5 cursor-pointer hover:bg-muted"
                    >
                      <Checkbox
                        checked={checked}
                        onCheckedChange={(value) =>
                          setEditChannels((prev) =>
                            value
                              ? prev.includes(c.value)
                                ? prev
                                : [...prev, c.value]
                              : prev.filter((x) => x !== c.value),
                          )
                        }
                      />
                      <Icon className="size-4 text-muted-foreground" />
                      <span className="text-sm font-medium">{c.label}</span>
                    </label>
                  );
                })}
              </div>
            </div>
            {editError && <p className="text-sm text-destructive">{editError}</p>}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setEditTemplate(null)}
              disabled={editSubmitting}
            >
              Cancel
            </Button>
            <Button
              onClick={handleEditSave}
              disabled={editSubmitting || !editName.trim() || editChannels.length === 0}
            >
              {editSubmitting ? "Saving…" : "Save"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!deleteDialog} onOpenChange={() => setDeleteDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete template</DialogTitle>
            <DialogDescription>
              Permanently delete {deleteDialog?.name} ({deleteDialog?.id})? This
              cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialog(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => deleteDialog && handleDelete([deleteDialog])}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!bulkDelete} onOpenChange={() => setBulkDelete(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete {bulkDelete?.length} templates</DialogTitle>
            <DialogDescription>
              Permanently delete {bulkDelete?.length} selected template
              {bulkDelete && bulkDelete.length !== 1 ? "s" : ""}? This cannot be
              undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setBulkDelete(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => bulkDelete && handleDelete(bulkDelete)}
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
