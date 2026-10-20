"use client";

import { useState, useCallback, useMemo } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import { StatCard } from "@/components/custom/stat-card";
import { initials } from "@/lib/utils";
import { PageHeader } from "@/components/custom/page-header";
import {
  RecipientContactsEditor,
  contactsForEdit,
  type ContactsEditorValue,
} from "@/components/custom/recipient-contacts-editor";
import { useRecipients, useRecipientActions } from "@/hooks/use-recipients";
import type { Recipient } from "@/lib/types";
import { ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
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
  User,
  Users,
  Mail,
  Phone,
  UserX,
  AlertTriangle,
  Loader2,
} from "lucide-react";

function contactString(recipient: Recipient, key: string): string | undefined {
  const value = recipient.contacts?.[key];
  return typeof value === "string" && value.length > 0 ? value : undefined;
}
export default function RecipientsPage() {
  const { recipients, loading, error, refresh } = useRecipients();
  const { create, remove, update } = useRecipientActions();

  const [createOpen, setCreateOpen] = useState(false);
  const [deleteDialog, setDeleteDialog] = useState<Recipient | null>(null);
  const [bulkDelete, setBulkDelete] = useState<Recipient[] | null>(null);

  // Right-side detail panel selection.
  const [details, setDetails] = useState<Recipient | null>(null);
  // Edit side panel (replaces the details panel while open).
  const [editRecipient, setEditRecipient] = useState<Recipient | null>(null);

  const stats = useMemo(() => {
    let withEmail = 0;
    let withPhone = 0;
    let unreachable = 0;
    for (const r of recipients) {
      const hasEmail = !!contactString(r, "email");
      const hasPhone = !!contactString(r, "phone");
      if (hasEmail) withEmail += 1;
      if (hasPhone) withPhone += 1;
      if (!hasEmail && !hasPhone) unreachable += 1;
    }
    return { total: recipients.length, withEmail, withPhone, unreachable };
  }, [recipients]);

  const handleDelete = useCallback(
    async (targets: Recipient[]) => {
      setDeleteDialog(null);
      setBulkDelete(null);
      try {
        for (const recipient of targets) {
          await remove(recipient.id);
        }
        const ids = new Set(targets.map((r) => r.id));
        setDetails((cur) => (cur && ids.has(cur.id) ? null : cur));
        refresh();
      } catch {
        // refresh will surface the source of truth; leave silently
      }
    },
    [remove, refresh],
  );

  const columns: ColumnDef<Recipient, unknown>[] = [
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
        const r = row.original;
        const email = contactString(r, "email");
        return (
          <div className="flex items-center gap-2">
            <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium">
              {initials(r.name)}
            </div>
            <div className="min-w-0">
              <div className="text-sm font-medium truncate">{r.name}</div>
              <div className="text-xs text-muted-foreground truncate">
                {email ?? "No email"}
              </div>
            </div>
          </div>
        );
      },
    },
    {
      accessorKey: "userId",
      header: "User ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs">{row.original.userId}</span>
      ),
    },
    {
      id: "id",
      header: "ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs text-muted-foreground">
          {row.original.id}
        </span>
      ),
    },
    {
      accessorKey: "phone",
      header: "Phone",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground">
          {contactString(row.original, "phone") ?? "—"}
        </span>
      ),
    },
    {
      id: "createdAt",
      accessorKey: "createdAt",
      header: "Created",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {format(new Date(row.original.createdAt), "MMM d, HH:mm")}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Recipients"
        description={`${recipients.length} recipients`}
        actions={
          <Button
            size="sm"
            onClick={() => {
              setDetails(null);
              setCreateOpen(true);
            }}
          >
            <User className="size-3.5 mr-1" /> Add recipient
          </Button>
        }
      />

      <div className="flex items-start">
        <div className="min-w-0 flex-1">
          {!error && (recipients.length > 0 || loading) && (
            <div className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
              <StatCard
                label="Total recipients"
                value={loading ? "–" : stats.total}
                icon={Users}
              />
              <StatCard
                label="With email"
                value={loading ? "–" : stats.withEmail}
                icon={Mail}
              />
              <StatCard
                label="With phone"
                value={loading ? "–" : stats.withPhone}
                icon={Phone}
              />
              <StatCard
                label="Unreachable"
                value={loading ? "–" : stats.unreachable}
                sub="No email or phone set"
                icon={UserX}
              />
            </div>
          )}
          {loading ? (
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="flex items-center justify-center py-16 text-muted-foreground">
                <Loader2 className="size-5 animate-spin mr-2" /> Loading recipients…
              </div>
            </div>
          ) : error ? (
            <div className="overflow-hidden rounded-lg border bg-card">
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
              data={recipients}
              searchKey="name"
              searchPlaceholder="Search by name or user ID..."
              pageSize={10}
              onRowClick={(r) => setDetails(r)}
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
        </div>

        {details && (
          <DetailPanel
            key={details.id}
            onClose={() => setDetails(null)}
            headerClassName="p-4"
            header={
              <div>
                <h3 className="truncate font-medium text-base text-foreground">
                  {details.name}
                </h3>
                <p className="mt-0.5 truncate text-sm text-muted-foreground">
                  {contactString(details, "email") ??
                    `User ID: ${details.userId}`}
                </p>
              </div>
            }
          >

            <div className="flex-1 min-h-0 overflow-y-auto p-4">
              <dl className="grid grid-cols-2 gap-x-4 gap-y-4 text-sm">
                <div className="col-span-2">
                  <dt className="text-muted-foreground">Name</dt>
                  <dd className="mt-1 font-medium">{details.name}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">User ID</dt>
                  <dd className="mt-1 font-mono text-xs break-all">
                    {details.userId}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">ID</dt>
                  <dd className="mt-1 font-mono text-xs break-all">
                    {details.id}
                  </dd>
                </div>
                <div className="col-span-2">
                  <dt className="text-muted-foreground">Project</dt>
                  <dd className="mt-1 font-mono text-xs break-all">
                    {details.projectId}
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Email</dt>
                  <dd className="mt-1">{contactString(details, "email") ?? "—"}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">Phone</dt>
                  <dd className="mt-1">{contactString(details, "phone") ?? "—"}</dd>
                </div>
                <div className="col-span-2">
                  <dt className="text-muted-foreground">Created</dt>
                  <dd className="mt-1">
                    {format(new Date(details.createdAt), "MMM d, yyyy HH:mm")}
                  </dd>
                </div>
              </dl>

              <Separator className="my-4" />

              <div>
                <h4 className="text-sm font-medium mb-3">Contacts</h4>
                {Object.keys(details.contacts ?? {}).length > 0 ? (
                  <dl className="grid grid-cols-1 gap-3 text-sm">
                    {Object.entries(details.contacts ?? {}).map(([key, value]) => (
                      <div key={key}>
                        <dt className="text-muted-foreground capitalize">{key}</dt>
                        <dd className="mt-0.5 font-mono text-xs break-all">
                          {typeof value === "string" ? value : JSON.stringify(value)}
                        </dd>
                      </div>
                    ))}
                  </dl>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    No contact details set.
                  </p>
                )}
              </div>
            </div>

            <div className="flex gap-2 border-t border-border bg-muted/30 p-4">
              <Button
                variant="outline"
                onClick={() => {
                  setDetails(null);
                  setEditRecipient(details);
                }}
              >
                <Pencil className="size-3.5 mr-1" /> Edit
              </Button>
              <Button variant="destructive" onClick={() => setDeleteDialog(details)}>
                <Trash2 className="size-3.5 mr-1" /> Delete
              </Button>
            </div>
          </DetailPanel>
        )}
      </div>

      {/* Add recipient panel */}
      {createOpen && (
        <AddRecipientPanel
          create={create}
          refresh={refresh}
          onClose={() => setCreateOpen(false)}
        />
      )}

      {/* Edit recipient panel — replaces the details panel while open */}
      {editRecipient && (
        <EditRecipientPanel
          key={editRecipient.id}
          recipient={editRecipient}
          update={update}
          onClose={() => {
            // Cancel/Escape: hand back the pre-edit details panel.
            setDetails(editRecipient);
            setEditRecipient(null);
          }}
          onSaved={(updated) => {
            setDetails(updated);
            refresh();
          }}
        />
      )}

      <Dialog open={!!deleteDialog} onOpenChange={() => setDeleteDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete recipient</DialogTitle>
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
            <DialogTitle>Delete {bulkDelete?.length} recipients</DialogTitle>
            <DialogDescription>
              Permanently delete {bulkDelete?.length} selected recipient
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

function AddRecipientPanel({
  create,
  refresh,
  onClose,
}: {
  create: ReturnType<typeof useRecipientActions>["create"];
  refresh: () => Promise<void>;
  onClose: () => void;
}) {
  // Seeded at mount; unmounting the panel clears the form, so no reset needed.
  const [name, setName] = useState("");
  const [userId, setUserId] = useState("");
  const [contacts, setContacts] = useState<ContactsEditorValue>({});
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const handleCreate = useCallback(async () => {
    setSubmitting(true);
    setSubmitError(null);
    try {
      const contactBlob: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(contacts)) {
        const trimmedKey = key.trim();
        if (trimmedKey && val.trim()) contactBlob[trimmedKey] = val.trim();
      }
      await create({
        userId: userId.trim(),
        name: name.trim(),
        contacts: contactBlob,
      });
      onClose();
      refresh();
    } catch (e) {
      setSubmitError(
        e instanceof ApiError ? e.message : "Failed to create recipient",
      );
    } finally {
      setSubmitting(false);
    }
  }, [create, name, userId, contacts, refresh, onClose]);

  return (
    <DetailPanel
      onClose={onClose}
      headerClassName="p-4"
      className="w-[min(28rem,92%)]"
      header={
        <h3 className="truncate font-medium text-base text-foreground">
          Add recipient
        </h3>
      }
    >
      {/* Form */}
      <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4">
        <p className="text-xs text-muted-foreground">
          Create a recipient (one of your end-users) in this project. The User
          ID is the identifier your system uses to target them — it is unique
          within this project.
        </p>
        <div className="space-y-1.5">
          <Label htmlFor="recipient-name">Name</Label>
          <Input
            id="recipient-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Ada Lovelace"
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="recipient-user-id">User ID</Label>
          <Input
            id="recipient-user-id"
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            placeholder="user_abc123"
          />
        </div>
        <RecipientContactsEditor value={contacts} onChange={setContacts} />
        {submitError && (
          <p className="text-sm text-destructive">{submitError}</p>
        )}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-end gap-2 border-t border-border bg-muted/30 px-4 pb-4 pt-3">
        <Button variant="outline" onClick={onClose}>
          Cancel
        </Button>
        <Button
          onClick={handleCreate}
          disabled={submitting || !name.trim() || !userId.trim()}
        >
          {submitting ? "Creating…" : "Create recipient"}
        </Button>
      </div>
    </DetailPanel>
  );
}

function EditRecipientPanel({
  recipient,
  update,
  onClose,
  onSaved,
}: {
  recipient: Recipient;
  update: ReturnType<typeof useRecipientActions>["update"];
  onClose: () => void;
  onSaved: (updated: Recipient) => void;
}) {
  // Seeded from the recipient at mount; unmounting the panel clears the
  // form, so no reset needed.
  const [name, setName] = useState(recipient.name);
  const [contacts, setContacts] = useState<ContactsEditorValue>(
    contactsForEdit(recipient.contacts),
  );
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const handleSave = useCallback(async () => {
    setSubmitting(true);
    setSubmitError(null);
    try {
      const filteredContacts: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(contacts)) {
        const trimmedKey = key.trim();
        if (trimmedKey && val.trim()) filteredContacts[trimmedKey] = val.trim();
      }
      const updated = await update(recipient.id, {
        name: name.trim(),
        contacts: filteredContacts,
      });
      onClose();
      onSaved(updated);
    } catch (e) {
      setSubmitError(
        e instanceof ApiError ? e.message : "Failed to update recipient",
      );
    } finally {
      setSubmitting(false);
    }
  }, [recipient, name, contacts, update, onClose, onSaved]);

  return (
    <DetailPanel
      onClose={onClose}
      headerClassName="p-4"
      className="w-[min(28rem,92%)]"
      header={
        <h3 className="truncate font-medium text-base text-foreground">
          Edit recipient
        </h3>
      }
    >
      {/* Form */}
      <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4">
        <p className="text-xs text-muted-foreground">
          Update the name and contact details. The user ID cannot be changed.
        </p>
        <div className="space-y-1.5">
          <Label htmlFor="edit-recipient-name">Name</Label>
          <Input
            id="edit-recipient-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        <RecipientContactsEditor value={contacts} onChange={setContacts} />
        {submitError && (
          <p className="text-sm text-destructive">{submitError}</p>
        )}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-end gap-2 border-t border-border bg-muted/30 px-4 pb-4 pt-3">
        <Button variant="outline" onClick={onClose} disabled={submitting}>
          Cancel
        </Button>
        <Button onClick={handleSave} disabled={submitting || !name.trim()}>
          {submitting ? "Saving…" : "Save"}
        </Button>
      </div>
    </DetailPanel>
  );
}
