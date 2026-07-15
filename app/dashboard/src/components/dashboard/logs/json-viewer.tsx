"use client";

interface JsonViewerProps {
  data: Record<string, unknown>;
}

function stringify(obj: unknown, indent = 0): string {
  const pad = "  ".repeat(indent);
  if (obj === null) return "null";
  if (typeof obj === "string") return `"${obj}"`;
  if (typeof obj === "number" || typeof obj === "boolean") return String(obj);
  if (Array.isArray(obj)) {
    if (obj.length === 0) return "[]";
    const items = obj.map((item) => `${pad}  ${stringify(item, indent + 1)}`);
    return `[\n${items.join(",\n")}\n${pad}]`;
  }
  if (typeof obj === "object") {
    const entries = Object.entries(obj as Record<string, unknown>);
    if (entries.length === 0) return "{}";
    const items = entries.map(
      ([key, val]) => `${pad}  "${key}": ${stringify(val, indent + 1)}`,
    );
    return `{\n${items.join(",\n")}\n${pad}}`;
  }
  return String(obj);
}

export function JsonViewer({ data }: JsonViewerProps) {
  const json = stringify(data);

  return (
    <pre className="overflow-auto rounded-lg bg-[#0d1117] p-4 text-xs leading-relaxed">
      <code>{json}</code>
    </pre>
  );
}
