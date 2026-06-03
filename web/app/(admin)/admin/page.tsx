"use client";

import React from "react";
import { useRouter } from "next/navigation";
import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import CardActions from "@mui/material/CardActions";
import Button from "@mui/material/Button";
import ShieldOutlinedIcon from "@mui/icons-material/ShieldOutlined";
import PeopleOutlinedIcon from "@mui/icons-material/PeopleOutlined";
import HistoryOutlinedIcon from "@mui/icons-material/HistoryOutlined";
import MonitorHeartOutlinedIcon from "@mui/icons-material/MonitorHeartOutlined";
import ArrowForwardOutlinedIcon from "@mui/icons-material/ArrowForwardOutlined";
import AssessmentOutlinedIcon from "@mui/icons-material/AssessmentOutlined";

export default function AdminDashboardPage() {
  const router = useRouter();

  const panels = [
    {
      title: "Manage Users",
      description:
        "Search and filter platform users. Instantly manage roles, change subscription plans, or disable/enable accounts.",
      icon: <PeopleOutlinedIcon sx={{ fontSize: 40, color: "primary.main" }} />,
      link: "/admin/users",
      actionLabel: "Users Console",
    },
    {
      title: "Moderate Assessments",
      description:
        "Audit and moderate assessments created on the platform. Review question contents and perform administrative soft deactivations.",
      icon: (
        <AssessmentOutlinedIcon sx={{ fontSize: 40, color: "warning.main" }} />
      ),
      link: "/admin/assessments",
      actionLabel: "Moderation Console",
    },
    {
      title: "Audit Logs",
      description:
        "Inspect the append-only system audit log. Filter by event action type, actor, or target identifier to trace operations.",
      icon: (
        <HistoryOutlinedIcon sx={{ fontSize: 40, color: "success.main" }} />
      ),
      link: "/admin/audit",
      actionLabel: "Audit Trail",
    },
    {
      title: "System Health",
      description:
        "Check database and Valkey live statuses, view table row counts, and monitor rate-limiting quota rejections.",
      icon: (
        <MonitorHeartOutlinedIcon sx={{ fontSize: 40, color: "error.main" }} />
      ),
      link: "/admin/health",
      actionLabel: "System Status",
    },
  ];

  return (
    <Container maxWidth="lg" sx={{ py: 6 }}>
      {/* Header */}
      <Box sx={{ mb: 6, display: "flex", alignItems: "center", gap: 2 }}>
        <ShieldOutlinedIcon color="primary" sx={{ fontSize: 48 }} />
        <Box>
          <Typography
            variant="h4"
            component="h1"
            fontWeight="bold"
            gutterBottom
          >
            Admin Console
          </Typography>
          <Typography variant="body1" color="text.secondary">
            Operational dashboard and system controls for the AME platform.
          </Typography>
        </Box>
      </Box>

      {/* Cards Grid */}
      <Box
        sx={{
          display: "grid",
          gridTemplateColumns: {
            xs: "1fr",
            sm: "1fr 1fr",
            md: "1fr 1fr 1fr 1fr",
          },
          gap: 3.5,
        }}
      >
        {panels.map((panel) => (
          <Card
            key={panel.title}
            sx={{
              display: "flex",
              flexDirection: "column",
              height: "100%",
              borderRadius: 3,
              boxShadow: "0 4px 20px rgba(0, 0, 0, 0.05)",
              border: "1px solid",
              borderColor: "divider",
              background: (theme) =>
                theme.palette.mode === "dark"
                  ? "linear-gradient(135deg, rgba(30, 41, 59, 0.4) 0%, rgba(15, 23, 42, 0.6) 100%)"
                  : "linear-gradient(135deg, rgba(255, 255, 255, 0.9) 0%, rgba(248, 250, 252, 0.9) 100%)",
              backdropFilter: "blur(10px)",
              transition:
                "transform 0.25s ease-in-out, box-shadow 0.25s ease-in-out",
              "&:hover": {
                transform: "translateY(-6px)",
                boxShadow: (theme) =>
                  theme.palette.mode === "dark"
                    ? "0 12px 30px rgba(0, 0, 0, 0.4), 0 0 15px rgba(59, 130, 246, 0.2)"
                    : "0 12px 30px rgba(0, 0, 0, 0.08), 0 0 15px rgba(59, 130, 246, 0.1)",
              },
            }}
          >
            <CardContent sx={{ flexGrow: 1, p: 3.5 }}>
              <Box
                sx={{
                  mb: 2.5,
                  display: "inline-flex",
                  p: 1.5,
                  borderRadius: 2.5,
                  bgcolor: (theme) =>
                    theme.palette.mode === "dark"
                      ? "rgba(255,255,255,0.03)"
                      : "rgba(0,0,0,0.015)",
                }}
              >
                {panel.icon}
              </Box>
              <Typography
                variant="h5"
                component="h2"
                fontWeight="600"
                gutterBottom
              >
                {panel.title}
              </Typography>
              <Typography
                variant="body2"
                color="text.secondary"
                sx={{ mt: 1.5, lineHeight: 1.6 }}
              >
                {panel.description}
              </Typography>
            </CardContent>
            <CardActions sx={{ px: 3.5, pb: 3.5, pt: 0 }}>
              <Button
                variant="outlined"
                color="inherit"
                fullWidth
                endIcon={<ArrowForwardOutlinedIcon />}
                onClick={() => router.push(panel.link)}
                sx={{
                  justifyContent: "space-between",
                  borderRadius: 2,
                  textTransform: "none",
                  fontWeight: 600,
                  py: 1.2,
                  px: 2,
                  borderColor: "divider",
                  "&:hover": {
                    bgcolor: "primary.main",
                    color: "primary.contrastText",
                    borderColor: "primary.main",
                  },
                }}
              >
                {panel.actionLabel}
              </Button>
            </CardActions>
          </Card>
        ))}
      </Box>
    </Container>
  );
}
