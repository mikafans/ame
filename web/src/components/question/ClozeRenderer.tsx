"use client";
interface Props {
  prompt: string;
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}
const inputClass =
  "h-9 rounded-md border border-input bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50";
export function ClozeRenderer({
  prompt,
  value,
  onChange,
  disabled = false,
}: Props) {
  const parts = prompt.split("___");
  if (parts.length <= 1)
    return (
      <input
        className={`${inputClass} w-full`}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        placeholder="Fill in the blank…"
      />
    );
  return (
    <div className="text-[15px] leading-[2.4]">
      {parts.map((part, i) => (
        <span key={i}>
          {part}
          {i < parts.length - 1 && (
            <input
              className={`${inputClass} mx-2 w-40 text-center align-baseline`}
              value={value}
              onChange={(e) => onChange(e.target.value)}
              disabled={disabled}
              placeholder="___"
            />
          )}
        </span>
      ))}
    </div>
  );
}
