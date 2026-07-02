"use client";

import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import { tagColor } from "@/lib/tagColor";
import { useColorMode } from "@/components/ThemeRegistry";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import TextField from "@mui/material/TextField";
import Select from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import Autocomplete from "@mui/material/Autocomplete";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Chip from "@mui/material/Chip";
import Pagination from "@mui/material/Pagination";
import Skeleton from "@mui/material/Skeleton";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import Divider from "@mui/material/Divider";
import IconButton from "@mui/material/IconButton";
import CircularProgress from "@mui/material/CircularProgress";
import SearchIcon from "@mui/icons-material/Search";
import CloseIcon from "@mui/icons-material/Close";
import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import { PageShell } from "@/components/PageShell";
import { HighlightedCode } from "@/components/HighlightedCode";

interface Question {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  status: string;
  tags: string[];
}

interface QuestionDetail extends Question {
  payload: Record<string, unknown>;
  explanation: string | null;
}

interface Tag {
  id: string;
  name: string;
}

const KIND_COLORS: Record<
  string,
  "primary" | "success" | "warning" | "secondary" | "info"
> = {
  mc: "primary",
  tf: "success",
  short: "warning",
  essay: "secondary",
  code: "info",
};

const KIND_LABELS: Record<string, string> = {
  all: "All",
  mc: "MC",
  tf: "T/F",
  short: "Short",
  essay: "Essay",
  code: "Code",
};

export default function QuestionsPage() {
  const { user } = useAuth();
  const { mode: colorMode } = useColorMode();
  const isDark = colorMode === "dark";
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [kind, setKind] = useState("all");
  const [tagId, setTagId] = useState<string | null>(null);
  const [status, setStatus] = useState("all");
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(25);
  const [loading, setLoading] = useState(false);
  const [questions, setQuestions] = useState<Question[]>([]);
  const [total, setTotal] = useState(0);
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTag, setSelectedTag] = useState<Tag | null>(null);
  const [previewRow, setPreviewRow] = useState<Question | null>(null);
  const [previewDetail, setPreviewDetail] = useState<QuestionDetail | null>(
    null,
  );
  const [previewLoading, setPreviewLoading] = useState(false);
  const [deepDiveRequests, setDeepDiveRequests] = useState<
    Record<string, "saving" | "requested">
  >({});

  // Debounce search input
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      setPage(1);
    }, 300);
    return () => clearTimeout(timer);
  }, [search]);

  // Fetch tags
  useEffect(() => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (api as any)
      .GET("/v1/tags")
      .then(({ data }: { data?: Tag[] }) => {
        const list = Array.isArray(data) ? data : [];
        setTags(
          [...list].sort((a, b) =>
            a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
          ),
        );
      })
      .catch(() => {});
  }, []);

  // Fetch questions
  const fetchQuestions = useCallback(async () => {
    setLoading(true);
    try {
      const query: Record<string, unknown> = {
        page,
        pageSize,
      };
      if (debouncedSearch) query.search = debouncedSearch;
      if (kind !== "all") query.kind = kind;
      if (tagId) query.tag = selectedTag?.name;
      if (status !== "all") query.status = status;

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (api as any).GET("/v1/questions", {
        params: { query },
      });

      if (data?.questions) {
        setQuestions(data.questions as Question[]);
        setTotal(data.total ?? 0);
      } else {
        setQuestions([]);
        setTotal(0);
      }
    } catch (err) {
      console.error(err);
      setQuestions([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [page, pageSize, debouncedSearch, kind, tagId, selectedTag?.name, status]);

  useEffect(() => {
    fetchQuestions();
  }, [fetchQuestions]);

  const handleFilterChange = useCallback(() => {
    setPage(1);
  }, []);

  const handleKindChange = (_: unknown, newKind: string) => {
    setKind(newKind);
    handleFilterChange();
  };

  const handleStatusChange = (newStatus: string) => {
    setStatus(newStatus);
    handleFilterChange();
  };

  const handleTagChange = (newTag: Tag | null) => {
    setSelectedTag(newTag);
    setTagId(newTag?.id ?? null);
    handleFilterChange();
  };

  const handlePageChange = (_: unknown, newPage: number) => {
    setPage(newPage);
  };

  const openPreview = useCallback(async (row: Question) => {
    setPreviewRow(row);
    setPreviewDetail(null);
    setPreviewLoading(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const { data } = await (api as any).GET("/v1/questions/{id}", {
        params: { path: { id: row.id } },
      });
      if (data) {
        setPreviewDetail({ ...row, ...data });
      }
    } catch (err) {
      console.error(err);
    } finally {
      setPreviewLoading(false);
    }
  }, []);

  const closePreview = useCallback(() => {
    setPreviewRow(null);
    setPreviewDetail(null);
  }, []);

  const requestDeepDive = async (questionId: string) => {
    if (deepDiveRequests[questionId] === "saving") return;
    setDeepDiveRequests((prev) => ({ ...prev, [questionId]: "saving" }));
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (api as any).POST("/v1/deep-dives", {
        body: {
          questionId,
          reason: "Marked from question bank",
        },
      });
      setDeepDiveRequests((prev) => ({ ...prev, [questionId]: "requested" }));
    } catch {
      setDeepDiveRequests((prev) => {
        const next = { ...prev };
        delete next[questionId];
        return next;
      });
    }
  };

  const handlePageSizeChange = (newSize: unknown) => {
    setPageSize(newSize as number);
    setPage(1);
  };

  const startRow = (page - 1) * pageSize + 1;
  const endRow = Math.min(page * pageSize, total);

  return (
    <PageShell
      kicker="Library"
      title="Question Bank"
      subtitle="Browse, search, and review all questions in your organization"
    >
      {/* Filters toolbar */}
      <Stack
        direction={{ xs: "column", sm: "row" }}
        spacing={2}
        sx={{ mb: 3, alignItems: { sm: "flex-end" } }}
      >
        <TextField
          placeholder="Search questions..."
          size="small"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          InputProps={{
            startAdornment: (
              <SearchIcon sx={{ mr: 1, color: "text.secondary" }} />
            ),
          }}
          sx={{ flex: 1, minWidth: 250 }}
        />

        <Box
          sx={{
            overflowX: "auto",
            maxWidth: "100%",
            WebkitOverflowScrolling: "touch",
            "&::-webkit-scrollbar": { display: "none" },
            msOverflowStyle: "none",
            scrollbarWidth: "none",
          }}
        >
          <ToggleButtonGroup
            value={kind}
            exclusive
            onChange={handleKindChange}
            size="small"
            sx={{
              display: "flex",
              flexWrap: "nowrap",
              minWidth: "max-content",
              "& .MuiToggleButtonGroup-grouped": { height: 40 },
            }}
          >
            {Object.entries(KIND_LABELS).map(([k, label]) => (
              <ToggleButton key={k} value={k} sx={{ px: 2 }}>
                {label}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </Box>

        <Autocomplete
          options={tags}
          getOptionLabel={(tag) => tag.name}
          value={selectedTag}
          onChange={(_, newValue) => handleTagChange(newValue)}
          size="small"
          sx={{ minWidth: 200 }}
          renderInput={(params) => <TextField {...params} placeholder="Tag" />}
        />

        <Select
          value={status}
          onChange={(e) => handleStatusChange(e.target.value)}
          size="small"
          sx={{ minWidth: 150 }}
        >
          <MenuItem value="all">All status</MenuItem>
          <MenuItem value="draft">Draft</MenuItem>
          <MenuItem value="live">Live</MenuItem>
          <MenuItem value="archived">Archived</MenuItem>
        </Select>
      </Stack>

      {/* Table */}
      <Box sx={{ overflowX: "auto", mb: 3 }}>
        <Table stickyHeader sx={{ minWidth: 560 }}>
          <TableHead>
            <TableRow>
              <TableCell sx={{ width: "80px" }}>Kind</TableCell>
              <TableCell>Prompt</TableCell>
              <TableCell sx={{ width: "200px" }}>Tags</TableCell>
              <TableCell align="right" sx={{ width: "60px" }}>
                Pts
              </TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <TableRow key={i}>
                  <TableCell>
                    <Skeleton variant="text" />
                  </TableCell>
                  <TableCell>
                    <Skeleton variant="text" />
                    <Skeleton variant="text" />
                  </TableCell>
                  <TableCell>
                    <Skeleton variant="text" />
                  </TableCell>
                  <TableCell>
                    <Skeleton variant="text" />
                  </TableCell>
                </TableRow>
              ))
            ) : questions.length === 0 ? (
              <TableRow>
                <TableCell colSpan={4} align="center" sx={{ py: 3 }}>
                  <Typography color="text.secondary">
                    No questions found.
                  </Typography>
                </TableCell>
              </TableRow>
            ) : (
              questions.map((q) => (
                <TableRow
                  key={q.id}
                  hover
                  onClick={() => openPreview(q)}
                  sx={{ cursor: "pointer" }}
                >
                  <TableCell>
                    <Chip
                      label={KIND_LABELS[q.kind] || q.kind}
                      size="small"
                      variant="outlined"
                      sx={tagColor(q.kind, isDark)}
                    />
                  </TableCell>
                  <TableCell>
                    <Typography
                      variant="body2"
                      sx={{
                        display: "-webkit-box",
                        WebkitLineClamp: 2,
                        WebkitBoxOrient: "vertical",
                        overflow: "hidden",
                      }}
                    >
                      {q.prompt}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Box
                      sx={{
                        display: "flex",
                        flexWrap: "wrap",
                        gap: 0.5,
                        alignItems: "center",
                      }}
                    >
                      {q.tags.slice(0, 3).map((tag) => (
                        <Chip
                          key={tag}
                          label={tag}
                          size="small"
                          variant="outlined"
                          sx={tagColor(tag, isDark)}
                        />
                      ))}
                      {q.tags.length > 3 && (
                        <Chip
                          label={`+${q.tags.length - 3}`}
                          size="small"
                          variant="outlined"
                        />
                      )}
                    </Box>
                  </TableCell>
                  <TableCell align="right">
                    <Typography variant="body2">{q.points}</Typography>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </Box>

      {/* Footer with pagination */}
      <Stack
        direction={{ xs: "column", sm: "row" }}
        spacing={2}
        sx={{ alignItems: { sm: "center" }, justifyContent: "space-between" }}
      >
        <Typography variant="caption" color="text.secondary">
          {total === 0 ? "—" : `${startRow}–${endRow} of ${total}`}
        </Typography>

        <Stack
          direction={{ xs: "column", sm: "row" }}
          spacing={2}
          sx={{ alignItems: "center" }}
        >
          <Pagination
            count={Math.ceil(total / pageSize) || 1}
            page={page}
            onChange={handlePageChange}
            size="small"
          />

          <Select
            value={pageSize}
            onChange={(e) => handlePageSizeChange(e.target.value)}
            size="small"
            sx={{ minWidth: 100 }}
          >
            <MenuItem value={25}>25 / page</MenuItem>
            <MenuItem value={50}>50 / page</MenuItem>
            <MenuItem value={100}>100 / page</MenuItem>
          </Select>
        </Stack>
      </Stack>

      <Dialog
        open={previewRow !== null}
        onClose={closePreview}
        maxWidth="sm"
        fullWidth
      >
        {previewRow && (
          <>
            <DialogTitle
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 1.5,
                pr: 6,
              }}
            >
              <Chip
                label={KIND_LABELS[previewRow.kind] || previewRow.kind}
                size="small"
                variant="outlined"
                sx={tagColor(previewRow.kind, isDark)}
              />
              <Typography variant="body2" color="text.secondary">
                {previewRow.points} pt{previewRow.points === 1 ? "" : "s"} ·{" "}
                {previewRow.status}
              </Typography>
              <IconButton
                onClick={closePreview}
                size="small"
                sx={{ position: "absolute", right: 8, top: 8 }}
              >
                <CloseIcon fontSize="small" />
              </IconButton>
            </DialogTitle>
            <DialogContent dividers>
              {previewLoading || !previewDetail ? (
                <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
                  <CircularProgress size={24} />
                </Box>
              ) : (
                <Stack spacing={2}>
                  <Typography sx={{ whiteSpace: "pre-wrap" }}>
                    {previewDetail.prompt}
                  </Typography>
                  <AnswerBlock detail={previewDetail} />
                  {previewDetail.explanation && (
                    <>
                      <Divider />
                      <Box>
                        <Typography variant="overline" color="text.secondary">
                          Explanation
                        </Typography>
                        <Typography
                          variant="body2"
                          sx={{ whiteSpace: "pre-wrap" }}
                        >
                          {previewDetail.explanation}
                        </Typography>
                      </Box>
                    </>
                  )}
                  {previewRow.tags.length > 0 && (
                    <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
                      {previewRow.tags.map((tag) => (
                        <Chip
                          key={tag}
                          label={tag}
                          size="small"
                          variant="outlined"
                          sx={tagColor(tag, isDark)}
                        />
                      ))}
                    </Box>
                  )}
                  <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
                    <Button
                      size="small"
                      variant={
                        deepDiveRequests[previewRow.id] === "requested"
                          ? "outlined"
                          : "contained"
                      }
                      disabled={deepDiveRequests[previewRow.id] === "saving"}
                      onClick={() => requestDeepDive(previewRow.id)}
                    >
                      {deepDiveRequests[previewRow.id] === "saving"
                        ? "Requesting"
                        : deepDiveRequests[previewRow.id] === "requested"
                          ? "Deep dive requested"
                          : "Request deep dive"}
                    </Button>
                  </Box>
                </Stack>
              )}
            </DialogContent>
          </>
        )}
      </Dialog>
    </PageShell>
  );
}

const MCQ_LABELS = ["A", "B", "C", "D", "E", "F"];

// Read-only answer view for the preview dialog — one branch per question kind.
function AnswerBlock({ detail }: { detail: QuestionDetail }) {
  const p = detail.payload;

  if (detail.kind === "mc") {
    const options = (p.options as string[]) ?? [];
    const correct = p.correct_index as number;
    return (
      <Stack spacing={0.75}>
        {options.map((opt, i) => {
          const isCorrect = i === correct;
          return (
            <Box
              key={i}
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 1.5,
                px: 1.5,
                py: 1,
                borderRadius: 1,
                border: 1,
                borderColor: isCorrect ? "success.main" : "divider",
                bgcolor: isCorrect ? "success.50" : "transparent",
              }}
            >
              <Typography
                variant="body2"
                sx={{ fontWeight: 700, color: "text.secondary" }}
              >
                {MCQ_LABELS[i] ?? i + 1}
              </Typography>
              <Typography variant="body2" sx={{ flex: 1 }}>
                {opt}
              </Typography>
              {isCorrect && (
                <CheckCircleIcon color="success" fontSize="small" />
              )}
            </Box>
          );
        })}
      </Stack>
    );
  }

  if (detail.kind === "tf") {
    const correct = p.correct as boolean;
    return (
      <Stack direction="row" spacing={1}>
        {[true, false].map((val) => (
          <Chip
            key={String(val)}
            label={val ? "True" : "False"}
            color={val === correct ? "success" : "default"}
            variant={val === correct ? "filled" : "outlined"}
            icon={val === correct ? <CheckCircleIcon /> : undefined}
          />
        ))}
      </Stack>
    );
  }

  if (detail.kind === "short") {
    const accepted = (p.accepted as string[]) ?? [];
    return (
      <Box>
        <Typography variant="overline" color="text.secondary">
          Accepted answers
        </Typography>
        <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
          {accepted.map((a, i) => (
            <Chip key={i} label={a} size="small" color="success" />
          ))}
        </Box>
      </Box>
    );
  }

  if (detail.kind === "essay") {
    const rubric = p.rubric as string | undefined;
    const minWords = p.min_words as number | undefined;
    return (
      <Box>
        <Typography variant="body2" color="text.secondary">
          Open response{minWords ? ` · min ${minWords} words` : ""}
        </Typography>
        {rubric && (
          <Typography variant="body2" sx={{ mt: 1, whiteSpace: "pre-wrap" }}>
            {rubric}
          </Typography>
        )}
      </Box>
    );
  }

  if (detail.kind === "code") {
    const language = p.language as string | undefined;
    const starter = p.starter as string | undefined;
    const tests = (p.tests as unknown[]) ?? [];
    return (
      <Box>
        <Typography variant="overline" color="text.secondary">
          Code{language ? ` · ${language}` : ""} · {tests.length} test
          {tests.length === 1 ? "" : "s"}
        </Typography>
        {starter && (
          <Box sx={{ mt: 1 }}>
            <HighlightedCode code={starter} language={language} />
          </Box>
        )}
      </Box>
    );
  }

  return null;
}
