import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogContent,
  DialogTitle,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Snackbar,
  Tab,
  Tabs,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import {
  AttachFile as AttachFileIcon,
  AutoFixHigh as AutoFixHighIcon,
  Close as CloseIcon,
  FolderOpen as FolderOpenIcon,
  HelpOutline as HelpOutlineIcon,
} from "@mui/icons-material";
import type { BoardSettings, UpdateBoardSettingsRequest } from "../../types/kanban";
import { api } from "../../services/api";

interface BoardSettingsDialogProps {
  open: boolean;
  boardId: string;
  boardName: string;
  onClose: () => void;
}

type EditableBoardSettings = Required<UpdateBoardSettingsRequest>;

const EMPTY_SETTINGS: EditableBoardSettings = {
  ai_concurrency: "1",
  codebase_path: "",
  github_repo: "",
  context_markdown: "",
  document_links: "",
  variables: "",
  tech_stack: "",
  communication_patterns: "",
  environments: "",
  code_conventions: "",
  testing_requirements: "",
  api_conventions: "",
  infrastructure: "",
};

const toEditableSettings = (settings: BoardSettings): EditableBoardSettings => ({
  ai_concurrency:
    typeof settings.ai_concurrency === "number" && settings.ai_concurrency >= 0 && settings.ai_concurrency <= 10
      ? String(settings.ai_concurrency)
      : "1",
  codebase_path: settings.codebase_path || "",
  github_repo: settings.github_repo || "",
  context_markdown: settings.context_markdown || "",
  document_links: settings.document_links || "",
  variables: settings.variables || "",
  tech_stack: settings.tech_stack || "",
  communication_patterns: settings.communication_patterns || "",
  environments: settings.environments || "",
  code_conventions: settings.code_conventions || "",
  testing_requirements: settings.testing_requirements || "",
  api_conventions: settings.api_conventions || "",
  infrastructure: settings.infrastructure || "",
});

const FieldLabel: React.FC<{ label: string; tooltip: string }> = ({ label, tooltip }) => (
  <Box sx={{ display: "flex", alignItems: "center", gap: 0.5, mb: 0.5 }}>
    <Typography variant="subtitle2">{label}</Typography>
    <Tooltip title={tooltip} arrow placement="right">
      <HelpOutlineIcon sx={{ fontSize: 16, color: "text.secondary", cursor: "help" }} />
    </Tooltip>
  </Box>
);

export const BoardSettingsDialog: React.FC<BoardSettingsDialogProps> = ({
  open,
  boardId,
  boardName,
  onClose,
}) => {
  const [tab, setTab] = useState(0);
  const [loading, setLoading] = useState(false);
  const [settings, setSettings] = useState<EditableBoardSettings>(EMPTY_SETTINGS);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [snackbarMessage, setSnackbarMessage] = useState<string | null>(null);

  const loadedSettingsRef = useRef<EditableBoardSettings | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }

    let cancelled = false;

    const loadSettings = async () => {
      try {
        setLoading(true);
        setSaveState("idle");
        const response = await api.getBoardSettings(boardId);
        if (cancelled) {
          return;
        }

        const mapped = toEditableSettings(response);
        loadedSettingsRef.current = mapped;
        setSettings(mapped);
      } catch {
        if (!cancelled) {
          setSaveState("error");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    loadSettings();

    return () => {
      cancelled = true;
    };
  }, [boardId, open]);

  const settingsHash = useMemo(() => JSON.stringify(settings), [settings]);

  useEffect(() => {
    if (!open || loading || !loadedSettingsRef.current) {
      return;
    }

    const baselineHash = JSON.stringify(loadedSettingsRef.current);
    if (settingsHash === baselineHash) {
      return;
    }

    let cancelled = false;
    const timeout = window.setTimeout(async () => {
      try {
        setSaveState("saving");
        const payload = {
          ...settings,
          ai_concurrency: settings.ai_concurrency ? Number(settings.ai_concurrency) : undefined,
        };
        const updated = await api.updateBoardSettings(boardId, payload);
        if (cancelled) {
          return;
        }

        loadedSettingsRef.current = toEditableSettings(updated);
        setSaveState("saved");
      } catch {
        if (!cancelled) {
          setSaveState("error");
        }
      }
    }, 500);

    return () => {
      cancelled = true;
      window.clearTimeout(timeout);
    };
  }, [boardId, loading, open, settings, settingsHash]);

  const handleAutoDetect = async () => {
    if (!settings.codebase_path.trim()) {
      return;
    }

    try {
      await api.autoDetectBoardSettings(boardId, settings.codebase_path);
      setSnackbarMessage("AI is analyzing your codebase...");
    } catch {
      setSnackbarMessage("Failed to queue auto-detect.");
    }
  };

  const handlePickCodebasePath = async () => {
    try {
      const result = await api.pickDirectory();
      if (result.path) {
        setSettings((prev) => ({ ...prev, codebase_path: result.path || "" }));
      }
    } catch {
      setSnackbarMessage("Could not open folder picker.");
    }
  };

  const handlePickReferenceFiles = async () => {
    try {
      const result = await api.pickFiles();
      if (!result.paths || result.paths.length === 0) {
        return;
      }

      setSettings((prev) => {
        const existing = prev.document_links.trim();
        const next = existing ? `${existing}\n${result.paths.join("\n")}` : result.paths.join("\n");
        return { ...prev, document_links: next };
      });
    } catch {
      setSnackbarMessage("Could not open file picker.");
    }
  };

  const updateField = (key: keyof EditableBoardSettings, value: string | number) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  };

  const saveStatusLabel =
    saveState === "saving"
      ? "Saving..."
      : saveState === "saved"
      ? "Saved"
      : saveState === "error"
      ? "Save failed"
      : "";

  return (
    <>
      <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
        <DialogTitle sx={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <Box>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              <Typography variant="h6">{boardName} Settings</Typography>
              <Tooltip
                title="Everything you fill in here becomes shared context for AI when it works on any card in this board. Think of it as giving AI a project briefing - it reads this before starting any task. Your AI provider (Anthropic, OpenAI, Gemini) automatically caches this context, so it only costs full price once, then up to 90% cheaper on repeat use."
                arrow
                placement="bottom"
              >
                <HelpOutlineIcon sx={{ fontSize: 18, color: "text.secondary", cursor: "help" }} />
              </Tooltip>
            </Box>
            {saveStatusLabel && (
              <Typography variant="caption" color={saveState === "error" ? "error" : "text.secondary"}>
                {saveStatusLabel}
              </Typography>
            )}
          </Box>
          <IconButton onClick={onClose}>
            <CloseIcon />
          </IconButton>
        </DialogTitle>

        <DialogContent dividers>
          {loading ? (
            <Box sx={{ py: 6, display: "flex", justifyContent: "center" }}>
              <CircularProgress size={28} />
            </Box>
          ) : (
            <>
              <Alert severity="info" sx={{ mb: 2 }}>
                Board settings are your shared project briefing for AI. The clearer this is, the more accurate and consistent AI output will be across every card.
              </Alert>

              <Tabs value={tab} onChange={(_, value) => setTab(value)} sx={{ mb: 3 }}>
                <Tab label="General" />
                <Tab label="Technical" />
                <Tab label="Conventions" />
              </Tabs>

              {tab === 0 && (
                <Box sx={{ display: "grid", gap: 2 }}>
                  <Box>
                    <FieldLabel
                      label="AI Concurrency"
                      tooltip="How many AI jobs can run in parallel on this board. Higher values increase throughput but may hit API rate limits."
                    />
                    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                      <FormControl size="small" sx={{ minWidth: 220 }}>
                        <InputLabel id="ai-concurrency-label">AI Concurrency</InputLabel>
                        <Select
                          labelId="ai-concurrency-label"
                          label="AI Concurrency"
                          value={settings.ai_concurrency}
                          onChange={(event) => updateField("ai_concurrency", event.target.value)}
                        >
                          {Array.from({ length: 10 }, (_, index) => {
                            const value = String(index + 1);
                            return (
                              <MenuItem key={value} value={value}>
                                {value}
                              </MenuItem>
                            );
                          })}
                          <MenuItem value="0">Unlimited</MenuItem>
                        </Select>
                      </FormControl>
                      <Tooltip title="Unlimited can trigger API rate limits quickly. Prefer 1-3 unless you have high quotas." arrow>
                        <HelpOutlineIcon sx={{ fontSize: 18, color: "warning.main", cursor: "help" }} />
                      </Tooltip>
                    </Box>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Codebase Path"
                      tooltip="The folder path where your project code lives on the server. AI uses this to know where to look for files and run commands. Example: /home/user/projects/my-saas-app"
                    />
                    <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
                      <TextField
                        fullWidth
                        label="Codebase Path"
                        value={settings.codebase_path}
                        onChange={(event) => updateField("codebase_path", event.target.value)}
                        placeholder="/home/user/projects/my-saas-app"
                        InputLabelProps={{ shrink: true }}
                      />
                      <IconButton onClick={handlePickCodebasePath} sx={{ mt: 1 }} aria-label="Pick codebase directory">
                        <FolderOpenIcon />
                      </IconButton>
                    </Box>
                    <Box sx={{ mt: 1 }}>
                      <Tooltip
                        title="AI will scan your codebase and try to automatically fill in Tech Stack, Communication Patterns, Code Conventions, and other fields based on what it finds in your code."
                        arrow
                      >
                        <span>
                          <Button
                            variant="contained"
                            startIcon={<AutoFixHighIcon />}
                            onClick={handleAutoDetect}
                            disabled={!settings.codebase_path.trim()}
                          >
                            Auto-detect
                          </Button>
                        </span>
                      </Tooltip>
                    </Box>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="GitHub Repository"
                      tooltip="Link to your project's GitHub repository. AI can use this to understand your project structure, check existing issues, and reference documentation. Example: https://github.com/your-org/your-repo"
                    />
                    <TextField
                      fullWidth
                      label="GitHub Repository"
                      value={settings.github_repo}
                      onChange={(event) => updateField("github_repo", event.target.value)}
                      placeholder="https://github.com/org/repo"
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Context (Free-form Notes)"
                      tooltip="Write anything you want AI to always keep in mind when working on this board. This is free-form - write it like you're briefing a new team member. For example: 'Our app has a checkout service that handles payments. Never create new services for payment-related features - use the existing checkout service.' This text is included as-is in every AI prompt."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={6}
                      label="Context (Free-form Notes)"
                      value={settings.context_markdown}
                      onChange={(event) => updateField("context_markdown", event.target.value)}
                      placeholder={
                        "Write project context here...\n\nExample:\n- Our microservice architecture has 12 services\n- The checkout-service handles all payment logic\n- Never create new services for existing functionality"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Reference Documents"
                      tooltip="Files or documents that AI should reference when working on cards. These could be architecture docs, API specs, design documents, or any file that helps AI understand your project. AI will read these before starting work."
                    />
                    <Box sx={{ display: "grid", gap: 1 }}>
                      <TextField
                        fullWidth
                        multiline
                        rows={3}
                        label="Reference Documents"
                        value={settings.document_links}
                        onChange={(event) => updateField("document_links", event.target.value)}
                        placeholder={"One file path or URL per line\n\nExamples:\n/docs/architecture.md\nhttps://wiki.internal/api-guide"}
                        InputLabelProps={{ shrink: true }}
                      />
                      <Box sx={{ display: "flex", justifyContent: "flex-start" }}>
                        <Button variant="outlined" startIcon={<AttachFileIcon />} onClick={handlePickReferenceFiles}>
                          Browse Files
                        </Button>
                      </Box>
                    </Box>
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Environment Variables"
                      tooltip="Key-value pairs that AI should know about your project. These are NOT secrets - don't put passwords here. Use this for URLs, port numbers, service names, or any project constants that AI needs when writing code or configs."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Environment Variables"
                      value={settings.variables}
                      onChange={(event) => updateField("variables", event.target.value)}
                      placeholder={
                        "One per line in KEY=VALUE format\n\nExamples:\nSTAGING_URL=https://staging.example.com\nGRPC_PORT=50051\nDEFAULT_DB=postgresql"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>
                </Box>
              )}

              {tab === 1 && (
                <Box sx={{ display: "grid", gap: 2 }}>
                  <Box>
                    <FieldLabel
                      label="Tech Stack"
                      tooltip="List the programming languages, frameworks, databases, and tools your project uses, including version numbers. AI uses this to write code in the correct language and version, and to suggest compatible libraries."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Tech Stack"
                      value={settings.tech_stack}
                      onChange={(event) => updateField("tech_stack", event.target.value)}
                      placeholder={
                        "Examples:\n- Go 1.22 (backend services)\n- React 18 + TypeScript (frontend)\n- PostgreSQL 16 (main database)\n- Redis 7 (caching)\n- gRPC (service communication)"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Communication Patterns"
                      tooltip="How do your services talk to each other? AI needs this to write correct integration code and avoid creating incompatible communication methods."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Communication Patterns"
                      value={settings.communication_patterns}
                      onChange={(event) => updateField("communication_patterns", event.target.value)}
                      placeholder={
                        "Examples:\n- Internal services: gRPC on port 50051\n- External APIs: REST with JSON\n- Async events: Kafka topics\n- Service discovery: Consul"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Environments"
                      tooltip="List your deployment environments and their details. AI uses this to write correct configuration, deployment scripts, and environment-specific code."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Environments"
                      value={settings.environments}
                      onChange={(event) => updateField("environments", event.target.value)}
                      placeholder={
                        "Examples:\n- Dev: localhost:8080, local PostgreSQL\n- Staging: staging.example.com, shared DB\n- Production: api.example.com, replicated DB cluster"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Infrastructure"
                      tooltip="Describe your deployment infrastructure, CI/CD pipeline, and DevOps setup. AI uses this to write correct Dockerfiles, CI configs, and deployment scripts."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Infrastructure"
                      value={settings.infrastructure}
                      onChange={(event) => updateField("infrastructure", event.target.value)}
                      placeholder={
                        "Examples:\n- Kubernetes on AWS EKS\n- GitHub Actions for CI/CD\n- ArgoCD for GitOps deployment\n- Docker with multi-stage builds"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>
                </Box>
              )}

              {tab === 2 && (
                <Box sx={{ display: "grid", gap: 2 }}>
                  <Box>
                    <FieldLabel
                      label="Code Conventions"
                      tooltip="Your team's coding standards and rules. AI will follow these conventions when writing code, so the output matches your existing codebase style."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Code Conventions"
                      value={settings.code_conventions}
                      onChange={(event) => updateField("code_conventions", event.target.value)}
                      placeholder={
                        "Examples:\n- Go standard project layout\n- ESLint + Prettier for frontend\n- snake_case for DB columns, camelCase for Go/TS\n- All errors must be wrapped with context"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="Testing Requirements"
                      tooltip="Your team's testing standards. AI will write tests that meet these requirements and follow your existing test patterns."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="Testing Requirements"
                      value={settings.testing_requirements}
                      onChange={(event) => updateField("testing_requirements", event.target.value)}
                      placeholder={
                        "Examples:\n- Minimum 80% code coverage\n- Integration tests with testcontainers\n- Jest + React Testing Library for frontend\n- Table-driven tests for Go"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>

                  <Box>
                    <FieldLabel
                      label="API Conventions"
                      tooltip="Your API design rules and standards. AI will follow these when creating new endpoints or modifying existing ones."
                    />
                    <TextField
                      fullWidth
                      multiline
                      rows={3}
                      label="API Conventions"
                      value={settings.api_conventions}
                      onChange={(event) => updateField("api_conventions", event.target.value)}
                      placeholder={
                        "Examples:\n- RESTful with JSON:API envelope\n- JWT auth via Authorization header\n- Standard error format: { error: { code, message } }\n- API versioning via URL path (/v1/, /v2/)"
                      }
                      InputLabelProps={{ shrink: true }}
                    />
                  </Box>
                </Box>
              )}
            </>
          )}
        </DialogContent>
      </Dialog>

      <Snackbar
        open={Boolean(snackbarMessage)}
        autoHideDuration={3000}
        onClose={() => setSnackbarMessage(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        <Alert severity="info" onClose={() => setSnackbarMessage(null)}>
          {snackbarMessage}
        </Alert>
      </Snackbar>
    </>
  );
};

export default BoardSettingsDialog;
