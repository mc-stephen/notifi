"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { CodeBlock } from "@/components/custom/code-block";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Package,
  Copy,
  Check,
  ExternalLink,
  BookOpen,
  Terminal,
} from "lucide-react";

type SdkInfo = {
  id: string;
  name: string;
  language: string;
  version: string;
  install: string;
  code: string;
  docs: string;
  status: "stable" | "beta" | "alpha";
};

const SDKS: SdkInfo[] = [
  {
    id: "node",
    name: "Node.js",
    language: "JavaScript / TypeScript",
    version: "2.4.1",
    install: "npm install @notifi/node",
    code: `import { NotifiClient } from "@notifi/node";

const client = new NotifiClient({
  apiKey: process.env.NOTIFI_API_KEY,
});

await client.sendNotification({
  recipientId: "rcp_0001",
  channel: "email",
  subject: "Welcome!",
  body: "Thank you for signing up.",
  priority: "normal",
});`,
    docs: "https://docs.notifi.dev/sdks/node",
    status: "stable",
  },
  {
    id: "python",
    name: "Python",
    language: "Python 3.9+",
    version: "1.8.0",
    install: "pip install notifi-sdk",
    code: `from notifi import NotifiClient

client = NotifiClient(api_key="YOUR_API_KEY")

client.send_notification(
    recipient_id="rcp_0001",
    channel="email",
    subject="Welcome!",
    body="Thank you for signing up.",
    priority="normal",
)`,
    docs: "https://docs.notifi.dev/sdks/python",
    status: "stable",
  },
  {
    id: "go",
    name: "Go",
    language: "Go 1.21+",
    version: "0.12.0",
    install: "go get github.com/notifi/notifi-go",
    code: `package main

import (
    "fmt"
    "github.com/notifi/notifi-go"
)

func main() {
    client := notifi.NewClient("YOUR_API_KEY")

    result, err := client.SendNotification(notifi.NotificationParams{
        RecipientID: "rcp_0001",
        Channel:     "email",
        Subject:     "Welcome!",
        Body:        "Thank you for signing up.",
        Priority:    "normal",
    })
    if err != nil {
        panic(err)
    }
    fmt.Println("Sent:", result.ID)
}`,
    docs: "https://docs.notifi.dev/sdks/go",
    status: "stable",
  },
  {
    id: "rust",
    name: "Rust",
    language: "Rust 1.75+",
    version: "0.5.2",
    install: 'cargo add notifi',
    code: `use notifi::{Client, NotificationParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("YOUR_API_KEY".to_string());

    let result = client
        .send_notification(NotificationParams {
            recipient_id: "rcp_0001".to_string(),
            channel: "email".to_string(),
            subject: Some("Welcome!".to_string()),
            body: "Thank you for signing up.".to_string(),
            priority: "normal".to_string(),
            ..Default::default()
        })
        .await?;

    println!("Sent: {}", result.id);
    Ok(())
}`,
    docs: "https://docs.notifi.dev/sdks/rust",
    status: "beta",
  },
  {
    id: "react-native",
    name: "React Native",
    language: "TypeScript / JavaScript",
    version: "1.3.0",
    install: "npm install @notifi/react-native",
    code: `import { NotifiSDK } from "@notifi/react-native";

const notifi = new NotifiSDK({
  apiKey: "YOUR_API_KEY",
});

await notifi.registerDevice({
  platform: "ios",
  token: "device_token_here",
});

await notifi.sendNotification({
  recipientId: "rcp_0001",
  channel: "push-ios",
  body: "New message received",
});`,
    docs: "https://docs.notifi.dev/sdks/react-native",
    status: "stable",
  },
  {
    id: "flutter",
    name: "Flutter",
    language: "Dart",
    version: "0.9.1",
    install: "flutter pub add notifi_flutter",
    code: `import 'package:notifi_flutter/notifi.dart';

final client = NotifiClient(
  apiKey: 'YOUR_API_KEY',
);

await client.sendNotification(
  recipientId: 'rcp_0001',
  channel: 'push-android',
  body: 'New message received',
  priority: 'normal',
);`,
    docs: "https://docs.notifi.dev/sdks/flutter",
    status: "beta",
  },
  {
    id: "swift",
    name: "Swift",
    language: "Swift 5.9+",
    version: "0.4.0",
    install: '.package(url: "https://github.com/notifi/notifi-swift", from: "0.4.0")',
    code: `import Notifi

let client = NotifiClient(apiKey: "YOUR_API_KEY")

let result = try await client.sendNotification(
    recipientId: "rcp_0001",
    channel: "push-ios",
    body: "New message received",
    priority: .normal
)

print("Sent: \\(result.id)")`,
    docs: "https://docs.notifi.dev/sdks/swift",
    status: "alpha",
  },
  {
    id: "kotlin",
    name: "Kotlin",
    language: "Kotlin 1.9+",
    version: "0.3.0",
    install: 'implementation("dev.notifi:notifi-android:0.3.0")',
    code: `import dev.notifi.NotifiClient

val client = NotifiClient("YOUR_API_KEY")

val result = client.sendNotification(
    recipientId = "rcp_0001",
    channel = "push-android",
    body = "New message received",
    priority = "normal"
)

println("Sent: \${result.id}")`,
    docs: "https://docs.notifi.dev/sdks/kotlin",
    status: "alpha",
  },
  {
    id: "curl",
    name: "cURL",
    language: "Shell / Bash",
    version: "API v1",
    install: "No installation required",
    code: `curl -X POST https://api.notifi.dev/v1/notifications \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "recipient_id": "rcp_0001",
    "channel": "email",
    "subject": "Welcome!",
    "body": "Thank you for signing up.",
    "priority": "normal"
  }'`,
    docs: "https://docs.notifi.dev/api",
    status: "stable",
  },
];

const STATUS_COLORS: Record<string, string> = {
  stable: "bg-success/15 text-success border-success/20",
  beta: "bg-warning/15 text-warning border-warning/20",
  alpha: "bg-info/15 text-info border-info/20",
};

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <Button variant="ghost" size="icon-xs" onClick={handleCopy}>
      {copied ? <Check className="size-3 text-success" /> : <Copy className="size-3" />}
    </Button>
  );
}

export default function SdkPage() {
  const [selectedSdk, setSelectedSdk] = useState("node");

  return (
    <div className="space-y-6">
      <PageHeader
        title="SDKs & Libraries"
        description="Client libraries for every platform"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "SDKs" }]}
        actions={
          <Button size="sm" variant="outline" className="gap-1.5" render={<a href="https://docs.notifi.dev" target="_blank" rel="noopener noreferrer" />}>
            <BookOpen className="size-3.5" /> API docs <ExternalLink className="size-3 ml-1" />
          </Button>
        }
      />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total SDKs</span>
              <Package className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{SDKS.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Stable</span>
              <Check className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{SDKS.filter((s) => s.status === "stable").length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Latest Version</span>
              <Terminal className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">v2.4.1</div>
          </CardContent>
        </Card>
      </div>

      <Tabs value={selectedSdk} onValueChange={(v) => { if (v) setSelectedSdk(v); }}>
        <div className="overflow-x-auto">
          <TabsList className="w-full justify-start flex-wrap h-auto gap-1 p-1 bg-muted">
            {SDKS.map((sdk) => (
              <TabsTrigger key={sdk.id} value={sdk.id} className="text-xs gap-1.5">
                {sdk.name}
                <Badge variant="secondary" className={`text-[9px] px-1 h-3.5 ${STATUS_COLORS[sdk.status]}`}>
                  {sdk.status}
                </Badge>
              </TabsTrigger>
            ))}
          </TabsList>
        </div>

        {SDKS.map((sdk) => (
          <TabsContent key={sdk.id} value={sdk.id} className="mt-4">
            <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
              <div className="space-y-4">
                <Card>
                  <CardHeader>
                    <div className="flex items-center justify-between">
                      <CardTitle>{sdk.name} SDK</CardTitle>
                      <div className="flex items-center gap-2">
                        <Badge variant="secondary" className={`text-xs ${STATUS_COLORS[sdk.status]}`}>
                          {sdk.status}
                        </Badge>
                        <Badge variant="outline" className="text-xs font-mono">v{sdk.version}</Badge>
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium">Installation</label>
                        <CopyButton text={sdk.install} />
                      </div>
                      <div className="relative">
                        <Terminal className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
                        <code className="block w-full bg-muted rounded-md px-8 py-2 text-sm font-mono">
                          $ {sdk.install}
                        </code>
                      </div>
                    </div>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium">Usage Example</label>
                        <CopyButton text={sdk.code} />
                      </div>
                      <CodeBlock code={sdk.code} language={sdk.id === "curl" ? "bash" : sdk.id === "python" ? "python" : sdk.id === "go" ? "go" : "typescript"} />
                    </div>
                  </CardContent>
                </Card>
              </div>

              <div className="space-y-4">
                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm">Details</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">Language</span>
                      <p className="text-sm">{sdk.language}</p>
                    </div>
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">Version</span>
                      <p className="text-sm font-mono">{sdk.version}</p>
                    </div>
                    <div className="space-y-1">
                      <span className="text-xs text-muted-foreground">Status</span>
                      <Badge variant="secondary" className={`text-xs capitalize ${STATUS_COLORS[sdk.status]}`}>
                        {sdk.status}
                      </Badge>
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm">Resources</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-2">
                    <a
                      href={sdk.docs}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 text-sm text-primary hover:underline"
                    >
                      <BookOpen className="size-3.5" /> Documentation
                    </a>
                    <a
                      href="https://github.com/notifi"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <Package className="size-3.5" /> GitHub Repository
                    </a>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="text-sm">API Key</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <code className="block bg-muted rounded-md px-3 py-2 text-xs font-mono break-all">
                      napi_prod_sk_••••••••••••
                    </code>
                    <p className="text-xs text-muted-foreground mt-2">
                      Set <code className="bg-muted px-1 rounded">NOTIFI_API_KEY</code> environment variable.
                    </p>
                  </CardContent>
                </Card>
              </div>
            </div>
          </TabsContent>
        ))}
      </Tabs>
    </div>
  );
}
