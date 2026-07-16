"use client";
import { useEffect, useRef } from "react";
import Prism from "prismjs";
import "prismjs/components/prism-python";
import "prismjs/components/prism-javascript";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-json";
import "prismjs/components/prism-sql";
import { toPrismLanguage } from "@/lib/prismTheme";
interface Props {
  code: string;
  language?: string;
}
export function HighlightedCode({ code, language = "python" }: Props) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    if (ref.current) Prism.highlightElement(ref.current);
  }, [code, language]);
  const lang = toPrismLanguage(language);
  return (
    <pre className={`prism-code language-${lang}`}>
      <code ref={ref} className={`language-${lang}`}>
        {code.trim()}
      </code>
    </pre>
  );
}
