"use client";

import { useEffect, useState } from "react";
import { formatDateTime } from "@/utils/format";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TablePagination from "@mui/material/TablePagination";
import Paper from "@mui/material/Paper";
import Chip from "@mui/material/Chip";
import Skeleton from "@mui/material/Skeleton";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import CalendarMonthOutlinedIcon from "@mui/icons-material/CalendarMonthOutlined";
import OpenInNewOutlinedIcon from "@mui/icons-material/OpenInNewOutlined";
import HistoryOutlinedIcon from "@mui/icons-material/HistoryOutlined";
import { tagColor } from "@/lib/tagColor";

interface SessionSummary {
  id: string;
  kind: "quiz" | "exam" | "practice";
  status: string;
  assessmentId?: string | null;
  assessmentTitle?: string | null;
  pointsAwarded?: number | null;
  maxPoints?: number | null;
  startedAt: string;
  finishedAt?: string | null;
  attemptNumber: number;
  totalAttempts: number;
}

export default function ResultsHistoryPage() {
  const router = useRouter();
  const [loading, setLoading] = useState(true);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [total, setTotal] = useState(0);

  const paginatedSessions = sessions;

  useEffect(() => {
    setLoading(true);
    api
      .GET(
        "/v1/sessions" as never,
        {
          params: {
            query: {
              limit: rowsPerPage,
              offset: page * rowsPerPage,
            },
          },
        } as never,
      )
      .then(({ data, error }) => {
        if (error) {
          if ((error as any).status !== 401) {
            setError("Failed to load attempt history.");
          }
        } else if (data) {
          setSessions((data as any).sessions || []);
          setTotal((data as any).total || 0);
        }
      })
      .catch((err) => {
        console.error(err);
        setError("Could not establish connection to the API.");
      })
      .finally(() => setLoading(false));
  }, [page, rowsPerPage]);

  if (loading) {
    return (
      <Box
        sx={{
          minHeight: "80vh",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <CircularProgress size={36} />
      </Box>
    );
  }

  return (
    <Box sx={{ p: "32px 40px", maxWidth: 1100 }}>
      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" sx={{ fontWeight: 500, mb: 1 }}>
          Attempt History
        </Typography>
        <Typography variant="body2" color="text.secondary">
          Review your recent practice sessions, quiz attempts, and exam results.
        </Typography>
      </Box>

      {error && (
        <Alert severity="error" sx={{ mb: 3 }}>
          {error}
        </Alert>
      )}

      {sessions.length === 0 ? (
        <Paper
          variant="outlined"
          sx={{
            py: 8,
            px: 4,
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 2,
            bgcolor: "transparent",
            borderColor: "divider",
          }}
        >
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              width: 56,
              height: 56,
              borderRadius: "50%",
              bgcolor: "action.hover",
              color: "text.secondary",
            }}
          >
            <HistoryOutlinedIcon sx={{ fontSize: 28 }} />
          </Box>
          <Typography variant="h6" sx={{ fontWeight: 500 }}>
            No recent attempts found
          </Typography>
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ maxWidth: 400, mb: 1 }}
          >
            It looks like you haven't taken any quizzes or exams yet. Once you
            complete an assessment, your scores and logs will show up here.
          </Typography>
          <Button variant="contained" onClick={() => router.push("/explore")}>
            Go to Explore
          </Button>
        </Paper>
      ) : (
        <Box sx={{ overflowX: "auto", mb: 3 }}>
          <TableContainer>
            <Table stickyHeader>
              <TableHead>
                <TableRow>
                  <TableCell>Assessment</TableCell>
                  <TableCell>Mode</TableCell>
                  <TableCell>Attempt</TableCell>
                  <TableCell align="right">Score</TableCell>
                  <TableCell align="right">Completed</TableCell>
                  <TableCell align="center">Action</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {loading
                  ? Array.from({ length: 5 }).map((_, i) => (
                      <TableRow key={i}>
                        <TableCell>
                          <Skeleton variant="text" />
                        </TableCell>
                        <TableCell>
                          <Skeleton variant="text" />
                        </TableCell>
                        <TableCell>
                          <Skeleton variant="text" />
                        </TableCell>
                        <TableCell align="right">
                          <Skeleton variant="text" />
                        </TableCell>
                        <TableCell align="right">
                          <Skeleton variant="text" />
                        </TableCell>
                        <TableCell align="center">
                          <Skeleton variant="text" />
                        </TableCell>
                      </TableRow>
                    ))
                  : paginatedSessions.map((s) => {
                      const percentage =
                        s.pointsAwarded != null && s.maxPoints
                          ? (s.pointsAwarded / s.maxPoints) * 100
                          : null;

                      const scoreColor =
                        percentage == null
                          ? "default"
                          : percentage >= 80
                            ? "success"
                            : percentage >= 60
                              ? "warning"
                              : "error";

                      const dateStr = s.finishedAt
                        ? formatDateTime(s.finishedAt)
                        : "—";

                      const modeLabel = s.kind === "exam" ? "Exam" : "Practice";
                      const modeColors = tagColor(modeLabel);

                      return (
                        <TableRow key={s.id} hover>
                          <TableCell>
                            <Typography
                              variant="body2"
                              sx={{ fontWeight: 500 }}
                            >
                              {s.assessmentTitle || "Untitled Assessment"}
                            </Typography>
                          </TableCell>
                          <TableCell>
                            <Chip
                              label={modeLabel}
                              size="small"
                              variant="outlined"
                              sx={modeColors}
                            />
                          </TableCell>
                          <TableCell>
                            <Typography
                              variant="caption"
                              color="text.secondary"
                            >
                              {s.attemptNumber} of {s.totalAttempts}
                            </Typography>
                          </TableCell>
                          <TableCell align="right">
                            {s.pointsAwarded != null && s.maxPoints != null ? (
                              <Stack
                                direction="row"
                                spacing={1}
                                sx={{
                                  justifyContent: "flex-end",
                                  alignItems: "center",
                                }}
                              >
                                <Typography
                                  variant="body2"
                                  sx={{ fontWeight: 500 }}
                                >
                                  {s.pointsAwarded} / {s.maxPoints} pts
                                </Typography>
                                <Chip
                                  label={`${Math.round(percentage || 0)}%`}
                                  size="small"
                                  color={scoreColor}
                                  variant="outlined"
                                  sx={{ height: 20, fontSize: "0.75rem" }}
                                />
                              </Stack>
                            ) : (
                              <Typography
                                variant="body2"
                                color="text.secondary"
                              >
                                {s.pointsAwarded != null
                                  ? `${s.pointsAwarded} pts`
                                  : "—"}
                              </Typography>
                            )}
                          </TableCell>
                          <TableCell align="right">
                            <Stack
                              direction="row"
                              spacing={0.75}
                              sx={{
                                justifyContent: "flex-end",
                                alignItems: "center",
                                color: "text.secondary",
                              }}
                            >
                              <CalendarMonthOutlinedIcon
                                sx={{ fontSize: 16 }}
                              />
                              <Typography variant="caption">
                                {dateStr}
                              </Typography>
                            </Stack>
                          </TableCell>
                          <TableCell align="center">
                            <Button
                              size="small"
                              variant="outlined"
                              endIcon={
                                <OpenInNewOutlinedIcon sx={{ fontSize: 14 }} />
                              }
                              onClick={() =>
                                router.push(`/sessions/${s.id}/results`)
                              }
                            >
                              Review
                            </Button>
                          </TableCell>
                        </TableRow>
                      );
                    })}
              </TableBody>
            </Table>
          </TableContainer>
          <TablePagination
            rowsPerPageOptions={[5, 10, 25, 50]}
            component="div"
            count={total}
            rowsPerPage={rowsPerPage}
            page={page}
            onPageChange={(_, newPage) => setPage(newPage)}
            onRowsPerPageChange={(event) => {
              setRowsPerPage(parseInt(event.target.value, 10));
              setPage(0);
            }}
            sx={{ borderTop: 1, borderColor: "divider" }}
          />
        </Box>
      )}
    </Box>
  );
}
