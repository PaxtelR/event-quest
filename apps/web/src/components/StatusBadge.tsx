import { messages } from "@/i18n/en-US";

const STATUS_STYLES: Record<string, string> = {
  draft: "bg-surface-elevated text-text-secondary border-border-strong",
  active: "bg-accent-soft text-accent border-accent/40",
  paused: "bg-warning/10 text-warning border-warning/40",
  finished: "bg-info/10 text-info border-info/40",
  cancelled: "bg-error/10 text-error border-error/40",
  closed: "bg-surface-elevated text-text-secondary border-border-strong",
};

export function StatusBadge({ status }: { status: keyof typeof messages.events.status }) {
  return (
    <span
      className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium ${STATUS_STYLES[status] ?? STATUS_STYLES.draft}`}
    >
      {messages.events.status[status]}
    </span>
  );
}
