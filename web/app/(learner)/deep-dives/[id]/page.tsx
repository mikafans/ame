"use client";

import { use, useEffect, useState, useMemo } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { PageShell } from "@/components/PageShell";
import { MarkdownView } from "@/components/MarkdownView";
import { formatDate } from "@/utils/format";
import { tagColor } from "@/lib/tagColor";
import { useColorMode } from "@/components/ThemeRegistry";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import Divider from "@mui/material/Divider";
import Grid from "@mui/material/Grid";
import TextField from "@mui/material/TextField";
import Drawer from "@mui/material/Drawer";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import ArchiveOutlinedIcon from "@mui/icons-material/ArchiveOutlined";
import UnarchiveOutlinedIcon from "@mui/icons-material/UnarchiveOutlined";
import HistoryIcon from "@mui/icons-material/History";

interface DeepDive {
  id: string;
  questionId: string;
  sourceSessionId?: string | null;
  sourceAttemptId?: string | null;
  status: string;
  reason?: string | null;
  bodyMarkdown?: string | null;
  createdAt: string;
  updatedAt: string;
  publishedAt?: string | null;
  archivedAt?: string | null;
  category?: string | null;
  userNote?: string | null;
  noteUpdatedAt?: string | null;
  questionPrompt: string;
  questionKind: string;
  questionTags: string[];
  assessmentTitle?: string | null;
  course?: string | null;
}

interface DeepDiveRevision {
  id: string;
  deepDiveId: string;
  revision: number;
  bodyMarkdown?: string | null;
  category?: string | null;
  userNote?: string | null;
  createdBy?: string | null;
  createdAt: string;
}

const STATUS_LABELS: Record<string, string> = {
  requested: "Requested",
  drafting: "Drafting",
  published: "Published",
  needs_revision: "Needs revision",
  archived: "Archived",
};

export default function DeepDiveDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const router = useRouter();
  const { mode } = useColorMode();
  const isDark = mode === "dark";
  const [item, setItem] = useState<DeepDive | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Notes Autosave state
  const [userNote, setUserNote] = useState("");
  const [hasInitializedNote, setHasInitializedNote] = useState(false);
  const [noteStatus, setNoteStatus] = useState<"idle" | "saving" | "saved">(
    "idle",
  );

  // Revisions state
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [revisions, setRevisions] = useState<DeepDiveRevision[]>([]);
  const [revisionsLoading, setRevisionsLoading] = useState(false);

  useEffect(() => {
    let ignore = false;
    setLoading(true);
    setError(null);
    (api as any)
      .GET("/v1/deep-dives/{id}", { params: { path: { id } } })
      .then(({ data }: { data?: DeepDive }) => {
        if (!ignore) {
          setItem(data ?? null);
          if (data && !hasInitializedNote) {
            setUserNote(data.userNote ?? "");
            setHasInitializedNote(true);
          }
        }
      })
      .catch(() => {
        if (!ignore) setError("Could not load this deep dive.");
      })
      .finally(() => {
        if (!ignore) setLoading(false);
      });
    return () => {
      ignore = true;
    };
  }, [id]);

  // Debounced Autosaving
  useEffect(() => {
    if (!hasInitializedNote) return;
    if (userNote === (item?.userNote ?? "")) return;

    setNoteStatus("idle");
    const timer = setTimeout(async () => {
      setNoteStatus("saving");
      try {
        const { data } = await (api as any).PATCH("/v1/deep-dives/{id}", {
          params: { path: { id } },
          body: { userNote },
        });
        if (data) {
          setItem(data);
          setNoteStatus("saved");
        }
      } catch (err) {
        console.error("Autosave error:", err);
        setNoteStatus("idle");
      }
    }, 1000);

    return () => clearTimeout(timer);
  }, [userNote, hasInitializedNote, id]);

  // Fetch revisions when drawer opens
  useEffect(() => {
    if (drawerOpen) {
      setRevisionsLoading(true);
      (api as any)
        .GET("/v1/deep-dives/{id}/revisions", { params: { path: { id } } })
        .then(({ data }: { data?: { revisions?: DeepDiveRevision[] } }) => {
          setRevisions(data?.revisions ?? []);
        })
        .catch((err: any) => console.error("Error fetching revisions:", err))
        .finally(() => setRevisionsLoading(false));
    }
  }, [drawerOpen, id]);

  async function updateStatus(status: "archived" | "requested") {
    if (!item) return;
    setSaving(true);
    setError(null);
    try {
      const { data } = await (api as any).PATCH("/v1/deep-dives/{id}", {
        params: { path: { id: item.id } },
        body: { status },
      });
      setItem(data ?? item);
    } catch {
      setError("Could not update this deep dive.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <PageShell
      kicker="Deep dives"
      title="Deep Dive"
      subtitle="A durable explanation request tied to one question."
      maxWidth={1200}
      action={
        <Stack direction="row" spacing={1.5}>
          <Button
            variant="outlined"
            startIcon={<HistoryIcon />}
            onClick={() => setDrawerOpen(true)}
            sx={{ textTransform: "none", borderRadius: 2 }}
          >
            Revision history
          </Button>
          <Button
            startIcon={<ArrowBackIcon />}
            onClick={() => router.push("/deep-dives")}
            sx={{ textTransform: "none", borderRadius: 2 }}
          >
            Back
          </Button>
        </Stack>
      }
    >
      {loading ? (
        <Box sx={{ display: "flex", justifyContent: "center", py: 6 }}>
          <CircularProgress size={28} />
        </Box>
      ) : error ? (
        <Alert severity="error">{error}</Alert>
      ) : !item ? (
        <Alert severity="warning">Deep dive not found.</Alert>
      ) : (
        <Stack spacing={2.5}>
          <Card variant="outlined" sx={{ borderRadius: 2 }}>
            <CardContent>
              <Stack spacing={1.5}>
                <Stack
                  direction={{ xs: "column", sm: "row" }}
                  sx={{
                    justifyContent: "space-between",
                    alignItems: { xs: "flex-start", sm: "center" },
                    gap: 1,
                  }}
                >
                  <Stack
                    direction="row"
                    spacing={1}
                    sx={{ flexWrap: "wrap", alignItems: "center" }}
                  >
                    <Chip
                      label={STATUS_LABELS[item.status] ?? item.status}
                      size="small"
                      color={
                        item.status === "published" ? "success" : "default"
                      }
                      variant={
                        item.status === "published" ? "filled" : "outlined"
                      }
                    />
                    <Chip
                      label={item.questionKind.toUpperCase()}
                      size="small"
                      variant="outlined"
                    />
                    {item.category && (
                      <Chip
                        label={`Category: ${item.category}`}
                        size="small"
                        color="primary"
                        variant="outlined"
                      />
                    )}
                  </Stack>
                  <Typography variant="caption" color="text.secondary">
                    Updated {formatDate(item.updatedAt)}
                  </Typography>
                </Stack>

                <Typography variant="h6" sx={{ fontWeight: 600 }}>
                  {item.questionPrompt}
                </Typography>
                {(item.assessmentTitle || item.course) && (
                  <Typography variant="body2" color="text.secondary">
                    {[item.course, item.assessmentTitle]
                      .filter(Boolean)
                      .join(" · ")}
                  </Typography>
                )}
                {item.questionTags.length > 0 && (
                  <Stack
                    direction="row"
                    spacing={0.5}
                    sx={{ flexWrap: "wrap" }}
                  >
                    {item.questionTags.map((tag) => (
                      <Chip
                        key={tag}
                        label={tag}
                        size="small"
                        variant="outlined"
                        sx={tagColor(tag, isDark)}
                      />
                    ))}
                  </Stack>
                )}
                {item.reason && (
                  <>
                    <Divider />
                    <Box>
                      <Typography variant="overline" color="text.secondary">
                        Request note
                      </Typography>
                      <Typography variant="body2">{item.reason}</Typography>
                    </Box>
                  </>
                )}
              </Stack>
            </CardContent>
          </Card>

          <Grid container spacing={3}>
            {/* Left Panel - 70% */}
            <Grid size={{ xs: 12, md: 8 }}>
              <Stack spacing={2}>
                <Card
                  variant="outlined"
                  sx={{ flexGrow: 1, minHeight: 400, borderRadius: 2 }}
                >
                  <CardContent>
                    {item.bodyMarkdown ? (
                      <MarkdownView content={item.bodyMarkdown} />
                    ) : (
                      <Typography variant="body2" color="text.secondary">
                        Explanation not published yet.
                      </Typography>
                    )}
                  </CardContent>
                </Card>

                <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
                  {item.status === "archived" ? (
                    <Button
                      startIcon={<UnarchiveOutlinedIcon />}
                      disabled={saving}
                      onClick={() => updateStatus("requested")}
                      sx={{ textTransform: "none", borderRadius: 2 }}
                    >
                      Restore
                    </Button>
                  ) : (
                    <Button
                      startIcon={<ArchiveOutlinedIcon />}
                      disabled={saving}
                      onClick={() => updateStatus("archived")}
                      sx={{ textTransform: "none", borderRadius: 2 }}
                    >
                      Archive
                    </Button>
                  )}
                </Box>
              </Stack>
            </Grid>

            {/* Right Panel - 30% */}
            <Grid size={{ xs: 12, md: 4 }}>
              <Box sx={{ position: "sticky", top: 24 }}>
                <Card variant="outlined" sx={{ borderRadius: 2, p: 2 }}>
                  <Stack spacing={1.5}>
                    <Stack
                      direction="row"
                      sx={{
                        justifyContent: "space-between",
                        alignItems: "center",
                      }}
                    >
                      <Typography
                        variant="subtitle2"
                        sx={{
                          fontWeight: 700,
                          fontSize: "0.85rem",
                          textTransform: "uppercase",
                          letterSpacing: "0.05em",
                        }}
                      >
                        My Study Notes
                      </Typography>
                      {noteStatus === "saving" && (
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ fontStyle: "italic" }}
                        >
                          Saving...
                        </Typography>
                      )}
                      {noteStatus === "saved" && (
                        <Typography
                          variant="caption"
                          color="success.main"
                          sx={{ fontWeight: 500 }}
                        >
                          Saved
                        </Typography>
                      )}
                    </Stack>
                    <TextField
                      multiline
                      rows={15}
                      fullWidth
                      placeholder="Take notes to summarize or synthesize the explanation..."
                      value={userNote}
                      onChange={(e) => setUserNote(e.target.value)}
                      sx={{
                        "& .MuiOutlinedInput-root": {
                          fontSize: "0.875rem",
                          borderRadius: 2,
                        },
                      }}
                    />
                    <Typography variant="caption" color="text.secondary">
                      Notes are autosaved as you type.
                    </Typography>
                  </Stack>
                </Card>
              </Box>
            </Grid>
          </Grid>
        </Stack>
      )}

      {/* Revision Drawer */}
      <Drawer
        anchor="right"
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        PaperProps={{
          sx: { width: { xs: "100%", sm: 400 }, borderRadius: "16px 0 0 16px" },
        }}
      >
        <Stack spacing={2} sx={{ p: 3, height: "100%", overflowY: "auto" }}>
          <Stack
            direction="row"
            sx={{ justifyContent: "space-between", alignItems: "center" }}
          >
            <Typography variant="h6" sx={{ fontWeight: 600 }}>
              Revision History
            </Typography>
            <Button
              size="small"
              onClick={() => setDrawerOpen(false)}
              sx={{ textTransform: "none" }}
            >
              Close
            </Button>
          </Stack>
          <Divider />
          {revisionsLoading ? (
            <Box sx={{ display: "flex", justifyContent: "center", py: 6 }}>
              <CircularProgress size={24} />
            </Box>
          ) : revisions.length === 0 ? (
            <Typography
              variant="body2"
              color="text.secondary"
              align="center"
              sx={{ py: 4 }}
            >
              No revisions recorded.
            </Typography>
          ) : (
            <Stack spacing={3}>
              {revisions.map((rev) => (
                <Box
                  key={rev.id}
                  sx={{
                    borderLeft: "2px solid",
                    borderColor: "primary.main",
                    pl: 2,
                    py: 0.5,
                  }}
                >
                  <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                    Revision #{rev.revision}
                  </Typography>
                  <Typography
                    variant="caption"
                    color="text.secondary"
                    display="block"
                  >
                    {formatDate(rev.createdAt)}
                  </Typography>
                  {rev.category && (
                    <Typography
                      variant="caption"
                      color="primary"
                      display="block"
                      sx={{ mt: 0.5 }}
                    >
                      Category: {rev.category}
                    </Typography>
                  )}
                  {rev.userNote && (
                    <Box
                      sx={{
                        mt: 1,
                        bgcolor: "action.hover",
                        p: 1,
                        borderRadius: 1,
                      }}
                    >
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ fontWeight: 600, display: "block" }}
                      >
                        Study Note:
                      </Typography>
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{
                          display: "-webkit-box",
                          WebkitLineClamp: 3,
                          WebkitBoxOrient: "vertical",
                          overflow: "hidden",
                        }}
                      >
                        {rev.userNote}
                      </Typography>
                    </Box>
                  )}
                </Box>
              ))}
            </Stack>
          )}
        </Stack>
      </Drawer>
    </PageShell>
  );
}
