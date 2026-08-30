export function StatusBadge({
  label,
  tone = "neutral",
}: {
  label: string;
  tone?: "neutral" | "positive" | "warning";
}) {
  return <span className={`status-text status-${tone}`}>{label}</span>;
}
