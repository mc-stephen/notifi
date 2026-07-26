"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { RoleBadge } from "@/components/custom/role-badge";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
} from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";

type TeamMember = {
  id: string;
  name: string;
  email: string;
  role: "owner" | "admin" | "developer" | "viewer" | "billing";
  lastActiveAt: string;
  avatar?: string;
};

const MOCK_MEMBERS: TeamMember[] = [
  { id: "usr_1", name: "Alice Chen", email: "alice@example.com", role: "owner", lastActiveAt: "2025-06-25T10:00:00Z" },
  { id: "usr_2", name: "Bob Kim", email: "bob@example.com", role: "admin", lastActiveAt: "2025-06-25T09:30:00Z" },
  { id: "usr_3", name: "Carol Wu", email: "carol@example.com", role: "developer", lastActiveAt: "2025-06-24T16:00:00Z" },
  { id: "usr_4", name: "David Park", email: "david@example.com", role: "developer", lastActiveAt: "2025-06-20T11:00:00Z" },
  { id: "usr_5", name: "Eve Johnson", email: "eve@example.com", role: "viewer", lastActiveAt: "2025-06-18T14:00:00Z" },
  { id: "usr_6", name: "Frank Lee", email: "frank@example.com", role: "billing", lastActiveAt: "2025-06-15T09:00:00Z" },
];

const ROLE_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  owner: Crown,
  admin: Shield,
  developer: Code2,
  viewer: Eye,
  billing: CreditCard,
};

export default function TeamPage() {
  const [members] = useState(MOCK_MEMBERS);
  const [inviteDialog, setInviteDialog] = useState(false);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Team"
        description={`${members.length} team members`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Team" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setInviteDialog(true)}>
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
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Member</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Last Active</TableHead>
                <TableHead className="w-[60px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {members.map((member) => {
                return (
                  <TableRow key={member.id}>
                    <TableCell>
                      <div className="flex items-center gap-3">
                        <div className="flex size-8 items-center justify-center rounded-full bg-muted text-xs font-medium">
                          {member.name.split(" ").map((n) => n[0]).join("")}
                        </div>
                        <div>
                          <div className="text-sm font-medium">{member.name}</div>
                          <div className="text-xs text-muted-foreground">{member.email}</div>
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <RoleBadge role={member.role} />
                    </TableCell>
                    <TableCell>
                      <span className="text-xs text-muted-foreground">
                        {format(new Date(member.lastActiveAt), "MMM d, HH:mm")}
                      </span>
                    </TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" />}>
                          <MoreHorizontal className="size-3.5" />
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem>
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
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Dialog open={inviteDialog} onOpenChange={setInviteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Invite team member</DialogTitle>
            <DialogDescription>Send an invitation to join your organization.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Email address</label>
              <Input placeholder="colleague@company.com" type="email" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Role</label>
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option value="developer">Developer</option>
                <option value="viewer">Viewer</option>
                <option value="admin">Admin</option>
                <option value="billing">Billing</option>
              </select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setInviteDialog(false)}>Cancel</Button>
            <Button onClick={() => setInviteDialog(false)}>Send invite</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
