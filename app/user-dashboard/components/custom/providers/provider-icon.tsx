"use client";

import { useState } from "react";

export function ProviderIcon({
  name,
  iconUrl,
  size = "md",
}: {
  name: string;
  iconUrl?: string;
  size?: "md" | "sm";
}) {
  const [failed, setFailed] = useState(false);
  const box =
    size === "sm"
      ? "size-8 rounded-lg text-xs"
      : "size-10 rounded-xl text-sm";

  if (iconUrl && !failed) {
    return (
      <img
        src={iconUrl}
        alt={name}
        className={`${box} border bg-white object-contain`}
        onError={() => setFailed(true)}
      />
    );
  }

  return (
    <div
      className={`flex ${box} items-center justify-center bg-muted font-bold`}
    >
      {name.charAt(0)}
    </div>
  );
}
