"use client";
import Editor from "react-simple-code-editor";
import Prism from "prismjs";
import "prismjs/components/prism-python";
import "prismjs/components/prism-javascript";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-json";
import "prismjs/components/prism-sql";
import { Code2 } from "lucide-react";
import { toPrismLanguage } from "@/lib/prismTheme";
interface Props {
  value: string;
  onChange: (v: string) => void;
  disabled?: boolean;
  language?: string;
  filename?: string;
  starter?: string;
}
export function CodeRenderer({
  value,
  onChange,
  disabled = false,
  language = "python",
  filename = "solution.py",
  starter = "",
}: Props) {
  const current = value || starter;
  const lang = toPrismLanguage(language);
  const highlight = (code: string) =>
    Prism.highlight(code, Prism.languages[lang] ?? Prism.languages.clike, lang);
  return (
    <div className="overflow-hidden rounded-lg border border-border">
      <div className="flex items-center gap-2 border-b border-border bg-muted/50 px-4 py-2 text-xs tracking-wide text-muted-foreground">
        <Code2 size={14} />
        {filename} · {language}
      </div>
      <div className="prism-editor min-h-60 bg-[#fafafa] text-[13.5px] leading-[1.6] dark:bg-[#282c34] dark:text-[#abb2bf]">
        <Editor
          value={current}
          onValueChange={disabled ? () => {} : onChange}
          highlight={highlight}
          disabled={disabled}
          tabSize={2}
          insertSpaces
          padding={16}
          placeholder="Write your solution here…"
          spellCheck={false}
          style={{
            fontFamily: "monospace",
            fontSize: "inherit",
            minHeight: 240,
          }}
        />
      </div>
    </div>
  );
}
