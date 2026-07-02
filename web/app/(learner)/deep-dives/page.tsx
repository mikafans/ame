"use client";

import { useEffect, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { api } from "@/api/client";
import { PageShell } from "@/components/PageShell";
import { formatDate } from "@/utils/format";
import { tagColor } from "@/lib/tagColor";
import { useColorMode } from "@/components/ThemeRegistry";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import CardActionArea from "@mui/material/CardActionArea";
import Chip from "@mui/material/Chip";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import Grid from "@mui/material/Grid";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import InputAdornment from "@mui/material/InputAdornment";
import List from "@mui/material/List";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemText from "@mui/material/ListItemText";
import ListItemIcon from "@mui/material/ListItemIcon";
import Divider from "@mui/material/Divider";
import IconButton from "@mui/material/IconButton";

// Icons
import TipsAndUpdatesOutlinedIcon from "@mui/icons-material/TipsAndUpdatesOutlined";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import SearchIcon from "@mui/icons-material/Search";
import DownloadIcon from "@mui/icons-material/Download";
import ClearIcon from "@mui/icons-material/Clear";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";

interface DeepDive {
  id: string;
  questionId: string;
  status: string;
  reason?: string | null;
  bodyMarkdown?: string | null;
  createdAt: string;
  updatedAt: string;
  publishedAt?: string | null;
  category?: string | null;
  questionPrompt: string;
  questionKind: string;
  questionTags: string[];
  assessmentTitle?: string | null;
  course?: string | null;
}

const STATUSES = [
  { value: "active", label: "Active" },
  { value: "published", label: "Published" },
  { value: "requested", label: "Requested" },
  { value: "drafting", label: "Drafting" },
  { value: "needs_revision", label: "Needs revision" },
  { value: "archived", label: "Archived" },
];

const STATUS_LABELS: Record<string, string> = {
  requested: "Requested",
  drafting: "Drafting",
  published: "Published",
  needs_revision: "Needs revision",
  archived: "Archived",
};

export default function DeepDivesPage() {
  const router = useRouter();
  const { mode } = useColorMode();
  const isDark = mode === "dark";

  // Filter States
  const [status, setStatus] = useState("active");
  const [category, setCategory] = useState<string | null>(null);
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [searchVal, setSearchVal] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");

  const [items, setItems] = useState<DeepDive[]>([]);
  const [allDives, setAllDives] = useState<DeepDive[]>([]);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Get API Base URL for direct fetch downloads
  const getApiUrl = () => {
    if (typeof window === "undefined") {
      return process.env.API_URL ?? "http://localhost:28080";
    }
    return (
      process.env.NEXT_PUBLIC_API_URL ??
      `http://${window.location.hostname}:28080`
    );
  };

  // Debounce search val
  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedSearch(searchVal);
    }, 350);
    return () => clearTimeout(handler);
  }, [searchVal]);

  // Fetch all deep dives (without status or other filters) to compile categories and tags
  useEffect(() => {
    (api as any)
      .GET("/v1/deep-dives", { params: { query: {} } })
      .then(({ data }: { data?: { deepDives?: DeepDive[] } }) => {
        setAllDives(data?.deepDives ?? []);
      })
      .catch((err: any) =>
        console.error("Error loading categories/tags:", err),
      );
  }, []);

  // Fetch filtered items
  useEffect(() => {
    let ignore = false;
    setLoading(true);
    setError(null);
    const query: any = {};
    if (status !== "active") {
      query.status = status;
    }
    if (category) {
      query.category = category;
    }
    if (debouncedSearch) {
      query.search = debouncedSearch;
    }

    (api as any)
      .GET("/v1/deep-dives", { params: { query } })
      .then(({ data }: { data?: { deepDives?: DeepDive[] } }) => {
        if (!ignore) setItems(data?.deepDives ?? []);
      })
      .catch(() => {
        if (!ignore) setError("Could not load deep dives.");
      })
      .finally(() => {
        if (!ignore) setLoading(false);
      });

    return () => {
      ignore = true;
    };
  }, [status, category, debouncedSearch]);

  // Extract unique categories and tags from all non-archived deep dives
  const categories = useMemo(() => {
    const cats = new Set<string>();
    allDives.forEach((d) => {
      if (d.category) cats.add(d.category);
    });
    return Array.from(cats).sort();
  }, [allDives]);

  const tags = useMemo(() => {
    const tg = new Set<string>();
    allDives.forEach((d) => {
      d.questionTags?.forEach((t) => tg.add(t));
    });
    return Array.from(tg).sort();
  }, [allDives]);

  // Apply client-side tag filtering
  const displayedItems = useMemo(() => {
    if (!selectedTag) return items;
    return items.filter((item) => item.questionTags?.includes(selectedTag));
  }, [items, selectedTag]);

  const countLabel = useMemo(() => {
    if (loading) return "Loading";
    return `${displayedItems.length} ${displayedItems.length === 1 ? "item" : "items"}`;
  }, [displayedItems.length, loading]);

  const handleExport = async () => {
    setExporting(true);
    try {
      const res = await fetch(`${getApiUrl()}/v1/deep-dives/export`, {
        credentials: "include",
      });
      if (!res.ok) throw new Error("Export failed");
      const blob = await res.blob();
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "ame-deep-dives-export.zip";
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.URL.revokeObjectURL(url);
    } catch (err) {
      console.error("Export KB error:", err);
      setError("Failed to export Obsidian KB. Please try again.");
    } finally {
      setExporting(false);
    }
  };

  return (
    <PageShell
      kicker="Deep dives"
      title="Knowledge Base & Study Notes"
      subtitle="Track hard questions, read agent explanations, and organize your personal study space."
      maxWidth={1200}
    >
      <Grid container spacing={3}>
        {/* Left pane: Sidebar filters */}
        <Grid size={{ xs: 12, md: 3.5 }}>
          <Paper
            variant="outlined"
            sx={{ p: 2.5, borderRadius: 2, position: "sticky", top: 24 }}
          >
            <Stack spacing={3}>
              <Button
                variant="contained"
                color="primary"
                startIcon={
                  exporting ? (
                    <CircularProgress size={20} color="inherit" />
                  ) : (
                    <DownloadIcon />
                  )
                }
                onClick={handleExport}
                disabled={exporting}
                fullWidth
                sx={{
                  py: 1,
                  borderRadius: 2,
                  textTransform: "none",
                  fontWeight: 600,
                  boxShadow: "none",
                }}
              >
                {exporting ? "Exporting..." : "Export KB (Obsidian)"}
              </Button>

              <Divider />

              <Box>
                <Typography
                  variant="subtitle2"
                  color="text.secondary"
                  sx={{
                    mb: 1,
                    fontWeight: 700,
                    fontSize: "0.75rem",
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                  }}
                >
                  Categories
                </Typography>
                <List dense disablePadding>
                  <ListItemButton
                    selected={category === null}
                    onClick={() => setCategory(null)}
                    sx={{ borderRadius: 1.5, mb: 0.5 }}
                  >
                    <ListItemIcon sx={{ minWidth: 32 }}>
                      <FolderOpenIcon fontSize="small" />
                    </ListItemIcon>
                    <ListItemText
                      primary="All Categories"
                      primaryTypographyProps={{
                        fontSize: "0.875rem",
                        fontWeight: category === null ? 600 : 400,
                      }}
                    />
                  </ListItemButton>
                  {categories.map((cat) => (
                    <ListItemButton
                      key={cat}
                      selected={category === cat}
                      onClick={() => setCategory(cat)}
                      sx={{ borderRadius: 1.5, mb: 0.5 }}
                    >
                      <ListItemIcon sx={{ minWidth: 32 }}>
                        <FolderOpenIcon fontSize="small" />
                      </ListItemIcon>
                      <ListItemText
                        primary={cat}
                        primaryTypographyProps={{
                          fontSize: "0.875rem",
                          fontWeight: category === cat ? 600 : 400,
                        }}
                      />
                    </ListItemButton>
                  ))}
                </List>
              </Box>

              <Divider />

              <Box>
                <Typography
                  variant="subtitle2"
                  color="text.secondary"
                  sx={{
                    mb: 1.5,
                    fontWeight: 700,
                    fontSize: "0.75rem",
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                  }}
                >
                  Tags
                </Typography>
                <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.75 }}>
                  <Chip
                    label="All Tags"
                    size="small"
                    onClick={() => setSelectedTag(null)}
                    color={selectedTag === null ? "primary" : "default"}
                    variant={selectedTag === null ? "filled" : "outlined"}
                    sx={{ cursor: "pointer", borderRadius: 1.5 }}
                  />
                  {tags.map((tag) => (
                    <Chip
                      key={tag}
                      label={tag}
                      size="small"
                      onClick={() => setSelectedTag(tag)}
                      color={selectedTag === tag ? "primary" : "default"}
                      variant={selectedTag === tag ? "filled" : "outlined"}
                      sx={{ cursor: "pointer", borderRadius: 1.5 }}
                    />
                  ))}
                </Box>
              </Box>
            </Stack>
          </Paper>
        </Grid>

        {/* Right pane: Main Area */}
        <Grid size={{ xs: 12, md: 8.5 }}>
          <Stack spacing={2.5}>
            <Stack
              direction={{ xs: "column", md: "row" }}
              spacing={2}
              sx={{ alignItems: "center", width: "100%" }}
            >
              <TextField
                placeholder="Search prompt, content, notes..."
                value={searchVal}
                onChange={(e) => setSearchVal(e.target.value)}
                size="small"
                fullWidth
                InputProps={{
                  startAdornment: (
                    <InputAdornment position="start">
                      <SearchIcon color="action" fontSize="small" />
                    </InputAdornment>
                  ),
                  endAdornment: searchVal && (
                    <InputAdornment position="end">
                      <IconButton size="small" onClick={() => setSearchVal("")}>
                        <ClearIcon fontSize="small" />
                      </IconButton>
                    </InputAdornment>
                  ),
                }}
                sx={{
                  "& .MuiOutlinedInput-root": {
                    borderRadius: 2,
                  },
                }}
              />

              <ToggleButtonGroup
                value={status}
                exclusive
                size="small"
                onChange={(_, next) => next && setStatus(next)}
                sx={{
                  flexShrink: 0,
                  "& .MuiToggleButton-root": {
                    borderRadius: 2,
                    px: 1.75,
                    textTransform: "none",
                    fontWeight: 500,
                  },
                }}
              >
                {STATUSES.map((item) => (
                  <ToggleButton key={item.value} value={item.value}>
                    {item.label}
                  </ToggleButton>
                ))}
              </ToggleButtonGroup>

              <Chip
                label={countLabel}
                size="small"
                variant="outlined"
                sx={{ flexShrink: 0, height: 32, borderRadius: 2 }}
              />
            </Stack>

            {error && <Alert severity="error">{error}</Alert>}

            {loading ? (
              <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
                <CircularProgress size={28} />
              </Box>
            ) : displayedItems.length === 0 ? (
              <Card variant="outlined" sx={{ borderRadius: 2 }}>
                <CardContent sx={{ py: 6, textAlign: "center" }}>
                  <TipsAndUpdatesOutlinedIcon
                    color="disabled"
                    sx={{ mb: 1.5, fontSize: 36 }}
                  />
                  <Typography variant="body2" color="text.secondary">
                    No deep dives found matching your filters.
                  </Typography>
                </CardContent>
              </Card>
            ) : (
              <Stack spacing={2}>
                {displayedItems.map((item) => (
                  <Card
                    key={item.id}
                    variant="outlined"
                    sx={{
                      borderRadius: 2,
                      transition:
                        "transform 0.15s, box-shadow 0.15s, border-color 0.15s",
                      "&:hover": {
                        transform: "translateY(-2px)",
                        boxShadow: "0 4px 12px rgba(0,0,0,0.05)",
                        borderColor: "primary.main",
                      },
                    }}
                  >
                    <CardActionArea
                      onClick={() => router.push(`/deep-dives/${item.id}`)}
                      sx={{ width: "100%", textAlign: "left" }}
                    >
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
                                label={
                                  STATUS_LABELS[item.status] ?? item.status
                                }
                                size="small"
                                color={
                                  item.status === "published"
                                    ? "success"
                                    : "default"
                                }
                                variant={
                                  item.status === "published"
                                    ? "filled"
                                    : "outlined"
                                }
                              />
                              <Chip
                                label={item.questionKind.toUpperCase()}
                                size="small"
                                variant="outlined"
                              />
                              {item.category && (
                                <Chip
                                  label={item.category}
                                  size="small"
                                  color="primary"
                                  variant="outlined"
                                />
                              )}
                            </Stack>
                            <Typography
                              variant="caption"
                              color="text.secondary"
                            >
                              Updated {formatDate(item.updatedAt)}
                            </Typography>
                          </Stack>

                          <Typography
                            variant="subtitle1"
                            sx={{ fontWeight: 600 }}
                          >
                            {item.questionPrompt}
                          </Typography>

                          {(item.assessmentTitle || item.course) && (
                            <Typography
                              variant="caption"
                              color="text.secondary"
                            >
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

                          <Box
                            sx={{
                              display: "flex",
                              justifyContent: "flex-end",
                              pt: 0.5,
                            }}
                          >
                            <Typography
                              variant="body2"
                              color="primary"
                              sx={{
                                fontWeight: 600,
                                fontSize: "0.85rem",
                                display: "flex",
                                alignItems: "center",
                                gap: 0.5,
                              }}
                            >
                              Open <ArrowForwardIcon sx={{ fontSize: 16 }} />
                            </Typography>
                          </Box>
                        </Stack>
                      </CardContent>
                    </CardActionArea>
                  </Card>
                ))}
              </Stack>
            )}
          </Stack>
        </Grid>
      </Grid>
    </PageShell>
  );
}
