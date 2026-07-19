"use client";
interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}
export function EssayRenderer({ value, onChange, disabled = false }: Props) {
  const wordCount = value.trim() ? value.trim().split(/\s+/).length : 0;
  return (
    <div>
      <textarea
        className="min-h-48 w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder="Write your answer here…"
      />
      <p className="mt-1 text-right text-xs text-muted-foreground">
        {wordCount} word{wordCount !== 1 ? "s" : ""}
      </p>
    </div>
  );
}
