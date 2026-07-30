export function GetMethod() {
  return (
    <span className="inline-flex items-center gap-2 rounded-md bg-emerald-50 px-2.5 py-1 text-xs font-semibold text-emerald-700 ring-1 ring-emerald-600/20 dark:bg-emerald-950 dark:text-emerald-300 dark:ring-emerald-400/20">
      <span className="font-mono text-[10px] uppercase tracking-wider">GET</span>
    </span>
  );
}

export function PostMethod() {
  return (
    <span className="inline-flex items-center gap-2 rounded-md bg-blue-50 px-2.5 py-1 text-xs font-semibold text-blue-700 ring-1 ring-blue-600/20 dark:bg-blue-950 dark:text-blue-300 dark:ring-blue-400/20">
      <span className="font-mono text-[10px] uppercase tracking-wider">POST</span>
    </span>
  );
}

export function PutMethod() {
  return (
    <span className="inline-flex items-center gap-2 rounded-md bg-amber-50 px-2.5 py-1 text-xs font-semibold text-amber-700 ring-1 ring-amber-600/20 dark:bg-amber-950 dark:text-amber-300 dark:ring-amber-400/20">
      <span className="font-mono text-[10px] uppercase tracking-wider">PUT</span>
    </span>
  );
}

export function DeleteMethod() {
  return (
    <span className="inline-flex items-center gap-2 rounded-md bg-red-50 px-2.5 py-1 text-xs font-semibold text-red-700 ring-1 ring-red-600/20 dark:bg-red-950 dark:text-red-300 dark:ring-red-400/20">
      <span className="font-mono text-[10px] uppercase tracking-wider">DELETE</span>
    </span>
  );
}
