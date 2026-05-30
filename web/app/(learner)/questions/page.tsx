"use client";

import { useState, useEffect, useCallback } from "react";
import { api } from "@/api/client";
import { useAuth } from "@/hooks/useAuth";
import Box from "@mui/material/Box";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
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
import SearchIcon from "@mui/icons-material/Search";

interface Question {
  id: string;
  kind: string;
  prompt: string;
  points: number;
  status: string;
  tags: string[];
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
        setQuestions(data.questions);
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

  const handlePageSizeChange = (newSize: unknown) => {
    setPageSize(newSize as number);
    setPage(1);
  };

  const startRow = (page - 1) * pageSize + 1;
  const endRow = Math.min(page * pageSize, total);

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h5" sx={{ fontWeight: 500, mb: 3 }}>
        Question Bank
      </Typography>

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

        <ToggleButtonGroup
          value={kind}
          exclusive
          onChange={handleKindChange}
          size="small"
          sx={{ "& .MuiToggleButtonGroup-grouped": { height: 40 } }}
        >
          {Object.entries(KIND_LABELS).map(([k, label]) => (
            <ToggleButton key={k} value={k}>
              {label}
            </ToggleButton>
          ))}
        </ToggleButtonGroup>

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
        <Table stickyHeader>
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
                <TableRow key={q.id} hover>
                  <TableCell>
                    <Chip
                      label={KIND_LABELS[q.kind] || q.kind}
                      size="small"
                      color={KIND_COLORS[q.kind] || "default"}
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
    </Box>
  );
}
