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
import Avatar from "@mui/material/Avatar";
import Chip from "@mui/material/Chip";
import Menu from "@mui/material/Menu";
import MenuItem from "@mui/material/MenuItem";
import MoreVertIcon from "@mui/icons-material/MoreVert";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import BlockOutlinedIcon from "@mui/icons-material/BlockOutlined";
import CheckCircleOutlinedIcon from "@mui/icons-material/CheckCircleOutlined";
import StarBorderOutlinedIcon from "@mui/icons-material/StarBorderOutlined";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";
import Button from "@mui/material/Button";
import Alert from "@mui/material/Alert";
import FormControl from "@mui/material/FormControl";
import FormLabel from "@mui/material/FormLabel";
import RadioGroup from "@mui/material/RadioGroup";
import FormControlLabel from "@mui/material/FormControlLabel";
import Radio from "@mui/material/Radio";
import Checkbox from "@mui/material/Checkbox";
import CircularProgress from "@mui/material/CircularProgress";
import { api } from "@/api/client";
import { formatDateTime } from "@/utils/format";
import { useAuth } from "@/hooks/useAuth";

interface UserType {
  id: string;
  email?: string | null;
  displayName: string;
  role: "admin" | "agent" | "user";
  plan: string;
  deactivatedAt?: string | null;
  createdAt: string;
}

export default function ManageUsersPage() {
  const { user: currentUser } = useAuth();
  const [users, setUsers] = useState<UserType[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);

  // Pagination & Search
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [search, setSearch] = useState("");
  const [searchInput, setSearchInput] = useState("");

  // Error/Success Notification
  const [pageError, setPageError] = useState<string | null>(null);

  // Menu Anchor
  const [menuAnchor, setMenuAnchor] = useState<null | HTMLElement>(null);
  const [selectedUser, setSelectedUser] = useState<UserType | null>(null);

  // Dialog states
  const [roleDialogOpen, setRoleDialogOpen] = useState(false);
  const [targetRole, setTargetRole] = useState<string>("user");

  const [planDialogOpen, setPlanDialogOpen] = useState(false);
  const [targetPlan, setTargetPlan] = useState<string>("free");

  const [statusDialogOpen, setStatusDialogOpen] = useState(false);
  const [understandDisable, setUnderstandDisable] = useState(false);

  const [dialogLoading, setDialogLoading] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  const fetchUsers = useCallback(async () => {
    setLoading(true);
    setPageError(null);
    try {
      const { data, error } = await api.GET("/v1/admin/users", {
        params: {
          query: {
            limit: rowsPerPage,
            offset: page * rowsPerPage,
            q: search || undefined,
          },
        },
      });

      if (error) {
        setPageError("Failed to load users: " + (error as any)?.message);
        return;
      }

      if (data) {
        setUsers(data.users as UserType[]);
        setTotal(data.total);
      }
    } catch (err) {
      console.error(err);
      setPageError("An unexpected error occurred while fetching users.");
    } finally {
      setLoading(false);
    }
  }, [page, rowsPerPage, search]);

  useEffect(() => {
    fetchUsers();
  }, [fetchUsers]);

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

  // Menu Handlers
  const handleOpenMenu = (
    event: React.MouseEvent<HTMLButtonElement>,
    user: UserType,
  ) => {
    setMenuAnchor(event.currentTarget);
    setSelectedUser(user);
  };

  const handleCloseMenu = () => {
    setMenuAnchor(null);
  };

  // Dialog openers
  const handleOpenRoleDialog = () => {
    if (selectedUser) {
      setTargetRole(selectedUser.role);
      setDialogError(null);
      setRoleDialogOpen(true);
    }
    handleCloseMenu();
  };

  const handleOpenPlanDialog = () => {
    if (selectedUser) {
      setTargetPlan(selectedUser.plan);
      setDialogError(null);
      setPlanDialogOpen(true);
    }
    handleCloseMenu();
  };

  const handleOpenStatusDialog = () => {
    if (selectedUser) {
      setUnderstandDisable(false);
      setDialogError(null);
      setStatusDialogOpen(true);
    }
    handleCloseMenu();
  };

  // Error Helper
  const getErrorMessage = (error: any): string => {
    if (!error) return "An error occurred";
    const errObj = error?.error;
    if (errObj?.code === "validation_failed") {
      const fields = errObj?.details?.fields;
      if (Array.isArray(fields) && fields.length > 0) {
        return fields.map((f: any) => f.message).join(", ");
      }
    }
    return errObj?.message || error?.message || "An unexpected error occurred.";
  };

  // API submit helpers
  const handleSaveRole = async () => {
    if (!selectedUser) return;
    setDialogLoading(true);
    setDialogError(null);

    try {
      const { error } = await api.PATCH("/v1/admin/users/{id}", {
        params: { path: { id: selectedUser.id } },
        body: { role: targetRole },
      });

      if (error) {
        setDialogError(getErrorMessage(error));
      } else {
        setRoleDialogOpen(false);
        fetchUsers();
      }
    } catch (err) {
      setDialogError("Network or unexpected server error.");
    } finally {
      setDialogLoading(false);
    }
  };

  const handleSavePlan = async () => {
    if (!selectedUser) return;
    setDialogLoading(true);
    setDialogError(null);

    try {
      const { error } = await api.PATCH("/v1/admin/users/{id}", {
        params: { path: { id: selectedUser.id } },
        body: { plan: targetPlan },
      });

      if (error) {
        setDialogError(getErrorMessage(error));
      } else {
        setPlanDialogOpen(false);
        fetchUsers();
      }
    } catch (err) {
      setDialogError("Network or unexpected server error.");
    } finally {
      setDialogLoading(false);
    }
  };

  const handleToggleStatus = async () => {
    if (!selectedUser) return;
    const currentlyDisabled = !!selectedUser.deactivatedAt;

    // Safety guard UI validation check
    if (!currentlyDisabled && !understandDisable) {
      setDialogError(
        "You must check the confirmation box to deactivate this user.",
      );
      return;
    }

    setDialogLoading(true);
    setDialogError(null);

    try {
      const { error } = await api.PATCH("/v1/admin/users/{id}", {
        params: { path: { id: selectedUser.id } },
        body: { disabled: !currentlyDisabled },
      });

      if (error) {
        setDialogError(getErrorMessage(error));
      } else {
        setStatusDialogOpen(false);
        fetchUsers();
      }
    } catch (err) {
      setDialogError("Network or unexpected server error.");
    } finally {
      setDialogLoading(false);
    }
  };

  const getInitials = (user: UserType) => {
    return (
      user.displayName
        ?.split(" ")
        .map((n) => n[0])
        .join("")
        .toUpperCase()
        .slice(0, 2) || "?"
    );
  };

  return (
    <Container maxWidth="lg" sx={{ py: 6 }}>
      {/* Title */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" component="h1" fontWeight="bold" gutterBottom>
          Manage Users
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Paginate, search, and update roles, plans, or account suspension
          status.
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
            alignItems: "center",
            gap: 2,
            bgcolor: (theme) =>
              theme.palette.mode === "dark"
                ? "rgba(255,255,255,0.015)"
                : "rgba(0,0,0,0.005)",
          }}
        >
          <TextField
            placeholder="Search by display name or email..."
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
                <TableCell>User</TableCell>
                <TableCell>Role</TableCell>
                <TableCell>Plan</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Joined</TableCell>
                <TableCell align="right" sx={{ pr: 3 }}>
                  Actions
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 8 }}>
                    <CircularProgress size={32} sx={{ mb: 1 }} />
                    <Typography variant="body2" color="text.secondary">
                      Loading user accounts...
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : users.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 8 }}>
                    <Typography
                      variant="body1"
                      color="text.secondary"
                      fontWeight={500}
                    >
                      No users found.
                    </Typography>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ mt: 0.5 }}
                    >
                      Try adjusting your search criteria or clear the query.
                    </Typography>
                  </TableCell>
                </TableRow>
              ) : (
                users.map((row) => {
                  const isDisabled = !!row.deactivatedAt;
                  const isSelf = currentUser?.id === row.id;
                  return (
                    <TableRow
                      key={row.id}
                      hover
                      sx={{
                        "&:last-child td, &:last-child th": { border: 0 },
                        opacity: isDisabled ? 0.65 : 1,
                      }}
                    >
                      <TableCell>
                        <Box
                          sx={{
                            display: "flex",
                            alignItems: "center",
                            gap: 1.5,
                          }}
                        >
                          <Avatar
                            sx={{
                              width: 36,
                              height: 36,
                              fontSize: 14,
                              fontWeight: 600,
                              bgcolor: (theme) =>
                                theme.palette.mode === "dark"
                                  ? "rgba(255,255,255,0.08)"
                                  : "rgba(0,0,0,0.08)",
                              color: "text.primary",
                              border: "1px solid",
                              borderColor: "divider",
                            }}
                          >
                            {getInitials(row)}
                          </Avatar>
                          <Box sx={{ minWidth: 0 }}>
                            <Typography variant="body2" fontWeight={600} noWrap>
                              {row.displayName}{" "}
                              {isSelf && (
                                <Typography
                                  variant="caption"
                                  color="primary"
                                  sx={{ fontStyle: "italic", ml: 0.5 }}
                                >
                                  (You)
                                </Typography>
                              )}
                            </Typography>
                            <Typography
                              variant="caption"
                              color="text.secondary"
                              noWrap
                              sx={{ display: "block" }}
                            >
                              {row.email}
                            </Typography>
                          </Box>
                        </Box>
                      </TableCell>
                      <TableCell>
                        <Chip
                          label={row.role.toUpperCase()}
                          size="small"
                          color={
                            row.role === "admin"
                              ? "primary"
                              : row.role === "agent"
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
                        <Chip
                          label={row.plan.toUpperCase()}
                          size="small"
                          variant="outlined"
                          color={row.plan === "premium" ? "warning" : "default"}
                          sx={{
                            fontWeight: 600,
                            fontSize: 10,
                            borderRadius: 1.5,
                          }}
                        />
                      </TableCell>
                      <TableCell>
                        {isDisabled ? (
                          <Chip
                            label="DISABLED"
                            size="small"
                            color="error"
                            sx={{
                              fontWeight: 600,
                              fontSize: 10,
                              borderRadius: 1.5,
                            }}
                          />
                        ) : (
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
                        )}
                      </TableCell>
                      <TableCell>
                        <Typography
                          variant="body2"
                          color="text.secondary"
                          sx={{ fontSize: 13 }}
                        >
                          {formatDateTime(row.createdAt)}
                        </Typography>
                      </TableCell>
                      <TableCell align="right" sx={{ pr: 2 }}>
                        <IconButton
                          size="small"
                          onClick={(e) => handleOpenMenu(e, row)}
                        >
                          <MoreVertIcon fontSize="small" />
                        </IconButton>
                      </TableCell>
                    </TableRow>
                  );
                })
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

      {/* Row Operations Menu */}
      <Menu
        anchorEl={menuAnchor}
        open={Boolean(menuAnchor)}
        onClose={handleCloseMenu}
      >
        <MenuItem onClick={handleOpenRoleDialog}>
          <ListItemIcon>
            <EditOutlinedIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>Change User Role</ListItemText>
        </MenuItem>
        <MenuItem onClick={handleOpenPlanDialog}>
          <ListItemIcon>
            <StarBorderOutlinedIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>Change Subscription Plan</ListItemText>
        </MenuItem>
        {selectedUser && (
          <MenuItem onClick={handleOpenStatusDialog}>
            <ListItemIcon>
              {selectedUser.deactivatedAt ? (
                <CheckCircleOutlinedIcon fontSize="small" color="success" />
              ) : (
                <BlockOutlinedIcon fontSize="small" color="error" />
              )}
            </ListItemIcon>
            <ListItemText>
              {selectedUser.deactivatedAt
                ? "Enable Account"
                : "Disable Account"}
            </ListItemText>
          </MenuItem>
        )}
      </Menu>

      {/* Dialog: Change Role */}
      <Dialog
        open={roleDialogOpen}
        onClose={() => !dialogLoading && setRoleDialogOpen(false)}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">Change User Role</DialogTitle>
        <DialogContent dividers>
          {dialogError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {dialogError}
            </Alert>
          )}
          <DialogContentText sx={{ mb: 3 }}>
            Modify the role for <strong>{selectedUser?.displayName}</strong>.
          </DialogContentText>
          <FormControl component="fieldset">
            <FormLabel component="legend" sx={{ mb: 1, fontSize: 13 }}>
              Assign Role
            </FormLabel>
            <RadioGroup
              value={targetRole}
              onChange={(e) => setTargetRole(e.target.value)}
            >
              <FormControlLabel
                value="user"
                control={<Radio size="small" />}
                label="User (Standard access)"
              />
              <FormControlLabel
                value="agent"
                control={<Radio size="small" />}
                label="Agent (API/Integrations)"
              />
              <FormControlLabel
                value="admin"
                control={<Radio size="small" />}
                label="Admin (Full control)"
              />
            </RadioGroup>
          </FormControl>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={() => setRoleDialogOpen(false)}
            color="inherit"
            disabled={dialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleSaveRole}
            variant="contained"
            disabled={dialogLoading}
          >
            {dialogLoading ? <CircularProgress size={20} /> : "Save Changes"}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Dialog: Change Plan */}
      <Dialog
        open={planDialogOpen}
        onClose={() => !dialogLoading && setPlanDialogOpen(false)}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">Change Subscription Plan</DialogTitle>
        <DialogContent dividers>
          {dialogError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {dialogError}
            </Alert>
          )}
          <DialogContentText sx={{ mb: 3 }}>
            Modify the tier plan for{" "}
            <strong>{selectedUser?.displayName}</strong>.
          </DialogContentText>
          <FormControl component="fieldset">
            <FormLabel component="legend" sx={{ mb: 1, fontSize: 13 }}>
              Assign Plan
            </FormLabel>
            <RadioGroup
              value={targetPlan}
              onChange={(e) => setTargetPlan(e.target.value)}
            >
              <FormControlLabel
                value="free"
                control={<Radio size="small" />}
                label="Free tier"
              />
              <FormControlLabel
                value="premium"
                control={<Radio size="small" />}
                label="Premium tier"
              />
            </RadioGroup>
          </FormControl>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={() => setPlanDialogOpen(false)}
            color="inherit"
            disabled={dialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleSavePlan}
            variant="contained"
            disabled={dialogLoading}
          >
            {dialogLoading ? <CircularProgress size={20} /> : "Save Changes"}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Dialog: Suspend / Enable Account */}
      <Dialog
        open={statusDialogOpen}
        onClose={() => !dialogLoading && setStatusDialogOpen(false)}
        fullWidth
        maxWidth="xs"
      >
        <DialogTitle fontWeight="bold">
          {selectedUser?.deactivatedAt ? "Enable Account" : "Disable Account"}
        </DialogTitle>
        <DialogContent dividers>
          {dialogError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {dialogError}
            </Alert>
          )}

          {selectedUser?.deactivatedAt ? (
            <DialogContentText>
              Reactivating the account for{" "}
              <strong>{selectedUser?.displayName}</strong>. The user will be
              allowed to log in and create API tokens again.
            </DialogContentText>
          ) : (
            <Box>
              <Alert severity="warning" sx={{ mb: 2.5 }}>
                Disabling an account is a destructive admin action.
              </Alert>
              <DialogContentText sx={{ mb: 3 }}>
                Are you sure you want to deactivate the account for{" "}
                <strong>{selectedUser?.displayName}</strong>? The user will be
                immediately logged out, any active API tokens will be
                permanently revoked, and all future authentication attempts will
                be blocked.
              </DialogContentText>
              <FormControlLabel
                control={
                  <Checkbox
                    size="small"
                    checked={understandDisable}
                    onChange={(e) => setUnderstandDisable(e.target.checked)}
                  />
                }
                label={
                  <Typography variant="body2" color="text.secondary">
                    I understand the implications of deactivating this user.
                  </Typography>
                }
              />
            </Box>
          )}
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button
            onClick={() => setStatusDialogOpen(false)}
            color="inherit"
            disabled={dialogLoading}
          >
            Cancel
          </Button>
          <Button
            onClick={handleToggleStatus}
            variant="contained"
            color={selectedUser?.deactivatedAt ? "success" : "error"}
            disabled={
              dialogLoading ||
              (!selectedUser?.deactivatedAt && !understandDisable)
            }
          >
            {dialogLoading ? (
              <CircularProgress size={20} />
            ) : selectedUser?.deactivatedAt ? (
              "Enable Account"
            ) : (
              "Disable Account"
            )}
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
}
