"use client";
import { CheckCircle2, XCircle } from "lucide-react";
interface Props {
  value: boolean | null;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}
const OPTIONS = [
  {
    v: true,
    label: "True",
    Icon: CheckCircle2,
    selected: "border-emerald-500 bg-emerald-500/10",
    hover: "hover:border-emerald-500/60 hover:bg-emerald-500/5",
    icon: "text-emerald-500",
    text: "text-emerald-600 dark:text-emerald-400",
  },
  {
    v: false,
    label: "False",
    Icon: XCircle,
    selected: "border-red-500 bg-red-500/10",
    hover: "hover:border-red-500/60 hover:bg-red-500/5",
    icon: "text-red-500",
    text: "text-red-600 dark:text-red-400",
  },
];
export function TfRenderer({ value, onChange, disabled = false }: Props) {
  return (
    <div className="grid grid-cols-2 gap-4">
      {OPTIONS.map(
        ({ v, label, Icon, selected: selectedClass, hover, icon, text }) => {
          const selected = value === v;
          return (
            <div
              key={label}
              role="button"
              tabIndex={disabled ? -1 : 0}
              onClick={() => !disabled && onChange(v)}
              onKeyDown={(e) => {
                if ((e.key === "Enter" || e.key === " ") && !disabled)
                  onChange(v);
              }}
              className={`flex flex-col items-center justify-center gap-3 rounded-lg border-2 py-10 transition-colors ${selected ? selectedClass : `border-border ${hover}`} ${disabled ? "cursor-default" : "cursor-pointer"}`}
            >
              <Icon
                size={36}
                className={selected ? icon : "text-muted-foreground"}
              />
              <span
                className={`text-lg font-semibold ${selected ? text : "text-muted-foreground"}`}
              >
                {label}
              </span>
            </div>
          );
        },
      )}
    </div>
  );
}
