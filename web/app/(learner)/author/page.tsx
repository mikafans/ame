"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { makeClient } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardActionArea from "@mui/material/CardActionArea";
import CardContent from "@mui/material/CardContent";
import Chip from "@mui/material/Chip";
import CircularProgress from "@mui/material/CircularProgress";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import AddOutlinedIcon from "@mui/icons-material/AddOutlined";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";

interface Quiz {
  id: string;
  title: string;
  status: string;
  course?: string;
  updated_at?: string;
  questionCount?: number;
}

export default function AuthorIndexPage() {
  const { token } = useAuth();
  const router = useRouter();
  const [drafts, setDrafts] = useState<Quiz[] | null>(null);
  const [creating, setCreating] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [confirmQuiz, setConfirmQuiz] = useState<Quiz | null>(null);

  useEffect(() => {
    if (!token) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (makeClient(token) as any)
      .GET("/v1/quizzes", { params: { query: { status: "draft" } } })
      .then(({ data }: { data?: { quizzes: Quiz[] } }) => {
        setDrafts(data?.quizzes ?? []);
      })
      .catch(() => setDrafts([]));
  }, [token]);

  async function discardDraft(id: string) {
    if (!token) return;
    setConfirmQuiz(null);
    setDeleting(id);
    setDrafts((d) => d?.filter((q) => q.id !== id) ?? d);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (makeClient(token) as any).DELETE(`/v1/quizzes/${id}`);
    } catch {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (makeClient(token) as any)
        .GET("/v1/quizzes", { params: { query: { status: "draft" } } })
        .then(({ data }: { data?: { quizzes: Quiz[] } }) =>
          setDrafts(data?.quizzes ?? []),
        )
        .catch(() => {});
    } finally {
      setDeleting(null);
    }
  }

  async function createNew() {
    if (!token) return;
    setCreating(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (makeClient(token) as any).POST("/v1/quizzes", {
        body: { title: "Untitled quiz" },
      });
      if (data?.quiz?.id) router.push(`/author/${data.quiz.id}`);
    } finally {
      setCreating(false);
    }
  }

  return (
    <Box sx={{ p: "28px 36px 56px", maxWidth: 720 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{
          fontFamily: "monospace",
          letterSpacing: 1.3,
          textTransform: "uppercase",
          display: "block",
          mb: 0.75,
        }}
      >
        Teach
      </Typography>
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          mb: 3,
        }}
      >
        <Typography variant="h5" sx={{ fontWeight: 400 }}>
          Author studio
        </Typography>
        <Button
          variant="contained"
          startIcon={<AddOutlinedIcon />}
          onClick={createNew}
          disabled={creating}
        >
          {creating ? "Creating…" : "New quiz"}
        </Button>
      </Box>

      {drafts === null ? (
        <Box sx={{ display: "flex", justifyContent: "center", pt: 6 }}>
          <CircularProgress />
        </Box>
      ) : drafts.length === 0 ? (
        <Typography color="text.secondary" variant="body2">
          No drafts yet — create a new quiz to get started.
        </Typography>
      ) : (
        <Stack spacing={1.5}>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: "monospace" }}
          >
            {drafts.length} draft{drafts.length !== 1 ? "s" : ""}
          </Typography>
          {drafts.map((q) => (
            <Card key={q.id} variant="outlined">
              <Box sx={{ display: "flex", alignItems: "center" }}>
                <CardActionArea
                  onClick={() => router.push(`/author/${q.id}`)}
                  sx={{ flex: 1 }}
                >
                  <CardContent
                    sx={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                    }}
                  >
                    <Box>
                      <Typography variant="subtitle2" sx={{ fontWeight: 500 }}>
                        {q.title}
                      </Typography>
                      {q.course && (
                        <Typography variant="caption" color="text.secondary">
                          {q.course}
                        </Typography>
                      )}
                    </Box>
                    <Stack direction="row" spacing={1} alignItems="center">
                      {q.questionCount != null && (
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ fontFamily: "monospace" }}
                        >
                          {q.questionCount} q
                        </Typography>
                      )}
                      <Chip label="draft" size="small" variant="outlined" />
                    </Stack>
                  </CardContent>
                </CardActionArea>
                <Box
                  component="button"
                  onClick={() => setConfirmQuiz(q)}
                  disabled={deleting === q.id}
                  title="Discard draft"
                  sx={{
                    px: 1.5,
                    alignSelf: "stretch",
                    border: "none",
                    borderLeft: 1,
                    borderColor: "divider",
                    bgcolor: "transparent",
                    cursor: "pointer",
                    color: "text.disabled",
                    display: "flex",
                    alignItems: "center",
                    "&:hover": { color: "error.main", bgcolor: "error.50" },
                  }}
                >
                  <DeleteOutlineIcon sx={{ fontSize: 18 }} />
                </Box>
              </Box>
            </Card>
          ))}
        </Stack>
      )}

      <Dialog
        open={confirmQuiz !== null}
        onClose={() => setConfirmQuiz(null)}
        maxWidth="xs"
        fullWidth
      >
        <DialogTitle>Discard draft?</DialogTitle>
        <DialogContent>
          <DialogContentText>
            <strong>&ldquo;{confirmQuiz?.title}&rdquo;</strong> and all its
            questions will be permanently deleted. This cannot be undone.
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmQuiz(null)}>Cancel</Button>
          <Button
            color="error"
            variant="contained"
            onClick={() => confirmQuiz && discardDraft(confirmQuiz.id)}
          >
            Discard
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
