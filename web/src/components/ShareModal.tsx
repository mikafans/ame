"use client";

import { useState, useCallback } from "react";
import { api } from "@/api/client";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import IconButton from "@mui/material/IconButton";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Tabs from "@mui/material/Tabs";
import Tab from "@mui/material/Tab";
import TextField from "@mui/material/TextField";
import Button from "@mui/material/Button";
import Typography from "@mui/material/Typography";
import CloseOutlinedIcon from "@mui/icons-material/CloseOutlined";

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
}

type Tab = "link" | "embed";

export function ShareModal({ payload, onClose }: Props) {
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
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (api as any).POST("/v1/shares", {
        body: {
          kind: payload.kind,
          id: payload.id,
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
  }, [payload, slug, shareUrl]);

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
    <Dialog open onClose={onClose} fullWidth maxWidth="xs">
      <DialogTitle sx={{ pr: 6 }}>
        Share
        <IconButton
          onClick={onClose}
          sx={{ position: "absolute", right: 8, top: 8 }}
        >
          <CloseOutlinedIcon />
        </IconButton>
      </DialogTitle>
      <DialogContent>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {payload.title}
        </Typography>

        <Tabs
          value={tab}
          onChange={(_, v) => {
            setTab(v);
            createShare();
          }}
          sx={{ mb: 2, borderBottom: 1, borderColor: "divider" }}
        >
          <Tab value="link" label="Link" sx={{ textTransform: "none" }} />
          <Tab value="embed" label="Embed" sx={{ textTransform: "none" }} />
        </Tabs>

        {tab === "link" && (
          <Box>
            {loading && (
              <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                Creating link…
              </Typography>
            )}
            {!loading && !shareUrl && (
              <Button
                variant="contained"
                disableElevation
                onClick={createShare}
              >
                Generate link
              </Button>
            )}
            {shareUrl && (
              <Stack direction="row" spacing={1}>
                <TextField
                  fullWidth
                  size="small"
                  value={shareUrl}
                  slotProps={{ input: { readOnly: true } }}
                />
                <Button
                  variant="outlined"
                  onClick={() => copy(shareUrl)}
                  sx={{ whiteSpace: "nowrap" }}
                >
                  {copied ? "Copied!" : "Copy"}
                </Button>
              </Stack>
            )}
          </Box>
        )}

        {tab === "embed" && (
          <Box>
            {!shareUrl && (
              <Button
                variant="contained"
                disableElevation
                onClick={createShare}
              >
                Generate embed
              </Button>
            )}
            {embedCode && (
              <Stack spacing={1}>
                <TextField
                  fullWidth
                  multiline
                  minRows={3}
                  value={embedCode}
                  slotProps={{ input: { readOnly: true } }}
                  sx={{
                    "& textarea": { fontFamily: "monospace", fontSize: 12 },
                  }}
                />
                <Button
                  variant="outlined"
                  onClick={() => copy(embedCode)}
                  sx={{ alignSelf: "flex-start" }}
                >
                  {copied ? "Copied!" : "Copy embed code"}
                </Button>
              </Stack>
            )}
          </Box>
        )}
      </DialogContent>
    </Dialog>
  );
}
