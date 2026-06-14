"use client";

import React, { useState, useEffect, useCallback } from "react";
import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import InputAdornment from "@mui/material/InputAdornment";
import SearchIcon from "@mui/icons-material/Search";
import ClearIcon from "@mui/icons-material/Clear";
import RestoreIcon from "@mui/icons-material/Restore";
import IconButton from "@mui/material/IconButton";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TablePagination from "@mui/material/TablePagination";
import Chip from "@mui/material/Chip";
import Button from "@mui/material/Button";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import VisibilityOutlinedIcon from "@mui/icons-material/VisibilityOutlined";
import Drawer from "@mui/material/Drawer";
import Divider from "@mui/material/Divider";
import Tooltip from "@mui/material/Tooltip";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import Alert from "@mui/material/Alert";
import Checkbox from "@mui/material/Checkbox";
import FormControlLabel from "@mui/material/FormControlLabel";
import CircularProgress from "@mui/material/CircularProgress";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import Stack from "@mui/material/Stack";
import Select from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import FormControl from "@mui/material/FormControl";
import { useColorMode } from "@/components/ThemeRegistry";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { formatDateTime } from "@/utils/format";
import { vibrantTagColor } from "@/lib/tagColor";

interface AssessmentEntry {
  id: string;
  title: string;
  description?: string | null;
  status: string;
  mode: string;
  createdBy: string;
  createdByEmail?: string | null;
  objectives: string[];
  createdAt: string;
  deletedAt?: string | null;
}

interface PreviewQuestion {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  orderIndex: number;
  payload: unknown;
}

interface AssessmentDetail extends AssessmentEntry {
  questions: PreviewQuestion[];
}

const KIND_LABEL: Record<string, string> = {
  mc: "MC",
  tf: "T/F",
  short: "Short",
  essay: "Essay",
  code: "Code",
};

// Admin-only answer reveal: renders the moderation-relevant slice of a question's
// payload per kind. The learner preview deliberately omits this.
function QuestionAnswer({ kind, payload }: { kind: string; payload: unknown }) {
  const p = (payload ?? {}) as Record<string, unknown>;

  if (kind === "mc") {
    const options = Array.isArray(p.options) ? (p.options as string[]) : [];
    const correct = typeof p.correct_index === "number" ? p.correct_index : -1;
    if (options.length === 0) return null;
    return (
      <Stack spacing={0.5} sx={{ mt: 1 }}>
        {options.map((opt, i) => {
          const isCorrect = i === correct;
          return (
            <Stack
              key={i}
              direction="row"
              spacing={1}
              alignItems="center"
              sx={{ color: isCorrect ? "success.main" : "text.secondary" }}
            >
              <Typography
                variant="caption"
                sx={{ fontFamily: "monospace", minWidth: 20 }}
              >
                {String.fromCharCode(65 + i)}.
              </Typography>
              <Typography
                variant="body2"
                sx={{ fontWeight: isCorrect ? 600 : 400 }}
              >
                {opt}
              </Typography>
              {isCorrect && (
                <Chip
                  label="CORRECT"
                  size="small"
                  color="success"
                  sx={{ height: 16, fontSize: 9, fontWeight: 700 }}
                />
              )}
            </Stack>
          );
        })}
      </Stack>
    );
  }

  if (kind === "tf") {
    const correct =
      p.correct === true ? "True" : p.correct === false ? "False" : "—";
    return (
      <Typography
        variant="body2"
        sx={{ mt: 1, color: "success.main", fontWeight: 600 }}
      >
        Answer: {correct}
      </Typography>
    );
  }

  if (kind === "short") {
    const accepted = Array.isArray(p.accepted) ? (p.accepted as string[]) : [];
    if (accepted.length === 0) return null;
    return (
      <Box sx={{ mt: 1, display: "flex", flexWrap: "wrap", gap: 0.5 }}>
        {accepted.map((a, i) => (
          <Chip
            key={i}
            label={a}
            size="small"
            color="success"
            variant="outlined"
          />
        ))}
      </Box>
    );
  }

  if (kind === "code") {
    const exemplar = typeof p.exemplar === "string" ? p.exemplar : null;
    if (!exemplar) return null;
    return (
      <Box
        component="pre"
        sx={{
          mt: 1,
          p: 1.5,
          borderRadius: 1.5,
          bgcolor: "action.hover",
          fontSize: 12,
          overflow: "auto",
          fontFamily: "monospace",
          m: 0,
        }}
      >
        {exemplar}
      </Box>
    );
  }

  if (kind === "essay") {
    const rubric = typeof p.rubric === "string" ? p.rubric : null;
    return (
      <Typography
        variant="body2"
        color="text.secondary"
        sx={{ mt: 1, fontStyle: "italic" }}
      >
        {rubric ? `Rubric: ${rubric}` : "Manually graded — no fixed answer."}
      </Typography>
    );
  }

  return null;
}

export default function AdminAssessmentsPage() {
  const { mode } = useColorMode();
  const isDark = mode === "dark";
  const [assessments, setAssessments] = useState<AssessmentEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);

  // Pagination & Search & Filtering
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [search, setSearch] = useState("");
  const [searchInput, setSearchInput] = useState("");
  const [selectedMode, setSelectedMode] = useState<string>("all"); // "all" | "practice" | "graded"
  const [selectedStatus, setSelectedStatus] = useState<string>("all"); // "all" | "draft" | "active" | "archived"

  // Error/Success Notification
  const [pageError, setPageError] = useState<string | null>(null);

  // Dialog State
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [restoreDialogOpen, setRestoreDialogOpen] = useState(false);
  const [selectedAssessment, setSelectedAssessment] =
    useState<AssessmentEntry | null>(null);
  const [understandDelete, setUnderstandDelete] = useState(false);
  const [dialogLoading, setDialogLoading] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  // Preview Drawer State
  const [previewOpen, setPreviewOpen] = useState(false);
  const [previewRow, setPreviewRow] = useState<AssessmentEntry | null>(null);
  const [previewDetail, setPreviewDetail] = useState<AssessmentDetail | null>(
    null,
  );
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const fetchAssessments = useCallback(async () => {
    setLoading(true);
    setPageError(null);
    try {
      const { data, error } = await api.GET("/v1/admin/assessments", {
        params: {
          query: {
            limit: rowsPerPage,
            offset: page * rowsPerPage,
            q: search || undefined,
            mode: selectedMode === "all" ? undefined : selectedMode,
            status: selectedStatus === "all" ? undefined : selectedStatus,
          },
        },
      });

      if (error) {
        setPageError(
          "Failed to load assessments: " + errorMessage(error, "Unknown error"),
        );
        return;
      }

      if (data) {
        setAssessments(data.assessments as AssessmentEntry[]);
        setTotal(data.total);
      }
    } catch (err) {
      console.error(err);
      setPageError("An unexpected error occurred while fetching assessments.");
    } finally {
      setLoading(false);
    }
  }, [page, rowsPerPage, search, selectedMode, selectedStatus]);

  useEffect(() => {
    fetchAssessments();
  }, [fetchAssessments]);

  // Reset page to 0 when filters change
  const handleModeChange = (
    _event: React.MouseEvent<HTMLElement>,
    newMode: string | null,
  ) => {
    if (newMode !== null) {
      setSelectedMode(newMode);
      setPage(0);
    }
  };

  const handleStatusChange = (status: string) => {
    setSelectedStatus(status);
    setPage(0);
  };

  // Search Handlers
  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setPage(0);
    setSearch(searchInput);
  };

  const handleClearSearch = () => {
    setSearchInput("");
    setSearch("");
    setPage(0);
  };

  const handlePageChange = (_: unknown, newPage: number) => {
    setPage(newPage);
  };

  const handleRowsPerPageChange = (
    event: React.ChangeEvent<HTMLInputElement>,
  ) => {
    setRowsPerPage(parseInt(event.target.value, 10));
    setPage(0);
  };

  const handleOpenPreview = async (assessment: AssessmentEntry) => {
    setPreviewRow(assessment);
    setPreviewDetail(null);
    setPreviewError(null);
    setPreviewOpen(true);
    setPreviewLoading(true);
    try {
      const { data, error } = await api.GET("/v1/admin/assessments/{id}", {
        params: { path: { id: assessment.id } },
      });
      if (error) {
        setPreviewError(errorMessage(error, "Failed to load assessment."));
      } else if (data) {
        setPreviewDetail(data as AssessmentDetail);
      }
    } catch (err) {
      console.error(err);
      setPreviewError(
        "An unexpected error occurred while loading the preview.",
      );
    } finally {
      setPreviewLoading(false);
    }
  };

  const handleOpenDelete = (assessment: AssessmentEntry) => {
    setSelectedAssessment(assessment);
    setUnderstandDelete(false);
    setDialogError(null);
    setDeleteDialogOpen(true);
  };

  const handleDeleteSubmit = async () => {
    if (!selectedAssessment) return;
    if (!understandDelete) {
      setDialogError(
        "You must check the confirmation box to delete this assessment.",
      );
      return;
    }

    setDialogLoading(true);
    setDialogError(null);

    try {
      const { error } = await api.DELETE("/v1/admin/assessments/{id}", {
        params: { path: { id: selectedAssessment.id } },
      });

      if (error) {
        setDialogError(errorMessage(error, "Failed to delete assessment."));
      } else {
        setDeleteDialogOpen(false);
        setPreviewOpen(false);
        fetchAssessments();
      }
    } catch (err) {
      setDialogError("Network or unexpected server error.");
    } finally {
      setDialogLoading(false);
    }
  };

  const handleOpenRestore = (assessment: AssessmentEntry) => {
    setSelectedAssessment(assessment);
    setDialogError(null);
    setRestoreDialogOpen(true);
  };

  const handleRestoreSubmit = async () => {
    if (!selectedAssessment) return;

    setDialogLoading(true);
    setDialogError(null);

    try {
      const { error } = await api.POST("/v1/admin/assessments/{id}/restore", {
        params: { path: { id: selectedAssessment.id } },
      });

      if (error) {
        setDialogError(errorMessage(error, "Failed to restore assessment."));
      } else {
        setRestoreDialogOpen(false);
        setPreviewOpen(false);
        fetchAssessments();
      }
    } catch (err) {
      setDialogError("Network or unexpected server error.");
    } finally {
      setDialogLoading(false);
    }
  };

  return (
    <Container maxWidth={false} sx={{ py: 6, px: { xs: 3, sm: 5 } }}>
      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" fontWeight="bold" gutterBottom>
          Manage Assessments
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Monitor all practice and graded assessments on the platform. Delete
          problematic content if necessary.
        </Typography>
      </Box>

      {/* Main Panel */}
      <Paper
        sx={{
          borderRadius: 3,
          overflow: "hidden",
          border: "1px solid",
          borderColor: "divider",
          boxShadow: "0 4px 20px rgba(0,0,0,0.02)",
        }}
      >
        {/* Top Control Bar with Glassmorphic Filters */}
        <Box
          component="form"
          onSubmit={handleSearchSubmit}
          sx={{
            p: 3,
            borderBottom: "1px solid",
            borderColor: "divider",
            display: "flex",
            flexDirection: "column",
            gap: 2.5,
            bgcolor: (theme) =>
              theme.palette.mode === "dark"
                ? "rgba(255,255,255,0.015)"
                : "rgba(0,0,0,0.005)",
          }}
        >
          <Stack
            direction={{ xs: "column", sm: "row" }}
            spacing={2}
            alignItems="center"
          >
            <TextField
              placeholder="Search title, description, or creator..."
              size="small"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              fullWidth
              sx={{ flexGrow: 1 }}
              slotProps={{
                input: {
                  startAdornment: (
                    <InputAdornment position="start">
                      <SearchIcon color="action" fontSize="small" />
                    </InputAdornment>
                  ),
                  endAdornment: searchInput ? (
                    <InputAdornment position="end">
                      <IconButton size="small" onClick={handleClearSearch}>
                        <ClearIcon fontSize="small" />
                      </IconButton>
                    </InputAdornment>
                  ) : null,
                },
              }}
            />
            <Button
              variant="contained"
              type="submit"
              sx={{
                minWidth: 100,
                textTransform: "none",
                borderRadius: 2,
                height: 40,
              }}
            >
              Search
            </Button>
          </Stack>

          <Stack
            direction={{ xs: "column", md: "row" }}
            spacing={3}
            alignItems={{ xs: "stretch", md: "center" }}
            justifyContent="space-between"
          >
            {/* Mode Toggle */}
            <Stack
              direction={{ xs: "column", sm: "row" }}
              spacing={2}
              alignItems={{ xs: "flex-start", sm: "center" }}
            >
              <Typography
                variant="body2"
                sx={{ fontWeight: 600, color: "text.secondary" }}
              >
                Type:
              </Typography>
              <ToggleButtonGroup
                value={selectedMode}
                exclusive
                onChange={handleModeChange}
                size="small"
                color="primary"
              >
                <ToggleButton
                  value="all"
                  sx={{ textTransform: "none", px: 2.5 }}
                >
                  All Types
                </ToggleButton>
                <ToggleButton
                  value="practice"
                  sx={{ textTransform: "none", px: 2.5 }}
                >
                  Practice
                </ToggleButton>
                <ToggleButton
                  value="graded"
                  sx={{ textTransform: "none", px: 2.5 }}
                >
                  Exam
                </ToggleButton>
              </ToggleButtonGroup>
            </Stack>

            {/* Status Selector */}
            <Stack
              direction={{ xs: "column", sm: "row" }}
              spacing={2}
              alignItems={{ xs: "flex-start", sm: "center" }}
            >
              <Typography
                variant="body2"
                sx={{ fontWeight: 600, color: "text.secondary" }}
              >
                Status:
              </Typography>
              <FormControl size="small" sx={{ minWidth: 150 }}>
                <Select
                  value={selectedStatus}
                  onChange={(e) => handleStatusChange(e.target.value as string)}
                  displayEmpty
                >
                  <MenuItem value="all">All Statuses</MenuItem>
                  <MenuItem value="draft">Draft</MenuItem>
                  <MenuItem value="active">Active</MenuItem>
                  <MenuItem value="archived">Archived</MenuItem>
                </Select>
              </FormControl>
            </Stack>
          </Stack>
        </Box>

        {pageError && (
          <Box sx={{ p: 2 }}>
            <Alert severity="error">{pageError}</Alert>
          </Box>
        )}

        {/* Table */}
        <TableContainer>
          <Table sx={{ minWidth: 800 }}>
            <TableHead>
              <TableRow
                sx={{
                  bgcolor: (theme) =>
                    theme.palette.mode === "dark"
                      ? "rgba(255,255,255,0.01)"
                      : "rgba(0,0,0,0.01)",
                }}
              >
                <TableCell>Title</TableCell>
                <TableCell>Type</TableCell>
                <TableCell>Learning Objectives</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Creator</TableCell>
                <TableCell>Created At</TableCell>
                <TableCell align="right" sx={{ pr: 3 }}>
                  Actions
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={7} align="center" sx={{ py: 8 }}>
                    <CircularProgress size={32} sx={{ mb: 1 }} />
                    <Typography variant="body2" color="text.secondary">
                      Loading assessments...
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : assessments.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={7} align="center" sx={{ py: 8 }}>
                    <Typography
                      variant="body1"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      No assessments found matching the criteria.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                assessments.map((row) => (
                  <TableRow
                    key={row.id}
                    hover
                    sx={{ "&:last-child td, &:last-child th": { border: 0 } }}
                  >
                    <TableCell>
                      <Box sx={{ minWidth: 200, maxWidth: 300 }}>
                        <Typography variant="body2" fontWeight={600} noWrap>
                          {row.title}
                        </Typography>
                        {row.description && (
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            noWrap
                            sx={{ display: "block", mt: 0.5 }}
                          >
                            {row.description}
                          </Typography>
                        )}
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Chip
                        label={row.mode.toUpperCase()}
                        size="small"
                        color={row.mode === "graded" ? "error" : "primary"}
                        variant="outlined"
                        sx={{
                          fontWeight: 600,
                          fontSize: 10,
                          borderRadius: 1.5,
                        }}
                      />
                    </TableCell>
                    <TableCell>
                      <Box
                        sx={{
                          display: "flex",
                          flexWrap: "wrap",
                          gap: 0.5,
                          maxWidth: 280,
                        }}
                      >
                        {row.objectives && row.objectives.length > 0 ? (
                          row.objectives.map((tag) => (
                            <Chip
                              key={tag}
                              label={tag}
                              size="small"
                              sx={{
                                ...vibrantTagColor(tag, isDark),
                                fontSize: 9,
                                height: 18,
                              }}
                            />
                          ))
                        ) : (
                          <Typography variant="caption" color="text.secondary">
                            None
                          </Typography>
                        )}
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Stack direction="row" spacing={1} alignItems="center">
                        <Chip
                          label={row.status.toUpperCase()}
                          size="small"
                          color={
                            row.status === "active"
                              ? "success"
                              : row.status === "draft"
                                ? "default"
                                : "warning"
                          }
                          sx={{
                            fontWeight: 600,
                            fontSize: 10,
                            borderRadius: 1.5,
                          }}
                        />
                        {row.deletedAt && (
                          <Chip
                            label="DELETED"
                            size="small"
                            color="error"
                            sx={{
                              fontWeight: 600,
                              fontSize: 10,
                              borderRadius: 1.5,
                            }}
                          />
                        )}
                      </Stack>
                    </TableCell>
                    <TableCell>
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        noWrap
                        sx={{ maxWidth: 180 }}
                      >
                        {row.createdByEmail || "Unknown Creator"}
                      </Typography>
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{
                          display: "block",
                          fontSize: 10,
                          fontFamily: "monospace",
                        }}
                      >
                        ID: {row.createdBy.substring(0, 8)}...
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ fontSize: 13, minWidth: 120 }}
                      >
                        {formatDateTime(row.createdAt)}
                      </Typography>
                    </TableCell>
                    <TableCell align="right" sx={{ pr: 2 }}>
                      <Tooltip title="Preview content">
                        <IconButton
                          size="small"
                          onClick={() => handleOpenPreview(row)}
                          color="primary"
                          aria-label="Preview content"
                        >
                          <VisibilityOutlinedIcon fontSize="small" />
                        </IconButton>
                      </Tooltip>
                      {row.deletedAt ? (
                        <IconButton
                          size="small"
                          onClick={() => handleOpenRestore(row)}
                          color="success"
                        >
                          <RestoreIcon fontSize="small" />
                        </IconButton>
                      ) : (
                        <IconButton
                          size="small"
                          onClick={() => handleOpenDelete(row)}
                          color="error"
                        >
                          <DeleteOutlineIcon fontSize="small" />
                        </IconButton>
                      )}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>

        <TablePagination
          rowsPerPageOptions={[5, 10, 25, 50]}
          component="div"
          count={total}
          rowsPerPage={rowsPerPage}
          page={page}
          onPageChange={handlePageChange}
          onRowsPerPageChange={handleRowsPerPageChange}
          sx={{ borderTop: "1px solid", borderColor: "divider" }}
        />
      </Paper>

      {/* Preview Drawer */}
      <Drawer
        anchor="right"
        open={previewOpen}
        onClose={() => setPreviewOpen(false)}
        slotProps={{
          paper: {
            sx: { width: { xs: "100%", sm: 540 }, maxWidth: "100%" },
          },
        }}
      >
        <Box
          sx={{
            display: "flex",
            flexDirection: "column",
            height: "100%",
          }}
        >
          {/* Drawer header */}
          <Box sx={{ p: 3, borderBottom: "1px solid", borderColor: "divider" }}>
            <Typography variant="h6" fontWeight="bold" gutterBottom>
              {previewRow?.title ?? "Assessment"}
            </Typography>
            <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
              {previewRow && (
                <Chip
                  label={previewRow.mode.toUpperCase()}
                  size="small"
                  color={previewRow.mode === "graded" ? "error" : "primary"}
                  variant="outlined"
                  sx={{ fontWeight: 600, fontSize: 10 }}
                />
              )}
              {previewRow && (
                <Chip
                  label={previewRow.status.toUpperCase()}
                  size="small"
                  color={
                    previewRow.status === "active"
                      ? "success"
                      : previewRow.status === "draft"
                        ? "default"
                        : "warning"
                  }
                  sx={{ fontWeight: 600, fontSize: 10 }}
                />
              )}
              {previewRow?.deletedAt && (
                <Chip
                  label="DELETED"
                  size="small"
                  color="error"
                  sx={{ fontWeight: 600, fontSize: 10 }}
                />
              )}
            </Stack>
            {previewDetail && (
              <Typography
                variant="caption"
                color="text.secondary"
                sx={{ display: "block", mt: 1.5, fontFamily: "monospace" }}
              >
                {previewDetail.questions.length} question
                {previewDetail.questions.length !== 1 ? "s" : ""} ·{" "}
                {previewDetail.questions.reduce((s, q) => s + q.points, 0)} pts
                · {previewRow?.createdByEmail || "Unknown creator"}
              </Typography>
            )}
          </Box>

          {/* Drawer body */}
          <Box sx={{ flex: 1, overflowY: "auto", p: 3 }}>
            {previewLoading ? (
              <Box sx={{ textAlign: "center", py: 8 }}>
                <CircularProgress size={28} />
              </Box>
            ) : previewError ? (
              <Alert severity="error">{previewError}</Alert>
            ) : previewDetail ? (
              <Stack spacing={2}>
                {previewDetail.description && (
                  <Typography variant="body2" color="text.secondary">
                    {previewDetail.description}
                  </Typography>
                )}
                {previewDetail.objectives &&
                  previewDetail.objectives.length > 0 && (
                    <Box>
                      <Typography
                        variant="caption"
                        color="text.secondary"
                        sx={{ fontWeight: 600, textTransform: "uppercase" }}
                      >
                        Objectives
                      </Typography>
                      <Box
                        sx={{
                          display: "flex",
                          flexWrap: "wrap",
                          gap: 0.5,
                          mt: 0.5,
                        }}
                      >
                        {previewDetail.objectives.map((obj) => (
                          <Chip
                            key={obj}
                            label={obj}
                            size="small"
                            sx={{
                              ...vibrantTagColor(obj, isDark),
                              fontSize: 10,
                            }}
                          />
                        ))}
                      </Box>
                    </Box>
                  )}
                <Divider />
                {previewDetail.questions.length === 0 ? (
                  <Typography variant="body2" color="text.secondary">
                    This assessment has no questions.
                  </Typography>
                ) : (
                  [...previewDetail.questions]
                    .sort((a, b) => a.orderIndex - b.orderIndex)
                    .map((q, i) => (
                      <Box key={q.id}>
                        <Stack
                          direction="row"
                          spacing={1}
                          alignItems="baseline"
                        >
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{ fontFamily: "monospace" }}
                          >
                            {i + 1}
                          </Typography>
                          <Chip
                            label={KIND_LABEL[q.kind] || q.kind}
                            size="small"
                            variant="outlined"
                            sx={{ height: 18, fontSize: 9, fontWeight: 600 }}
                          />
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            sx={{ ml: "auto", fontFamily: "monospace" }}
                          >
                            {q.points} pt{q.points !== 1 ? "s" : ""}
                          </Typography>
                        </Stack>
                        <Typography variant="body2" sx={{ mt: 0.5 }}>
                          {q.prompt}
                        </Typography>
                        <QuestionAnswer kind={q.kind} payload={q.payload} />
                        {i < previewDetail.questions.length - 1 && (
                          <Divider sx={{ mt: 2 }} />
                        )}
                      </Box>
                    ))
                )}
              </Stack>
            ) : null}
          </Box>

          {/* Drawer footer — review → decide */}
          <Box
            sx={{
              p: 2,
              borderTop: "1px solid",
              borderColor: "divider",
              display: "flex",
              justifyContent: "flex-end",
              gap: 1,
            }}
          >
            <Button onClick={() => setPreviewOpen(false)} color="inherit">
              Close
            </Button>
            {previewRow?.deletedAt ? (
              <Button
                variant="contained"
                color="success"
                startIcon={<RestoreIcon />}
                onClick={() => previewRow && handleOpenRestore(previewRow)}
              >
                Restore
              </Button>
            ) : (
              <Button
                variant="contained"
                color="error"
                startIcon={<DeleteOutlineIcon />}
                onClick={() => previewRow && handleOpenDelete(previewRow)}
                disabled={!previewRow}
              >
                Delete
              </Button>
            )}
          </Box>
        </Box>
      </Drawer>

      {/* Delete Dialog */}
      <Dialog
        open={deleteDialogOpen}
        onClose={() => !dialogLoading && setDeleteDialogOpen(false)}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">Delete Assessment</DialogTitle>
        <DialogContent dividers>
          {dialogError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {dialogError}
            </Alert>
          )}
          <Alert severity="info" sx={{ mb: 2.5 }}>
            This hides the assessment from its creator. It can be restored later
            from this page.
          </Alert>
          <DialogContentText sx={{ mb: 3 }}>
            Are you sure you want to delete the assessment{" "}
            <strong>"{selectedAssessment?.title}"</strong>?
          </DialogContentText>
          <FormControlLabel
            control={
              <Checkbox
                size="small"
                checked={understandDelete}
                onChange={(e) => setUnderstandDelete(e.target.checked)}
              />
            }
            label={
              <Typography variant="body2" color="text.secondary">
                I understand this removes the assessment from the creator's
                view.
              </Typography>
            }
          />
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={() => setDeleteDialogOpen(false)}
            color="inherit"
            disabled={dialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleDeleteSubmit}
            variant="contained"
            color="error"
            disabled={dialogLoading || !understandDelete}
          >
            {dialogLoading ? (
              <CircularProgress size={20} />
            ) : (
              "Delete Assessment"
            )}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Restore Dialog */}
      <Dialog
        open={restoreDialogOpen}
        onClose={() => !dialogLoading && setRestoreDialogOpen(false)}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">Restore Assessment</DialogTitle>
        <DialogContent dividers>
          {dialogError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {dialogError}
            </Alert>
          )}
          <DialogContentText sx={{ mb: 3 }}>
            Restore the assessment{" "}
            <strong>"{selectedAssessment?.title}"</strong>? It will become
            visible to its creator again.
          </DialogContentText>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={() => setRestoreDialogOpen(false)}
            color="inherit"
            disabled={dialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleRestoreSubmit}
            variant="contained"
            color="success"
            disabled={dialogLoading}
          >
            {dialogLoading ? (
              <CircularProgress size={20} />
            ) : (
              "Restore Assessment"
            )}
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
}
