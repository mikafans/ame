"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { formatDate } from "@/utils/format";
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import Stack from "@mui/material/Stack";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Chip from "@mui/material/Chip";
import Button from "@mui/material/Button";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import Autocomplete from "@mui/material/Autocomplete";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import Link from "next/link";
import { api } from "@/api/client";
import { errorMessage } from "@/api/errors";
import { useAuth } from "@/hooks/useAuth";
import { vibrantTagColor } from "@/lib/tagColor";
import { useColorMode } from "@/components/ThemeRegistry";
import { PageShell } from "@/components/PageShell";

import PlayArrowOutlinedIcon from "@mui/icons-material/PlayArrowOutlined";
import VisibilityOutlinedIcon from "@mui/icons-material/VisibilityOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import LaunchOutlinedIcon from "@mui/icons-material/LaunchOutlined";

export default function ExplorePage() {
  const router = useRouter();
  const { user, loading: authLoading } = useAuth();
  const { mode: colorMode } = useColorMode();
  const isDark = colorMode === "dark";
  const [searchText, setSearchText] = useState("");
  const [search, setSearch] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [mode, setMode] = useState<string>("all"); // "all" | "practice" | "graded"
  const [facets, setFacets] = useState<{
    tags: string[];
    counts: { practice: number; graded: number };
  } | null>(null);

  const [items, setItems] = useState<any[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [starting, setStarting] = useState<string | null>(null);
  const [startError, setStartError] = useState<string | null>(null);

  const fetchFacets = useCallback(async () => {
    if (authLoading || !user) return;
    try {
      const { data } = await api.GET("/v1/explore/facets");
      if (data) {
        setFacets(data as any);
      }
    } catch (err) {
      console.error("Failed to fetch facets:", err);
    }
  }, [authLoading, user]);

  useEffect(() => {
    fetchFacets();
  }, [fetchFacets]);

  const fetchItems = useCallback(
    async (cursor: string | null = null) => {
      if (authLoading || !user) return;
      setLoading(true);
      const tagsParam = selectedTags.join(",");
      try {
        const { data } = await api.GET("/v1/explore", {
          params: {
            query: {
              search: search || undefined,
              tags: tagsParam || undefined,
              mode: mode === "all" ? undefined : mode,
              kind: "active",
              limit: 50,
              after: cursor ?? undefined,
            } as any,
          },
        });
        setLoading(false);
        if (data) {
          setItems((prev) => (cursor ? [...prev, ...data.items] : data.items));
          setNextCursor(data.nextCursor ?? null);
        }
      } catch (err) {
        console.error("Failed to fetch items:", err);
        setLoading(false);
      }
    },
    [search, selectedTags, mode, authLoading, user],
  );

  const applyFilters = () => {
    setSearch(searchText);
  };

  useEffect(() => {
    if (authLoading || !user) return;
    setItems([]);
    setNextCursor(null);
    fetchItems();
  }, [search, selectedTags, mode, fetchItems, authLoading, user]);

  async function startAssessment(assessmentId: string) {
    setStarting(assessmentId);
    setStartError(null);
    try {
      const { data, error } = await api.POST("/v1/sessions", {
        body: { assessmentId },
      });
      if (error) {
        setStartError(errorMessage(error, "Failed to start assessment"));
        return;
      }
      if (data?.sessionId) {
        router.push(`/sessions/${data.sessionId}`);
      }
    } catch (err) {
      console.error(err);
      setStartError("Unexpected error — check the console");
    } finally {
      setStarting(null);
    }
  }

  const handleModeChange = (
    _event: React.MouseEvent<HTMLElement>,
    newMode: string | null,
  ) => {
    if (newMode !== null) {
      setMode(newMode);
    }
  };

  const practiceCount = facets?.counts?.practice ?? 0;
  const gradedCount = facets?.counts?.graded ?? 0;

  return (
    <PageShell
      kicker="Assessments"
      title="Explore"
      subtitle="Browse and search practice assessments and graded exams"
    >
      <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
        {startError && (
          <Alert severity="error" onClose={() => setStartError(null)}>
            {startError}
          </Alert>
        )}

        {/* Filter Panel */}
        <Paper sx={{ p: 3, borderRadius: 2 }} variant="outlined">
          <Stack spacing={2.5}>
            {/* Mode Selector */}
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                gap: 2,
                flexWrap: "wrap",
              }}
            >
              <Typography
                variant="body2"
                sx={{ fontWeight: 600, color: "text.secondary" }}
              >
                Filter by Type:
              </Typography>
              <ToggleButtonGroup
                value={mode}
                exclusive
                onChange={handleModeChange}
                size="small"
                color="primary"
              >
                <ToggleButton value="all" sx={{ textTransform: "none", px: 2 }}>
                  All
                </ToggleButton>
                <ToggleButton
                  value="practice"
                  sx={{ textTransform: "none", px: 2 }}
                >
                  Practice ({practiceCount})
                </ToggleButton>
                <ToggleButton
                  value="graded"
                  sx={{ textTransform: "none", px: 2 }}
                >
                  Exam ({gradedCount})
                </ToggleButton>
              </ToggleButtonGroup>
            </Box>

            <Stack
              direction={{ xs: "column", md: "row" }}
              spacing={2}
              alignItems="center"
            >
              <TextField
                fullWidth
                label="Search Title"
                size="small"
                value={searchText}
                onChange={(e) => setSearchText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") applyFilters();
                }}
              />

              <Autocomplete
                multiple
                freeSolo
                fullWidth
                size="small"
                options={facets?.tags ?? []}
                value={selectedTags}
                onChange={(_event, newValue) => {
                  setSelectedTags(newValue as string[]);
                }}
                renderTags={(value: readonly string[], getTagProps) =>
                  value.map((option: string, index: number) => {
                    const { key, ...tagProps } = getTagProps({ index });
                    return (
                      <Chip
                        key={key}
                        label={option}
                        size="small"
                        sx={vibrantTagColor(option, isDark)}
                        {...tagProps}
                      />
                    );
                  })
                }
                renderInput={(params) => (
                  <TextField
                    {...params}
                    label="Filter by Learning Objectives"
                    placeholder="Select tags"
                  />
                )}
              />

              <Button
                variant="contained"
                onClick={applyFilters}
                sx={{ minWidth: 140, height: 40, textTransform: "none" }}
              >
                Apply Filters
              </Button>
            </Stack>
          </Stack>
        </Paper>

        {/* Main Table */}
        <TableContainer
          component={Paper}
          variant="outlined"
          sx={{ borderRadius: 2 }}
        >
          <Table sx={{ minWidth: 720 }}>
            <TableHead>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Title</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Type</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>
                  Learning Objectives
                </TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Status</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Created At</TableCell>
                <TableCell sx={{ fontWeight: 600 }} align="right">
                  Actions
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading && items.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 8 }}>
                    <CircularProgress size={32} />
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ mt: 1.5 }}
                    >
                      Loading assessments...
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : items.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 8 }}>
                    <Typography variant="body2" color="text.secondary">
                      No assessments or exams found matching your criteria.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                items.map((item) => {
                  const isDraft = item.status === "draft";
                  const isGraded = item.mode === "graded";

                  return (
                    <TableRow key={item.id} hover>
                      <TableCell sx={{ fontWeight: 500 }}>
                        {item.title}
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={isGraded ? "Exam" : "Practice"}
                          size="small"
                          color={isGraded ? "secondary" : "primary"}
                          variant="outlined"
                          sx={{ fontWeight: 500 }}
                        />
                      </TableCell>
                      <TableCell>
                        <Stack
                          direction="row"
                          spacing={0.5}
                          sx={{ flexWrap: "wrap", gap: 0.5 }}
                        >
                          {(item.tags as string[]).map((t) => (
                            <Chip
                              key={t}
                              label={t}
                              size="small"
                              sx={vibrantTagColor(t, isDark)}
                            />
                          ))}
                        </Stack>
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={
                            item.status ? item.status.toUpperCase() : "UNKNOWN"
                          }
                          size="small"
                          variant="outlined"
                          color={
                            item.status === "active" || item.status === "live"
                              ? "success"
                              : isDraft
                                ? "warning"
                                : "default"
                          }
                        />
                      </TableCell>
                      <TableCell>{formatDate(item.createdAt)}</TableCell>
                      <TableCell align="right">
                        <Stack
                          direction="row"
                          spacing={1}
                          justifyContent="flex-end"
                        >
                          {isDraft ? (
                            <Button
                              size="small"
                              variant="outlined"
                              startIcon={<EditOutlinedIcon />}
                              component={Link}
                              href={`/author/${item.id}`}
                              sx={{ textTransform: "none" }}
                            >
                              Edit
                            </Button>
                          ) : (
                            <>
                              {isGraded ? (
                                <Button
                                  size="small"
                                  variant="contained"
                                  color="secondary"
                                  startIcon={<LaunchOutlinedIcon />}
                                  component={Link}
                                  href={`/assessments/${item.id}/preview`}
                                  sx={{ textTransform: "none" }}
                                >
                                  Open
                                </Button>
                              ) : (
                                <>
                                  <Button
                                    size="small"
                                    variant="outlined"
                                    startIcon={<VisibilityOutlinedIcon />}
                                    component={Link}
                                    href={`/assessments/${item.id}/preview`}
                                    sx={{ textTransform: "none" }}
                                  >
                                    Preview
                                  </Button>
                                  <Button
                                    size="small"
                                    variant="contained"
                                    color="primary"
                                    startIcon={<PlayArrowOutlinedIcon />}
                                    disabled={starting === item.id}
                                    onClick={() => startAssessment(item.id)}
                                    sx={{ textTransform: "none" }}
                                  >
                                    {starting === item.id
                                      ? "Starting…"
                                      : item.completed
                                        ? "Re-take"
                                        : "Start"}
                                  </Button>
                                </>
                              )}
                            </>
                          )}
                        </Stack>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
          {nextCursor && (
            <Box
              sx={{
                p: 2.5,
                textAlign: "center",
                borderTop: "1px solid",
                borderColor: "divider",
              }}
            >
              <Button
                variant="outlined"
                onClick={() => fetchItems(nextCursor)}
                disabled={loading}
                size="small"
                sx={{ textTransform: "none", px: 4 }}
              >
                {loading ? <CircularProgress size={16} sx={{ mr: 1 }} /> : null}
                {loading ? "Loading more..." : "Load More"}
              </Button>
            </Box>
          )}
        </TableContainer>
      </Box>
    </PageShell>
  );
}
