"use client";

import Link from "next/link";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { truncateId } from "@/lib/utils";
import type { Subscriber } from "@/lib/types";

interface SubscriberTableProps {
  subscribers: Subscriber[];
  loading: boolean;
}

export function SubscriberTable({ subscribers, loading }: SubscriberTableProps) {
  if (loading) {
    return (
      <div className="rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>ID</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Email</TableHead>
              <TableHead>Phone</TableHead>
              <TableHead>Devices</TableHead>
              <TableHead className="text-right">Created</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {Array.from({ length: 8 }).map((_, i) => (
              <TableRow key={i}>
                {Array.from({ length: 6 }).map((_, j) => (
                  <TableCell key={j}>
                    <Skeleton className="h-4 w-full" />
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>ID</TableHead>
            <TableHead>Name</TableHead>
            <TableHead>Email</TableHead>
            <TableHead>Phone</TableHead>
            <TableHead>Devices</TableHead>
            <TableHead className="text-right">Created</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {subscribers.map((sub) => (
            <TableRow key={sub.id} className="hover:bg-secondary/50 transition-colors">
              <TableCell>
                <Link
                  href={`/subscribers/${sub.id}`}
                  className="font-mono text-xs text-primary hover:underline"
                >
                  {truncateId(sub.id)}
                </Link>
              </TableCell>
              <TableCell className="text-sm">{sub.name || "—"}</TableCell>
              <TableCell className="text-sm text-muted-foreground">
                {sub.email || "—"}
              </TableCell>
              <TableCell className="text-sm text-muted-foreground">
                {sub.phone || "—"}
              </TableCell>
              <TableCell>
                <Badge variant="outline">{sub.tokens.length} token{sub.tokens.length !== 1 ? "s" : ""}</Badge>
              </TableCell>
              <TableCell className="text-right text-sm text-muted-foreground">
                {new Date(sub.createdAt).toLocaleDateString()}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
