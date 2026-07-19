"use client";
interface McqOption {
  text: string;
  index: number;
}
interface Props {
  options: McqOption[];
  value: number | null;
  onChange: (position: number) => void;
  disabled?: boolean;
}
const LABELS = ["A", "B", "C", "D", "E", "F"];
export function McqRenderer({
  options,
  value,
  onChange,
  disabled = false,
}: Props) {
  return (
    <div className="flex flex-col">
      {options.map((opt, displayIdx) => {
        const selected = value === opt.index;
        return (
          <div
            key={opt.index}
            role="button"
            tabIndex={disabled ? -1 : 0}
            onClick={() => !disabled && onChange(opt.index)}
            onKeyDown={(e) => {
              if ((e.key === "Enter" || e.key === " ") && !disabled)
                onChange(opt.index);
            }}
            className={`flex items-center gap-4 border-b px-5 py-4 transition-colors ${displayIdx === 0 ? "border-t" : ""} ${selected ? "border-l-[3px] border-l-primary bg-primary/10" : "border-border hover:bg-muted/50"} ${disabled ? "cursor-default" : "cursor-pointer"}`}
          >
            <span
              className={`flex size-7 shrink-0 items-center justify-center rounded-full border-2 text-xs font-bold ${selected ? "border-primary bg-primary text-primary-foreground" : "border-border text-muted-foreground"}`}
            >
              {LABELS[displayIdx] ?? displayIdx + 1}
            </span>
            <span
              className={`leading-6 ${selected ? "text-primary" : "text-foreground"}`}
            >
              {opt.text}
            </span>
          </div>
        );
      })}
    </div>
  );
}
