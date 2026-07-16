"use client";
/* eslint-disable @next/next/no-img-element -- sanitized external images are intentionally rendered verbatim. */
import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import { HighlightedCode } from "./HighlightedCode";

export const customSanitizeSchema = {
  ...defaultSchema,
  tagNames: [
    "p",
    "br",
    "strong",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "li",
    "code",
    "pre",
    "a",
    "img",
    "blockquote",
    "hr",
    "del",
    "span",
    "div",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
  ],
  attributes: {
    ...defaultSchema.attributes,
    a: ["href", "title", "target", "rel"],
    img: ["src", "alt", "title", "width", "height"],
  },
  protocols: {
    ...defaultSchema.protocols,
    href: ["http", "https", "mailto", "tel"],
    src: ["https"],
  },
};
export function sanitizeSvg(svg: string): string {
  return svg
    .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "")
    .replace(/\s+on[a-z]+\s*=\s*['"][^'"]*['"]/gi, "")
    .replace(/\s+on[a-z]+\s*=\s*[^>\s]+/gi, "")
    .replace(/href\s*=\s*['"]javascript:[^'"]*['"]/gi, 'href="#"')
    .replace(/xlink:href\s*=\s*['"]javascript:[^'"]*['"]/gi, 'xlink:href="#"');
}
export function MermaidView({ code }: { code: string }) {
  const [svgContent, setSvgContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let mounted = true;
    (async () => {
      try {
        setLoading(true);
        const { default: mermaid } = await import("mermaid");
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: "default",
        });
        const { svg } = await mermaid.render(
          `mermaid-${Math.random().toString(36).slice(2, 9)}`,
          code,
        );
        if (mounted) {
          setSvgContent(sanitizeSvg(svg));
          setError(null);
          setLoading(false);
        }
      } catch (err) {
        if (mounted) {
          console.error("Mermaid rendering failed:", err);
          setError(
            "Failed to render diagram. Please verify the mermaid syntax.",
          );
          setLoading(false);
        }
      }
    })();
    return () => {
      mounted = false;
    };
  }, [code]);
  return (
    <div className="my-4 flex w-full flex-col items-center justify-center overflow-x-auto rounded-lg border border-border bg-muted/30 p-4">
      {loading && (
        <div className="flex items-center gap-2 py-2 text-sm text-muted-foreground">
          <span className="size-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          Rendering diagram...
        </div>
      )}
      {error && (
        <div
          role="alert"
          className="w-full rounded-md border border-destructive/40 px-3 py-2 text-sm text-destructive"
        >
          {error}
        </div>
      )}
      {!loading && !error && svgContent && (
        <div
          className="flex w-full justify-center [&>svg]:h-auto [&>svg]:max-w-full"
          dangerouslySetInnerHTML={{ __html: svgContent }}
        />
      )}
    </div>
  );
}

export function MarkdownView({ content }: { content: string }) {
  return (
    <div className="markdown-view w-full text-[15px] leading-[1.6] [&_h1]:mb-6 [&_h1]:mt-8 [&_h1]:text-3xl [&_h1]:font-bold [&_h2]:mb-5 [&_h2]:mt-7 [&_h2]:text-2xl [&_h2]:font-semibold [&_h3]:mb-4 [&_h3]:mt-6 [&_h3]:text-xl [&_h3]:font-semibold [&_p]:mb-4 [&_a]:text-primary [&_a]:no-underline [&_a:hover]:underline [&_ul]:mb-4 [&_ul]:list-disc [&_ul]:pl-6 [&_ol]:mb-4 [&_ol]:list-decimal [&_ol]:pl-6 [&_li]:mb-1 [&_blockquote]:my-4 [&_blockquote]:border-l-4 [&_blockquote]:border-primary [&_blockquote]:bg-muted/30 [&_blockquote]:py-2 [&_blockquote]:pl-4 [&_table]:my-4 [&_table]:w-full [&_table]:border-collapse [&_th]:border [&_th]:border-border [&_th]:bg-muted/40 [&_th]:p-3 [&_th]:text-left [&_td]:border [&_td]:border-border [&_td]:p-3">
      <ReactMarkdown
        rehypePlugins={[[rehypeSanitize, customSanitizeSchema]]}
        components={{
          pre: ({ children }) => <>{children}</>,
          code: ({ children, className, ...rest }) => {
            const match = /language-(\w+)/.exec(className || "");
            const text = String(children).replace(/\n$/, "");
            if (match?.[1] === "mermaid") return <MermaidView code={text} />;
            if (match)
              return <HighlightedCode code={text} language={match[1]} />;
            return (
              <code className={className} {...rest}>
                {children}
              </code>
            );
          },
          img: ({ src, alt, ...rest }) => {
            const url = typeof src === "string" ? src : "";
            if (
              !url.startsWith("https://") ||
              url.toLowerCase().includes(".svg") ||
              url.toLowerCase().startsWith("data:image/svg")
            )
              return null;
            return (
              <img
                src={url}
                alt={alt}
                className="my-3 block h-auto max-w-full rounded"
                {...rest}
              />
            );
          },
          a: ({ href, children }) => (
            <a href={href} target="_blank" rel="noopener noreferrer">
              {children}
            </a>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
