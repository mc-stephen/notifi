"use client";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { CopyButton } from "@/components/dashboard/shared/copy-button";
import { RevokeTokenButton } from "./revoke-token-button";
import { CHANNEL_LABELS } from "@/lib/constants";
import type { DeviceToken, Channel } from "@/lib/types";

interface DeviceTokenRegistryProps {
  tokens: DeviceToken[];
}

const channels: Channel[] = ["email", "fcm", "apns", "sms", "webpush"];

export function DeviceTokenRegistry({ tokens }: DeviceTokenRegistryProps) {
  const activeTokens = tokens.filter((t) => t.active);

  return (
    <Tabs defaultValue={activeTokens[0]?.channel || channels[0]}>
      <TabsList>
        {channels.map((ch) => {
          const count = activeTokens.filter((t) => t.channel === ch).length;
          return (
            <TabsTrigger key={ch} value={ch} disabled={count === 0}>
              {CHANNEL_LABELS[ch]}
              {count > 0 && (
                <Badge variant="outline" className="ml-1.5 text-[10px] px-1 py-0">
                  {count}
                </Badge>
              )}
            </TabsTrigger>
          );
        })}
      </TabsList>

      {channels.map((ch) => {
        const channelTokens = tokens.filter((t) => t.channel === ch);
        return (
          <TabsContent key={ch} value={ch} className="mt-4">
            {channelTokens.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4 text-center">
                No {CHANNEL_LABELS[ch].toLowerCase()} tokens registered.
              </p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Token</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Last Used</TableHead>
                    <TableHead className="w-20 text-right">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {channelTokens.map((token) => (
                    <TableRow key={token.id}>
                      <TableCell>
                        <div className="flex items-center gap-1.5">
                          <span className="font-mono text-xs truncate max-w-[200px]">
                            {token.token}
                          </span>
                          <CopyButton value={token.token} />
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant={token.active ? "default" : "secondary"}
                          className="text-xs"
                        >
                          {token.active ? "Active" : "Revoked"}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">
                        {new Date(token.lastUsed).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="text-right">
                        {token.active && <RevokeTokenButton tokenId={token.id} />}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </TabsContent>
        );
      })}
    </Tabs>
  );
}
