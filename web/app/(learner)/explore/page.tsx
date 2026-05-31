"use client";

import { useState, useEffect, useCallback } from "react";
import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import Stack from "@mui/material/Stack";
import Checkbox from "@mui/material/Checkbox";
import FormControlLabel from "@mui/material/FormControlLabel";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Chip from "@mui/material/Chip";
import Button from "@mui/material/Button";
import { api } from "@/api/client";
import { vibrantTagColor } from "@/lib/tagColor";

export default function ExplorePage() {
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<any[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchItems = useCallback(
    async (cursor: string | null = null) => {
      setLoading(true);
      const { data, error } = await api.GET("/v1/explore", {
        params: {
          query: {
            search: search || undefined,
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
    [search],
  );

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
          label="Search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") fetchItems();
          }}
          sx={{ mb: 2 }}
        />
        <Stack>
          <FormControlLabel control={<Checkbox />} label="Assessments" />
          <FormControlLabel control={<Checkbox />} label="Exams" />
        </Stack>
      </Paper>

      {/* Main Table */}
      <TableContainer component={Paper} sx={{ flexGrow: 1 }}>
        <Table>
          <TableHead>
            <TableRow>
              <TableCell>Title</TableCell>
              <TableCell>Tags</TableCell>
              <TableCell>Created</TableCell>
              <TableCell>Visibility</TableCell>
              <TableCell>Actions</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {items.map((item) => (
              <TableRow key={item.id}>
                <TableCell>{item.title}</TableCell>
                <TableCell>
                  <Stack direction="row" spacing={1}>
                    {(item.tags as string[]).map((tag) => (
                      <Chip
                        key={tag}
                        label={tag}
                        size="small"
                        sx={vibrantTagColor(tag)}
                      />
                    ))}
                  </Stack>
                </TableCell>
                <TableCell>
                  {new Date(item.createdAt).toLocaleDateString()}
                </TableCell>
                <TableCell>{item.visibility}</TableCell>
                <TableCell>
                  <Button size="small">Preview</Button>
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
