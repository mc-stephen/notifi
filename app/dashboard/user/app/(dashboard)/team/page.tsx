"use client";

import { useState } from "react";
import { toast } from "sonner";
import { PageHeader } from "@/components/custom/page-header";
import { cn, initials } from "@/lib/utils";
import { RoleBadge } from "@/components/custom/role-badge";
import { StatCard } from "@/components/custom/stat-card";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import type { ColumnDef } from "@tanstack/react-table";
import { format } from "date-fns";
import {
  Plus,
  Mail,
  Trash2,
  MoreHorizontal,
  Crown,
  Shield,
  Code2,
  Eye,
  CreditCard,
  ShieldCheck,
  ShieldOff,
  LogOut,
} from "lucide-react";
import { useAuthStore } from "@/store/auth-store";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { useProjectStore } from "@/store/project-store";
import { useTeamMembers } from "@/hooks/use-team";
import type { TeamMemberRecord } from "@/hooks/use-team";
import { api } from "@/lib/api";
import type { Role } from "@/lib/types";

const ROLE_ICONS: Record<
  string,
  React.ComponentType<{ className?: string }>
> = {
  owner: Crown,
  admin: Shield,
  developer: Code2,
  viewer: Eye,
  billing: CreditCard,
};

function MemberRowActions({
  member,
  projectId,
  meId,
  refresh,
  onTransfer,
}: {
  member: TeamMemberRecord;
  projectId: string;
  meId: string | null;
  refresh: () => Promise<void>;
  onTransfer: (member: TeamMemberRecord) => void;
}) {
  const toMessage = (e: unknown, fallback: string) =>
    e instanceof Error ? e.message : fallback;

  async function act(request: () => Promise<unknown>, success: string) {
    try {
      await request();
      toast.success(success);
      void refresh();
    } catch (e) {
      toast.error(success === "Left the project" ? "Couldn't leave" : "Couldn't remove member", {
        description: toMessage(e, "Check your connection and try again."),
      });
    }
  }

  const isSelf = meId !== null && member.userId === meId;
  // The server holds the real rules: nobody removes the owner, admins
  // only remove plain members — the menu only hides what can never work.
  const removable = !isSelf && member.role !== "owner";

  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" />}>
        <MoreHorizontal className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          onClick={() => {
            window.location.href = `mailto:${member.email}`;
          }}
        >
          <Mail className="size-3.5" /> Send email
        </DropdownMenuItem>
        {!isSelf && member.role !== "owner" && (
          <DropdownMenuItem onClick={() => onTransfer(member)}>
            <Crown className="size-3.5" /> Transfer ownership
          </DropdownMenuItem>
        )}
        {removable && (
          <DropdownMenuItem
            className="text-destructive"
            onClick={() =>
              act(
                () =>
                  api(
                    `/app/projects/${projectId}/members/${member.userId}`,
                    { method: "DELETE" },
                  ),
                `${member.name} removed from the team`,
              )
            }
          >
            <Trash2 className="size-3.5" /> Remove
          </DropdownMenuItem>
        )}
        {isSelf && (
          <DropdownMenuItem
            className="text-destructive"
            onClick={() =>
              act(() => api(`/app/projects/${projectId}/members/me`, { method: "DELETE" }), "Left the project")
            }
          >
            <LogOut className="size-3.5" /> Leave project
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
const MEMBER_COLUMNS: ColumnDef<TeamMemberRecord, unknown>[] = [
  {
    accessorKey: "name",
    header: "Member",
    cell: ({ row }) => {
      const member = row.original;
      return (
        <div className="flex items-center gap-3">
          <div className="flex size-8 items-center justify-center rounded-full bg-muted text-xs font-medium">
            {initials(member.name)}
          </div>
          <div>
            <div className="text-sm font-medium">{member.name}</div>
            <div className="text-xs text-muted-foreground">{member.email}</div>
          </div>
        </div>
      );
    },
  },
  {
    accessorKey: "role",
    header: "Role",
    cell: ({ row }) => (
      <RoleBadge
        role={
          (row.original.role in ROLE_ICONS
            ? row.original.role
            : "viewer") as Role
        }
      />
    ),
  },
  {
    accessorKey: "has2fa",
    header: "2FA",
    cell: ({ row }) =>
      row.original.has2fa ? (
        <span className="inline-flex items-center gap-1 text-xs font-medium text-success">
          <ShieldCheck className="size-3.5" /> 2FA
        </span>
      ) : (
        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
          <ShieldOff className="size-3.5" /> —
        </span>
      ),
  },
  {
    accessorKey: "lastActiveAt",
    header: "Last Active",
    cell: ({ row }) => (
      <span className="text-xs text-muted-foreground">
        {row.original.lastActiveAt
          ? format(new Date(row.original.lastActiveAt), "MMM d, HH:mm")
          : "—"}
      </span>
    ),
  },
];

export default function TeamPage() {
  const { members, loading, error, refresh } = useTeamMembers();
  // Invite panel open state (form lives in InviteTeamMemberPanel).
  const [inviteDialog, setInviteDialog] = useState(false);
  // Ownership-transfer panel target (the picked roster row).
  const [transferTarget, setTransferTarget] =
    useState<TeamMemberRecord | null>(null);
  const [require2fa, setRequire2fa] = useState<boolean | null>(null);
  const [toggling2fa, setToggling2fa] = useState(false);
  const [flagError, setFlagError] = useState<string | null>(null);
  const project = useProjectStore((s) => s.currentProject);
  const updateProject = useProjectStore((s) => s.updateProject);
  const user = useAuthStore((s) => s.user);

  const require2faOn = require2fa ?? project?.require2fa ?? false;

  async function handleToggle2fa(next: boolean) {
    if (!project) return;
    setFlagError(null);
    setToggling2fa(true);
    try {
      const res = await api<{ require2fa: boolean }>(
        `/app/projects/${project.id}/require-2fa`,
        {
          method: "PATCH",
          body: JSON.stringify({ enabled: next }),
        },
      );
      setRequire2fa(res.require2fa);
      updateProject({ ...project, require2fa: res.require2fa });
    } catch (e) {
      setFlagError(
        e instanceof Error
          ? e.message
          : "Could not update the 2FA requirement.",
      );
    } finally {
      setToggling2fa(false);
    }
  }
  return (
    <div className="space-y-6">
      <PageHeader
        title="Team"
        description={`${members.length} team members`}
        actions={
          <Button
            size="sm"
            className="gap-1.5"
            onClick={() => setInviteDialog(true)}
          >
            <Plus className="size-3.5" /> Invite member
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-5">
        {(["owner", "admin", "developer", "viewer", "billing"] as const).map((role) => (
          <StatCard
            key={role}
            label={`${role[0].toUpperCase()}${role.slice(1)}s`}
            value={members.filter((m) => m.role === role).length}
            icon={ROLE_ICONS[role]}
          />
        ))}
      </div>

      <Card>
        <CardContent className="flex flex-wrap items-center justify-between gap-4 py-5">
          <div className="flex items-center gap-3">
            <ShieldCheck className="size-4 text-muted-foreground" />
            <div>
              <div className="text-sm font-medium">
                Require two-factor authentication
              </div>
              <div className="text-xs text-muted-foreground">
                New members must have 2FA enabled on their account before
                joining
                {project ? ` ${project.name}` : " this project"}. Owners and
                admins only.
              </div>
              {flagError && (
                <div className="text-xs text-destructive mt-1">{flagError}</div>
              )}
            </div>
          </div>
          <Switch
            checked={require2faOn}
            disabled={!project || toggling2fa}
            onCheckedChange={handleToggle2fa}
            aria-label="Require two-factor authentication for new members"
          />
        </CardContent>
      </Card>

      {loading ? (
        <Card>
          <CardContent>
            <div className="space-y-3 p-4" aria-label="Loading team">
              {[0, 1, 2].map((i) => (
                <div key={i} className="flex items-center gap-3 animate-pulse">
                  <div className="size-8 rounded-full bg-muted" />
                  <div className="flex-1 space-y-1.5">
                    <div className="h-3 w-1/4 rounded bg-muted" />
                    <div className="h-3 w-1/6 rounded bg-muted" />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      ) : error ? (
        <Card>
          <CardContent>
            <div className="space-y-3 p-6 text-center">
              <p className="text-sm font-medium">Couldn&apos;t load the team</p>
              <p className="text-sm text-muted-foreground">{error}</p>
              <Button size="sm" variant="outline" onClick={refresh}>
                Try again
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <DataTable
          columns={MEMBER_COLUMNS}
          data={members}
          searchKey="name"
          searchPlaceholder="Search members..."
          pageSize={10}
          emptyMessage="No members yet — invite your first team member to get started."
          rowActions={(member) => (
            <MemberRowActions
              member={member}
              projectId={project?.id ?? ""}
              meId={user?.id ?? null}
              refresh={refresh}
              onTransfer={setTransferTarget}
            />
          )}
        />
      )}
      {/* Invite member panel */}
      {inviteDialog && (
        <InviteTeamMemberPanel
          projectId={project?.id ?? ""}
          require2faOn={require2faOn}
          refresh={refresh}
          onClose={() => setInviteDialog(false)}
        />
      )}
      {/* Ownership transfer panel */}
      {transferTarget && project && (
        <TransferOwnershipPanel
          project={project.id}
          member={transferTarget}
          refresh={refresh}
          onClose={() => setTransferTarget(null)}
        />
      )}
    </div>
  );
}

/** Roles offered on the invite panel's chip row. */
const ROLE_OPTIONS: { value: Role; label: string }[] = [
  { value: "developer", label: "Developer" },
  { value: "viewer", label: "Viewer" },
  { value: "admin", label: "Admin" },
  { value: "billing", label: "Billing" },
];

/** Ownership moves to an accepted member — the one roof the server
 * honors. The previous owner lands as admin, and from there they can
 * stay, leave, or be kicked like anyone else. */
function TransferOwnershipPanel({
  project,
  member,
  refresh,
  onClose,
}: {
  project: string;
  member: TeamMemberRecord;
  refresh: () => Promise<void>;
  onClose: () => void;
}) {
  const [transferring, setTransferring] = useState(false);
  const [transferError, setTransferError] = useState<string | null>(null);

  const handleTransfer = async () => {
    setTransferError(null);
    setTransferring(true);
    try {
      await api(`/app/projects/${project}/ownership`, {
        method: "POST",
        body: JSON.stringify({ receiver: member.userId }),
      });
      toast.success("Ownership transferred", {
        description: `${member.name} is now the project owner — you stay on as admin.`,
      });
      onClose();
      void refresh();
    } catch (e) {
      setTransferError(
        e instanceof Error ? e.message : "Could not transfer ownership.",
      );
    } finally {
      setTransferring(false);
    }
  };

  return (
    <DetailPanel
      onClose={onClose}
      headerClassName="p-4"
      header={
        <h3 className="truncate font-medium text-base text-foreground">
          Transfer ownership
        </h3>
      }
    >
      <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4">
        <p className="text-xs text-muted-foreground">
          One owner at a time, and this is the only door: the receiver must
          already be an accepted team member.
        </p>
        <div className="flex items-center gap-3 rounded-md border border-border p-3">
          <div className="flex size-8 items-center justify-center rounded-full bg-muted text-xs font-medium">
            {initials(member.name)}
          </div>
          <div>
            <div className="text-sm font-medium">{member.name}</div>
            <div className="text-xs text-muted-foreground">{member.email}</div>
          </div>
        </div>
        <div className="text-xs text-muted-foreground">
          They get the crown and every owner power (deletion, further
          transfers). You stay on the team as <strong>admin</strong> — free to
          stay, leave, or be kicked according to your agreement with them.
        </div>
        {transferError && (
          <p className="text-xs text-destructive">{transferError}</p>
        )}
      </div>
      <div className="flex items-center justify-end gap-2 border-t border-border bg-muted/30 px-4 pb-4 pt-3">
        <Button variant="outline" onClick={onClose}>
          Cancel
        </Button>
        <Button onClick={handleTransfer} disabled={transferring}>
          {transferring ? "Transferring..." : "Transfer ownership"}
        </Button>
      </div>
    </DetailPanel>
  );
}

function InviteTeamMemberPanel({
  projectId,
  require2faOn,
  refresh,
  onClose,
}: {
  projectId: string;
  require2faOn: boolean;
  refresh: () => Promise<void>;
  onClose: () => void;
}) {
  // Seeded at mount; unmounting the panel clears the form, so no reset needed.
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState("developer");
  const [inviting, setInviting] = useState(false);
  const [inviteError, setInviteError] = useState<string | null>(null);

  const handleInvite = async () => {
    if (!projectId || !inviteEmail.trim()) return;
    setInviteError(null);
    setInviting(true);
    try {
      await api(`/app/projects/${projectId}/members`, {
        method: "POST",
        body: JSON.stringify({ email: inviteEmail.trim(), role: inviteRole }),
      });
      onClose();
      refresh();
    } catch (e) {
      setInviteError(
        e instanceof Error ? e.message : "Could not invite this member.",
      );
    } finally {
      setInviting(false);
    }
  };

  return (
    <DetailPanel
      onClose={onClose}
      headerClassName="p-4"
      header={
        <h3 className="truncate font-medium text-base text-foreground">
          Invite team member
        </h3>
      }
    >
      {/* Form */}
      <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4">
        <p className="text-xs text-muted-foreground">
          They&apos;ll get an email to accept within 7&nbsp;days — nothing joins
          until they do.
        </p>
        <div className="space-y-2">
          <label className="text-sm font-medium">Email address</label>
          <Input
            placeholder="colleague@company.com"
            type="email"
            value={inviteEmail}
            onChange={(e) => setInviteEmail(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium">Role</label>
          <div role="radiogroup" aria-label="Member role" className="flex flex-wrap gap-1.5">
            {ROLE_OPTIONS.map(({ value, label }) => {
              const selected = inviteRole === value;
              return (
                <button
                  key={value}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  onClick={() => setInviteRole(value)}
                  className={cn(
                    "h-8 rounded-md border px-2.5 text-xs font-medium transition-colors",
                    selected
                      ? "border-primary bg-primary text-primary-foreground"
                      : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
                  )}
                >
                  {label}
                </button>
              );
            })}
          </div>
        </div>
        {require2faOn && (
          <p className="text-xs text-muted-foreground">
            This project requires 2FA — the invited account must have it
            enabled first.
          </p>
        )}
        {inviteError && <p className="text-xs text-destructive">{inviteError}</p>}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-end gap-2 border-t border-border bg-muted/30 px-4 pb-4 pt-3">
        <Button variant="outline" onClick={onClose}>
          Cancel
        </Button>
        <Button
          onClick={handleInvite}
          disabled={inviting || !inviteEmail.trim() || !projectId}
        >
          {inviting ? "Inviting..." : "Send invite"}
        </Button>
      </div>
    </DetailPanel>
  );
}
