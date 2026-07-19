"use client";
interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}
export function ShortRenderer({ value, onChange, disabled = false }: Props) {
  return (
    <textarea
      className="min-h-16 w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      placeholder="Your answer…"
    />
  );
}
