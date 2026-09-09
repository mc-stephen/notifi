import { useEffect, useRef, useState } from "react";
import RichTextEditor, { type RichTextEditorHandle } from "./RichTextEditor";
import { notifications, users, projects } from "../../lib/api";
import type { BroadcastAudience } from "../../lib/types";

type Target = "all" | "specific" | "projects";

export default function ComposeForm() {
  const [subject, setSubject] = useState("");
  const [target, setTarget] = useState<Target>("all");
  const [channels, setChannels] = useState<string[]>(["in_app"]);
  const [canSend, setCanSend] = useState(false);
  const [sending, setSending] = useState(false);
  const [scheduling, setScheduling] = useState(false);
  const [sendAt, setSendAt] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const editorRef = useRef<RichTextEditorHandle>(null);

  const [userQuery, setUserQuery] = useState("");
  const [userResults, setUserResults] = useState<{ id: string; name: string; email: string }[]>([]);
  const [selectedUsers, setSelectedUsers] = useState<Map<string, { name: string; email: string }>>(
    new Map(),
  );
  const [projectQuery, setProjectQuery] = useState("");
  const [projectResults, setProjectResults] = useState<{ id: string; name: string; slug: string }[]>(
    [],
  );
  const [selectedProjects, setSelectedProjects] = useState<Map<string, { name: string }>>(
    new Map(),
  );

  useEffect(() => {
    if (target !== "specific" || !userQuery.trim()) {
      setUserResults([]);
      return;
    }
    const t = setTimeout(async () => {
      try {
        const res = await users.listUsers({ search: userQuery.trim(), limit: 8 });
        setUserResults(
          res.users
            .filter((u) => u.status === "active" && !selectedUsers.has(u.id))
            .map((u) => ({ id: u.id, name: u.name, email: u.email })),
        );
      } catch {
        setUserResults([]);
      }
    }, 300);
    return () => clearTimeout(t);
  }, [userQuery, target, selectedUsers]);

  useEffect(() => {
    if (target !== "projects" || !projectQuery.trim()) {
      setProjectResults([]);
      return;
    }
    const t = setTimeout(async () => {
      try {
        const res = await projects.listProjects({ search: projectQuery.trim(), limit: 8 });
        setProjectResults(
          res.projects
            .filter((p) => !selectedProjects.has(p.id))
            .map((p) => ({ id: p.id, name: p.name, slug: p.slug })),
        );
      } catch {
        setProjectResults([]);
      }
    }, 300);
    return () => clearTimeout(t);
  }, [projectQuery, target, selectedProjects]);

  const inAppSelected = channels.includes("in_app");
  const emailSelected = channels.includes("email");

  function toggleChannel(c: string) {
    setChannels((prev) =>
      prev.includes(c) ? prev.filter((x) => x !== c) : [...prev, c],
    );
  }

  function toggleUser(id: string, name: string, email: string) {
    setSelectedUsers((prev) => {
      const next = new Map(prev);
      if (next.has(id)) next.delete(id);
      else next.set(id, { name, email });
      return next;
    });
  }

  function toggleProject(id: string, name: string) {
    setSelectedProjects((prev) => {
      const next = new Map(prev);
      if (next.has(id)) next.delete(id);
      else next.set(id, { name });
      return next;
    });
  }

  function buildAudience(): BroadcastAudience | null {
    if (target === "all") return { kind: "all" };
    if (target === "specific") {
      if (selectedUsers.size === 0) {
        setError("Select at least one user.");
        return null;
      }
      return { kind: "users", userIds: [...selectedUsers.keys()] };
    }
    if (selectedProjects.size === 0) {
      setError("Select at least one project.");
      return null;
    }
    return { kind: "projects", projectIds: [...selectedProjects.keys()] };
  }

  async function submit(scheduledFor?: string) {
    if (sending || scheduling) return;
    setError(null);
    setSuccess(null);
    if (!subject.trim()) {
      setError("Add a subject.");
      return;
    }
    if (editorRef.current?.isEmpty()) {
      setError("Write a message first.");
      return;
    }
    const audience = buildAudience();
    if (!audience) return;

    const busy = scheduledFor ? setScheduling : setSending;
    busy(true);
    try {
      const res = await notifications.broadcast({
        title: subject.trim(),
        content: editorRef.current?.getHTML() ?? "",
        notificationType: "system",
        channels,
        audience,
        ...(scheduledFor ? { scheduledFor } : {}),
      });
      editorRef.current?.clear();
      setCanSend(false);
      setSubject("");
      setSelectedUsers(new Map());
      setSelectedProjects(new Map());
      setSendAt("");
      if (scheduledFor) {
        setSuccess(`Scheduled for ${res.broadcast.recipientCount} users.`);
      } else {
        setSuccess(`Sent to ${res.broadcast.recipientCount} users.`);
      }
      setTimeout(() => window.location.reload(), 1200);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Send failed. Please try again.");
    } finally {
      busy(false);
    }
  }

  function schedule() {
    if (!sendAt) {
      setError("Pick a date and time first.");
      return;
    }
    const at = new Date(sendAt);
    if (Number.isNaN(at.getTime()) || at <= new Date()) {
      setError("Scheduled time must be in the future.");
      return;
    }
    submit(at.toISOString());
  }

  const busy = sending || scheduling;

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground" htmlFor="compose-subject">
          Subject
        </label>
        <input
          id="compose-subject"
          type="text"
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          placeholder="Something important happened"
          disabled={busy}
          className="flex h-9 w-full rounded-lg border border-input bg-card px-3 py-1.5 text-sm transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
        />
      </div>

      <div className="space-y-2">
        <span className="text-sm font-medium text-foreground">Target</span>
        <div className="flex flex-wrap gap-4 text-sm">
          {(["all", "specific", "projects"] as Target[]).map((t) => (
            <label key={t} className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="radio"
                name="target"
                checked={target === t}
                onChange={() => setTarget(t)}
                disabled={busy}
                className="accent-primary"
              />
              <span className="capitalize">{t === "projects" ? "By project" : t === "all" ? "Everyone" : "Specific users"}</span>
            </label>
          ))}
        </div>

        {target === "specific" && (
          <div className="rounded-lg border border-border p-3 space-y-2">
            <div className="flex flex-wrap gap-1.5">
              {[...selectedUsers].map(([id, u]) => (
                <span
                  key={id}
                  className="inline-flex items-center gap-1 rounded-md bg-primary/10 text-primary border border-primary/20 px-2 py-0.5 text-xs font-medium"
                >
                  {u.name}
                  <button
                    type="button"
                    onClick={() => toggleUser(id, u.name, u.email)}
                    className="ml-0.5 hover:text-destructive"
                    aria-label={`Remove ${u.name}`}
                  >
                    ×
                  </button>
                </span>
              ))}
              {selectedUsers.size === 0 && (
                <span className="text-xs text-muted-foreground">No users selected yet.</span>
              )}
            </div>
            <input
              type="text"
              value={userQuery}
              onChange={(e) => setUserQuery(e.target.value)}
              placeholder="Search users by name or email..."
              disabled={busy}
              className="flex h-9 w-full rounded-lg border border-input bg-card px-3 py-1.5 text-sm transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
            />
            {userResults.length > 0 && (
              <ul className="max-h-[180px] overflow-y-auto divide-y divide-border/50 rounded-lg border border-border">
                {userResults.map((u) => (
                  <li key={u.id}>
                    <button
                      type="button"
                      onClick={() => toggleUser(u.id, u.name, u.email)}
                      className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-muted transition-colors"
                    >
                      <span className="text-sm font-medium text-foreground">{u.name}</span>
                      <span className="text-xs text-muted-foreground truncate">{u.email}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}

        {target === "projects" && (
          <div className="rounded-lg border border-border p-3 space-y-2">
            <div className="flex flex-wrap gap-1.5">
              {[...selectedProjects].map(([id, p]) => (
                <span
                  key={id}
                  className="inline-flex items-center gap-1 rounded-md bg-primary/10 text-primary border border-primary/20 px-2 py-0.5 text-xs font-medium"
                >
                  {p.name}
                  <button
                    type="button"
                    onClick={() => toggleProject(id, p.name)}
                    className="ml-0.5 hover:text-destructive"
                    aria-label={`Remove ${p.name}`}
                  >
                    ×
                  </button>
                </span>
              ))}
              {selectedProjects.size === 0 && (
                <span className="text-xs text-muted-foreground">No projects selected yet.</span>
              )}
            </div>
            <input
              type="text"
              value={projectQuery}
              onChange={(e) => setProjectQuery(e.target.value)}
              placeholder="Search projects by name or slug..."
              disabled={busy}
              className="flex h-9 w-full rounded-lg border border-input bg-card px-3 py-1.5 text-sm transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
            />
            {projectResults.length > 0 && (
              <ul className="max-h-[180px] overflow-y-auto divide-y divide-border/50 rounded-lg border border-border">
                {projectResults.map((p) => (
                  <li key={p.id}>
                    <button
                      type="button"
                      onClick={() => toggleProject(p.id, p.name)}
                      className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-muted transition-colors"
                    >
                      <span className="text-sm font-medium text-foreground">{p.name}</span>
                      <span className="text-xs text-muted-foreground truncate">{p.slug}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>

      <div className="space-y-2">
        <span className="text-sm font-medium text-foreground">Channels</span>
        <div className="flex flex-wrap gap-4 text-sm">
          {[
            { value: "in_app", label: "In-app" },
            { value: "email", label: "Email" },
          ].map((c) => (
            <label key={c.value} className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="checkbox"
                checked={channels.includes(c.value)}
                onChange={() => toggleChannel(c.value)}
                disabled={busy}
                className="accent-primary"
              />
              <span>{c.label}</span>
            </label>
          ))}
        </div>
        {emailSelected && (
          <p className="rounded-lg border border-warning/20 bg-warning/5 p-3 text-sm text-warning">
            Email delivery isn't implemented yet — selections are recorded and will
            send once available. In-app delivers now.
          </p>
        )}
        {!inAppSelected && (
          <p className="rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
            Select in-app to deliver — email alone can't send anything yet.
          </p>
        )}
      </div>

      <div className="space-y-2">
        <span className="text-sm font-medium text-foreground">Message</span>
        <RichTextEditor
          ref={editorRef}
          placeholder="Write your notification message..."
          onUpdate={(_html, empty) => setCanSend(!empty)}
        />
      </div>

      {error && (
        <p className="rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
          {error}
        </p>
      )}
      {success && (
        <p className="rounded-lg border border-success/20 bg-success/5 p-3 text-sm text-success">
          {success}
        </p>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => submit()}
          disabled={busy || !inAppSelected || !canSend}
          className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/80 disabled:opacity-50 disabled:pointer-events-none"
        >
          {sending ? "Sending..." : "Send Now"}
        </button>
        <input
          type="datetime-local"
          value={sendAt}
          onChange={(e) => setSendAt(e.target.value)}
          disabled={busy || !inAppSelected}
          className="flex h-9 rounded-lg border border-input bg-card px-3 py-1.5 text-sm transition-colors focus-visible:border-ring focus-visible:outline-none disabled:opacity-50"
          aria-label="Schedule date and time"
        />
        <button
          type="button"
          onClick={schedule}
          disabled={busy || !inAppSelected || !canSend}
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-muted transition-colors disabled:opacity-50 disabled:pointer-events-none"
        >
          {scheduling ? "Scheduling..." : "Schedule"}
        </button>
      </div>
    </div>
  );
}
