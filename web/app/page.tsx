"use client";

import Link from "next/link";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Card from "@mui/material/Card";
import Chip from "@mui/material/Chip";
import Container from "@mui/material/Container";
import Divider from "@mui/material/Divider";
import Grid from "@mui/material/Grid";
import IconButton from "@mui/material/IconButton";
import Stack from "@mui/material/Stack";
import Typography from "@mui/material/Typography";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import CheckIcon from "@mui/icons-material/Check";
import DarkModeOutlinedIcon from "@mui/icons-material/DarkModeOutlined";
import LightModeOutlinedIcon from "@mui/icons-material/LightModeOutlined";
import { useAuth } from "@/hooks/useAuth";
import { useColorMode } from "@/components/ThemeRegistry";
import { Logo } from "@/components/Logo";

const benefits = [
  {
    number: "01",
    title: "Study plans that adapt",
    body: "Plans start from your performance and change as you answer. Spend your next session where it matters most.",
  },
  {
    number: "02",
    title: "Questions, organized",
    body: "Collect questions from anywhere into tagged banks. Turn a bank into a timed assessment when you are ready.",
  },
  {
    number: "03",
    title: "Progress you can see",
    body: "Track movement by topic so you can replace 'I should study' with a clear next step.",
  },
];

const faqs = [
  {
    question: "Can I use AME without an AI assistant?",
    answer:
      "Yes. The web app works on its own. Agent support is an optional way to read your stats and build focused practice.",
  },
  {
    question: "Can I self-host AME?",
    answer:
      "AME is designed to be self-hostable. Follow the repository deployment documentation to run the platform on your own infrastructure.",
  },
  {
    question: "What should I do first?",
    answer:
      "Create an account, open Explore, and start an assessment from the question bank. Your first session gives you a useful baseline.",
  },
];

function QuizPreview() {
  return (
    <Card
      sx={{
        borderRadius: 2,
        p: { xs: 2, sm: 2.5 },
        bgcolor: "#fff",
        color: "#171717",
        boxShadow: "0 24px 60px rgba(0, 0, 0, 0.28)",
      }}
    >
      <Stack spacing={1.5}>
        <Stack
          direction="row"
          justifyContent="space-between"
          alignItems="center"
        >
          <Typography
            variant="caption"
            sx={{ color: "#8a8a8a", fontFamily: "monospace" }}
          >
            Q 7 / 20
          </Typography>
          <Chip
            label="Adaptive · ELO 1480"
            size="small"
            sx={{
              bgcolor: "#e8f0ff",
              color: "#1261e9",
              fontSize: 11,
              height: 24,
            }}
          />
        </Stack>
        <Box
          sx={{
            height: 4,
            borderRadius: 2,
            bgcolor: "#e8e8e8",
            overflow: "hidden",
          }}
        >
          <Box sx={{ width: "35%", height: "100%", bgcolor: "#1261e9" }} />
        </Box>
        <Typography
          sx={{ fontSize: { xs: 16, sm: 17 }, fontWeight: 700, pt: 0.5 }}
        >
          Which data structure gives O(1) average lookup?
        </Typography>
        <Stack spacing={1}>
          <Box
            sx={{
              border: "1px solid #d9d9d9",
              borderRadius: 1.5,
              px: 1.5,
              py: 1.1,
            }}
          >
            Binary search tree
          </Box>
          <Box
            sx={{
              border: "2px solid #1261e9",
              borderRadius: 1.5,
              px: 1.5,
              py: 1.05,
              bgcolor: "#e8f0ff",
              display: "flex",
              justifyContent: "space-between",
            }}
          >
            <span>Hash table</span>
            <CheckIcon sx={{ color: "#5b9700", fontSize: 19 }} />
          </Box>
          <Box
            sx={{
              border: "1px solid #d9d9d9",
              borderRadius: 1.5,
              px: 1.5,
              py: 1.1,
            }}
          >
            Linked list
          </Box>
        </Stack>
        <Stack
          direction="row"
          justifyContent="space-between"
          alignItems="center"
          sx={{ pt: 0.5 }}
        >
          <Typography variant="caption" sx={{ color: "#929292" }}>
            Hash maps · your weakest topic
          </Typography>
          <Button
            variant="contained"
            size="small"
            sx={{
              borderRadius: 5,
              bgcolor: "#050505",
              color: "#fff",
              minWidth: 72,
            }}
          >
            Next
          </Button>
        </Stack>
      </Stack>
    </Card>
  );
}

export default function LandingPage() {
  const { user } = useAuth();
  const { mode, toggle } = useColorMode();
  const signedIn = !!user;
  const entryHref = signedIn ? "/explore" : "/login";

  return (
    <Box
      sx={{
        minHeight: "100vh",
        bgcolor: "#fff",
        color: "#171717",
        fontFamily: "Inter, sans-serif",
      }}
    >
      <Container maxWidth="lg" sx={{ py: { xs: 1.5, sm: 2 } }}>
        <Box
          component="header"
          sx={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            minHeight: 48,
          }}
        >
          <Logo size={32} />
          <Stack
            direction="row"
            spacing={{ xs: 1, sm: 2.5 }}
            alignItems="center"
          >
            <Box sx={{ display: { xs: "none", sm: "flex" }, gap: 2.5 }}>
              <Typography
                component="a"
                href="#benefits"
                sx={{ color: "#555", fontSize: 14, textDecoration: "none" }}
              >
                Features
              </Typography>
              <Typography
                component="a"
                href="#agents"
                sx={{ color: "#555", fontSize: 14, textDecoration: "none" }}
              >
                For agents
              </Typography>
              <Typography
                component="a"
                href="#faq"
                sx={{ color: "#555", fontSize: 14, textDecoration: "none" }}
              >
                FAQ
              </Typography>
            </Box>
            <IconButton
              aria-label={mode === "dark" ? "Use light mode" : "Use dark mode"}
              onClick={toggle}
              size="small"
            >
              {mode === "dark" ? (
                <LightModeOutlinedIcon fontSize="small" />
              ) : (
                <DarkModeOutlinedIcon fontSize="small" />
              )}
            </IconButton>
            <Button
              component={Link}
              href={entryHref}
              variant="contained"
              sx={{
                bgcolor: "#050505",
                color: "#fff",
                borderRadius: 5,
                px: 2.5,
                textTransform: "none",
                "&:hover": { bgcolor: "#222" },
              }}
            >
              Sign up free
            </Button>
          </Stack>
        </Box>
      </Container>

      <Container maxWidth="lg">
        <Box component="main">
          <Box
            sx={{
              bgcolor: "#050505",
              color: "#fff",
              px: { xs: 3, sm: 5, md: 7 },
              py: { xs: 6, md: 9 },
              borderRadius: { xs: 0, md: 1 },
            }}
          >
            <Grid container spacing={{ xs: 5, md: 7 }} alignItems="center">
              <Grid size={{ xs: 12, md: 6 }}>
                <Stack spacing={3}>
                  <Chip
                    label="FREE · OPEN · SELF-HOSTABLE"
                    size="small"
                    sx={{
                      alignSelf: "flex-start",
                      color: "#b7ff19",
                      border: "1px solid #6c9b00",
                      bgcolor: "transparent",
                      fontFamily: "monospace",
                      letterSpacing: 0.8,
                    }}
                  />
                  <Typography
                    component="h1"
                    sx={{
                      fontSize: { xs: 42, sm: 56, md: 66 },
                      lineHeight: 0.99,
                      fontWeight: 800,
                      letterSpacing: -2.5,
                    }}
                  >
                    Study what you don&apos;t know yet.
                  </Typography>
                  <Typography
                    sx={{
                      color: "#b6b6b6",
                      fontSize: { xs: 17, md: 19 },
                      lineHeight: 1.55,
                      maxWidth: 500,
                    }}
                  >
                    AME builds question banks, tracks every answer, and adapts
                    each session to your weakest topics — so no minute of
                    studying is wasted.
                  </Typography>
                  <Stack direction={{ xs: "column", sm: "row" }} spacing={1.5}>
                    <Button
                      component={Link}
                      href={entryHref}
                      variant="contained"
                      endIcon={<ArrowForwardIcon />}
                      sx={{
                        alignSelf: "flex-start",
                        bgcolor: "#1261e9",
                        color: "#fff",
                        borderRadius: 5,
                        px: 2.75,
                        py: 1.25,
                        textTransform: "none",
                        fontWeight: 700,
                        "&:hover": { bgcolor: "#0d4fbe" },
                      }}
                    >
                      Sign up free
                    </Button>
                    <Button
                      component={Link}
                      href={entryHref}
                      variant="outlined"
                      sx={{
                        alignSelf: "flex-start",
                        color: "#fff",
                        borderColor: "#666",
                        borderRadius: 5,
                        px: 2.5,
                        py: 1.25,
                        textTransform: "none",
                        "&:hover": { borderColor: "#fff" },
                      }}
                    >
                      Try a sample quiz
                    </Button>
                  </Stack>
                  <Typography variant="caption" sx={{ color: "#777" }}>
                    No credit card required to get started.
                  </Typography>
                </Stack>
              </Grid>
              <Grid size={{ xs: 12, md: 6 }}>
                <QuizPreview />
              </Grid>
            </Grid>
          </Box>

          <Box
            id="benefits"
            sx={{ py: { xs: 6, md: 9 }, px: { xs: 2, md: 4 } }}
          >
            <Typography
              sx={{
                color: "#1261e9",
                fontFamily: "monospace",
                fontSize: 12,
                letterSpacing: 1.3,
                mb: 1,
              }}
            >
              WHY AME
            </Typography>
            <Typography
              component="h2"
              sx={{
                fontSize: { xs: 29, md: 38 },
                lineHeight: 1.15,
                fontWeight: 800,
                maxWidth: 700,
                mb: 4,
              }}
            >
              Everything between &quot;I should study&quot; and &quot;I
              passed.&quot;
            </Typography>
            <Grid container spacing={2}>
              {benefits.map((benefit) => (
                <Grid key={benefit.number} size={{ xs: 12, md: 4 }}>
                  <Card
                    variant="outlined"
                    sx={{
                      height: "100%",
                      p: 2.5,
                      borderColor: "#e0e0e0",
                      borderRadius: 1.5,
                      boxShadow: "0 4px 0 #f0f0f0",
                    }}
                  >
                    <Typography
                      sx={{
                        display: "inline-flex",
                        alignItems: "center",
                        justifyContent: "center",
                        width: 34,
                        height: 34,
                        borderRadius: "50%",
                        bgcolor: "#e8f0ff",
                        color: "#1261e9",
                        fontFamily: "monospace",
                        mb: 2,
                      }}
                    >
                      {benefit.number}
                    </Typography>
                    <Typography
                      component="h3"
                      sx={{ fontSize: 18, fontWeight: 700, mb: 1 }}
                    >
                      {benefit.title}
                    </Typography>
                    <Typography sx={{ color: "#656565", lineHeight: 1.55 }}>
                      {benefit.body}
                    </Typography>
                  </Card>
                </Grid>
              ))}
            </Grid>
          </Box>

          <Box
            id="agents"
            sx={{
              bgcolor: "#f7f7f7",
              px: { xs: 3, md: 5 },
              py: 3,
              display: "flex",
              gap: 3,
              justifyContent: "space-between",
              alignItems: { xs: "flex-start", md: "center" },
              flexDirection: { xs: "column", md: "row" },
            }}
          >
            <Box>
              <Typography sx={{ fontWeight: 700 }}>
                Bring your AI assistant
              </Typography>
              <Typography sx={{ color: "#666", fontSize: 14 }}>
                First-class agent API — your assistant reads your stats and
                builds practice for you.
              </Typography>
            </Box>
            <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
              {["/llms.txt", "/skill.json", "/openapi.yaml"].map((href) => (
                <Button
                  key={href}
                  component={Link}
                  href={href}
                  variant="outlined"
                  size="small"
                  sx={{
                    color: "#555",
                    borderColor: "#d0d0d0",
                    borderRadius: 5,
                    fontFamily: "monospace",
                    textTransform: "none",
                  }}
                >
                  {href}
                </Button>
              ))}
            </Stack>
          </Box>

          <Box id="faq" sx={{ py: { xs: 6, md: 9 }, px: { xs: 2, md: 4 } }}>
            <Grid container spacing={{ xs: 5, md: 8 }}>
              <Grid size={{ xs: 12, md: 7 }}>
                <Typography
                  component="h2"
                  sx={{ fontSize: 30, fontWeight: 800, mb: 2 }}
                >
                  Questions?
                </Typography>
                <Stack divider={<Divider flexItem />}>
                  {faqs.map((faq) => (
                    <Box key={faq.question} sx={{ py: 2 }}>
                      <Typography sx={{ fontWeight: 700, mb: 0.75 }}>
                        {faq.question}
                      </Typography>
                      <Typography sx={{ color: "#666", lineHeight: 1.55 }}>
                        {faq.answer}
                      </Typography>
                    </Box>
                  ))}
                </Stack>
              </Grid>
              <Grid size={{ xs: 12, md: 5 }}>
                <Card
                  variant="outlined"
                  sx={{
                    p: { xs: 3, md: 4 },
                    borderColor: "#e0e0e0",
                    bgcolor: "#fafafa",
                    borderRadius: 2,
                  }}
                >
                  <Typography
                    component="h2"
                    sx={{
                      fontSize: 27,
                      lineHeight: 1.1,
                      fontWeight: 800,
                      mb: 1.5,
                    }}
                  >
                    Your next exam is already easier.
                  </Typography>
                  <Typography sx={{ color: "#666", lineHeight: 1.5, mb: 2.5 }}>
                    Start with a few questions and turn your weakest topics into
                    a focused practice session.
                  </Typography>
                  <Button
                    component={Link}
                    href={entryHref}
                    variant="contained"
                    sx={{
                      bgcolor: "#1261e9",
                      borderRadius: 5,
                      textTransform: "none",
                      fontWeight: 700,
                    }}
                  >
                    Sign up free
                  </Button>
                </Card>
              </Grid>
            </Grid>
          </Box>
        </Box>
      </Container>

      <Box
        component="footer"
        sx={{ borderTop: "1px solid #e5e5e5", px: 3, py: 3 }}
      >
        <Container
          maxWidth="lg"
          sx={{
            display: "flex",
            justifyContent: "space-between",
            gap: 2,
            color: "#999",
            fontSize: 13,
            flexWrap: "wrap",
          }}
        >
          <span>© 2026 AME</span>
          <Stack direction="row" spacing={2}>
            <Link
              href="/llms.txt"
              style={{ color: "inherit", textDecoration: "none" }}
            >
              Docs
            </Link>
            <a
              href="https://github.com/mikafans/ame"
              style={{ color: "inherit", textDecoration: "none" }}
            >
              GitHub
            </a>
            <Link
              href="/llms.txt"
              style={{ color: "inherit", textDecoration: "none" }}
            >
              Self-hosting
            </Link>
          </Stack>
        </Container>
      </Box>
    </Box>
  );
}
