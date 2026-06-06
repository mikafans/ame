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
import IconButton from "@mui/material/IconButton";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TablePagination from "@mui/material/TablePagination";
import Chip from "@mui/material/Chip";
import DeleteOutlinedIcon from "@mui/icons-material/DeleteOutlined";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import Button from "@mui/material/Button";
import Alert from "@mui/material/Alert";
import CircularProgress from "@mui/material/CircularProgress";
import FormControl from "@mui/material/FormControl";
import FormLabel from "@mui/material/FormLabel";
import RadioGroup from "@mui/material/RadioGroup";
import FormControlLabel from "@mui/material/FormControlLabel";
import Radio from "@mui/material/Radio";
import { api } from "@/api/client";
import { formatDateTime } from "@/utils/format";

interface TokenEntry {
  id: string;
  name: string;
  ownerId: string;
  ownerEmail: string | null;
  ownerDisplayName?: string | null;
  ownerRole: string;
  status: string;
  scopes: string[];
  lastUsedAt: string | null;
  revokedAt: string | null;
  expiresAt: string;
  createdAt: string;
}

export default function TokensAuditPage() {
  const [tokens, setTokens] = useState<TokenEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);

  // Pagination & Search
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [search, setSearch] = useState("");
  const [searchInput, setSearchInput] = useState("");

  // Filters
  const [statusFilter, setStatusFilter] = useState<string>("");
  const [roleFilter, setRoleFilter] = useState<string>("");

  // Error/Success Notification
  const [pageError, setPageError] = useState<string | null>(null);

  // Dialog states
  const [revokeDialogOpen, setRevokeDialogOpen] = useState(false);
  const [selectedToken, setSelectedToken] = useState<TokenEntry | null>(null);
  const [dialogLoading, setDialogLoading] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  const fetchTokens = useCallback(async () => {
    setLoading(true);
    setPageError(null);
    try {
      const { data, error } = await api.GET("/v1/admin/tokens", {
        params: {
          query: {
            page: page + 1,
            pageSize: rowsPerPage,
            q: search || undefined,
            status: statusFilter || undefined,
            role: roleFilter || undefined,
          },
        },
      });

      if (error) {
        setPageError("Failed to load tokens: " + (error as any)?.message);
        return;
      }

      if (data) {
        setTokens(data.tokens as TokenEntry[]);
        setTotal(data.total);
      }
    } catch (err) {
      console.error(err);
      setPageError("An unexpected error occurred while fetching tokens.");
    } finally {
      setLoading(false);
    }
  }, [page, rowsPerPage, search, statusFilter, roleFilter]);

  useEffect(() => {
    fetchTokens();
  }, [fetchTokens]);

  // Search handlers
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

  const handleStatusFilterChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setStatusFilter(e.target.value);
    setPage(0);
  };

  const handleRoleFilterChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setRoleFilter(e.target.value);
    setPage(0);
  };

  // Dialog openers/closers
  const handleOpenRevokeDialog = (token: TokenEntry) => {
    setSelectedToken(token);
    setDialogError(null);
    setRevokeDialogOpen(true);
  };

  const handleCloseRevokeDialog = () => {
    if (!dialogLoading) {
      setRevokeDialogOpen(false);
    }
  };

  // Revoke handler
  const handleRevoke = async () => {
    if (!selectedToken) return;
    setDialogLoading(true);
    setDialogError(null);

    try {
      const { error } = await api.DELETE("/v1/admin/tokens/{id}", {
        params: { path: { id: selectedToken.id } },
      });

      if (error) {
        setDialogError(
          "Failed to revoke token: " +
            ((error as any)?.message || "Unknown error"),
        );
      } else {
        setRevokeDialogOpen(false);
        fetchTokens();
      }
    } catch (err) {
      setDialogError("Network or unexpected server error.");
    } finally {
      setDialogLoading(false);
    }
  };

  const getStatusChip = (token: TokenEntry) => {
    if (token.status === "revoked") {
      return (
        <Chip
          label="REVOKED"
          size="small"
          sx={{
            fontWeight: 600,
            fontSize: 10,
            borderRadius: 1.5,
          }}
        />
      );
    }

    if (token.status === "expired") {
      return (
        <Chip
          label="EXPIRED"
          size="small"
          color="warning"
          sx={{
            fontWeight: 600,
            fontSize: 10,
            borderRadius: 1.5,
          }}
        />
      );
    }

    return (
      <Chip
        label="ACTIVE"
        size="small"
        color="success"
        variant="outlined"
        sx={{
          fontWeight: 600,
          fontSize: 10,
          borderRadius: 1.5,
        }}
      />
    );
  };

  const canRevoke = (token: TokenEntry) => {
    return token.status !== "revoked";
  };

  return (
    <Container maxWidth="lg" sx={{ py: 6 }}>
      {/* Title */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" fontWeight="bold" gutterBottom>
          API Tokens Audit
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Review all API tokens platform-wide, check usage history, and revoke
          tokens on demand.
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
        {/* Top Control Bar */}
        <Box
          component="form"
          onSubmit={handleSearchSubmit}
          sx={{
            p: 2.5,
            borderBottom: "1px solid",
            borderColor: "divider",
            display: "flex",
            flexDirection: { xs: "column", sm: "row" },
            alignItems: { sm: "center" },
            gap: 2,
            bgcolor: (theme) =>
              theme.palette.mode === "dark"
                ? "rgba(255,255,255,0.015)"
                : "rgba(0,0,0,0.005)",
          }}
        >
          <TextField
            placeholder="Search by token name or owner email..."
            size="small"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            fullWidth
            sx={{ maxWidth: { sm: 400 } }}
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
            sx={{ minWidth: 100, textTransform: "none", borderRadius: 2 }}
          >
            Search
          </Button>
        </Box>

        {/* Filters Bar */}
        <Box
          sx={{
            p: 2.5,
            borderBottom: "1px solid",
            borderColor: "divider",
            display: "flex",
            flexDirection: { xs: "column", sm: "row" },
            gap: 3,
            bgcolor: (theme) =>
              theme.palette.mode === "dark"
                ? "rgba(255,255,255,0.01)"
                : "rgba(0,0,0,0.002)",
          }}
        >
          <FormControl sx={{ minWidth: 180 }}>
            <FormLabel component="legend" sx={{ fontSize: 13, mb: 1 }}>
              Status
            </FormLabel>
            <RadioGroup
              row
              value={statusFilter}
              onChange={handleStatusFilterChange}
              sx={{ gap: 2 }}
            >
              <FormControlLabel
                value=""
                control={<Radio size="small" />}
                label="All"
                sx={{ m: 0 }}
              />
              <FormControlLabel
                value="active"
                control={<Radio size="small" />}
                label="Active"
                sx={{ m: 0 }}
              />
              <FormControlLabel
                value="revoked"
                control={<Radio size="small" />}
                label="Revoked"
                sx={{ m: 0 }}
              />
              <FormControlLabel
                value="expired"
                control={<Radio size="small" />}
                label="Expired"
                sx={{ m: 0 }}
              />
            </RadioGroup>
          </FormControl>

          <FormControl sx={{ minWidth: 180 }}>
            <FormLabel component="legend" sx={{ fontSize: 13, mb: 1 }}>
              Owner Role
            </FormLabel>
            <RadioGroup
              row
              value={roleFilter}
              onChange={handleRoleFilterChange}
              sx={{ gap: 2 }}
            >
              <FormControlLabel
                value=""
                control={<Radio size="small" />}
                label="All"
                sx={{ m: 0 }}
              />
              <FormControlLabel
                value="admin"
                control={<Radio size="small" />}
                label="Admin"
                sx={{ m: 0 }}
              />
              <FormControlLabel
                value="agent"
                control={<Radio size="small" />}
                label="Agent"
                sx={{ m: 0 }}
              />
              <FormControlLabel
                value="user"
                control={<Radio size="small" />}
                label="User"
                sx={{ m: 0 }}
              />
            </RadioGroup>
          </FormControl>
        </Box>

        {pageError && (
          <Box sx={{ p: 2 }}>
            <Alert severity="error">{pageError}</Alert>
          </Box>
        )}

        {/* Table */}
        <TableContainer>
          <Table sx={{ minWidth: 650 }}>
            <TableHead>
              <TableRow
                sx={{
                  bgcolor: (theme) =>
                    theme.palette.mode === "dark"
                      ? "rgba(255,255,255,0.01)"
                      : "rgba(0,0,0,0.01)",
                }}
              >
                <TableCell>Name</TableCell>
                <TableCell>Owner</TableCell>
                <TableCell>Role</TableCell>
                <TableCell>Scopes</TableCell>
                <TableCell>Last Used</TableCell>
                <TableCell>Expires</TableCell>
                <TableCell>Status</TableCell>
                <TableCell align="right" sx={{ pr: 3 }}>
                  Actions
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={8} align="center" sx={{ py: 8 }}>
                    <CircularProgress size={32} sx={{ mb: 1 }} />
                    <Typography variant="body2" color="text.secondary">
                      Loading tokens...
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : tokens.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={8} align="center" sx={{ py: 8 }}>
                    <Typography
                      variant="body1"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      No tokens found.
                    </Typography>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ mt: 0.5 }}
                    >
                      Try adjusting your filters or search criteria.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                tokens.map((row) => (
                  <TableRow
                    key={row.id}
                    hover
                    sx={{
                      "&:last-child td, &:last-child th": { border: 0 },
                      opacity: row.revokedAt ? 0.65 : 1,
                    }}
                  >
                    <TableCell>
                      <Typography variant="body2" fontWeight={600} noWrap>
                        {row.name}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Box>
                        <Typography variant="body2" fontWeight={600} noWrap>
                          {row.ownerDisplayName || "—"}
                        </Typography>
                        {row.ownerEmail && (
                          <Typography
                            variant="caption"
                            color="text.secondary"
                            noWrap
                            display="block"
                          >
                            {row.ownerEmail}
                          </Typography>
                        )}
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Chip
                        label={row.ownerRole.toUpperCase()}
                        size="small"
                        color={
                          row.ownerRole === "admin"
                            ? "primary"
                            : row.ownerRole === "agent"
                              ? "secondary"
                              : "default"
                        }
                        sx={{
                          fontWeight: 600,
                          fontSize: 10,
                          borderRadius: 1.5,
                        }}
                      />
                    </TableCell>
                    <TableCell>
                      <Box sx={{ display: "flex", gap: 0.5, flexWrap: "wrap" }}>
                        {row.scopes.length > 0 ? (
                          row.scopes.slice(0, 3).map((scope, idx) => (
                            <Chip
                              key={idx}
                              label={scope}
                              size="small"
                              variant="outlined"
                              sx={{
                                fontWeight: 500,
                                fontSize: 11,
                                borderRadius: 1,
                              }}
                            />
                          ))
                        ) : (
                          <Typography variant="caption" color="text.secondary">
                            None
                          </Typography>
                        )}
                        {row.scopes.length > 3 && (
                          <Chip
                            label={`+${row.scopes.length - 3}`}
                            size="small"
                            variant="outlined"
                            sx={{
                              fontWeight: 500,
                              fontSize: 11,
                              borderRadius: 1,
                            }}
                          />
                        )}
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ fontSize: 13 }}
                      >
                        {row.lastUsedAt ? (
                          formatDateTime(row.lastUsedAt)
                        ) : (
                          <span>—</span>
                        )}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography
                        variant="body2"
                        color="text.secondary"
                        sx={{ fontSize: 13 }}
                      >
                        {formatDateTime(row.expiresAt)}
                      </Typography>
                    </TableCell>
                    <TableCell>{getStatusChip(row)}</TableCell>
                    <TableCell align="right" sx={{ pr: 2 }}>
                      <IconButton
                        size="small"
                        disabled={!canRevoke(row)}
                        onClick={() => handleOpenRevokeDialog(row)}
                        color="error"
                      >
                        <DeleteOutlinedIcon fontSize="small" />
                      </IconButton>
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

      {/* Dialog: Revoke Token */}
      <Dialog
        open={revokeDialogOpen}
        onClose={handleCloseRevokeDialog}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">Revoke API Token</DialogTitle>
        <DialogContent dividers>
          {dialogError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {dialogError}
            </Alert>
          )}
          <DialogContentText sx={{ mb: 2 }}>
            Revoke the API token <strong>{selectedToken?.name}</strong>? This
            action is idempotent and cannot be undone. The token owner will need
            to create a new token to continue using integrations.
          </DialogContentText>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={handleCloseRevokeDialog}
            color="inherit"
            disabled={dialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleRevoke}
            variant="contained"
            color="error"
            disabled={dialogLoading}
          >
            {dialogLoading ? <CircularProgress size={20} /> : "Revoke"}
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
}
