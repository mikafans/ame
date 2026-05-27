"use client";

import { useState, useCallback } from "react";
import { makeClient } from "@/api/client";

export type ShareKind = "quiz" | "exam" | "item";

interface SharePayload {
  kind: ShareKind;
  id: string;
  title: string;
  explanation?: string;
  attribution?: string;
  course?: string;
}

interface Props {
  payload: SharePayload;
  onClose: () => void;
  bearerToken?: string;
}

type Tab = "link" | "embed";

export function ShareModal({ payload, onClose, bearerToken }: Props) {
  const [tab, setTab] = useState<Tab>("link");
  const [shareUrl, setShareUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  const slug =
    payload.kind === "item" ? "q" : payload.kind === "exam" ? "x" : "qz";

  const createShare = useCallback(async () => {
    if (shareUrl) return;
    setLoading(true);
    try {
      const client = makeClient(bearerToken);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (client as any).POST("/v1/shares", {
        body: {
          kind: payload.kind,
          targetId: payload.id,
          visibility: "public",
          includeExplanation: true,
          includeAttribution: true,
        },
      });
      if (data) {
        setShareUrl(
          `https://ame-platform.app/${slug}/${(data as { id: string }).id}`,
        );
      }
    } finally {
      setLoading(false);
    }
  }, [payload, slug, bearerToken, shareUrl]);

  const copy = useCallback((text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, []);

  const embedCode = shareUrl
    ? `<iframe src="${shareUrl}/embed" width="100%" height="480" frameborder="0"></iframe>`
    : null;

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.65)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          borderRadius: 8,
          width: 480,
          maxWidth: "calc(100vw - 32px)",
          padding: 24,
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 16,
          }}
        >
          <span style={{ fontWeight: 600, color: "var(--text)" }}>Share</span>
          <button
            onClick={onClose}
            style={{
              background: "none",
              border: "none",
              color: "var(--muted)",
              cursor: "pointer",
              fontSize: 18,
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>

        {/* Title */}
        <div
          style={{
            color: "var(--text-2)",
            fontSize: 13,
            marginBottom: 20,
            lineHeight: 1.4,
          }}
        >
          {payload.title}
        </div>

        {/* Tabs */}
        <div
          style={{
            display: "flex",
            gap: 4,
            marginBottom: 16,
            borderBottom: "1px solid var(--border)",
            paddingBottom: 0,
          }}
        >
          {(["link", "embed"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => {
                setTab(t);
                createShare();
              }}
              style={{
                padding: "6px 14px",
                background: "none",
                border: "none",
                borderBottom:
                  tab === t
                    ? "2px solid var(--accent)"
                    : "2px solid transparent",
                color: tab === t ? "var(--accent)" : "var(--muted)",
                cursor: "pointer",
                fontFamily: "var(--mono)",
                fontSize: 12,
                textTransform: "uppercase",
                letterSpacing: 0.8,
                marginBottom: -1,
              }}
            >
              {t}
            </button>
          ))}
        </div>

        {/* Content */}
        {tab === "link" && (
          <div>
            {loading && (
              <div
                style={{
                  color: "var(--muted)",
                  fontSize: 13,
                  marginBottom: 12,
                }}
              >
                Creating link…
              </div>
            )}
            {!loading && !shareUrl && (
              <button
                onClick={createShare}
                style={{
                  padding: "8px 16px",
                  background: "var(--accent)",
                  color: "#000",
                  border: "none",
                  borderRadius: 4,
                  cursor: "pointer",
                  fontWeight: 600,
                  fontSize: 13,
                  marginBottom: 12,
                }}
              >
                Generate link
              </button>
            )}
            {shareUrl && (
              <div style={{ display: "flex", gap: 8 }}>
                <input
                  readOnly
                  value={shareUrl}
                  style={{
                    flex: 1,
                    padding: "8px 12px",
                    background: "var(--surface-2)",
                    border: "1px solid var(--border)",
                    borderRadius: 4,
                    color: "var(--text)",
                    fontSize: 13,
                    fontFamily: "var(--mono)",
                  }}
                />
                <button
                  onClick={() => copy(shareUrl)}
                  style={{
                    padding: "8px 14px",
                    background: copied
                      ? "var(--accent-dim)"
                      : "var(--surface-2)",
                    border: "1px solid var(--border)",
                    borderRadius: 4,
                    color: copied ? "var(--accent)" : "var(--text-2)",
                    cursor: "pointer",
                    fontSize: 12,
                    fontFamily: "var(--mono)",
                    whiteSpace: "nowrap",
                  }}
                >
                  {copied ? "Copied!" : "Copy"}
                </button>
              </div>
            )}
          </div>
        )}

        {tab === "embed" && (
          <div>
            {!shareUrl && (
              <button
                onClick={createShare}
                style={{
                  padding: "8px 16px",
                  background: "var(--accent)",
                  color: "#000",
                  border: "none",
                  borderRadius: 4,
                  cursor: "pointer",
                  fontWeight: 600,
                  fontSize: 13,
                  marginBottom: 12,
                }}
              >
                Generate embed
              </button>
            )}
            {embedCode && (
              <div>
                <textarea
                  readOnly
                  value={embedCode}
                  rows={3}
                  style={{
                    width: "100%",
                    padding: "8px 12px",
                    background: "var(--surface-2)",
                    border: "1px solid var(--border)",
                    borderRadius: 4,
                    color: "var(--text)",
                    fontSize: 12,
                    fontFamily: "var(--mono)",
                    resize: "none",
                    boxSizing: "border-box",
                  }}
                />
                <button
                  onClick={() => copy(embedCode)}
                  style={{
                    marginTop: 8,
                    padding: "6px 14px",
                    background: copied
                      ? "var(--accent-dim)"
                      : "var(--surface-2)",
                    border: "1px solid var(--border)",
                    borderRadius: 4,
                    color: copied ? "var(--accent)" : "var(--text-2)",
                    cursor: "pointer",
                    fontSize: 12,
                    fontFamily: "var(--mono)",
                  }}
                >
                  {copied ? "Copied!" : "Copy embed code"}
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
