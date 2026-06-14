"use client";

import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import Link from "@mui/material/Link";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import { HighlightedCode } from "./HighlightedCode";

// Define the custom schema for rehype-sanitize
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
    src: ["https"], // Strict https only for images
  },
};

/**
 * Sanitizes an SVG string to protect against potential XSS/malicious scripts.
 * Acts as a defense-in-depth layer on top of Mermaid's strict securityLevel.
 */
export function sanitizeSvg(svg: string): string {
  // 1. Remove any <script> tags and their contents
  let cleaned = svg.replace(
    /<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi,
    "",
  );

  // 2. Remove event handlers like onload, onclick, onerror, etc.
  cleaned = cleaned.replace(/\s+on[a-z]+\s*=\s*['"][^'"]*['"]/gi, "");
  cleaned = cleaned.replace(/\s+on[a-z]+\s*=\s*[^>\s]+/gi, "");

  // 3. Remove javascript: URLs in href or xlink:href
  cleaned = cleaned.replace(
    /href\s*=\s*['"]javascript:[^'"]*['"]/gi,
    'href="#"',
  );
  cleaned = cleaned.replace(
    /xlink:href\s*=\s*['"]javascript:[^'"]*['"]/gi,
    'xlink:href="#"',
  );

  return cleaned;
}

interface MermaidViewProps {
  code: string;
}

/**
 * Client component to render mermaid diagrams dynamically.
 */
export function MermaidView({ code }: MermaidViewProps) {
  const [svgContent, setSvgContent] = useState<string>("");
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    const renderDiagram = async () => {
      try {
        setLoading(true);
        // Dynamically import mermaid to avoid server-side document references
        const { default: mermaid } = await import("mermaid");

        mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: "default",
        });

        const id = `mermaid-${Math.random().toString(36).substring(2, 9)}`;
        const { svg } = await mermaid.render(id, code);

        if (isMounted) {
          const sanitized = sanitizeSvg(svg);
          setSvgContent(sanitized);
          setError(null);
          setLoading(false);
        }
      } catch (err) {
        if (isMounted) {
          console.error("Mermaid rendering failed:", err);
          setError(
            "Failed to render diagram. Please verify the mermaid syntax.",
          );
          setLoading(false);
        }
      }
    };

    renderDiagram();

    return () => {
      isMounted = false;
    };
  }, [code]);

  return (
    <Box
      sx={{
        my: 2,
        p: 2,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        bgcolor: (theme) =>
          theme.palette.mode === "dark"
            ? "rgba(255,255,255,0.02)"
            : "rgba(0,0,0,0.02)",
        border: "1px solid",
        borderColor: "divider",
        borderRadius: 2,
        overflowX: "auto",
        width: "100%",
        boxSizing: "border-box",
      }}
    >
      {loading && (
        <Box sx={{ display: "flex", alignItems: "center", gap: 2, py: 2 }}>
          <CircularProgress size={20} />
          <Typography variant="body2" color="text.secondary">
            Rendering diagram...
          </Typography>
        </Box>
      )}
      {error && (
        <Alert severity="error" variant="outlined" sx={{ width: "100%" }}>
          {error}
        </Alert>
      )}
      {!loading && !error && svgContent && (
        <Box
          dangerouslySetInnerHTML={{ __html: svgContent }}
          sx={{
            width: "100%",
            display: "flex",
            justifyContent: "center",
            "& svg": {
              maxWidth: "100%",
              height: "auto",
            },
          }}
        />
      )}
    </Box>
  );
}

interface MarkdownViewProps {
  content: string;
}

export function MarkdownView({ content }: MarkdownViewProps) {
  return (
    <Box className="markdown-view" sx={{ width: "100%" }}>
      <ReactMarkdown
        rehypePlugins={[[rehypeSanitize, customSanitizeSchema]]}
        components={{
          // Render headings
          h1: ({ children }) => (
            <Typography
              variant="h4"
              component="h1"
              gutterBottom
              sx={{ mt: 3, mb: 1.5, fontWeight: 700 }}
            >
              {children}
            </Typography>
          ),
          h2: ({ children }) => (
            <Typography
              variant="h5"
              component="h2"
              gutterBottom
              sx={{ mt: 2.5, mb: 1.25, fontWeight: 600 }}
            >
              {children}
            </Typography>
          ),
          h3: ({ children }) => (
            <Typography
              variant="h6"
              component="h3"
              gutterBottom
              sx={{ mt: 2, mb: 1, fontWeight: 600 }}
            >
              {children}
            </Typography>
          ),
          // Render paragraphs
          p: ({ children }) => (
            <Typography
              component="p"
              variant="body1"
              sx={{ mb: 2, lineHeight: 1.6, color: "text.primary" }}
            >
              {children}
            </Typography>
          ),
          // Render links
          a: ({ href, children }) => (
            <Link
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              sx={{
                color: "primary.main",
                textDecoration: "none",
                "&:hover": { textDecoration: "underline" },
              }}
            >
              {children}
            </Link>
          ),
          // Render list containers
          ul: ({ children }) => (
            <Box component="ul" sx={{ pl: 3, mb: 2, listStyleType: "disc" }}>
              {children}
            </Box>
          ),
          ol: ({ children }) => (
            <Box component="ol" sx={{ pl: 3, mb: 2, listStyleType: "decimal" }}>
              {children}
            </Box>
          ),
          li: ({ children }) => (
            <Box component="li" sx={{ mb: 0.5 }}>
              <Typography variant="body1" component="span">
                {children}
              </Typography>
            </Box>
          ),
          // Render blockquotes
          blockquote: ({ children }) => (
            <Box
              component="blockquote"
              sx={{
                borderLeft: "4px solid",
                borderColor: "primary.light",
                pl: 2,
                py: 0.5,
                my: 2,
                color: "text.secondary",
                bgcolor: (theme) =>
                  theme.palette.mode === "dark"
                    ? "rgba(255,255,255,0.02)"
                    : "rgba(0,0,0,0.02)",
                borderRadius: "0 4px 4px 0",
              }}
            >
              {children}
            </Box>
          ),
          // Render tables
          table: ({ children }) => (
            <Box sx={{ overflowX: "auto", my: 2 }}>
              <Box
                component="table"
                sx={{
                  width: "100%",
                  borderCollapse: "collapse",
                  "& th, & td": {
                    border: "1px solid",
                    borderColor: "divider",
                    p: 1.5,
                    textAlign: "left",
                  },
                  "& th": {
                    bgcolor: (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(255,255,255,0.04)"
                        : "rgba(0,0,0,0.02)",
                    fontWeight: 600,
                  },
                }}
              >
                {children}
              </Box>
            </Box>
          ),
          // Custom code block & mermaid rendering
          pre: ({ children }) => <>{children}</>,
          code: ({ children, className, ...rest }) => {
            const match = /language-(\w+)/.exec(className || "");
            const codeText = String(children).replace(/\n$/, "");

            if (match) {
              const lang = match[1];
              if (lang === "mermaid") {
                return <MermaidView code={codeText} />;
              }
              return <HighlightedCode code={codeText} language={lang} />;
            }

            return (
              <code className={className} {...rest}>
                {children}
              </code>
            );
          },
          // Custom img component to enforce strict safe requirements
          img: ({ src, alt, ...rest }) => {
            const srcStr = typeof src === "string" ? src : "";
            if (!srcStr) return null;

            // Enforce https-only
            if (!srcStr.startsWith("https://")) {
              return null;
            }

            // Exclude SVG images and data URIs
            const urlLower = srcStr.toLowerCase();
            if (
              urlLower.includes(".svg") ||
              urlLower.startsWith("data:image/svg")
            ) {
              return null;
            }

            return (
              <Box
                component="img"
                src={srcStr}
                alt={alt}
                sx={{
                  maxWidth: "100%",
                  height: "auto",
                  borderRadius: 1,
                  my: 1.5,
                  display: "block",
                }}
                {...rest}
              />
            );
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </Box>
  );
}
