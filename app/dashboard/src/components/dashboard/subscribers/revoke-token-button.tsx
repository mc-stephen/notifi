"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Trash2 } from "lucide-react";

interface RevokeTokenButtonProps {
  tokenId: string;
}

export function RevokeTokenButton({ tokenId }: RevokeTokenButtonProps) {
  const [revoking, setRevoking] = useState(false);

  const handleRevoke = async () => {
    setRevoking(true);
    await new Promise((r) => setTimeout(r, 600));
    setRevoking(false);
    alert(`Token ${tokenId} revoked (mock)`);
  };

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      onClick={handleRevoke}
      disabled={revoking}
      className="text-muted-foreground hover:text-red-500"
      title="Revoke token"
    >
      <Trash2 className="h-3.5 w-3.5" />
    </Button>
  );
}
