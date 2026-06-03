"use client";

import React, { useState, useEffect, useCallback } from "react";
import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import Button from "@mui/material/Button";
import Grid from "@mui/material/Grid";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TablePagination from "@mui/material/TablePagination";
import Chip from "@mui/material/Chip";
import IconButton from "@mui/material/IconButton";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogActions from "@mui/material/DialogActions";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import { api } from "@/api/client";
import { formatDateTime } from "@/utils/format";

interface AuditLogEntry {
  id: string;
  actorUserId?: string | null;
  actorEmail?: string | null;
  actorName?: string | null;
  action: string;
  targetType?: string | null;
  targetId?: string | null;
  targetEmail?: string | null;
  targetName?: string | null;
  metadata: any;
  createdAt: string;
}

export default function AuditLogsPage() {
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [pageError, setPageError] = useState<string | null>(null);

  // Pagination & Filters state
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);

  // Active filters sent to API
  const [filterAction, setFilterAction] = useState("");
  const [filterActorId, setFilterActorId] = useState("");
  const [filterTargetId, setFilterTargetId] = useState("");

  // Input fields state (for form submit)
  const [inputAction, setInputAction] = useState("");
  const [inputActorId, setInputActorId] = useState("");
  const [inputTargetId, setInputTargetId] = useState("");

  // Detail Modal
  const [selectedLog, setSelectedLog] = useState<AuditLogEntry | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    setPageError(null);
    try {
      const { data, error } = await api.GET("/v1/admin/audit", {
        params: {
          query: {
            limit: rowsPerPage,
            offset: page * rowsPerPage,
            action: filterAction.trim() || undefined,
            actorId: filterActorId.trim() || undefined,
            targetId: filterTargetId.trim() || undefined,
          },
        },
      });

      if (error) {
        setPageError("Failed to load audit logs: " + (error as any)?.message);
        return;
      }

      if (data) {
        setLogs(data.logs as AuditLogEntry[]);
        setTotal(data.total);
      }
    } catch (err) {
      console.error(err);
      setPageError("An unexpected error occurred while fetching audit logs.");
    } finally {
      setLoading(false);
    }
  }, [page, rowsPerPage, filterAction, filterActorId, filterTargetId]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const handleFilterSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setPage(0);
    setFilterAction(inputAction);
    setFilterActorId(inputActorId);
    setFilterTargetId(inputTargetId);
  };

  const handleClearFilters = () => {
    setPage(0);
    setInputAction("");
    setInputActorId("");
    setInputTargetId("");
    setFilterAction("");
    setFilterActorId("");
    setFilterTargetId("");
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

  const handleOpenDetails = (log: AuditLogEntry) => {
    setSelectedLog(log);
    setDetailOpen(true);
  };

  // Prettify action text
  const formatActionName = (action: string): string => {
    return action
      .split(/[._]/)
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(" ");
  };

  // Get action color
  const getActionColor = (action: string) => {
    if (
      action.includes("reject") ||
      action.includes("fail") ||
      action.includes("disable")
    )
      return "error";
    if (
      action.includes("create") ||
      action.includes("enable") ||
      action.includes("register")
    )
      return "success";
    if (action.includes("update") || action.includes("patch")) return "warning";
    return "default";
  };

  return (
    <Container maxWidth="lg" sx={{ py: 6 }}>
      {/* Title */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" fontWeight="bold" gutterBottom>
          Audit Logs
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Track and inspect write actions and security events on the platform.
        </Typography>
      </Box>

      {/* Filters Form */}
      <Paper
        component="form"
        onSubmit={handleFilterSubmit}
        sx={{
          p: 3,
          mb: 4,
          borderRadius: 3,
          border: "1px solid",
          borderColor: "divider",
          boxShadow: "0 4px 20px rgba(0,0,0,0.02)",
          bgcolor: (theme) =>
            theme.palette.mode === "dark"
              ? "rgba(255,255,255,0.01)"
              : "rgba(0,0,0,0.005)",
        }}
      >
        <Typography variant="subtitle2" sx={{ mb: 2, fontWeight: 600 }}>
          Filter Logs
        </Typography>
        <Grid container spacing={2.5} alignItems="center">
          <Grid size={{ xs: 12, sm: 3 }}>
            <TextField
              label="Action"
              placeholder="e.g. user.disable"
              size="small"
              fullWidth
              value={inputAction}
              onChange={(e) => setInputAction(e.target.value)}
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 3.5 }}>
            <TextField
              label="Actor User ID"
              placeholder="UUID"
              size="small"
              fullWidth
              value={inputActorId}
              onChange={(e) => setInputActorId(e.target.value)}
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 3.5 }}>
            <TextField
              label="Target ID"
              placeholder="UUID"
              size="small"
              fullWidth
              value={inputTargetId}
              onChange={(e) => setInputTargetId(e.target.value)}
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 2 }} sx={{ display: "flex", gap: 1 }}>
            <Button
              variant="contained"
              type="submit"
              fullWidth
              sx={{ textTransform: "none", borderRadius: 2, py: 1 }}
            >
              Filter
            </Button>
            <Button
              variant="outlined"
              onClick={handleClearFilters}
              sx={{ textTransform: "none", borderRadius: 2, py: 1 }}
            >
              Clear
            </Button>
          </Grid>
        </Grid>
      </Paper>

      {/* Main Table Paper */}
      <Paper
        sx={{
          borderRadius: 3,
          overflow: "hidden",
          border: "1px solid",
          borderColor: "divider",
          boxShadow: "0 4px 20px rgba(0,0,0,0.02)",
        }}
      >
        {pageError && (
          <Box sx={{ p: 2 }}>
            <Alert severity="error">{pageError}</Alert>
          </Box>
        )}

        <TableContainer>
          <Table>
            <TableHead>
              <TableRow
                sx={{
                  bgcolor: (theme) =>
                    theme.palette.mode === "dark"
                      ? "rgba(255,255,255,0.01)"
                      : "rgba(0,0,0,0.01)",
                }}
              >
                <TableCell>Timestamp</TableCell>
                <TableCell>Action</TableCell>
                <TableCell>Actor</TableCell>
                <TableCell>Target Type</TableCell>
                <TableCell>Target</TableCell>
                <TableCell align="right" sx={{ pr: 3 }}>
                  Details
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 8 }}>
                    <CircularProgress size={32} sx={{ mb: 1 }} />
                    <Typography variant="body2" color="text.secondary">
                      Loading audit logs...
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : logs.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 8 }}>
                    <Typography
                      variant="body1"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      No audit records found.
                    </Typography>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ mt: 0.5 }}
                    >
                      Try adjusting the filter parameters.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                logs.map((row) => (
                  <TableRow
                    key={row.id}
                    hover
                    sx={{ "&:last-child td, &:last-child th": { border: 0 } }}
                  >
                    <TableCell sx={{ fontSize: 13 }}>
                      {formatDateTime(row.createdAt)}
                    </TableCell>
                    <TableCell>
                      <Chip
                        label={formatActionName(row.action)}
                        size="small"
                        color={getActionColor(row.action)}
                        variant="outlined"
                        sx={{
                          fontWeight: 600,
                          fontSize: 10,
                          borderRadius: 1.5,
                        }}
                      />
                    </TableCell>
                    <TableCell>
                      {row.actorEmail ? (
                        <Box>
                          {row.actorName && (
                            <Typography variant="body2" fontWeight={500}>
                              {row.actorName}
                            </Typography>
                          )}
                          <Typography variant="caption" color="text.secondary">
                            {row.actorEmail}
                          </Typography>
                        </Box>
                      ) : (
                        <Typography
                          variant="body2"
                          sx={{
                            fontFamily: "monospace",
                            fontSize: 11,
                            color: "text.secondary",
                          }}
                        >
                          {row.actorUserId || "SYSTEM"}
                        </Typography>
                      )}
                    </TableCell>
                    <TableCell>
                      {row.targetType ? (
                        <Chip
                          label={row.targetType.toUpperCase()}
                          size="small"
                          sx={{ fontSize: 9, height: 18, borderRadius: 1 }}
                        />
                      ) : (
                        "-"
                      )}
                    </TableCell>
                    <TableCell>
                      {row.targetEmail ? (
                        <Box>
                          {row.targetName && (
                            <Typography variant="body2" fontWeight={500}>
                              {row.targetName}
                            </Typography>
                          )}
                          <Typography variant="caption" color="text.secondary">
                            {row.targetEmail}
                          </Typography>
                        </Box>
                      ) : row.targetId ? (
                        <Typography
                          variant="body2"
                          sx={{
                            fontFamily: "monospace",
                            fontSize: 11,
                            color: "text.secondary",
                          }}
                        >
                          {row.targetId}
                        </Typography>
                      ) : (
                        "-"
                      )}
                    </TableCell>
                    <TableCell align="right" sx={{ pr: 2 }}>
                      <IconButton
                        size="small"
                        onClick={() => handleOpenDetails(row)}
                        color="primary"
                      >
                        <InfoOutlinedIcon fontSize="small" />
                      </IconButton>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>

        <TablePagination
          rowsPerPageOptions={[10, 25, 50, 100]}
          component="div"
          count={total}
          rowsPerPage={rowsPerPage}
          page={page}
          onPageChange={handlePageChange}
          onRowsPerPageChange={handleRowsPerPageChange}
          sx={{ borderTop: "1px solid", borderColor: "divider" }}
        />
      </Paper>

      {/* Details Dialog */}
      <Dialog
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        fullWidth
        maxWidth="sm"
      >
        <DialogTitle fontWeight="bold">Audit Event Details</DialogTitle>
        <DialogContent dividers>
          {selectedLog && (
            <Box sx={{ display: "flex", flexDirection: "column", gap: 2.5 }}>
              <Box
                sx={{
                  display: "grid",
                  gridTemplateColumns: "130px 1fr",
                  gap: 1,
                }}
              >
                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                >
                  Event ID:
                </Typography>
                <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                  {selectedLog.id}
                </Typography>

                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                >
                  Created At:
                </Typography>
                <Typography variant="body2">
                  {formatDateTime(selectedLog.createdAt)}
                </Typography>

                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                >
                  Action:
                </Typography>
                <Typography variant="body2" fontWeight={600}>
                  {selectedLog.action}
                </Typography>

                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                >
                  Actor ID:
                </Typography>
                <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                  {selectedLog.actorUserId || "SYSTEM"}
                </Typography>

                {selectedLog.actorName && (
                  <>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      Actor Name:
                    </Typography>
                    <Typography variant="body2">
                      {selectedLog.actorName}
                    </Typography>
                  </>
                )}

                {selectedLog.actorEmail && (
                  <>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      Actor Email:
                    </Typography>
                    <Typography variant="body2">
                      {selectedLog.actorEmail}
                    </Typography>
                  </>
                )}

                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                >
                  Target Type:
                </Typography>
                <Typography variant="body2">
                  {selectedLog.targetType || "None"}
                </Typography>

                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                >
                  Target ID:
                </Typography>
                <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                  {selectedLog.targetId || "None"}
                </Typography>

                {selectedLog.targetName && (
                  <>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      Target Name:
                    </Typography>
                    <Typography variant="body2">
                      {selectedLog.targetName}
                    </Typography>
                  </>
                )}

                {selectedLog.targetEmail && (
                  <>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      Target Email:
                    </Typography>
                    <Typography variant="body2">
                      {selectedLog.targetEmail}
                    </Typography>
                  </>
                )}
              </Box>

              <Box>
                <Typography
                  variant="body2"
                  color="text.secondary"
                  fontWeight={500}
                  sx={{ mb: 1 }}
                >
                  Event Metadata:
                </Typography>
                <Paper
                  variant="outlined"
                  sx={{
                    p: 2,
                    borderRadius: 2,
                    bgcolor: (theme) =>
                      theme.palette.mode === "dark"
                        ? "rgba(0,0,0,0.2)"
                        : "rgba(0,0,0,0.02)",
                    fontFamily: "monospace",
                    fontSize: 12,
                    maxHeight: 250,
                    overflowY: "auto",
                    whiteSpace: "pre-wrap",
                  }}
                >
                  {JSON.stringify(selectedLog.metadata, null, 2)}
                </Paper>
              </Box>
            </Box>
          )}
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button onClick={() => setDetailOpen(false)} variant="contained">
            Close
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
}
