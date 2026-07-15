"use client";

import { useState } from "react";
import type { ChannelConfig } from "@/lib/types";

const mockChannels: ChannelConfig[] = [
  {
    id: "sendgrid",
    provider: "SendGrid",
    channel: "email",
    configured: true,
    configFields: [
      { key: "apiKey", label: "API Key", type: "password", required: true, placeholder: "SG.xxxxxxxx" },
      { key: "senderEmail", label: "Sender Email", type: "email", required: true, placeholder: "noreply@acme.com" },
    ],
  },
  {
    id: "postmark",
    provider: "Postmark",
    channel: "email",
    configured: false,
    configFields: [
      { key: "serverToken", label: "Server Token", type: "password", required: true },
      { key: "senderEmail", label: "Sender Email", type: "email", required: true },
    ],
  },
  {
    id: "smtp",
    provider: "Custom SMTP",
    channel: "email",
    configured: false,
    configFields: [
      { key: "host", label: "SMTP Host", type: "text", required: true, placeholder: "smtp.example.com" },
      { key: "port", label: "Port", type: "text", required: true, placeholder: "587" },
      { key: "username", label: "Username", type: "text", required: true },
      { key: "password", label: "Password", type: "password", required: true },
    ],
  },
  {
    id: "fcm",
    provider: "Firebase Cloud Messaging",
    channel: "fcm",
    configured: true,
    configFields: [
      { key: "serverKey", label: "Server Key", type: "password", required: true },
      { key: "senderId", label: "Sender ID", type: "text", required: true, placeholder: "123456789" },
    ],
  },
  {
    id: "apns",
    provider: "Apple Push Notification service",
    channel: "apns",
    configured: false,
    configFields: [
      { key: "keyId", label: "Key ID", type: "text", required: true, placeholder: "ABC123DEFG" },
      { key: "teamId", label: "Team ID", type: "text", required: true, placeholder: "TEAM12345" },
      { key: "privateKey", label: "Private Key (.p8)", type: "file", required: true },
      { key: "bundleId", label: "Bundle ID", type: "text", required: true, placeholder: "com.example.app" },
    ],
  },
  {
    id: "twilio",
    provider: "Twilio",
    channel: "sms",
    configured: true,
    configFields: [
      { key: "accountSid", label: "Account SID", type: "password", required: true },
      { key: "authToken", label: "Auth Token", type: "password", required: true },
      { key: "fromNumber", label: "From Number", type: "text", required: true, placeholder: "+12025551234" },
    ],
  },
  {
    id: "infobip",
    provider: "Infobip",
    channel: "sms",
    configured: false,
    configFields: [
      { key: "apiKey", label: "API Key", type: "password", required: true },
      { key: "baseUrl", label: "Base URL", type: "text", required: true, placeholder: "https://api.infobip.com" },
    ],
  },
  {
    id: "webpush",
    provider: "WebPush",
    channel: "webpush",
    configured: false,
    configFields: [
      { key: "vapidPublic", label: "VAPID Public Key", type: "text", required: true },
      { key: "vapidPrivate", label: "VAPID Private Key", type: "password", required: true },
      { key: "contactEmail", label: "Contact Email", type: "email", required: true },
    ],
  },
];

export function useChannels() {
  const [channels, setChannels] = useState<ChannelConfig[]>(mockChannels);
  const [selectedChannel, setSelectedChannel] = useState<ChannelConfig | null>(null);
  const [configuring, setConfiguring] = useState(false);

  const startConfiguring = (channel: ChannelConfig) => {
    setSelectedChannel(channel);
    setConfiguring(true);
  };

  const saveConfig = (id: string) => {
    setChannels((prev) =>
      prev.map((c) => (c.id === id ? { ...c, configured: true } : c)),
    );
    setConfiguring(false);
    setSelectedChannel(null);
  };

  const cancelConfig = () => {
    setConfiguring(false);
    setSelectedChannel(null);
  };

  return {
    channels,
    selectedChannel,
    configuring,
    startConfiguring,
    saveConfig,
    cancelConfig,
  };
}
