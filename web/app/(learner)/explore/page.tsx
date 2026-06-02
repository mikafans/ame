"use client";

import { useState, useEffect, useCallback } from "react";
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
import Link from "next/link";
import { api } from "@/api/client";
import { vibrantTagColor } from "@/lib/tagColor";

export default function ExplorePage() {
  const [search, setSearch] = useState("");
  const [tags, setTags] = useState("");
  const [items, setItems] = useState<any[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchItems = useCallback(
    async (cursor: string | null = null) => {
      setLoading(true);
      const { data } = await api.GET("/v1/explore", {
        params: {
          query: {
            search: search || undefined,
            tags: tags || undefined,
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
    },
    [search, tags],
  );

  // Reset list when filters change
  const applyFilters = () => {
    setItems([]);
    setNextCursor(null);
    fetchItems();
  };

  useEffect(() => {
    fetchItems();
  }, [fetchItems]);

  return (
    <Box sx={{ display: "flex", p: 3, gap: 3 }}>
      {/* Sidebar Filters */}
      <Paper sx={{ width: 300, p: 2, height: "fit-content" }}>
        <Typography variant="h6" sx={{ mb: 2 }}>
          Filters
        </Typography>
        <TextField
          fullWidth
          label="Search Title"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") applyFilters();
          }}
          sx={{ mb: 2 }}
        />
        <TextField
          fullWidth
          label="Filter by Tags (comma separated)"
          value={tags}
          onChange={(e) => setTags(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") applyFilters();
          }}
          sx={{ mb: 2 }}
        />
        <Button variant="contained" fullWidth onClick={applyFilters}>
          Apply Filters
        </Button>
      </Paper>

      {/* Main Table */}
      <TableContainer component={Paper} sx={{ flexGrow: 1 }}>
        <Table>
          <TableHead>
            <TableRow>
              <TableCell>Title</TableCell>
              <TableCell>Tags</TableCell>
              <TableCell>Created</TableCell>
              <TableCell>Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {items.map((item) => (
              <TableRow key={item.id}>
                <TableCell>{item.title}</TableCell>
                <TableCell>
                  <Stack
                    direction="row"
                    spacing={1}
                    sx={{ flexWrap: "wrap", gap: 0.5 }}
                  >
                    {(item.tags as string[]).map((t) => (
                      <Chip
                        key={t}
                        label={t}
                        size="small"
                        sx={vibrantTagColor(t)}
                      />
                    ))}
                  </Stack>
                </TableCell>
                <TableCell>
                  {new Date(item.createdAt).toLocaleDateString()}
                </TableCell>
                <TableCell>
                  <Button
                    size="small"
                    component={Link}
                    href={`/assessments/${item.id}/preview`}
                  >
                    Preview
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {nextCursor && (
          <Box sx={{ p: 2, textAlign: "center" }}>
            <Button onClick={() => fetchItems(nextCursor)} disabled={loading}>
              {loading ? "Loading..." : "Load More"}
            </Button>
          </Box>
        )}
      </TableContainer>
    </Box>
  );
}
