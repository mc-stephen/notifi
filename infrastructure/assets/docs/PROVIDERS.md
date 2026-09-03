# Provider Registry & Channel Configs API Contract

This document describes the API endpoints for the provider registry and
project channel configs.

---

## GET /v1/providers

Returns the full provider registry — all channels and their available providers
with config field definitions.

### Response (200)

```json
{
  "version": "1.0.0",
  "last_updated": "2026-09-03T00:00:00Z",
  "channels": [
    {
      "channel_id": "email",
      "channel_name": "Email",
      "providers": [
        {
          "provider_id": "smtp",
          "name": "SMTP (Generic)",
          "scope": "global",
          "primary_regions": [],
          "config_fields": [
            {
              "key": "host",
              "label": "SMTP Host",
              "type": "text",
              "required": true
            },
            {
              "key": "port",
              "label": "SMTP Port",
              "type": "number",
              "required": true
            },
            {
              "key": "username",
              "label": "Username",
              "type": "text",
              "required": true
            },
            {
              "key": "password",
              "label": "Password",
              "type": "password",
              "required": true
            },
            {
              "key": "tls",
              "label": "Use TLS",
              "type": "boolean",
              "required": false
            }
          ]
        },
        {
          "provider_id": "sendgrid",
          "name": "SendGrid",
          "scope": "global",
          "primary_regions": [],
          "config_fields": [
            {
              "key": "api_key",
              "label": "API Key",
              "type": "password",
              "required": true
            }
          ],
          "smtp_fallback": {
            "fields": [
              {
                "key": "host",
                "label": "SMTP Host",
                "type": "text",
                "required": true
              },
              {
                "key": "port",
                "label": "SMTP Port",
                "type": "number",
                "required": true
              },
              {
                "key": "username",
                "label": "Username",
                "type": "text",
                "required": true
              },
              {
                "key": "password",
                "label": "Password",
                "type": "password",
                "required": true
              }
            ]
          }
        }
      ]
    }
  ]
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Registry version |
| `last_updated` | string (ISO 8601) | When the registry was last updated |
| `channels[]` | array | List of channel definitions |
| `channels[].channel_id` | string | Unique channel identifier (e.g., "email", "sms") |
| `channels[].channel_name` | string | Human-readable channel name |
| `channels[].providers[]` | array | List of provider definitions |
| `providers[].provider_id` | string | Unique provider identifier |
| `providers[].name` | string | Human-readable provider name |
| `providers[].scope` | string | "global" or "regional" |
| `providers[].primary_regions[]` | string[] | Required if scope is "regional" |
| `providers[].platforms[]` | string[] | Required for push channel (e.g., ["ios", "android"]) |
| `providers[].config_fields[]` | array | Config fields for this provider |
| `providers[].smtp_fallback` | object | Optional SMTP fallback config |

### Config Field Types

| Type | Description |
|------|-------------|
| `text` | Text input |
| `password` | Password input (masked) |
| `email` | Email input |
| `number` | Numeric input |
| `boolean` | Toggle/checkbox |

---

## Channel Configs CRUD

Project-scoped endpoints for managing provider configurations.

### List configs

```
GET /v1/projects/{project_id}/channel-configs
```

**Response (200)**:
```json
{
  "configs": [
    {
      "id": "01J0V1K3...",
      "project_id": "01H1...",
      "channel_id": "email",
      "provider_id": "sendgrid",
      "config": { "api_key": "SG.xxx" },
      "enabled": true,
      "created_at": "2026-09-03T00:00:00Z",
      "updated_at": "2026-09-03T00:00:00Z"
    }
  ]
}
```

### Create config

```
POST /v1/projects/{project_id}/channel-configs
```

**Request body**:
```json
{
  "channel_id": "email",
  "provider_id": "sendgrid",
  "config": {
    "api_key": "SG.xxx"
  },
  "enabled": true
}
```

**Response (201)**: Returns the created config.

### Update config

```
PATCH /v1/projects/{project_id}/channel-configs/{config_id}
```

**Request body** (partial update):
```json
{
  "config": {
    "api_key": "SG.yyy"
  },
  "enabled": false
}
```

**Response (200)**: Returns the updated config.

### Delete config

```
DELETE /v1/projects/{project_id}/channel-configs/{config_id}
```

**Response (204)**: No content.

---

## Frontend Integration

The providers page (`/providers`) consumes these endpoints:

1. **Load registry**: `useProviderRegistry()` hook calls `GET /v1/providers`
2. **Connect provider**: User clicks "Connect" → modal with dynamic config fields
3. **Save config**: `POST /v1/projects/{id}/channel-configs` with form data
4. **Manage configs**: View/edit/delete from project settings

### Hook: `useProviderRegistry`

```typescript
import { useProviderRegistry } from "@/hooks/use-provider-registry";

const { registry, loading, error } = useProviderRegistry();

// registry.channels[].providers[].config_fields → renders form inputs dynamically
```

### Config Field Rendering

The frontend renders config fields dynamically based on `config_fields[]`:

| Type | Component |
|------|-----------|
| `text` | `<Input type="text" />` |
| `password` | `<Input type="password" />` |
| `email` | `<Input type="email" />` |
| `number` | `<Input type="number" />` |
| `boolean` | `<Switch />` |

---

## Error Responses

All errors follow RFC 9457 Problem Details:

```json
{
  "type": "https://api.notifi.dev/errors/validation",
  "title": "Validation Error",
  "status": 422,
  "detail": "channel_id is required",
  "instance": "/v1/projects/xxx/channel-configs"
}
```
