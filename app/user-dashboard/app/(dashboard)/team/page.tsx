"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { RoleBadge } from "@/components/custom/role-badge";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { DataTable } from "@/components/custom/data-table";
import type { ColumnDef } from "@tanstack/react-table";
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
  UserCog,
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
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
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

function MemberRowActions({ member }: { member: TeamMemberRecord }) {
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
        <DropdownMenuItem>
          <UserCog className="size-3.5" /> Change role
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-destructive">
          <Trash2 className="size-3.5" /> Remove
        </DropdownMenuItem>
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
            {member.name
              .split(" ")
              .map((n) => n[0])
              .join("")}
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
  const [inviteDialog, setInviteDialog] = useState(false);
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState("developer");
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [inviting, setInviting] = useState(false);
  const [require2fa, setRequire2fa] = useState<boolean | null>(null);
  const [toggling2fa, setToggling2fa] = useState(false);
  const [flagError, setFlagError] = useState<string | null>(null);
  const project = useProjectStore((s) => s.currentProject);
  const updateProject = useProjectStore((s) => s.updateProject);

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

  async function handleInvite() {
    if (!project || !inviteEmail.trim()) return;
    setInviteError(null);
    setInviting(true);
    try {
      await api(`/app/projects/${project.id}/members`, {
        method: "POST",
        body: JSON.stringify({ email: inviteEmail.trim(), role: inviteRole }),
      });
      setInviteDialog(false);
      setInviteEmail("");
      setInviteRole("developer");
      refresh();
    } catch (e) {
      setInviteError(
        e instanceof Error ? e.message : "Could not invite this member.",
      );
    } finally {
      setInviting(false);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Team"
        description={`${members.length} team members`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Team" }]}
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

      <div className="grid gap-4 md:grid-cols-4">
        {(["owner", "admin", "developer", "viewer"] as const).map((role) => {
          const count = members.filter((m) => m.role === role).length;
          const Icon = ROLE_ICONS[role];
          return (
            <Card key={role}>
              <CardContent className="pt-5">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Icon className="size-4 text-muted-foreground" />
                    <span className="text-sm capitalize">{role}s</span>
                  </div>
                  <span className="text-2xl font-bold">{count}</span>
                </div>
              </CardContent>
            </Card>
          );
        })}
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
          tableClassName="bg-card"
          searchKey="name"
          searchPlaceholder="Search members..."
          pageSize={10}
          emptyMessage="No members yet — invite your first team member to get started."
          rowActions={(member) => <MemberRowActions member={member} />}
        />
      )}

      <Dialog
        open={inviteDialog}
        onOpenChange={(open) => {
          setInviteDialog(open);
          if (!open) {
            setInviteError(null);
            setInviteEmail("");
            setInviteRole("developer");
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Invite team member</DialogTitle>
            <DialogDescription>
              They&apos;ll get an email to accept within 7 days — nothing joins until they do.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
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
              <select
                className="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
                value={inviteRole}
                onChange={(e) => setInviteRole(e.target.value)}
              >
                <option value="developer">Developer</option>
                <option value="viewer">Viewer</option>
                <option value="admin">Admin</option>
                <option value="billing">Billing</option>
              </select>
            </div>
            {require2faOn && (
              <p className="text-xs text-muted-foreground">
                This project requires 2FA — the invited account must have it
                enabled first.
              </p>
            )}
            {inviteError && (
              <p className="text-xs text-destructive">{inviteError}</p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setInviteDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleInvite}
              disabled={inviting || !inviteEmail.trim() || !project}
            >
              {inviting ? "Inviting..." : "Send invite"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
